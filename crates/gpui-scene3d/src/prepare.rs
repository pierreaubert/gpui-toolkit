//! Chunked GPU-ready buffer preparation for large datasets.

// Rust guideline compliant 2026-02-21

use crate::SceneError;
use crate::scene::Actor;
use gpui_mesh::Association;
use std::borrow::Cow;

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
    /// Wireframe edge indices derived from `indices`, when requested.
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
/// gathered per vertex (cell scalars are expanded by their owning point's
/// first use; point scalars are gathered exactly).
///
/// # Errors
///
/// Returns [`SceneError::FieldNotFound`] when the actor's colouring field
/// is missing or not scalar.
pub fn prepare_chunks(actor: &Actor, max_triangles: usize) -> Result<Vec<GpuChunk>, SceneError> {
    prepare_chunks_with_bounds(actor, max_triangles).map(|(_, chunks)| chunks)
}

pub(crate) fn prepare_chunks_with_bounds(
    actor: &Actor,
    max_triangles: usize,
) -> Result<(Option<gpui_mesh::Bounds>, Vec<GpuChunk>), SceneError> {
    let dataset = &actor.dataset;
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
    // Cell scalars need triangle -> cell mapping; boundary extraction loses
    // it, so expand cell values to points by averaging owning cells first.
    let valid = actor
        .field
        .as_ref()
        .and_then(|(association, name)| dataset.field(*association, name))
        .and_then(|field| field.valid.as_deref());
    let point_scalars: Option<Cow<'_, [f64]>> = match scalar_lookup {
        Some((Association::Point, values)) => Some(Cow::Borrowed(values)),
        Some((Association::Cell, values)) => {
            let mut sums = vec![0.0f64; dataset.point_count()];
            let mut counts = vec![0.0f64; dataset.point_count()];
            for (cell, &value) in values.iter().enumerate() {
                if !value.is_finite() || valid.is_some_and(|mask| !mask[cell]) {
                    continue;
                }
                let Some(nodes) = dataset.cells.cell(cell) else {
                    continue;
                };
                for &node in nodes {
                    sums[node as usize] += value;
                    counts[node as usize] += 1.0;
                }
            }
            Some(Cow::Owned(
                sums.iter()
                    .zip(counts.iter())
                    .map(|(s, c)| if *c > 0.0 { s / c } else { f64::NAN })
                    .collect(),
            ))
        }
        None => None,
    };

    let chunks = gpui_mesh::surface_chunks(dataset, max_triangles.max(1));
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
            wireframe_edges(&indices)
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

fn smooth_normals(positions: &[f32], indices: &[u32]) -> Vec<f32> {
    let mut normals = vec![0.0f32; positions.len()];
    for tri in indices.as_chunks::<3>().0 {
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

fn wireframe_edges(indices: &[u32]) -> Vec<u32> {
    let mut edges = Vec::with_capacity(indices.len() * 2);
    for tri in indices.as_chunks::<3>().0 {
        edges.extend_from_slice(&[tri[0], tri[1], tri[1], tri[2], tri[2], tri[0]]);
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::{DataSet, Field};

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
