use super::{FieldRevision, GeometryRevision};
use crate::mesh::MeshUpload;

/// Replace a retained scalar buffer without reallocating when its capacity is
/// already sufficient. The first field upload may allocate; subsequent field
/// patches reuse the same backing storage.
pub(crate) fn replace_retained_field(slot: &mut Option<Vec<f32>>, values: &[f32]) {
    match slot {
        Some(buffer) => {
            buffer.clear();
            buffer.extend_from_slice(values);
        }
        None => *slot = Some(values.to_vec()),
    }
}

/// Common renderer contract shared by wgpu, Metal, and readback backends.
pub trait MeshGpuRenderer {
    fn upload_geometry(&mut self, rev: GeometryRevision, upload: &MeshUpload);
    fn write_field(&mut self, rev: FieldRevision, values: &[f32]);
    fn geometry_revision(&self) -> Option<GeometryRevision>;
}

/// Mesh rendering configuration consumed by all platform backends.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshColorConfig {
    pub colormap: u32,
    pub range: [f32; 2],
    pub wireframe: bool,
    pub isoline_step: f32,
    pub isoline_width_px: f32,
    pub unlit: bool,
}

impl Default for MeshColorConfig {
    fn default() -> Self {
        Self {
            colormap: 0,
            range: [0.0, 1.0],
            wireframe: false,
            isoline_step: 0.0,
            isoline_width_px: 1.0,
            unlit: true,
        }
    }
}

/// Retained state shared by the GPUI element and platform custom draws.
#[derive(Debug, Clone)]
pub struct MeshSceneState {
    pub geometry_rev: GeometryRevision,
    pub field_rev: FieldRevision,
    pub upload: Option<MeshUpload>,
    /// Number of backend geometry uploads represented by this scene.
    pub geometry_upload_count: u64,
    /// Sum of retained geometry payload bytes sent to the backend.
    pub geometry_upload_bytes: u64,
    pub view_transform: [[f32; 4]; 4],
    pub color: MeshColorConfig,
}

impl Default for MeshSceneState {
    fn default() -> Self {
        Self {
            geometry_rev: GeometryRevision(0),
            field_rev: FieldRevision(0),
            upload: None,
            geometry_upload_count: 0,
            geometry_upload_bytes: 0,
            view_transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            color: MeshColorConfig::default(),
        }
    }
}

impl MeshSceneState {
    /// Record one geometry upload without counting scalar field bytes.
    pub fn record_geometry_upload(&mut self, upload: &MeshUpload) {
        self.geometry_upload_count = self.geometry_upload_count.saturating_add(1);
        self.geometry_upload_bytes = self
            .geometry_upload_bytes
            .saturating_add(upload.geometry_byte_len());
    }
}

/// A small renderer useful for tests and for a backend that is not available.
/// It retains uploads and revisions without allocating on camera-only frames.
#[derive(Debug, Clone, Default)]
pub struct RetainedMeshRenderer {
    state: MeshSceneState,
    values: Vec<f32>,
}

impl RetainedMeshRenderer {
    pub fn state(&self) -> &MeshSceneState {
        &self.state
    }

    pub fn field_values(&self) -> &[f32] {
        &self.values
    }
}

impl MeshGpuRenderer for RetainedMeshRenderer {
    fn upload_geometry(&mut self, rev: GeometryRevision, upload: &MeshUpload) {
        self.state.record_geometry_upload(upload);
        self.state.geometry_rev = rev;
        self.state.upload = Some(upload.clone());
    }

    fn write_field(&mut self, rev: FieldRevision, values: &[f32]) {
        self.state.field_rev = rev;
        self.values.clear();
        self.values.extend_from_slice(values);
        if let Some(upload) = &mut self.state.upload {
            if upload.cell_values_f32.is_some() {
                replace_retained_field(&mut upload.cell_values_f32, &self.values);
            } else {
                replace_retained_field(&mut upload.values_f32, &self.values);
            }
        }
    }

    fn geometry_revision(&self) -> Option<GeometryRevision> {
        self.state.upload.as_ref().map(|_| self.state.geometry_rev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::{MeshTopology, TriangleMesh, prepare_upload};
    use std::sync::Arc;

    fn upload() -> MeshUpload {
        let mesh = TriangleMesh {
            id: "m".into(),
            positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        prepare_upload(&mesh, &MeshTopology::build(&mesh.triangles))
    }

    #[test]
    fn field_write_does_not_change_geometry_revision() {
        let mut renderer = RetainedMeshRenderer::default();
        renderer.upload_geometry(GeometryRevision(4), &upload());
        renderer.write_field(FieldRevision(7), &[1.0, 2.0, 3.0]);
        assert_eq!(renderer.geometry_revision(), Some(GeometryRevision(4)));
        assert_eq!(renderer.field_values(), &[1.0, 2.0, 3.0]);
        assert_eq!(renderer.state().geometry_upload_count, 1);
        assert_eq!(
            renderer.state().geometry_upload_bytes,
            upload().geometry_byte_len()
        );
    }

    #[test]
    fn repeated_field_writes_reuse_retained_capacity() {
        let mut renderer = RetainedMeshRenderer::default();
        renderer.upload_geometry(GeometryRevision(1), &upload());
        renderer.write_field(FieldRevision(1), &[1.0, 2.0, 3.0]);
        let capacity = renderer
            .state()
            .upload
            .as_ref()
            .and_then(|upload| upload.values_f32.as_ref())
            .map(Vec::capacity)
            .unwrap();
        renderer.write_field(FieldRevision(2), &[4.0, 5.0, 6.0]);
        assert_eq!(renderer.field_values(), &[4.0, 5.0, 6.0]);
        assert_eq!(
            renderer
                .state()
                .upload
                .as_ref()
                .unwrap()
                .values_f32
                .as_ref()
                .unwrap()
                .capacity(),
            capacity
        );
        assert_eq!(renderer.state().geometry_upload_count, 1);
    }
}
