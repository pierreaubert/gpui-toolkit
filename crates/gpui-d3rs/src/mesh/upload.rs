//! CPU-side preparation of mesh data for retained GPU renderers.
//!
//! This module deliberately has no GPU or GPUI dependencies.  Positions stay
//! in `f64` until the final upload conversion; the bounds-centre origin is
//! subtracted in `f64` first so large-world-coordinate meshes retain local
//! precision in their `f32` vertex buffers.

use super::{MeshBounds, MeshTopology, ScalarField, TriangleMesh};

/// Maximum payload written by one GPU queue upload operation.
///
/// Keeping individual writes below this limit avoids exceeding the staging
/// limits of the smallest supported adapters while still allowing large mesh
/// resources to remain retained and indexed.
pub const MAX_GPU_UPLOAD_BYTES: usize = 64 * 1024 * 1024;

/// Split a byte payload into deterministic, non-overlapping upload chunks.
///
/// The helper is feature-independent so the chunking contract can be tested
/// in headless builds and reused by both the WGPU and Metal upload paths.
pub fn upload_chunks(bytes: &[u8]) -> impl Iterator<Item = (usize, &[u8])> {
    bytes
        .chunks(MAX_GPU_UPLOAD_BYTES.max(1))
        .enumerate()
        .map(move |(index, chunk)| (index * MAX_GPU_UPLOAD_BYTES.max(1), chunk))
}

/// Data laid out for the mesh GPU backends.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshUpload {
    /// Vertex positions relative to [`Self::origin`].
    pub positions_f32: Vec<[f32; 3]>,
    /// The f64 origin subtracted from every position before conversion.
    pub origin: [f64; 3],
    /// Flattened indexed triangle list.
    pub indices: Vec<u32>,
    /// Flattened unique-edge line list.
    pub edge_indices: Vec<u32>,
    /// Optional vertex-associated scalar values. Masked values are NaN.
    pub values_f32: Option<Vec<f32>>,
    /// Optional cell-associated scalar values. Masked values are NaN.
    pub cell_values_f32: Option<Vec<f32>>,
}

impl MeshUpload {
    /// Return the retained geometry payload size in bytes.
    ///
    /// Scalar values are intentionally excluded: field-only updates are
    /// tracked separately and must not look like geometry re-uploads.
    pub fn geometry_byte_len(&self) -> u64 {
        let bytes = |count: usize, element_size: usize| {
            u64::try_from(count)
                .unwrap_or(u64::MAX)
                .saturating_mul(u64::try_from(element_size).unwrap_or(u64::MAX))
        };
        bytes(self.positions_f32.len(), std::mem::size_of::<[f32; 3]>())
            .saturating_add(bytes(self.indices.len(), std::mem::size_of::<u32>()))
            .saturating_add(bytes(self.edge_indices.len(), std::mem::size_of::<u32>()))
    }
}

/// Convert mesh geometry to a retained GPU upload representation.
pub fn prepare_upload(mesh: &TriangleMesh, topology: &MeshTopology) -> MeshUpload {
    let origin = MeshBounds::from_positions(&mesh.positions).origin();
    let positions_f32 = mesh
        .positions
        .iter()
        .map(|p| {
            [
                (p[0] - origin[0]) as f32,
                (p[1] - origin[1]) as f32,
                (p[2] - origin[2]) as f32,
            ]
        })
        .collect();
    let indices = mesh
        .triangles
        .iter()
        .flat_map(|triangle| triangle.iter().copied())
        .collect();
    let edge_indices = topology
        .unique_edges
        .iter()
        .flat_map(|edge| edge.iter().copied())
        .collect();

    MeshUpload {
        positions_f32,
        origin,
        indices,
        edge_indices,
        values_f32: None,
        cell_values_f32: None,
    }
}

/// Convert a scalar field to the shader representation.
///
/// The explicit validity mask is applied here rather than in a renderer so
/// all backends share the same NaN-discard contract.
pub fn prepare_field(field: &ScalarField) -> Vec<f32> {
    field
        .values
        .iter()
        .enumerate()
        .map(|(index, &value)| {
            if field
                .valid
                .as_ref()
                .is_some_and(|valid| valid.get(index) != Some(&true))
            {
                f32::NAN
            } else {
                value as f32
            }
        })
        .collect()
}

/// Expand indexed triangles so each triangle has independent vertices.
///
/// This is the representation required for flat/cell shading.  The function
/// preserves the origin and edge list, while converting cell values into a
/// per-vertex value buffer with three equal values per triangle.
pub fn expand_cell_shading(upload: &MeshUpload) -> MeshUpload {
    let mut positions_f32 = Vec::with_capacity(upload.indices.len());
    let mut indices = Vec::with_capacity(upload.indices.len());
    let mut values_f32 = upload
        .values_f32
        .as_ref()
        .map(|_| Vec::with_capacity(upload.indices.len()));

    for triangle in upload.indices.chunks_exact(3) {
        for &source_index in triangle {
            let source = source_index as usize;
            let Some(position) = upload.positions_f32.get(source).copied() else {
                // `prepare_upload` is normally fed a validated mesh. Keep the
                // public transformation total nevertheless: malformed input
                // must not underflow the output index.
                continue;
            };
            positions_f32.push(position);
            indices.push((positions_f32.len() - 1) as u32);
            if let (Some(out), Some(source_values)) = (&mut values_f32, &upload.values_f32) {
                let value = source_values.get(source).copied().unwrap_or(f32::NAN);
                out.push(value);
            }
        }
    }

    if let Some(cell_values) = &upload.cell_values_f32 {
        values_f32 = Some(
            cell_values
                .iter()
                .flat_map(|&value| [value, value, value])
                .take(upload.indices.len())
                .collect(),
        );
    }

    MeshUpload {
        positions_f32,
        origin: upload.origin,
        indices,
        edge_indices: upload.edge_indices.clone(),
        values_f32,
        cell_values_f32: None,
    }
}

/// Compute area-weighted smooth vertex normals in mesh vertex order.
pub fn compute_smooth_normals(mesh: &TriangleMesh) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0f32; 3]; mesh.positions.len()];
    for triangle in mesh.triangles.iter() {
        let Some(&a) = mesh.positions.get(triangle[0] as usize) else {
            continue;
        };
        let Some(&b) = mesh.positions.get(triangle[1] as usize) else {
            continue;
        };
        let Some(&c) = mesh.positions.get(triangle[2] as usize) else {
            continue;
        };
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let normal = [
            (ab[1] * ac[2] - ab[2] * ac[1]) as f32,
            (ab[2] * ac[0] - ab[0] * ac[2]) as f32,
            (ab[0] * ac[1] - ab[1] * ac[0]) as f32,
        ];
        for &vertex in triangle {
            let Some(out) = normals.get_mut(vertex as usize) else {
                continue;
            };
            out[0] += normal[0];
            out[1] += normal[1];
            out[2] += normal[2];
        }
    }
    for normal in &mut normals {
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if length > f32::EPSILON {
            normal[0] /= length;
            normal[1] /= length;
            normal[2] /= length;
        }
    }
    normals
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn mesh_at_large_offset() -> TriangleMesh {
        TriangleMesh {
            id: "large".into(),
            positions: Arc::from([
                [1_000_000_000.0, -2_000_000_000.0, 3.0],
                [1_000_001_000.0, -2_000_000_000.0, 3.0],
                [1_000_000_000.0, -1_999_999_000.0, 3.0],
            ]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        }
    }

    fn square_mesh() -> TriangleMesh {
        TriangleMesh {
            id: "square".into(),
            positions: Arc::from([
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ]),
            triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
            vertex_ids: None,
            cell_ids: None,
        }
    }

    #[test]
    fn upload_rebases_positions_to_f32_offsets() {
        let mesh = mesh_at_large_offset();
        let topology = MeshTopology::build(&mesh.triangles);
        let upload = prepare_upload(&mesh, &topology);
        let expected_origin = MeshBounds::from_positions(&mesh.positions).origin();
        assert_eq!(upload.origin, expected_origin);
        for (p32, p64) in upload.positions_f32.iter().zip(mesh.positions.iter()) {
            for axis in 0..3 {
                let relative = p64[axis] - expected_origin[axis];
                assert!((p32[axis] as f64 - relative).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn upload_edge_indices_match_unique_edges() {
        let mesh = square_mesh();
        let topology = MeshTopology::build(&mesh.triangles);
        let upload = prepare_upload(&mesh, &topology);
        assert_eq!(upload.edge_indices.len(), topology.unique_edges.len() * 2);
        assert_eq!(upload.indices.len(), mesh.triangles.len() * 3);
    }

    #[test]
    fn geometry_byte_len_excludes_scalar_values() {
        let mesh = square_mesh();
        let topology = MeshTopology::build(&mesh.triangles);
        let mut upload = prepare_upload(&mesh, &topology);
        let geometry_bytes = upload.geometry_byte_len();
        upload.values_f32 = Some(vec![0.0; mesh.positions.len()]);
        upload.cell_values_f32 = Some(vec![0.0; mesh.triangles.len()]);
        assert_eq!(upload.geometry_byte_len(), geometry_bytes);
        assert_eq!(
            geometry_bytes,
            (mesh.positions.len() * std::mem::size_of::<[f32; 3]>()
                + mesh.triangles.len() * 3 * std::mem::size_of::<u32>()
                + topology.unique_edges.len() * 2 * std::mem::size_of::<u32>()) as u64
        );
    }

    #[test]
    fn masked_field_values_become_nan_sentinel() {
        let field = ScalarField {
            id: "f".into(),
            label: "f".into(),
            unit: None,
            values: Arc::from([1.0, f64::NAN, 3.0]),
            association: super::super::ScalarAssociation::Vertex,
            valid: Some(Arc::from([true, false, true])),
        };
        let values = prepare_field(&field);
        assert!(values[1].is_nan());
    }

    #[test]
    fn large_offset_preserves_local_precision() {
        let mesh = mesh_at_large_offset();
        let topology = MeshTopology::build(&mesh.triangles);
        let upload = prepare_upload(&mesh, &topology);
        let span = upload
            .positions_f32
            .iter()
            .map(|p| p[0])
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), x| {
                (lo.min(x), hi.max(x))
            });
        assert!(span.1 - span.0 < 1e6);
    }

    #[test]
    fn cell_values_expand_to_three_vertices_per_triangle() {
        let mesh = square_mesh();
        let topology = MeshTopology::build(&mesh.triangles);
        let mut upload = prepare_upload(&mesh, &topology);
        upload.cell_values_f32 = Some(vec![0.5, 0.7]);
        let expanded = expand_cell_shading(&upload);
        assert_eq!(expanded.positions_f32.len(), upload.indices.len());
        assert_eq!(
            expanded.values_f32.as_deref(),
            Some(&[0.5, 0.5, 0.5, 0.7, 0.7, 0.7][..])
        );
    }

    #[test]
    fn smooth_normals_are_unit_length_for_planar_square() {
        let mesh = square_mesh();
        for n in compute_smooth_normals(&mesh) {
            assert!((n[2] - 1.0).abs() < 1e-6);
            assert!(n[0].abs() < 1e-6 && n[1].abs() < 1e-6);
        }
    }

    #[test]
    fn upload_chunks_cover_payload_without_exceeding_limit() {
        let bytes = vec![0u8; MAX_GPU_UPLOAD_BYTES * 2 + 17];
        let chunks = upload_chunks(&bytes).collect::<Vec<_>>();
        assert_eq!(chunks.len(), 3);
        assert!(
            chunks
                .iter()
                .all(|(_, chunk)| chunk.len() <= MAX_GPU_UPLOAD_BYTES)
        );
        let rebuilt = chunks
            .iter()
            .flat_map(|(_, chunk)| chunk.iter().copied())
            .collect::<Vec<_>>();
        assert_eq!(rebuilt, bytes);
        assert_eq!(chunks[1].0, MAX_GPU_UPLOAD_BYTES);
    }
}
