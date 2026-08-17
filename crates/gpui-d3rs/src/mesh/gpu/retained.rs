use super::{FieldRevision, GeometryRevision};
use crate::mesh::MeshUpload;
use std::time::Duration;

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
    /// Number of scalar field writes sent to the backend.
    pub field_write_count: u64,
    /// Sum of scalar field payload bytes sent to the backend.
    pub field_write_bytes: u64,
    /// Number of scalar writes submitted to an adapter-backed field buffer.
    ///
    /// This is separate from `field_write_count`: the latter records the
    /// logical retained-state mutation, while this counter is incremented by
    /// the custom draw only after a queue/private-buffer write succeeds.
    pub gpu_field_write_count: u64,
    /// Bytes submitted by adapter-backed scalar writes.
    pub gpu_field_write_bytes: u64,
    /// Number of adapter-backed geometry allocations/uploads.
    pub gpu_geometry_upload_count: u64,
    /// Bytes in adapter-backed geometry allocations/uploads.
    pub gpu_geometry_upload_bytes: u64,
    /// Bytes reserved for the current adapter-backed scalar buffer.
    pub gpu_field_capacity_bytes: u64,
    /// Approximate resident bytes for adapter-owned mesh buffers and depth
    /// targets. Pipeline/driver-private allocations are intentionally not
    /// included because wgpu and Metal do not expose them portably.
    pub gpu_resident_bytes: u64,
    /// Highest adapter-owned resident allocation observed by this retained
    /// scene. This remains available after the current resources are released
    /// so long-run churn tests can distinguish bounded residency from a
    /// transiently small final generation.
    pub gpu_peak_resident_bytes: u64,
    /// Highest adapter-owned scalar-buffer capacity observed by this scene.
    pub gpu_peak_field_capacity_bytes: u64,
    /// Number of times a non-empty adapter-owned resource set was released.
    pub gpu_memory_release_count: u64,
    /// CPU time spent creating/uploading the current adapter geometry
    /// resources, accumulated across retained resource generations.
    ///
    /// This measures the production submission path, not asynchronous GPU
    /// execution time; it is still useful for comparing adapter lanes and
    /// spotting regressions in resource churn.
    pub gpu_geometry_upload_time_ns: u64,
    /// CPU time spent submitting adapter-backed scalar-buffer writes.
    pub gpu_field_write_time_ns: u64,
    /// CPU time spent recording retained custom-draw frames.
    pub gpu_frame_time_ns: u64,
    /// Number of retained adapter-backed frames included in
    /// [`Self::gpu_frame_time_ns`].
    pub gpu_frame_count: u64,
    /// GPU execution time recovered asynchronously from adapter timestamps.
    pub gpu_frame_gpu_time_ns: u64,
    /// Number of retained frames with a completed GPU timestamp sample.
    pub gpu_frame_gpu_time_count: u64,
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
            field_write_count: 0,
            field_write_bytes: 0,
            gpu_field_write_count: 0,
            gpu_field_write_bytes: 0,
            gpu_geometry_upload_count: 0,
            gpu_geometry_upload_bytes: 0,
            gpu_field_capacity_bytes: 0,
            gpu_resident_bytes: 0,
            gpu_peak_resident_bytes: 0,
            gpu_peak_field_capacity_bytes: 0,
            gpu_memory_release_count: 0,
            gpu_geometry_upload_time_ns: 0,
            gpu_field_write_time_ns: 0,
            gpu_frame_time_ns: 0,
            gpu_frame_count: 0,
            gpu_frame_gpu_time_ns: 0,
            gpu_frame_gpu_time_count: 0,
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

    /// Record one scalar-field write without counting geometry bytes.
    pub fn record_field_write(&mut self, values: &[f32]) {
        self.field_write_count = self.field_write_count.saturating_add(1);
        self.field_write_bytes = self.field_write_bytes.saturating_add(
            (values.len() as u64).saturating_mul(std::mem::size_of::<f32>() as u64),
        );
    }

    /// Record one adapter-backed scalar-buffer write.
    pub fn record_gpu_field_write(&mut self, bytes: u64) {
        self.gpu_field_write_count = self.gpu_field_write_count.saturating_add(1);
        self.gpu_field_write_bytes = self.gpu_field_write_bytes.saturating_add(bytes);
    }

    /// Record a newly allocated adapter-backed geometry resource.
    pub fn record_gpu_geometry_upload(&mut self, bytes: u64) {
        self.gpu_geometry_upload_count = self.gpu_geometry_upload_count.saturating_add(1);
        self.gpu_geometry_upload_bytes = self.gpu_geometry_upload_bytes.saturating_add(bytes);
    }

    /// Publish the current adapter-owned buffer/depth allocation estimate.
    pub fn set_gpu_memory(&mut self, resident_bytes: u64, field_capacity_bytes: u64) {
        self.gpu_resident_bytes = resident_bytes;
        self.gpu_field_capacity_bytes = field_capacity_bytes;
        self.gpu_peak_resident_bytes = self.gpu_peak_resident_bytes.max(resident_bytes);
        self.gpu_peak_field_capacity_bytes =
            self.gpu_peak_field_capacity_bytes.max(field_capacity_bytes);
    }

    /// Mark the adapter-owned resources as released while retaining their
    /// peak counters for post-destruction diagnostics.
    pub fn clear_gpu_memory(&mut self) {
        if self.gpu_resident_bytes != 0 || self.gpu_field_capacity_bytes != 0 {
            self.gpu_memory_release_count = self.gpu_memory_release_count.saturating_add(1);
        }
        self.gpu_resident_bytes = 0;
        self.gpu_field_capacity_bytes = 0;
    }

    /// Accumulate adapter geometry resource creation/upload time.
    pub fn record_gpu_geometry_upload_time(&mut self, elapsed: Duration) {
        self.gpu_geometry_upload_time_ns = self
            .gpu_geometry_upload_time_ns
            .saturating_add(elapsed.as_nanos().min(u128::from(u64::MAX)) as u64);
    }

    /// Accumulate adapter scalar-buffer write submission time.
    pub fn record_gpu_field_write_time(&mut self, elapsed: Duration) {
        self.gpu_field_write_time_ns = self
            .gpu_field_write_time_ns
            .saturating_add(elapsed.as_nanos().min(u128::from(u64::MAX)) as u64);
    }

    /// Accumulate retained custom-draw frame recording time.
    pub fn record_gpu_frame_time(&mut self, elapsed: Duration) {
        self.gpu_frame_time_ns = self
            .gpu_frame_time_ns
            .saturating_add(elapsed.as_nanos().min(u128::from(u64::MAX)) as u64);
        self.gpu_frame_count = self.gpu_frame_count.saturating_add(1);
    }

    /// Record a completed asynchronous GPU timestamp sample.
    pub fn record_gpu_frame_gpu_time(&mut self, elapsed: Duration) {
        self.gpu_frame_gpu_time_ns = self
            .gpu_frame_gpu_time_ns
            .saturating_add(elapsed.as_nanos().min(u128::from(u64::MAX)) as u64);
        self.gpu_frame_gpu_time_count = self.gpu_frame_gpu_time_count.saturating_add(1);
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
        self.state.record_field_write(values);
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
        assert_eq!(renderer.state().field_write_count, 1);
        assert_eq!(
            renderer.state().field_write_bytes,
            3 * std::mem::size_of::<f32>() as u64
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
        assert_eq!(renderer.state().field_write_count, 2);
        assert_eq!(
            renderer.state().field_write_bytes,
            6 * std::mem::size_of::<f32>() as u64
        );
    }

    #[test]
    fn adapter_telemetry_is_separate_from_logical_retained_state() {
        let mut state = MeshSceneState::default();
        state.record_geometry_upload(&upload());
        state.record_gpu_geometry_upload(128);
        state.record_field_write(&[1.0, 2.0]);
        state.record_gpu_field_write(64);
        state.set_gpu_memory(512, 128);

        assert_eq!(state.geometry_upload_count, 1);
        assert_eq!(state.gpu_geometry_upload_count, 1);
        assert_eq!(state.gpu_geometry_upload_bytes, 128);
        assert_eq!(state.field_write_count, 1);
        assert_eq!(state.gpu_field_write_count, 1);
        assert_eq!(state.gpu_field_write_bytes, 64);
        assert_eq!(state.gpu_resident_bytes, 512);
        assert_eq!(state.gpu_field_capacity_bytes, 128);
        assert_eq!(state.gpu_peak_resident_bytes, 512);
        assert_eq!(state.gpu_peak_field_capacity_bytes, 128);
        assert_eq!(state.gpu_memory_release_count, 0);

        state.clear_gpu_memory();
        assert_eq!(state.gpu_resident_bytes, 0);
        assert_eq!(state.gpu_field_capacity_bytes, 0);
        assert_eq!(state.gpu_peak_resident_bytes, 512);
        assert_eq!(state.gpu_peak_field_capacity_bytes, 128);
        assert_eq!(state.gpu_memory_release_count, 1);

        state.record_gpu_geometry_upload_time(Duration::from_nanos(7));
        state.record_gpu_field_write_time(Duration::from_nanos(11));
        state.record_gpu_frame_time(Duration::from_nanos(13));
        state.record_gpu_frame_time(Duration::from_nanos(17));
        assert_eq!(state.gpu_geometry_upload_time_ns, 7);
        assert_eq!(state.gpu_field_write_time_ns, 11);
        assert_eq!(state.gpu_frame_time_ns, 30);
        assert_eq!(state.gpu_frame_count, 2);
        state.record_gpu_frame_gpu_time(Duration::from_nanos(19));
        assert_eq!(state.gpu_frame_gpu_time_ns, 19);
        assert_eq!(state.gpu_frame_gpu_time_count, 1);
    }
}
