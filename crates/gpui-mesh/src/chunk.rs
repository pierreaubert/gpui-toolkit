//! Boundary-surface extraction and bounded-size chunking for large datasets.

// Rust guideline compliant 2026-02-21

use crate::dataset::{CellType, DataSet};
use std::collections::HashMap;
use std::num::NonZeroU8;

#[derive(Clone, Copy)]
struct Face {
    nodes: [u32; 4],
    // The nonzero length also keeps Option<Face> the same size as Face.
    len: NonZeroU8,
}

impl Face {
    fn new(nodes: &[u32], indices: &[usize; 4]) -> Option<Self> {
        let mut polygon = [0; 4];
        let mut len = 0;
        for &index in indices {
            let point = nodes[index];
            if len == 0 || polygon[len - 1] != point {
                polygon[len] = point;
                len += 1;
            }
        }
        (len >= 3).then(|| Self {
            nodes: polygon,
            len: NonZeroU8::new(len as u8).expect("nonempty face"),
        })
    }

    fn key(self) -> [u32; 5] {
        let len = usize::from(self.len.get());
        let mut key = [0; 5];
        key[0] = len as u32;
        key[1..=len].copy_from_slice(&self.nodes[..len]);
        key[1..=len].sort_unstable();
        key
    }
}

/// One bounded chunk of extracted surface triangles.
///
/// Indices are local to the chunk (`u32`), positions keep the dataset's
/// `f64` coordinates, and `point_ids` maps each local vertex back to the
/// dataset point index so field lookups and picking stay exact. A chunk
/// never exceeds the caller's triangle budget, which is what lets a
/// multi-million-cell dataset upload to a GPU buffer (or a wasm heap)
/// in pieces instead of as one monolithic allocation.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceChunk {
    /// Dataset point index for each local vertex, in first-use order.
    pub point_ids: Vec<u32>,
    /// Local triangle indices, three per triangle.
    pub indices: Vec<u32>,
}

/// Face tables for volume cells, as local node-index quads/triangles.
/// Hexahedron faces follow VTK node ordering.
fn volume_faces(kind: CellType) -> &'static [[usize; 4]] {
    match kind {
        CellType::Tetra => &[[0, 2, 1, 1], [0, 1, 3, 3], [1, 2, 3, 3], [2, 0, 3, 3]],
        CellType::Hexahedron => &[
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ],
        CellType::Wedge => &[
            [0, 2, 1, 1],
            [3, 4, 5, 5],
            [0, 1, 4, 3],
            [1, 2, 5, 4],
            [2, 0, 3, 5],
        ],
        CellType::Pyramid => &[
            [0, 3, 2, 1],
            [0, 1, 4, 4],
            [1, 2, 4, 4],
            [2, 3, 4, 4],
            [3, 0, 4, 4],
        ],
        _ => &[],
    }
}

/// Extracts boundary triangles as dataset point-index triples.
///
/// Surface cells contribute directly (quads split on the 0-2 diagonal).
/// Volume cells contribute only faces owned by a single cell, so interior
/// faces between two volume cells are culled. Face identity is the sorted
/// node set of the whole face polygon, so the culling does not depend on
/// which diagonal each owner would have chosen; surviving faces are
/// triangulated from the owning cell's node order.
/// Boundary faces are emitted in canonical node-set order for repeatability.
#[must_use]
pub fn boundary_triangles(dataset: &DataSet) -> Vec<[u32; 3]> {
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    // Sorted-node face key -> owning polygon (None once a second owner appears).
    let mut faces: HashMap<[u32; 5], Option<Face>> = HashMap::new();
    for cell in 0..dataset.cell_count() {
        let Some(nodes) = dataset.cells.cell(cell) else {
            continue;
        };
        let kind = dataset.cells.types[cell];
        match kind {
            CellType::Triangle => {
                triangles.push([nodes[0], nodes[1], nodes[2]]);
            }
            CellType::Quad => {
                triangles.push([nodes[0], nodes[1], nodes[2]]);
                triangles.push([nodes[0], nodes[2], nodes[3]]);
            }
            CellType::Polygon => {
                // Fan triangulation around the first node.
                for i in 1..nodes.len() - 1 {
                    triangles.push([nodes[0], nodes[i], nodes[i + 1]]);
                }
            }
            CellType::Tetra | CellType::Hexahedron | CellType::Wedge | CellType::Pyramid => {
                for face in volume_faces(kind) {
                    // Triangular faces are stored with a repeated last node.
                    let Some(polygon) = Face::new(nodes, face) else {
                        continue;
                    };
                    match faces.entry(polygon.key()) {
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            entry.insert(Some(polygon));
                        }
                        std::collections::hash_map::Entry::Occupied(mut entry) => {
                            entry.insert(None);
                        }
                    }
                }
            }
            CellType::Vertex | CellType::Line => {}
        }
    }
    let mut boundary: Vec<_> = faces
        .into_iter()
        .filter_map(|(key, polygon)| polygon.map(|polygon| (key, polygon)))
        .collect();
    boundary.sort_unstable_by_key(|(key, _)| *key);
    for (_, polygon) in boundary {
        let nodes = polygon.nodes;
        triangles.push([nodes[0], nodes[1], nodes[2]]);
        if polygon.len.get() == 4 {
            triangles.push([nodes[0], nodes[2], nodes[3]]);
        }
    }
    triangles
}

/// Splits the boundary surface into chunks of at most `max_triangles` each.
///
/// # Panics
///
/// Panics when `max_triangles` is zero, which can never produce a chunk.
#[must_use]
pub fn surface_chunks(dataset: &DataSet, max_triangles: usize) -> Vec<SurfaceChunk> {
    assert!(max_triangles > 0, "max_triangles must be at least 1");
    let triangles = boundary_triangles(dataset);
    let mut chunks = Vec::new();
    for group in triangles.chunks(max_triangles) {
        let mut local: HashMap<u32, u32> = HashMap::new();
        let mut point_ids: Vec<u32> = Vec::new();
        let mut indices: Vec<u32> = Vec::with_capacity(group.len() * 3);
        for tri in group {
            for &point in tri {
                let id = *local.entry(point).or_insert_with(|| {
                    let id = point_ids.len() as u32;
                    point_ids.push(point);
                    id
                });
                indices.push(id);
            }
        }
        chunks.push(SurfaceChunk { point_ids, indices });
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::Cells;

    #[test]
    fn single_hexahedron_has_twelve_boundary_triangles() {
        let ds = DataSet::structured_grid("g", [2, 2, 2], [0.0; 3], [1.0; 3]).unwrap();
        assert_eq!(boundary_triangles(&ds).len(), 12);
    }

    #[test]
    fn two_hexahedra_cull_the_shared_face() {
        let ds = DataSet::structured_grid("g", [3, 2, 2], [0.0; 3], [1.0; 3]).unwrap();
        // Boundary quads: 2*6 - 2 for the culled shared face = 10 quads = 20 triangles.
        assert_eq!(boundary_triangles(&ds).len(), 20);
    }

    #[test]
    fn chunks_respect_budget_and_round_trip_points() {
        let ds = DataSet::structured_grid("g", [5, 5, 5], [0.0; 3], [1.0; 3]).unwrap();
        let total = boundary_triangles(&ds).len();
        let chunks = surface_chunks(&ds, 10);
        assert!(chunks.iter().all(|c| c.indices.len() / 3 <= 10));
        assert_eq!(
            chunks.iter().map(|c| c.indices.len() / 3).sum::<usize>(),
            total
        );
        for chunk in &chunks {
            for &local in &chunk.indices {
                assert!((local as usize) < chunk.point_ids.len());
            }
        }
        let _ = Cells::triangles(&[[0, 1, 2]]);
    }

    #[test]
    fn extraction_is_deterministic_and_tetra_shared_faces_are_culled() {
        let dataset = DataSet::structured_grid("g", [8; 3], [0.0; 3], [1.0; 3]).unwrap();
        let expected = boundary_triangles(&dataset);
        for _ in 0..4 {
            assert_eq!(boundary_triangles(&dataset), expected);
        }
        let tetrahedra = DataSet::new(
            "tets",
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, -1.0],
            ],
            Cells::from_cells(&[
                (CellType::Tetra, &[0, 1, 2, 3]),
                (CellType::Tetra, &[0, 2, 1, 4]),
            ]),
        );
        tetrahedra.validate().unwrap();
        assert_eq!(boundary_triangles(&tetrahedra).len(), 6);
    }

    #[test]
    fn face_arity_is_part_of_identity_and_optional_storage_stays_compact() {
        let triangle = Face::new(&[0, 1, 2], &[0, 1, 2, 2]).unwrap();
        let quad = Face::new(&[0, 1, 2, u32::MAX], &[0, 1, 2, 3]).unwrap();
        assert_ne!(triangle.key(), quad.key());
        assert_eq!(
            std::mem::size_of::<Face>(),
            std::mem::size_of::<Option<Face>>()
        );
    }
}
