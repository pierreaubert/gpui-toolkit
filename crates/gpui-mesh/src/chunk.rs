//! Boundary-surface extraction and bounded-size chunking for large datasets.

// Rust guideline compliant 2026-02-21

use crate::dataset::{CellType, DataSet, ValidatedDataSet};
use std::collections::HashMap;
use std::num::{NonZeroU8, NonZeroUsize};

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

#[derive(Clone, Copy)]
struct FaceOwner {
    // One-based so Option<FaceOwner> needs no extra discriminant.
    cell: NonZeroUsize,
    face: u8,
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
    let mut triangles = Vec::new();
    let result: Result<(), std::convert::Infallible> = visit_boundary(dataset, |_, triangle| {
        triangles.push(triangle);
        Ok(())
    });
    match result {
        Ok(()) => triangles,
        Err(never) => match never {},
    }
}

fn visit_boundary<E>(
    dataset: &DataSet,
    mut visit: impl FnMut(usize, [u32; 3]) -> Result<(), E>,
) -> Result<(), E> {
    if let Some(grid) = crate::structured_boundary::CanonicalHexGrid::detect(dataset) {
        return grid.visit(|cell, face| visit_face(dataset, cell, usize::from(face), &mut visit));
    }
    visit_general_boundary(dataset, visit)
}

fn visit_general_boundary<E>(
    dataset: &DataSet,
    mut visit: impl FnMut(usize, [u32; 3]) -> Result<(), E>,
) -> Result<(), E> {
    // Keep the source owner rather than copying its polygon. Reconstruct only
    // surviving boundary faces; the map entry is no larger than the old one.
    let mut faces: HashMap<[u32; 5], Option<FaceOwner>> = HashMap::new();
    for cell in 0..dataset.cell_count() {
        let Some(nodes) = dataset.cells.cell(cell) else {
            continue;
        };
        let kind = dataset.cells.types[cell];
        match kind {
            CellType::Triangle => {
                visit(cell, [nodes[0], nodes[1], nodes[2]])?;
            }
            CellType::Quad => {
                visit(cell, [nodes[0], nodes[1], nodes[2]])?;
                visit(cell, [nodes[0], nodes[2], nodes[3]])?;
            }
            CellType::Polygon => {
                // Fan triangulation around the first node.
                for i in 1..nodes.len() - 1 {
                    visit(cell, [nodes[0], nodes[i], nodes[i + 1]])?;
                }
            }
            CellType::Tetra | CellType::Hexahedron | CellType::Wedge | CellType::Pyramid => {
                for (face_index, face) in volume_faces(kind).iter().enumerate() {
                    // Triangular faces are stored with a repeated last node.
                    let Some(polygon) = Face::new(nodes, face) else {
                        continue;
                    };
                    match faces.entry(polygon.key()) {
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            entry.insert(Some(FaceOwner {
                                cell: NonZeroUsize::new(cell + 1).expect("one-based cell index"),
                                face: face_index as u8,
                            }));
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
        .filter_map(|(key, owner)| owner.map(|owner| (key, owner)))
        .collect();
    boundary.sort_unstable_by_key(|(key, _)| *key);
    for (_, owner) in boundary {
        visit_face(
            dataset,
            owner.cell.get() - 1,
            usize::from(owner.face),
            &mut visit,
        )?;
    }
    Ok(())
}

fn visit_face<E>(
    dataset: &DataSet,
    cell: usize,
    face: usize,
    visit: &mut impl FnMut(usize, [u32; 3]) -> Result<(), E>,
) -> Result<(), E> {
    let nodes = dataset.cells.cell(cell).expect("boundary owner cell");
    let face = &volume_faces(dataset.cells.types[cell])[face];
    let polygon = Face::new(nodes, face).expect("surviving nondegenerate face");
    let nodes = polygon.nodes;
    visit(cell, [nodes[0], nodes[1], nodes[2]])?;
    if polygon.len.get() == 4 {
        visit(cell, [nodes[0], nodes[2], nodes[3]])?;
    }
    Ok(())
}

/// Visits bounded surface chunks with each triangle's source cell index.
///
/// `cell_ids[i]` owns the triangle at `chunk.indices[i * 3..i * 3 + 3]`.
/// The cell IDs are borrowed only during the callback. Surface ordering and
/// volume-face culling match [`boundary_triangles`]. No full triangle array is
/// constructed. Verified canonical hexahedral grids use constant-space face
/// cursors; other volume topologies still use a dataset-wide face map.
/// The triangle budget is capped to keep every chunk-local index representable.
/// The validated view keeps source geometry immutable throughout traversal.
///
/// # Errors
///
/// Returns the first callback error and stops visiting immediately.
///
/// # Panics
///
/// Panics when `max_triangles` is zero.
pub fn try_for_each_surface_chunk<E>(
    dataset: ValidatedDataSet<'_>,
    max_triangles: usize,
    mut visit: impl FnMut(SurfaceChunk, &[usize]) -> Result<(), E>,
) -> Result<(), E> {
    assert!(max_triangles > 0, "max_triangles must be at least 1");
    let dataset = dataset.as_dataset();
    let max_triangles = max_triangles.min(u32::MAX as usize / 3);
    let mut local = HashMap::<u32, u32>::new();
    let mut point_ids = Vec::new();
    let mut indices = Vec::new();
    let mut cell_ids = Vec::new();
    visit_boundary(dataset, |cell, triangle| {
        for point in triangle {
            let id = *local.entry(point).or_insert_with(|| {
                let id = point_ids.len() as u32;
                point_ids.push(point);
                id
            });
            indices.push(id);
        }
        cell_ids.push(cell);
        if cell_ids.len() == max_triangles {
            visit(
                SurfaceChunk {
                    point_ids: std::mem::take(&mut point_ids),
                    indices: std::mem::take(&mut indices),
                },
                &cell_ids,
            )?;
            cell_ids.clear();
            local.clear();
        }
        Ok(())
    })?;
    if !cell_ids.is_empty() {
        visit(SurfaceChunk { point_ids, indices }, &cell_ids)?;
    }
    Ok(())
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
    fn canonical_grids_match_general_face_order_owners_and_winding() {
        for nx in 2..=6 {
            for ny in 2..=6 {
                for nz in 2..=6 {
                    let dims = [nx, ny, nz];
                    let dataset = DataSet::structured_grid(
                        "grid",
                        dims,
                        [1e12, -2.0, 0.25],
                        [-0.5, 1.25, 0.75],
                    )
                    .unwrap();
                    assert!(
                        crate::structured_boundary::CanonicalHexGrid::detect(&dataset).is_some()
                    );
                    let mut expected = Vec::new();
                    visit_general_boundary(&dataset, |cell, tri| {
                        expected.push((cell, tri));
                        Ok::<(), ()>(())
                    })
                    .unwrap();
                    let mut actual = Vec::new();
                    visit_boundary(&dataset, |cell, tri| {
                        actual.push((cell, tri));
                        Ok::<(), ()>(())
                    })
                    .unwrap();
                    assert_eq!(
                        actual.len(),
                        4 * ((nx - 1) * (ny - 1) + (nx - 1) * (nz - 1) + (ny - 1) * (nz - 1))
                    );
                    assert_eq!(
                        actual, expected,
                        "owner, order or winding differs for {dims:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn edited_or_reordered_grids_use_the_general_boundary_contract() {
        let original = DataSet::structured_grid("grid", [3, 4, 5], [0.0; 3], [1.0; 3]).unwrap();
        for edit in 0..4 {
            let mut dataset = original.clone();
            let nodes = std::sync::Arc::make_mut(&mut dataset.cells.connectivity);
            match edit {
                0 => {
                    let first: [u32; 8] = nodes[..8].try_into().unwrap();
                    let last = nodes.len() - 8;
                    nodes.copy_within(last..last + 8, 0);
                    nodes[last..last + 8].copy_from_slice(&first);
                }
                1 => {
                    let last = nodes.len() - 8;
                    nodes[last..last + 4].rotate_left(1);
                    nodes[last + 4..].rotate_left(1);
                }
                2 => {
                    nodes[nodes.len() - 1] = nodes[nodes.len() - 2];
                }
                _ => {
                    let mut positions = dataset.positions.to_vec();
                    positions.push([100.0; 3]);
                    dataset.positions = positions.into();
                }
            }
            dataset.validate().unwrap();
            assert!(crate::structured_boundary::CanonicalHexGrid::detect(&dataset).is_none());
            let mut expected = Vec::new();
            visit_general_boundary(&dataset, |cell, tri| {
                expected.push((cell, tri));
                Ok::<(), ()>(())
            })
            .unwrap();
            let mut actual = Vec::new();
            visit_boundary(&dataset, |cell, tri| {
                actual.push((cell, tri));
                Ok::<(), ()>(())
            })
            .unwrap();
            assert_eq!(actual, expected, "fallback differs for edit {edit}");
        }
    }

    #[test]
    fn thin_grids_preserve_canonical_order_at_long_row_and_plane_boundaries() {
        for dims in [
            [2, 2, 257],
            [2, 257, 2],
            [257, 2, 2],
            [2, 3, 257],
            [31, 2, 7],
            [2, 31, 7],
        ] {
            let dataset = DataSet::structured_grid("thin", dims, [0.0; 3], [1.0; 3]).unwrap();
            let mut expected = Vec::new();
            visit_general_boundary(&dataset, |cell, tri| {
                expected.push((cell, tri));
                Ok::<(), ()>(())
            })
            .unwrap();
            let mut actual = Vec::new();
            visit_boundary(&dataset, |cell, tri| {
                actual.push((cell, tri));
                Ok::<(), ()>(())
            })
            .unwrap();
            assert_eq!(actual, expected, "thin grid differs for {dims:?}");
        }
    }

    #[test]
    fn structured_boundary_visit_stops_on_the_first_callback_error() {
        let dataset = DataSet::structured_grid("grid", [4, 5, 6], [0.0; 3], [1.0; 3]).unwrap();
        let mut visits = 0;
        let result = visit_boundary(&dataset, |_, _| {
            visits += 1;
            Err("stop")
        });
        assert_eq!(result, Err("stop"));
        assert_eq!(visits, 1);
    }

    #[test]
    fn owned_chunks_keep_exact_surface_and_volume_cell_ids() {
        let volume = DataSet::structured_grid("pair", [3, 2, 2], [0.0; 3], [1.0; 3]).unwrap();
        for budget in [1, 3, usize::MAX] {
            let mut got = Vec::new();
            let mut counts = [0usize; 2];
            try_for_each_surface_chunk(volume.validated().unwrap(), budget, |chunk, owners| {
                assert_eq!(chunk.indices.len(), owners.len() * 3);
                assert!(owners.len() <= budget);
                for (tri, &cell) in chunk.indices.as_chunks::<3>().0.iter().zip(owners) {
                    let global = tri.map(|i| chunk.point_ids[i as usize]);
                    let xs = global.map(|i| volume.positions[i as usize][0]);
                    assert!(!xs.iter().all(|&x| x == 1.0), "interior face leaked");
                    let expected = usize::from(xs.iter().all(|&x| x >= 1.0));
                    assert_eq!(cell, expected);
                    counts[cell] += 1;
                    got.push(global);
                }
                Ok::<(), ()>(())
            })
            .unwrap();
            assert_eq!(got, boundary_triangles(&volume));
            assert_eq!(counts, [10, 10]);
        }
        let surface = DataSet::new(
            "mixed",
            vec![[0.0; 3]; 5],
            Cells::from_cells(&[
                (CellType::Line, &[0, 1]),
                (CellType::Vertex, &[0]),
                (CellType::Triangle, &[0, 1, 2]),
                (CellType::Quad, &[0, 1, 2, 3]),
                (CellType::Polygon, &[0, 1, 2, 3, 4]),
            ]),
        );
        let mut cells = Vec::new();
        try_for_each_surface_chunk(surface.validated().unwrap(), 2, |_, owners| {
            cells.extend_from_slice(owners);
            Ok::<(), ()>(())
        })
        .unwrap();
        assert_eq!(cells, [2, 3, 3, 4, 4, 4]);
    }

    #[test]
    fn every_volume_face_retains_its_owner_without_larger_map_entries() {
        let dataset = DataSet::new(
            "types",
            vec![[0.0; 3]; 23],
            Cells::from_cells(&[
                (CellType::Tetra, &[0, 1, 2, 3]),
                (CellType::Wedge, &[4, 5, 6, 7, 8, 9]),
                (CellType::Pyramid, &[10, 11, 12, 13, 14]),
                (CellType::Hexahedron, &[15, 16, 17, 18, 19, 20, 21, 22]),
            ]),
        );
        dataset.validate().unwrap();
        let mut counts = [0usize; 4];
        let mut got = Vec::new();
        try_for_each_surface_chunk(dataset.validated().unwrap(), 7, |chunk, owners| {
            for (tri, &cell) in chunk.indices.as_chunks::<3>().0.iter().zip(owners) {
                let global = tri.map(|i| chunk.point_ids[i as usize]);
                assert!(
                    global
                        .iter()
                        .all(|id| dataset.cells.cell(cell).unwrap().contains(id))
                );
                counts[cell] += 1;
                got.push(global);
            }
            Ok::<(), ()>(())
        })
        .unwrap();
        assert_eq!(counts, [4, 8, 6, 12]);
        assert_eq!(got, boundary_triangles(&dataset));
        assert!(
            size_of::<([u32; 5], Option<FaceOwner>)>() <= size_of::<([u32; 5], Option<Face>)>()
        );
    }

    #[test]
    fn owned_chunk_callback_error_stops_visiting() {
        let dataset = DataSet::structured_grid("pair", [3, 2, 2], [0.0; 3], [1.0; 3]).unwrap();
        let mut calls = 0;
        let result = try_for_each_surface_chunk(dataset.validated().unwrap(), 1, |_, _| {
            calls += 1;
            Err("stop")
        });
        assert_eq!(result, Err("stop"));
        assert_eq!(calls, 1);
    }

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
