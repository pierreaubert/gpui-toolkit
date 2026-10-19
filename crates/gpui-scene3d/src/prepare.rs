//! Chunked GPU-ready buffer preparation for large datasets.

// Rust guideline compliant 2026-02-21

use crate::SceneError;
use crate::scene::Actor;
use gpui_mesh::Association;

/// Default triangle budget per GPU chunk (about 12 MB of indices).
pub const DEFAULT_MAX_TRIANGLES: usize = 1_000_000;

/// One GPU-ready chunk: rebased `f32` positions, normals, scalars, indices.
///
/// Positions are rebased on the dataset centre before the `f64` to `f32`
/// conversion, so large world coordinates keep sub-millimetre precision in
/// the chunk while the host translates the chunk by `origin` in the model
/// matrix. Indices are chunk-local `u32`.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuChunk {
    /// Centre the positions were rebased on, in dataset coordinates.
    pub origin: [f64; 3],
    /// Rebased positions, three per vertex.
    pub positions: Vec<f32>,
    /// Smooth vertex normals, three per vertex.
    pub normals: Vec<f32>,
    /// Scalar per vertex, aligned with `positions` (zeros when uncoloured).
    pub scalars: Vec<f32>,
    /// Chunk-local triangle indices.
    pub indices: Vec<u32>,
    /// Canonical unique wireframe edge pairs derived from `indices`, when requested.
    pub edges: Vec<u32>,
}

impl GpuChunk {
    /// Total GPU byte size of the chunk buffers.
    #[must_use]
    pub fn byte_len(&self) -> u64 {
        ((self.positions.len() + self.normals.len() + self.scalars.len()) * 4
            + (self.indices.len() + self.edges.len()) * 4) as u64
    }

    /// Vertex count in the chunk.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }

    /// Triangle count in the chunk.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Prepares every visible surface chunk of an actor for GPU upload.
///
/// The boundary surface is extracted once per chunk budget, positions are
/// rebased and converted, smooth normals are accumulated, and scalars are
/// gathered per vertex. Cell values remain constant on their owner's triangles;
/// vertices split only where converted scalar values disagree. Smooth geometric
/// normals are preserved across those splits. Point scalars are gathered exactly.
///
/// # Errors
///
/// Returns a mesh error for invalid geometry or field storage, or
/// [`SceneError::FieldNotFound`] for a missing or nonscalar coloring field.
pub fn prepare_chunks(actor: &Actor, max_triangles: usize) -> Result<Vec<GpuChunk>, SceneError> {
    prepare_chunks_with_bounds(actor, max_triangles).map(|(_, chunks)| chunks)
}

pub(crate) fn prepare_chunks_with_bounds(
    actor: &Actor,
    max_triangles: usize,
) -> Result<(Option<gpui_mesh::Bounds>, Vec<GpuChunk>), SceneError> {
    let checked = actor.dataset.validated()?;
    let dataset = checked.as_dataset();
    let bounds = dataset.bounds();
    let origin = bounds.map_or([0.0; 3], |b| b.center());
    let scalar_lookup: Option<(Association, &[f64])> = if let Some(pair) = actor.scalars() {
        Some(pair)
    } else {
        if actor.field.is_some() {
            let name = actor
                .field
                .as_ref()
                .map(|(_, n)| n.clone())
                .unwrap_or_default();
            return Err(SceneError::FieldNotFound(name));
        }
        None
    };
    if dataset.cells.types.iter().all(|kind| kind.dimension() < 2) {
        return Ok((bounds, Vec::new()));
    }
    let valid = actor
        .field
        .as_ref()
        .and_then(|(association, name)| dataset.field(*association, name))
        .and_then(|field| field.valid.as_deref());
    if let Some((Association::Cell, values)) = scalar_lookup {
        let mut out = Vec::new();
        // Cell splits may duplicate a global edge within one chunk as well as
        // across chunks. The first emitted valid owner supplies its wire alpha.
        let mut seen = actor.wireframe.then(std::collections::HashSet::new);
        gpui_mesh::try_for_each_surface_chunk(
            checked,
            max_triangles.clamp(1, DEFAULT_MAX_TRIANGLES),
            |chunk, owners| {
                if let Some(chunk) =
                    cell_chunk(dataset, origin, chunk, owners, values, valid, seen.as_mut())
                {
                    out.push(chunk);
                }
                Ok::<(), SceneError>(())
            },
        )?;
        return Ok((bounds, out));
    }
    let point_scalars = scalar_lookup.map(|(_, values)| values);

    let chunks = gpui_mesh::surface_chunks(dataset, max_triangles.max(1));
    let mut seen_edges = (actor.wireframe && chunks.len() > 1).then(std::collections::HashSet::new);
    let mut out = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let vertex_count = chunk.point_ids.len();
        let mut positions = Vec::with_capacity(vertex_count * 3);
        let mut scalars = vec![0.0f32; vertex_count];
        for (local, &point) in chunk.point_ids.iter().enumerate() {
            let p = dataset.positions[point as usize];
            positions.push((p[0] - origin[0]) as f32);
            positions.push((p[1] - origin[1]) as f32);
            positions.push((p[2] - origin[2]) as f32);
            if let Some(values) = &point_scalars {
                let invalid = matches!(scalar_lookup, Some((Association::Point, _)))
                    && valid.is_some_and(|mask| !mask[point as usize]);
                let scalar = values[point as usize] as f32;
                scalars[local] = if invalid || !scalar.is_finite() {
                    f32::NAN
                } else {
                    scalar
                };
            }
        }
        let mut indices = chunk.indices;
        // Reject triangles touching missing samples before either backend.
        // GPU correctness must not depend on NaN interpolation/fast-math rules.
        if scalars.iter().any(|value| !value.is_finite()) {
            let mut write = 0;
            for read in (0..indices.len()).step_by(3) {
                if indices[read..read + 3]
                    .iter()
                    .all(|&index| scalars[index as usize].is_finite())
                {
                    indices.copy_within(read..read + 3, write);
                    write += 3;
                }
            }
            indices.truncate(write);
        }
        let normals = smooth_normals(&positions, &indices);
        let edges = if actor.wireframe {
            wireframe_edges(&indices, &chunk.point_ids, seen_edges.as_mut())
        } else {
            Vec::new()
        };
        out.push(GpuChunk {
            origin,
            positions,
            normals,
            scalars,
            indices,
            edges,
        });
    }
    Ok((bounds, out))
}

fn cell_chunk(
    dataset: &gpui_mesh::DataSet,
    origin: [f64; 3],
    mut chunk: gpui_mesh::SurfaceChunk,
    owners: &[usize],
    values: &[f64],
    valid: Option<&[bool]>,
    seen_edges: Option<&mut std::collections::HashSet<u64>>,
) -> Option<GpuChunk> {
    let valid_cell =
        |cell: usize| valid.is_none_or(|mask| mask[cell]) && (values[cell] as f32).is_finite();
    if !owners.iter().any(|&cell| valid_cell(cell)) {
        return None;
    }
    let mut scalars = vec![f32::NAN; chunk.point_ids.len()];
    for (tri, &cell) in chunk.indices.as_chunks::<3>().0.iter().zip(owners) {
        if valid_cell(cell) {
            for &point in tri {
                let scalar = &mut scalars[point as usize];
                if scalar.is_nan() {
                    *scalar = values[cell] as f32;
                }
            }
        }
    }
    let mut positions = Vec::with_capacity(chunk.point_ids.len() * 3);
    for &point in &chunk.point_ids {
        let p = dataset.positions[point as usize];
        positions.extend((0..3).map(|axis| (p[axis] - origin[axis]) as f32));
    }
    let mut normals = smooth_normals_for(
        &positions,
        chunk
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .zip(owners)
            .filter(|(_, cell)| valid_cell(**cell))
            .map(|(tri, _)| *tri),
    );
    // Reuse the ordinary vertices for their first scalar. Only discontinuities
    // need a map entry and appended position/normal/scalar storage.
    let mut splits = std::collections::HashMap::<(u32, u32), u32>::new();
    let wireframe = seen_edges.is_some();
    let mut write = 0;
    for (read, &cell) in owners.iter().enumerate() {
        if !valid_cell(cell) {
            continue;
        }
        let scalar = values[cell] as f32;
        let tri = [
            chunk.indices[read * 3],
            chunk.indices[read * 3 + 1],
            chunk.indices[read * 3 + 2],
        ];
        for point in tri {
            let local = point as usize;
            let id = if scalars[local].to_bits() == scalar.to_bits() {
                point
            } else {
                *splits.entry((point, scalar.to_bits())).or_insert_with(|| {
                    let id = u32::try_from(scalars.len()).expect("chunk vertex bound");
                    positions.extend_from_within(local * 3..local * 3 + 3);
                    normals.extend_from_within(local * 3..local * 3 + 3);
                    scalars.push(scalar);
                    if wireframe {
                        chunk.point_ids.push(chunk.point_ids[local]);
                    }
                    id
                })
            };
            chunk.indices[write] = id;
            write += 1;
        }
    }
    chunk.indices.truncate(write);
    chunk.indices.shrink_to_fit();
    positions.shrink_to_fit();
    normals.shrink_to_fit();
    scalars.shrink_to_fit();
    let edges = if wireframe {
        wireframe_edges(&chunk.indices, &chunk.point_ids, seen_edges)
    } else {
        Vec::new()
    };
    Some(GpuChunk {
        origin,
        positions,
        normals,
        scalars,
        indices: chunk.indices,
        edges,
    })
}

fn smooth_normals(positions: &[f32], indices: &[u32]) -> Vec<f32> {
    smooth_normals_for(positions, indices.as_chunks::<3>().0.iter().copied())
}

fn smooth_normals_for(positions: &[f32], triangles: impl Iterator<Item = [u32; 3]>) -> Vec<f32> {
    let mut normals = vec![0.0f32; positions.len()];
    for tri in triangles {
        let [ia, ib, ic] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
        let a = [
            positions[ia * 3],
            positions[ia * 3 + 1],
            positions[ia * 3 + 2],
        ];
        let b = [
            positions[ib * 3],
            positions[ib * 3 + 1],
            positions[ib * 3 + 2],
        ];
        let c = [
            positions[ic * 3],
            positions[ic * 3 + 1],
            positions[ic * 3 + 2],
        ];
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        for i in [ia, ib, ic] {
            normals[i * 3] += n[0];
            normals[i * 3 + 1] += n[1];
            normals[i * 3 + 2] += n[2];
        }
    }
    for n in normals.as_chunks_mut::<3>().0 {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len > 0.0 {
            n[0] /= len;
            n[1] /= len;
            n[2] /= len;
        }
    }
    normals
}

fn wireframe_edges(
    indices: &[u32],
    point_ids: &[u32],
    seen: Option<&mut std::collections::HashSet<u64>>,
) -> Vec<u32> {
    let mut edges = Vec::with_capacity(indices.len());
    for tri in indices.as_chunks::<3>().0 {
        for [a, b] in [[tri[0], tri[1]], [tri[1], tri[2]], [tri[2], tri[0]]] {
            if a != b {
                edges.push([a.min(b), a.max(b)]);
            }
        }
    }
    edges.sort_unstable();
    edges.dedup();
    if let Some(seen) = seen {
        edges.retain(|edge| {
            let [a, b] = edge.map(|i| point_ids[i as usize]);
            seen.insert((u64::from(a.min(b)) << 32) | u64::from(a.max(b)))
        });
    }
    edges.shrink_to_fit();
    edges.into_flattened()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::{DataSet, Field};

    #[test]
    fn public_preparation_rejects_invalid_cell_field_storage_before_gathering() {
        let dataset = DataSet::structured_grid("cell", [2; 3], [0.0; 3], [1.0; 3]).unwrap();
        for field in [Field::scalar("p", vec![]).on_cells(), {
            let mut field = Field::scalar("p", vec![0.0]).on_cells();
            field.valid = Some(vec![true, true].into());
            field
        }] {
            let actor = Actor::new(dataset.clone().with_field(field))
                .coloured_by_cell("p", crate::TransferFunction::viridis([0.0, 1.0]));
            assert!(matches!(
                prepare_chunks(&actor, 10),
                Err(SceneError::Mesh(_))
            ));
        }
    }

    #[test]
    fn cell_colors_split_only_discontinuities_and_keep_geometric_normals() {
        let dataset =
            DataSet::structured_grid("pair", [3, 2, 2], [-1.0, -0.5, -0.5], [1.0; 3]).unwrap();
        let reference = prepare_chunks(&Actor::new(dataset.clone()), 100).unwrap();
        let original = &reference[0];
        for values in [vec![0.0, 0.0], vec![0.0, 1.0]] {
            let expected_vertices = if values[0] == values[1] { 12 } else { 16 };
            let actor = Actor::new(
                dataset
                    .clone()
                    .with_field(Field::scalar("p", values).on_cells()),
            )
            .coloured_by_cell("p", crate::TransferFunction::viridis([0.0, 1.0]))
            .wireframe(true);
            let chunks = prepare_chunks(&actor, 100).unwrap();
            assert_eq!(chunks.len(), 1);
            let chunk = &chunks[0];
            assert_eq!(chunk.vertex_count(), expected_vertices);
            for (i, position) in chunk.positions.as_chunks::<3>().0.iter().enumerate() {
                let source = original
                    .positions
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .position(|p| p == position)
                    .unwrap();
                assert_eq!(
                    &chunk.normals[i * 3..i * 3 + 3],
                    &original.normals[source * 3..source * 3 + 3]
                );
            }
            for tri in chunk.indices.as_chunks::<3>().0 {
                let values = tri.map(|i| chunk.scalars[i as usize]);
                assert!(
                    values.iter().all(|v| *v == values[0]),
                    "cell value interpolates across owners"
                );
            }
            let expected_edges = prepare_chunks(&Actor::new(dataset.clone()).wireframe(true), 100)
                .unwrap()[0]
                .edges
                .len();
            assert_eq!(
                chunk.edges.len(),
                expected_edges,
                "scalar seams duplicate wire edges"
            );
        }
    }

    #[test]
    fn cell_scalar_seams_keep_one_wire_edge_across_chunk_boundaries() {
        let dataset = DataSet::surface(
            "quad",
            vec![
                [-1.0, -1.0, 0.0],
                [1.0, -1.0, 0.0],
                [1.0, 1.0, 0.0],
                [-1.0, 1.0, 0.0],
            ],
            &[[0, 1, 2], [0, 2, 3]],
        );
        for budget in [1, 2, 100] {
            let actor = Actor::new(
                dataset
                    .clone()
                    .with_field(Field::scalar("p", vec![0.0, 1.0]).on_cells()),
            )
            .coloured_by_cell("p", crate::TransferFunction::viridis([0.0, 1.0]))
            .wireframe(true);
            let chunks = prepare_chunks(&actor, budget).unwrap();
            assert_eq!(chunks.iter().map(|c| c.edges.len() / 2).sum::<usize>(), 5);
            let mut actor = actor;
            actor.dataset.cell_data[0].valid = Some(vec![false, true].into());
            let chunks = prepare_chunks(&actor, budget).unwrap();
            assert_eq!(chunks.iter().map(|c| c.edges.len() / 2).sum::<usize>(), 3);
        }
    }

    #[test]
    fn wire_edges_are_unique_across_winding_and_chunk_boundaries() {
        let dataset = DataSet::surface(
            "quad",
            vec![
                [-1.0, -1.0, 0.0],
                [1.0, -1.0, 0.0],
                [1.0, 1.0, 0.0],
                [-1.0, 1.0, 0.0],
            ],
            &[[0, 1, 2], [0, 2, 3]],
        );
        for budget in [1, 2] {
            for reversed in [false, true] {
                let mut dataset = dataset.clone();
                if reversed {
                    dataset.cells = gpui_mesh::Cells::triangles(&[[2, 1, 0], [3, 2, 0]]);
                }
                let chunks = prepare_chunks(&Actor::new(dataset).wireframe(true), budget).unwrap();
                assert_eq!(chunks.iter().map(|c| c.edges.len() / 2).sum::<usize>(), 5);
                let mut pairs = std::collections::HashSet::new();
                for chunk in chunks {
                    for edge in chunk.edges.as_chunks::<2>().0 {
                        let p = edge.map(|i| {
                            [
                                chunk.positions[i as usize * 3].to_bits(),
                                chunk.positions[i as usize * 3 + 1].to_bits(),
                                chunk.positions[i as usize * 3 + 2].to_bits(),
                            ]
                        });
                        let key = if p[0] < p[1] {
                            (p[0], p[1])
                        } else {
                            (p[1], p[0])
                        };
                        assert!(pairs.insert(key), "duplicate shared edge");
                    }
                }
            }
        }
        let mut field = Field::scalar("p", vec![0.0; 4]);
        field.valid = Some(vec![true, true, true, false].into());
        let actor = Actor::new(dataset.with_field(field))
            .coloured_by("p", crate::TransferFunction::viridis([0.0, 1.0]))
            .wireframe(true);
        assert_eq!(
            prepare_chunks(&actor, 1)
                .unwrap()
                .iter()
                .map(|c| c.edges.len() / 2)
                .sum::<usize>(),
            3
        );
    }

    #[test]
    fn prepares_rebased_chunks_with_scalars() {
        let ds = DataSet::structured_grid("g", [3, 3, 3], [1000.0; 3], [1.0; 3])
            .unwrap()
            .with_field(Field::scalar("p", vec![2.0; 27]));
        let actor = Actor::new(ds).coloured_by("p", crate::TransferFunction::viridis([0.0, 4.0]));
        let chunks = prepare_chunks(&actor, 10).unwrap();
        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|c| c.triangle_count() <= 10));
        // Rebased positions stay near the chunk origin despite the 1000 offset.
        assert!(
            chunks[0].positions.iter().all(|v| v.abs() <= 2.0),
            "positions not rebased"
        );
        assert!(chunks[0].scalars.iter().all(|s| *s == 2.0));
        assert!(chunks[0].byte_len() > 0);
    }

    #[test]
    fn missing_field_is_an_error_not_a_panic() {
        let ds = DataSet::surface(
            "s",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        );
        let actor =
            Actor::new(ds).coloured_by("nope", crate::TransferFunction::viridis([0.0, 1.0]));
        assert!(matches!(
            prepare_chunks(&actor, 100),
            Err(SceneError::FieldNotFound(_))
        ));
    }

    #[test]
    fn masked_point_and_cell_values_do_not_become_valid_colors() {
        for association in [Association::Point, Association::Cell] {
            let count = if association == Association::Point {
                3
            } else {
                1
            };
            let mut field = Field::scalar("p", vec![0.5; count]);
            field.association = association;
            field.valid = Some(vec![false; count].into());
            let dataset = DataSet::surface(
                "masked",
                vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                &[[0, 1, 2]],
            )
            .with_field(field);
            let mut actor = Actor::new(dataset);
            actor.field = Some((association, "p".into()));
            let chunks = prepare_chunks(&actor, 100).unwrap();
            assert!(
                chunks
                    .iter()
                    .flat_map(|c| &c.scalars)
                    .all(|value| value.is_nan())
            );
            assert!(chunks.iter().all(|chunk| chunk.indices.is_empty()));
        }
    }
}
