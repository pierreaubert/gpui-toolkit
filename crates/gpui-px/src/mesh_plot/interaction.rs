use super::MeshPlotPick;
use crate::interaction::ChartInteraction;
#[cfg(feature = "gpu-3d")]
use d3rs::gpu3d::{Camera3D, OrbitControls, StandardView};
#[cfg(feature = "gpu-3d")]
use d3rs::mesh::gpu::MeshSceneState;
#[cfg(all(feature = "gpu-3d", not(test)))]
use d3rs::mesh::gpu::WgpuMesh3DRenderer;
use d3rs::mesh::{
    ContourBand, CoordinateAxis, IsolineSegment, ScalarAssociation, ScalarField, TriGridIndex,
    TriangleMesh,
};
#[cfg(feature = "gpu-3d")]
use d3rs::mesh::{MeshBounds, MeshBvh, RevolveSpec, RevolvedMesh};
#[cfg(all(feature = "gpu-3d", not(test)))]
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// GPU resources retained by one live 3D mesh-plot instance.
///
/// The renderer is deliberately owned by [`MeshPlotState`] rather than a
/// single `MeshPlot::build` call. GPUI's draw registry then keeps referring to
/// the same custom ID while field, style, and camera patches mutate only their
/// respective retained state.
#[cfg(all(feature = "gpu-3d", not(test)))]
#[derive(Clone)]
pub(crate) struct RetainedMesh3D {
    pub(crate) scene: Rc<RefCell<MeshSceneState>>,
    pub(crate) renderer: Rc<WgpuMesh3DRenderer>,
    /// Full/proxy uploads for drag-time navigation. The same renderer owns
    /// both: swapping the scene revision rebuilds buffers once at drag start
    /// and once at drag end, never on camera-only frames.
    pub(crate) lod: Rc<RefCell<RetainedMeshLod>>,
    pub(crate) geometry_revision: u64,
}

/// Read-only diagnostics for the retained live 3D scene.
///
/// Native hosts and integration tests use this to verify dirty-domain
/// behavior without reaching into renderer-owned state. `scene_identity` is
/// process-local and intentionally suitable only for comparing two snapshots
/// from the same running plot instance.
#[cfg(all(feature = "gpu-3d", not(test)))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetainedMesh3DStats {
    pub geometry_revision: u64,
    pub scene_identity: usize,
    pub geometry_upload_count: u64,
    pub geometry_upload_bytes: u64,
    /// Geometry bytes currently resident in the retained upload.
    pub geometry_resident_bytes: u64,
    pub field_write_count: u64,
    pub field_write_bytes: u64,
    /// Scalar bytes currently resident in the retained upload.
    pub field_resident_bytes: u64,
    /// Scalar buffer capacity retained across field-only updates.
    pub field_capacity_bytes: u64,
    /// Adapter-backed geometry allocations observed by the custom draw.
    pub gpu_geometry_upload_count: u64,
    pub gpu_geometry_upload_bytes: u64,
    /// Adapter-backed field writes observed by the custom draw.
    pub gpu_field_write_count: u64,
    pub gpu_field_write_bytes: u64,
    /// Adapter-owned field capacity and approximate resident allocation.
    pub gpu_field_capacity_bytes: u64,
    pub gpu_resident_bytes: u64,
}

/// CPU-side timing counters for expensive MeshPlot operations.
///
/// The counters stay in retained plot state so release hosts can sample them
/// without adding file I/O to the render thread. Nanoseconds are totals across
/// the lifetime of one plot owner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MeshPlotTimingStats {
    pub contour_preparation_count: u64,
    pub contour_preparation_ns: u64,
    pub revolve_preparation_count: u64,
    pub revolve_preparation_ns: u64,
    pub pick_count: u64,
    pub pick_ns: u64,
}

impl MeshPlotTimingStats {
    fn duration_ns(duration: Duration) -> u64 {
        duration.as_nanos().min(u128::from(u64::MAX)) as u64
    }

    fn record_contour_preparation(&mut self, duration: Duration) {
        self.contour_preparation_count = self.contour_preparation_count.saturating_add(1);
        self.contour_preparation_ns = self
            .contour_preparation_ns
            .saturating_add(Self::duration_ns(duration));
    }

    #[cfg(feature = "gpu-3d")]
    fn record_revolve_preparation(&mut self, duration: Duration) {
        self.revolve_preparation_count = self.revolve_preparation_count.saturating_add(1);
        self.revolve_preparation_ns = self
            .revolve_preparation_ns
            .saturating_add(Self::duration_ns(duration));
    }

    fn record_pick(&mut self, duration: Duration) {
        self.pick_count = self.pick_count.saturating_add(1);
        self.pick_ns = self.pick_ns.saturating_add(Self::duration_ns(duration));
    }
}

#[cfg(feature = "gpu-3d")]
pub(crate) struct RetainedMeshLod {
    controller: d3rs::mesh::gpu::MeshLodController,
    full_upload: Option<d3rs::mesh::MeshUpload>,
    proxy_upload: Option<d3rs::mesh::MeshUpload>,
    displaying_proxy: bool,
}

type RetainedContourOutput = (Rc<Vec<ContourBand>>, Rc<Vec<IsolineSegment>>);

#[cfg(feature = "gpu-3d")]
type RetainedRevolve = (
    u64,
    usize,
    usize,
    RevolveSpec,
    Rc<RevolvedMesh>,
    Rc<MeshBvh>,
);

#[cfg(feature = "gpu-3d")]
type RetainedRevolvedField = (u64, u64, usize, Option<usize>, Rc<ScalarField>);

/// Prepared marching-triangle output owned by a live plot instance. Pointer
/// identities complement host revisions so direct native builders cannot reuse
/// contours after replacing an Arc-backed field without updating its revision.
#[derive(Clone)]
struct RetainedContours {
    geometry_revision: u64,
    field_revision: u64,
    positions_ptr: usize,
    triangles_ptr: usize,
    field_values_ptr: Option<usize>,
    valid_ptr: Option<usize>,
    association: Option<ScalarAssociation>,
    horizontal: CoordinateAxis,
    vertical: CoordinateAxis,
    mode: super::MeshRenderMode,
    range: Option<[f64; 2]>,
    bands: Rc<Vec<ContourBand>>,
    lines: Rc<Vec<IsolineSegment>>,
}

/// Sendable/cache-comparable identity of one complete contour preparation.
/// Keeping this separate from the Rc-owned draw data lets a background worker
/// prove that it is still returning the revision the live plot requested.
#[derive(Clone, PartialEq)]
pub(crate) struct ContourPreparationKey {
    geometry_revision: u64,
    field_revision: u64,
    positions_ptr: usize,
    triangles_ptr: usize,
    field_values_ptr: Option<usize>,
    valid_ptr: Option<usize>,
    association: Option<ScalarAssociation>,
    horizontal: CoordinateAxis,
    vertical: CoordinateAxis,
    mode: super::MeshRenderMode,
    range: Option<[f64; 2]>,
}

/// Identity of a complete axisymmetric-revolve preparation request. Geometry
/// buffer identities are included in addition to host revisions because a
/// direct native caller can replace an Arc-backed mesh before advancing its
/// revision counter.
#[cfg(feature = "gpu-3d")]
#[derive(Clone, PartialEq)]
pub(crate) struct RevolvePreparationKey {
    geometry_revision: u64,
    field_revision: u64,
    positions_ptr: usize,
    triangles_ptr: usize,
    field_values_ptr: Option<usize>,
    valid_ptr: Option<usize>,
    spec: RevolveSpec,
}

/// Owned result returned by the background executor. It intentionally has no
/// `Rc` members, so preparation can happen away from GPUI's UI thread and be
/// promoted into retained draw state only after its key is revalidated.
#[cfg(feature = "gpu-3d")]
pub(crate) struct PreparedRevolve {
    pub(crate) revolved: RevolvedMesh,
    pub(crate) bvh: MeshBvh,
    pub(crate) field: Option<ScalarField>,
}

#[cfg(feature = "gpu-3d")]
impl RetainedMeshLod {
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn new(mesh: TriangleMesh, field: Option<&ScalarField>) -> Self {
        Self::with_lod_threshold(mesh, field, d3rs::mesh::gpu::DEFAULT_LOD_THRESHOLD)
    }

    pub(crate) fn with_lod_threshold(
        mesh: TriangleMesh,
        field: Option<&ScalarField>,
        threshold: usize,
    ) -> Self {
        let controller = d3rs::mesh::gpu::MeshLodController::with_lod_threshold(mesh, threshold);
        let mut result = Self {
            controller,
            full_upload: None,
            proxy_upload: None,
            displaying_proxy: false,
        };
        result.update_field(field);
        result
    }

    pub(crate) fn update_field(&mut self, field: Option<&ScalarField>) {
        if self.controller.proxy_mesh().is_none() {
            self.proxy_upload = None;
            return;
        }
        // Temporarily select the proxy only to materialize its association-safe
        // scalar data. This does not change the live scene or camera state.
        self.controller.begin_camera_drag();
        let proxy_mesh = self.controller.active_mesh().clone();
        let proxy_field = field.and_then(|field| self.controller.active_field(field).ok());
        self.controller.end_camera_drag();
        self.proxy_upload = Some(mesh_upload_with_field(&proxy_mesh, proxy_field.as_ref()));
    }

    pub(crate) fn begin_drag(&mut self, scene: &mut MeshSceneState) -> bool {
        self.controller.begin_camera_drag();
        if self.displaying_proxy {
            return false;
        }
        let Some(proxy_upload) = self.proxy_upload.as_ref() else {
            return false;
        };
        self.full_upload = scene.upload.clone();
        scene.record_geometry_upload(proxy_upload);
        scene.geometry_rev.0 = scene.geometry_rev.0.saturating_add(1);
        scene.field_rev.0 = scene.field_rev.0.saturating_add(1);
        scene.upload = Some(proxy_upload.clone());
        self.displaying_proxy = true;
        true
    }

    pub(crate) fn end_drag(&mut self, scene: &mut MeshSceneState) -> bool {
        self.controller.end_camera_drag();
        if !self.displaying_proxy {
            return false;
        }
        let Some(full_upload) = self.full_upload.take() else {
            self.displaying_proxy = false;
            return false;
        };
        scene.record_geometry_upload(&full_upload);
        scene.geometry_rev.0 = scene.geometry_rev.0.saturating_add(1);
        scene.field_rev.0 = scene.field_rev.0.saturating_add(1);
        scene.upload = Some(full_upload);
        self.displaying_proxy = false;
        true
    }
}

#[cfg(feature = "gpu-3d")]
fn mesh_upload_with_field(
    mesh: &TriangleMesh,
    field: Option<&ScalarField>,
) -> d3rs::mesh::MeshUpload {
    use d3rs::mesh::{MeshTopology, ScalarAssociation, prepare_field, prepare_upload};

    let topology = MeshTopology::build(&mesh.triangles);
    let mut upload = prepare_upload(mesh, &topology);
    if let Some(field) = field {
        let values = prepare_field(field);
        match field.association {
            ScalarAssociation::Vertex => upload.values_f32 = Some(values),
            ScalarAssociation::Cell => upload.cell_values_f32 = Some(values),
        }
    }
    upload
}

#[derive(Clone)]
pub struct MeshPlotState {
    pub interaction: ChartInteraction,
    pub hover: Option<MeshPlotPick>,
    pub selection: Option<MeshPlotPick>,
    pub geometry_revision: u64,
    pub field_revision: u64,
    field_values: Vec<f32>,
    /// Current plot style as seen by retained toolbar/render callbacks.
    pub wireframe: super::Wireframe,
    pub render_mode: super::MeshRenderMode,
    pub color_range: crate::ColorRange,
    timing: MeshPlotTimingStats,
    /// The planar picker uses projected coordinates, so its accelerator is
    /// retained independently from the 3D BVH.  Besides the host geometry
    /// revision, retain the source buffer identities and projection axes:
    /// native callers can otherwise rebuild with a new mesh without first
    /// going through the revisioned Python cache.
    retained_planar_index: Option<(
        u64,
        CoordinateAxis,
        CoordinateAxis,
        usize,
        usize,
        Rc<TriGridIndex>,
    )>,
    retained_contours: Option<RetainedContours>,
    contour_preparation_inflight: Option<ContourPreparationKey>,
    #[cfg(feature = "gpu-3d")]
    pub camera: Camera3D,
    #[cfg(feature = "gpu-3d")]
    pub orbit: OrbitControls,
    #[cfg(feature = "gpu-3d")]
    pub camera_fitted: bool,
    #[cfg(feature = "gpu-3d")]
    retained_bvh: Option<(u64, Rc<MeshBvh>)>,
    /// Axisymmetric geometry has a different topology from its source
    /// profile. Keep both it and its accelerator together so repeated pointer
    /// inspection never regenerates the revolution surface.
    #[cfg(feature = "gpu-3d")]
    retained_revolve: Option<RetainedRevolve>,
    /// One field derivative for the retained revolved geometry. Its identity
    /// includes both the host field revision and the immutable Arc backing
    /// stores so native callers that do not use the Python cache cannot reuse
    /// stale values accidentally.
    #[cfg(feature = "gpu-3d")]
    retained_revolved_field: Option<RetainedRevolvedField>,
    /// At most one complete revolve/derived-field preparation may be queued
    /// for this retained plot. Replacement requests supersede the key, so
    /// late worker results cannot overwrite the current scene.
    #[cfg(feature = "gpu-3d")]
    revolve_preparation_inflight: Option<RevolvePreparationKey>,
    #[cfg(all(feature = "gpu-3d", not(test)))]
    pub(crate) retained_3d: Option<RetainedMesh3D>,
}

/// The subset of retained plot state that native declarative hosts update
/// while constructing a new element. It intentionally excludes geometry,
/// field buffers, BVHs, timing counters, and prepared draw results so a
/// failed builder validation can roll back configuration without cloning the
/// expensive retained resources.
#[derive(Clone)]
pub struct MeshPlotStateConfiguration {
    interaction: ChartInteraction,
    selection: Option<MeshPlotPick>,
    wireframe: super::Wireframe,
    render_mode: super::MeshRenderMode,
    color_range: crate::ColorRange,
    retained_contours: Option<RetainedContours>,
    contour_preparation_inflight: Option<ContourPreparationKey>,
    #[cfg(feature = "gpu-3d")]
    camera: Camera3D,
    #[cfg(feature = "gpu-3d")]
    orbit: OrbitControls,
    #[cfg(feature = "gpu-3d")]
    camera_fitted: bool,
}

impl MeshPlotState {
    pub fn new(x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> Self {
        #[cfg(feature = "gpu-3d")]
        let orbit = OrbitControls::default();
        Self {
            interaction: ChartInteraction::new(x_min, x_max, y_min, y_max),
            hover: None,
            selection: None,
            geometry_revision: 0,
            field_revision: 0,
            field_values: Vec::new(),
            wireframe: super::Wireframe::Overlay,
            render_mode: super::MeshRenderMode::Mesh,
            color_range: crate::ColorRange::Auto,
            timing: MeshPlotTimingStats::default(),
            retained_planar_index: None,
            retained_contours: None,
            contour_preparation_inflight: None,
            #[cfg(feature = "gpu-3d")]
            camera: orbit.to_camera(),
            #[cfg(feature = "gpu-3d")]
            orbit,
            #[cfg(feature = "gpu-3d")]
            camera_fitted: false,
            #[cfg(feature = "gpu-3d")]
            retained_bvh: None,
            #[cfg(feature = "gpu-3d")]
            retained_revolve: None,
            #[cfg(feature = "gpu-3d")]
            retained_revolved_field: None,
            #[cfg(feature = "gpu-3d")]
            revolve_preparation_inflight: None,
            #[cfg(all(feature = "gpu-3d", not(test)))]
            retained_3d: None,
        }
    }

    /// Capture only the configuration fields changed by a native declarative
    /// build. The snapshot is cheap even for a large retained mesh because it
    /// shares prepared contour buffers and excludes geometry/BVH ownership.
    pub fn configuration_snapshot(&self) -> MeshPlotStateConfiguration {
        MeshPlotStateConfiguration {
            interaction: self.interaction.clone(),
            selection: self.selection.clone(),
            wireframe: self.wireframe,
            render_mode: self.render_mode.clone(),
            color_range: self.color_range.clone(),
            retained_contours: self.retained_contours.clone(),
            contour_preparation_inflight: self.contour_preparation_inflight.clone(),
            #[cfg(feature = "gpu-3d")]
            camera: self.camera.clone(),
            #[cfg(feature = "gpu-3d")]
            orbit: self.orbit.clone(),
            #[cfg(feature = "gpu-3d")]
            camera_fitted: self.camera_fitted,
        }
    }

    /// Restore a configuration snapshot after a declarative build fails.
    /// Retained geometry, field revisions, BVHs, and prepared GPU resources
    /// are deliberately untouched by this operation.
    pub fn restore_configuration(&mut self, snapshot: MeshPlotStateConfiguration) {
        self.interaction = snapshot.interaction;
        self.selection = snapshot.selection;
        self.wireframe = snapshot.wireframe;
        self.render_mode = snapshot.render_mode;
        self.color_range = snapshot.color_range;
        self.retained_contours = snapshot.retained_contours;
        self.contour_preparation_inflight = snapshot.contour_preparation_inflight;
        #[cfg(feature = "gpu-3d")]
        {
            self.camera = snapshot.camera;
            self.orbit = snapshot.orbit;
            self.camera_fitted = snapshot.camera_fitted;
        }
    }

    /// Snapshot retained 3D upload ownership for runtime diagnostics.
    ///
    /// Returns `None` until a live 3D frame has created its retained scene.
    #[cfg(all(feature = "gpu-3d", not(test)))]
    pub fn retained_3d_stats(&self) -> Option<RetainedMesh3DStats> {
        let retained = self.retained_3d.as_ref()?;
        let scene = retained.scene.borrow();
        Some(RetainedMesh3DStats {
            geometry_revision: retained.geometry_revision,
            scene_identity: Rc::as_ptr(&retained.scene) as usize,
            geometry_upload_count: scene.geometry_upload_count,
            geometry_upload_bytes: scene.geometry_upload_bytes,
            geometry_resident_bytes: scene
                .upload
                .as_ref()
                .map_or(0, d3rs::mesh::MeshUpload::geometry_byte_len),
            field_write_count: scene.field_write_count,
            field_write_bytes: scene.field_write_bytes,
            field_resident_bytes: scene
                .upload
                .as_ref()
                .map_or(0, d3rs::mesh::MeshUpload::field_byte_len),
            field_capacity_bytes: scene
                .upload
                .as_ref()
                .map_or(0, d3rs::mesh::MeshUpload::field_capacity_byte_len),
            gpu_geometry_upload_count: scene.gpu_geometry_upload_count,
            gpu_geometry_upload_bytes: scene.gpu_geometry_upload_bytes,
            gpu_field_write_count: scene.gpu_field_write_count,
            gpu_field_write_bytes: scene.gpu_field_write_bytes,
            gpu_field_capacity_bytes: scene.gpu_field_capacity_bytes,
            gpu_resident_bytes: scene.gpu_resident_bytes,
        })
    }

    /// Return non-I/O timing counters collected by this retained plot owner.
    pub fn timing_stats(&self) -> MeshPlotTimingStats {
        self.timing
    }

    pub(crate) fn record_contour_preparation(&mut self, duration: Duration) {
        self.timing.record_contour_preparation(duration);
    }

    #[cfg(feature = "gpu-3d")]
    pub(crate) fn record_revolve_preparation(&mut self, duration: Duration) {
        self.timing.record_revolve_preparation(duration);
    }

    pub(crate) fn record_pick(&mut self, duration: Duration) {
        self.timing.record_pick(duration);
    }

    /// Apply independently-versioned native resource changes from a retained
    /// host cache. Geometry changes invalidate the renderer; field/style and
    /// camera changes retain it.
    pub fn mark_resources_changed(&mut self, geometry_changed: bool, field_changed: bool) {
        if geometry_changed {
            self.geometry_revision = self.geometry_revision.saturating_add(1).max(1);
            self.retained_planar_index = None;
            self.contour_preparation_inflight = None;
            #[cfg(all(feature = "gpu-3d", not(test)))]
            {
                self.retained_3d = None;
            }
            #[cfg(feature = "gpu-3d")]
            {
                self.retained_bvh = None;
                self.retained_revolve = None;
                self.retained_revolved_field = None;
                self.revolve_preparation_inflight = None;
            }
        }
        if field_changed {
            self.field_revision = self.field_revision.saturating_add(1).max(1);
            self.contour_preparation_inflight = None;
            #[cfg(feature = "gpu-3d")]
            {
                self.retained_revolved_field = None;
                self.revolve_preparation_inflight = None;
            }
        }
    }

    /// Return cached contour bands/isolines only when every preparation input
    /// is unchanged. Style-only changes intentionally retain this result.
    pub(crate) fn cached_contours(
        &self,
        mesh: &TriangleMesh,
        field: Option<&ScalarField>,
        horizontal: CoordinateAxis,
        vertical: CoordinateAxis,
        mode: &super::MeshRenderMode,
        range: Option<[f64; 2]>,
    ) -> Option<RetainedContourOutput> {
        let cached = self.retained_contours.as_ref()?;
        let values_ptr = field.map(|field| field.values.as_ptr() as usize);
        let valid_ptr = field
            .and_then(|field| field.valid.as_ref())
            .map(|valid| valid.as_ptr() as usize);
        let association = field.map(|field| field.association);
        (cached.geometry_revision == self.geometry_revision.max(1)
            && cached.field_revision == self.field_revision
            && cached.positions_ptr == mesh.positions.as_ptr() as usize
            && cached.triangles_ptr == mesh.triangles.as_ptr() as usize
            && cached.field_values_ptr == values_ptr
            && cached.valid_ptr == valid_ptr
            && cached.association == association
            && cached.horizontal == horizontal
            && cached.vertical == vertical
            && cached.mode == *mode
            && cached.range == range)
            .then(|| (cached.bands.clone(), cached.lines.clone()))
    }

    /// Atomically replace the prepared contour result after all bands and
    /// isolines for one complete input revision have been produced.
    pub(crate) fn store_contours(
        &mut self,
        mesh: &TriangleMesh,
        field: Option<&ScalarField>,
        horizontal: CoordinateAxis,
        vertical: CoordinateAxis,
        mode: &super::MeshRenderMode,
        range: Option<[f64; 2]>,
        bands: Rc<Vec<ContourBand>>,
        lines: Rc<Vec<IsolineSegment>>,
    ) {
        self.retained_contours = Some(RetainedContours {
            geometry_revision: self.geometry_revision.max(1),
            field_revision: self.field_revision,
            positions_ptr: mesh.positions.as_ptr() as usize,
            triangles_ptr: mesh.triangles.as_ptr() as usize,
            field_values_ptr: field.map(|field| field.values.as_ptr() as usize),
            valid_ptr: field
                .and_then(|field| field.valid.as_ref())
                .map(|valid| valid.as_ptr() as usize),
            association: field.map(|field| field.association),
            horizontal,
            vertical,
            mode: mode.clone(),
            range,
            bands,
            lines,
        });
    }

    fn contour_preparation_key(
        &self,
        mesh: &TriangleMesh,
        field: Option<&ScalarField>,
        horizontal: CoordinateAxis,
        vertical: CoordinateAxis,
        mode: &super::MeshRenderMode,
        range: Option<[f64; 2]>,
    ) -> ContourPreparationKey {
        ContourPreparationKey {
            geometry_revision: self.geometry_revision.max(1),
            field_revision: self.field_revision,
            positions_ptr: mesh.positions.as_ptr() as usize,
            triangles_ptr: mesh.triangles.as_ptr() as usize,
            field_values_ptr: field.map(|field| field.values.as_ptr() as usize),
            valid_ptr: field
                .and_then(|field| field.valid.as_ref())
                .map(|valid| valid.as_ptr() as usize),
            association: field.map(|field| field.association),
            horizontal,
            vertical,
            mode: mode.clone(),
            range,
        }
    }

    /// Return the last complete contour frame even when it belongs to an older
    /// revision. The live renderer uses it while a newer background result is
    /// still preparing, avoiding a blank or partially-updated plot.
    pub(crate) fn previous_contours(&self) -> Option<RetainedContourOutput> {
        self.retained_contours
            .as_ref()
            .map(|cached| (cached.bands.clone(), cached.lines.clone()))
    }

    /// Mark a complete input revision as being prepared. Returns its key only
    /// for a newly-started task, so repeated render passes cannot queue the
    /// same work repeatedly.
    pub(crate) fn begin_contour_preparation(
        &mut self,
        mesh: &TriangleMesh,
        field: Option<&ScalarField>,
        horizontal: CoordinateAxis,
        vertical: CoordinateAxis,
        mode: &super::MeshRenderMode,
        range: Option<[f64; 2]>,
    ) -> Option<ContourPreparationKey> {
        let key = self.contour_preparation_key(mesh, field, horizontal, vertical, mode, range);
        if self.contour_preparation_inflight.as_ref() == Some(&key) {
            return None;
        }
        self.contour_preparation_inflight = Some(key.clone());
        Some(key)
    }

    /// Accept a worker result only when it still corresponds to the latest
    /// request. A newer geometry/field/style mutation clears or replaces the
    /// in-flight key before this method can store stale draw data.
    pub(crate) fn finish_contour_preparation(&mut self, key: &ContourPreparationKey) -> bool {
        if self.contour_preparation_inflight.as_ref() != Some(key) {
            return false;
        }
        self.contour_preparation_inflight = None;
        true
    }

    pub(crate) fn cancel_contour_preparation(&mut self) {
        self.contour_preparation_inflight = None;
    }

    /// Return the retained planar spatial index for native hover/click
    /// inspection. Field and style changes deliberately do not invalidate the
    /// index; a geometry replacement or a different projected plane does.
    pub(crate) fn planar_index_for(
        &mut self,
        projected: &[[f64; 2]],
        mesh: &TriangleMesh,
        horizontal: CoordinateAxis,
        vertical: CoordinateAxis,
    ) -> Rc<TriGridIndex> {
        let revision = self.geometry_revision.max(1);
        let positions_ptr = mesh.positions.as_ptr() as usize;
        let triangles_ptr = mesh.triangles.as_ptr() as usize;
        if let Some((
            cached_revision,
            cached_horizontal,
            cached_vertical,
            cached_positions,
            cached_triangles,
            index,
        )) = &self.retained_planar_index
            && *cached_revision == revision
            && *cached_horizontal == horizontal
            && *cached_vertical == vertical
            && *cached_positions == positions_ptr
            && *cached_triangles == triangles_ptr
        {
            return index.clone();
        }

        let index = Rc::new(TriGridIndex::build(projected, &mesh.triangles));
        self.retained_planar_index = Some((
            revision,
            horizontal,
            vertical,
            positions_ptr,
            triangles_ptr,
            index.clone(),
        ));
        index
    }

    #[cfg(feature = "gpu-3d")]
    /// Return a geometry-revision-keyed BVH for live 3D picking.
    pub(crate) fn bvh_for(&mut self, mesh: &TriangleMesh) -> Rc<MeshBvh> {
        let revision = self.geometry_revision.max(1);
        if let Some((cached_revision, bvh)) = &self.retained_bvh
            && *cached_revision == revision
        {
            return bvh.clone();
        }
        let bvh = Rc::new(MeshBvh::build(mesh));
        self.retained_bvh = Some((revision, bvh.clone()));
        bvh
    }

    #[cfg(feature = "gpu-3d")]
    fn revolve_preparation_key(
        &self,
        mesh: &TriangleMesh,
        spec: &RevolveSpec,
        field: Option<&ScalarField>,
    ) -> RevolvePreparationKey {
        RevolvePreparationKey {
            geometry_revision: self.geometry_revision.max(1),
            field_revision: self.field_revision,
            positions_ptr: mesh.positions.as_ptr() as usize,
            triangles_ptr: mesh.triangles.as_ptr() as usize,
            field_values_ptr: field.map(|field| field.values.as_ptr() as usize),
            valid_ptr: field
                .and_then(|field| field.valid.as_ref())
                .map(|valid| valid.as_ptr() as usize),
            spec: spec.clone(),
        }
    }

    /// Start preparation only once for an exact source/spec/field revision.
    /// This is deliberately state-only: the GPUI live owner chooses when to
    /// dispatch it and atomically accepts the result on its UI thread.
    #[cfg(feature = "gpu-3d")]
    pub(crate) fn begin_revolve_preparation(
        &mut self,
        mesh: &TriangleMesh,
        spec: &RevolveSpec,
        field: Option<&ScalarField>,
    ) -> Option<RevolvePreparationKey> {
        let key = self.revolve_preparation_key(mesh, spec, field);
        if self.revolve_preparation_inflight.as_ref() == Some(&key) {
            return None;
        }
        self.revolve_preparation_inflight = Some(key.clone());
        Some(key)
    }

    /// Return true only for the still-current request. This rejects stale
    /// background work after geometry/field replacement and clears the
    /// duplicate-suppression marker before the caller stores its full result.
    #[cfg(feature = "gpu-3d")]
    pub(crate) fn finish_revolve_preparation(&mut self, key: &RevolvePreparationKey) -> bool {
        if self.revolve_preparation_inflight.as_ref() != Some(key) {
            return false;
        }
        self.revolve_preparation_inflight = None;
        true
    }

    #[cfg(feature = "gpu-3d")]
    pub(crate) fn revolve_preparation_pending(
        &self,
        mesh: &TriangleMesh,
        spec: &RevolveSpec,
        field: Option<&ScalarField>,
    ) -> bool {
        self.revolve_preparation_inflight.as_ref()
            == Some(&self.revolve_preparation_key(mesh, spec, field))
    }

    /// Promote one fully-prepared worker result into the retained caches.
    /// The caller must first call [`Self::finish_revolve_preparation`]; the
    /// key is retained as a final guard against accidental source mismatch.
    #[cfg(feature = "gpu-3d")]
    pub(crate) fn store_prepared_revolve(
        &mut self,
        key: &RevolvePreparationKey,
        mesh: &TriangleMesh,
        field: Option<&ScalarField>,
        prepared: PreparedRevolve,
    ) -> bool {
        if self.revolve_preparation_key(mesh, &key.spec, field) != *key {
            return false;
        }
        let revolved = Rc::new(prepared.revolved);
        let bvh = Rc::new(prepared.bvh);
        self.retained_revolve = Some((
            key.geometry_revision,
            key.positions_ptr,
            key.triangles_ptr,
            key.spec.clone(),
            revolved,
            bvh,
        ));
        self.retained_revolved_field = prepared.field.map(|field| {
            (
                key.geometry_revision,
                key.field_revision,
                key.field_values_ptr.unwrap_or_default(),
                key.valid_ptr,
                Rc::new(field),
            )
        });
        true
    }

    /// Whether the requested derived geometry (and, when supplied, derived
    /// scalar field) is already fully available without invoking `revolve`.
    #[cfg(feature = "gpu-3d")]
    pub(crate) fn has_prepared_revolve(
        &self,
        mesh: &TriangleMesh,
        spec: &RevolveSpec,
        field: Option<&ScalarField>,
    ) -> bool {
        let geometry_ready = self.retained_revolve.as_ref().is_some_and(
            |(revision, positions_ptr, triangles_ptr, cached_spec, _, _)| {
                *revision == self.geometry_revision.max(1)
                    && *positions_ptr == mesh.positions.as_ptr() as usize
                    && *triangles_ptr == mesh.triangles.as_ptr() as usize
                    && cached_spec == spec
            },
        );
        geometry_ready
            && field.is_none_or(|field| {
                self.retained_revolved_field.as_ref().is_some_and(
                    |(geometry, field_revision, values_ptr, valid_ptr, _)| {
                        *geometry == self.geometry_revision.max(1)
                            && *field_revision == self.field_revision
                            && *values_ptr == field.values.as_ptr() as usize
                            && *valid_ptr
                                == field.valid.as_ref().map(|valid| valid.as_ptr() as usize)
                    },
                )
            })
    }

    #[cfg(feature = "gpu-3d")]
    /// Return the retained revolved geometry and matching BVH for the current
    /// source-geometry revision. A changed revolve specification is itself a
    /// geometry change for this derived product.
    pub(crate) fn revolved_bvh_for(
        &mut self,
        mesh: &TriangleMesh,
        spec: &RevolveSpec,
    ) -> Result<(Rc<RevolvedMesh>, Rc<MeshBvh>), d3rs::mesh::MeshValidationError> {
        let revision = self.geometry_revision.max(1);
        if let Some((cached_revision, positions_ptr, triangles_ptr, cached_spec, revolved, bvh)) =
            &self.retained_revolve
            && *cached_revision == revision
            && *positions_ptr == mesh.positions.as_ptr() as usize
            && *triangles_ptr == mesh.triangles.as_ptr() as usize
            && cached_spec == spec
        {
            return Ok((revolved.clone(), bvh.clone()));
        }
        let revolved = Rc::new(d3rs::mesh::revolve(mesh, spec)?);
        let bvh = Rc::new(MeshBvh::build(&revolved.mesh));
        self.retained_revolve = Some((
            revision,
            mesh.positions.as_ptr() as usize,
            mesh.triangles.as_ptr() as usize,
            spec.clone(),
            revolved.clone(),
            bvh.clone(),
        ));
        Ok((revolved, bvh))
    }

    #[cfg(feature = "gpu-3d")]
    /// Return a field replicated onto the retained revolution surface. The
    /// source field's immutable buffers are part of the cache identity, which
    /// makes native retained-state callers safe even before they explicitly
    /// advance a field revision.
    pub(crate) fn revolved_field_for(
        &mut self,
        mesh: &TriangleMesh,
        spec: &RevolveSpec,
        field: &ScalarField,
    ) -> Result<Rc<ScalarField>, d3rs::mesh::MeshValidationError> {
        let (revolved, _) = self.revolved_bvh_for(mesh, spec)?;
        let geometry_revision = self.geometry_revision.max(1);
        let field_revision = self.field_revision;
        let values_ptr = field.values.as_ptr() as usize;
        let valid_ptr = field.valid.as_ref().map(|valid| valid.as_ptr() as usize);
        if let Some((cached_geometry, cached_field, cached_values, cached_valid, derived)) =
            &self.retained_revolved_field
            && *cached_geometry == geometry_revision
            && *cached_field == field_revision
            && *cached_values == values_ptr
            && *cached_valid == valid_ptr
        {
            return Ok(derived.clone());
        }
        let derived = Rc::new(super::picking3d::revolved_field(field, &revolved));
        self.retained_revolved_field = Some((
            geometry_revision,
            field_revision,
            values_ptr,
            valid_ptr,
            derived.clone(),
        ));
        Ok(derived)
    }
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Replace the retained hover result without affecting click selection.
    pub fn set_hover(&mut self, pick: Option<MeshPlotPick>) {
        self.hover = pick;
    }

    /// Replace the retained click selection.
    pub fn set_selection(&mut self, pick: Option<MeshPlotPick>) {
        self.selection = pick;
    }

    /// Pick one plot-relative data point and update hover/selection state.
    ///
    /// The caller supplies the retained spatial index so pointer motion does
    /// not rebuild an accelerator or allocate a candidate list.
    pub fn pick_at(
        &mut self,
        mesh: &TriangleMesh,
        field: Option<&ScalarField>,
        index: &TriGridIndex,
        horizontal: CoordinateAxis,
        vertical: CoordinateAxis,
        point_2d: [f64; 2],
        plot_id: &str,
        select: bool,
    ) -> Option<MeshPlotPick> {
        let started = Instant::now();
        let pick = super::super::mesh_plot::picking::pick_2d(
            mesh, field, index, horizontal, vertical, point_2d, plot_id,
        );
        self.record_pick(started.elapsed());
        self.hover = pick.clone();
        if select {
            self.selection = pick.clone();
        }
        pick
    }

    /// Human-readable tooltip text for the current hover target.
    pub fn hover_tooltip(&self) -> Option<String> {
        self.hover_tooltip_with_field(None)
    }

    /// Format the native tooltip payload with coordinates, IDs, field label,
    /// value, and unit. The field is borrowed so hover updates do not clone
    /// large scalar arrays.
    pub fn hover_tooltip_with_field(&self, field: Option<&ScalarField>) -> Option<String> {
        self.hover.as_ref().map(|pick| {
            let cell = pick
                .cell_id
                .map_or_else(String::new, |id| format!(" (id {id})"));
            let vertex = pick
                .vertex_id
                .map_or_else(String::new, |id| format!("; vertex id {id}"));
            let value = pick.displayed_value.map_or_else(String::new, |value| {
                let label = field.map_or("Value", |field| field.label.as_ref());
                let unit = field
                    .and_then(|field| field.unit.as_deref())
                    .map_or(String::new(), |unit| format!(" {unit}"));
                format!("; {label} {value:.6}{unit}")
            });
            format!(
                "({:.6}, {:.6}, {:.6}); Cell {}{cell}{vertex}{value}",
                pick.world_position[0],
                pick.world_position[1],
                pick.world_position[2],
                pick.cell_index,
            )
        })
    }

    /// Configure the style values shared by the native toolbar and retained
    /// render callbacks.
    pub fn set_style(
        &mut self,
        mode: super::MeshRenderMode,
        wireframe: super::Wireframe,
        color_range: crate::ColorRange,
    ) {
        self.render_mode = mode;
        self.wireframe = wireframe;
        self.color_range = color_range;
        self.contour_preparation_inflight = None;
    }

    /// Set the render mode selected by the native toolbar. A pending contour
    /// task is invalid once its level/mode policy changes.
    pub(crate) fn set_render_mode(&mut self, mode: super::MeshRenderMode) {
        if self.render_mode != mode {
            self.render_mode = mode;
            self.cancel_contour_preparation();
        }
    }

    /// Toggle the retained wireframe preference and return its new value.
    pub fn toggle_wireframe(&mut self) -> super::Wireframe {
        self.wireframe = match self.wireframe {
            super::Wireframe::Overlay => super::Wireframe::Hidden,
            super::Wireframe::Hidden => super::Wireframe::Overlay,
        };
        self.wireframe
    }

    /// Restore automatic scalar-range selection for retained renderers.
    pub fn reset_color_range(&mut self) {
        self.color_range = crate::ColorRange::Auto;
    }

    /// Apply a keyboard navigation action while retaining selection/hover.
    pub fn handle_key(&mut self, key: &str) -> bool {
        self.handle_key_with_permissions(key, true, true, true)
    }

    /// Apply a keyboard navigation action subject to the plot's declared
    /// interaction capabilities.
    pub fn handle_key_with_permissions(
        &mut self,
        key: &str,
        allow_pan: bool,
        allow_zoom: bool,
        allow_reset: bool,
    ) -> bool {
        let Some(action) = crate::interaction::keyboard_action_for_key(key) else {
            return false;
        };
        use crate::interaction::ChartKeyboardAction;
        match action {
            ChartKeyboardAction::ZoomIn if allow_zoom => {
                self.interaction.zoom_around_pixel(300.0, 200.0, 0.8)
            }
            ChartKeyboardAction::ZoomOut if allow_zoom => {
                self.interaction.zoom_around_pixel(300.0, 200.0, 1.25)
            }
            ChartKeyboardAction::PanLeft if allow_pan => self.interaction.pan_by_pixels(-24.0, 0.0),
            ChartKeyboardAction::PanRight if allow_pan => self.interaction.pan_by_pixels(24.0, 0.0),
            ChartKeyboardAction::PanUp if allow_pan => self.interaction.pan_by_pixels(0.0, -24.0),
            ChartKeyboardAction::PanDown if allow_pan => self.interaction.pan_by_pixels(0.0, 24.0),
            ChartKeyboardAction::ResetZoom if allow_reset => self.interaction.reset_zoom(),
            _ => return false,
        }
        true
    }

    /// Reserve the retained field buffer before entering the render loop.
    pub fn reserve_field_capacity(&mut self, capacity: usize) {
        self.field_values
            .reserve(capacity.saturating_sub(self.field_values.len()));
    }

    /// Replace field values while preserving viewport and selection state.
    ///
    /// Revisions are monotonic: a late worker result cannot overwrite a newer
    /// field. Replaying the exact current revision is idempotent, while a
    /// different payload at that revision is rejected as ambiguous.
    pub fn replace_field_values(&mut self, revision: u64, values: &[f32]) -> bool {
        if revision < self.field_revision {
            return false;
        }
        if revision == self.field_revision {
            return self.field_values.as_slice() == values;
        }
        self.field_values.clear();
        self.field_values.extend_from_slice(values);
        self.field_revision = revision;
        #[cfg(feature = "gpu-3d")]
        {
            self.retained_revolved_field = None;
        }
        true
    }

    /// Return the retained field values for a backend upload.
    pub fn field_values(&self) -> &[f32] {
        &self.field_values
    }

    /// Apply camera navigation without growing zoom history.
    pub fn set_viewport_without_history(&mut self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) {
        self.interaction
            .set_viewport_without_history(x_min, x_max, y_min, y_max);
    }

    #[cfg(feature = "gpu-3d")]
    /// Rotate the retained 3D camera and preserve its current target/distance.
    pub fn orbit_rotate(&mut self, delta_x: f32, delta_y: f32) {
        self.orbit.rotate(delta_x, delta_y);
        self.orbit.update_camera(&mut self.camera);
    }

    #[cfg(feature = "gpu-3d")]
    /// Zoom the retained 3D camera around its orbit target.
    pub fn orbit_zoom(&mut self, delta: f32) {
        self.orbit.zoom(delta);
        self.orbit.update_camera(&mut self.camera);
    }

    #[cfg(feature = "gpu-3d")]
    /// Pan the retained 3D orbit target in camera-relative screen space.
    pub fn orbit_pan(&mut self, delta_x: f32, delta_y: f32) {
        self.orbit.pan(delta_x, delta_y, &self.camera);
        self.orbit.update_camera(&mut self.camera);
    }

    #[cfg(feature = "gpu-3d")]
    /// Move to a conventional camera orientation while preserving the fitted
    /// target and distance.
    pub fn orbit_standard_view(&mut self, view: StandardView) {
        self.orbit.set_standard_view(view);
        self.orbit.update_camera(&mut self.camera);
    }

    #[cfg(feature = "gpu-3d")]
    /// Toggle perspective/orthographic projection without disturbing the
    /// retained orbit target or distance.
    pub fn toggle_projection(&mut self) {
        self.camera.toggle_projection();
    }

    #[cfg(feature = "gpu-3d")]
    /// Apply the shared chart keys to 3D orbit navigation. Arrow keys pan,
    /// plus/minus zoom, and Home/0/R restore the fitted camera.
    pub fn handle_3d_key(&mut self, key: &str) -> bool {
        self.handle_3d_key_with_permissions(key, true, true, true, true)
    }

    #[cfg(feature = "gpu-3d")]
    /// Apply a 3D keyboard action subject to the plot's declared capabilities.
    pub fn handle_3d_key_with_permissions(
        &mut self,
        key: &str,
        allow_pan: bool,
        allow_zoom: bool,
        allow_reset: bool,
        allow_fit: bool,
    ) -> bool {
        match key.to_ascii_lowercase().as_str() {
            "1" if allow_fit => self.orbit_standard_view(StandardView::Front),
            "2" if allow_fit => self.orbit_standard_view(StandardView::Back),
            "3" if allow_fit => self.orbit_standard_view(StandardView::Left),
            "4" if allow_fit => self.orbit_standard_view(StandardView::Right),
            "5" if allow_fit => self.orbit_standard_view(StandardView::Top),
            "6" if allow_fit => self.orbit_standard_view(StandardView::Bottom),
            "i" if allow_fit => self.orbit_standard_view(StandardView::Isometric),
            "p" if allow_fit => self.toggle_projection(),
            _ => {
                use crate::interaction::ChartKeyboardAction;
                let Some(action) = crate::interaction::keyboard_action_for_key(key) else {
                    return false;
                };
                match action {
                    ChartKeyboardAction::ZoomIn if allow_zoom => self.orbit_zoom(0.5),
                    ChartKeyboardAction::ZoomOut if allow_zoom => self.orbit_zoom(-0.5),
                    ChartKeyboardAction::PanLeft if allow_pan => self.orbit_pan(-24.0, 0.0),
                    ChartKeyboardAction::PanRight if allow_pan => self.orbit_pan(24.0, 0.0),
                    ChartKeyboardAction::PanUp if allow_pan => self.orbit_pan(0.0, -24.0),
                    ChartKeyboardAction::PanDown if allow_pan => self.orbit_pan(0.0, 24.0),
                    ChartKeyboardAction::ResetZoom if allow_reset => self.orbit_reset(),
                    _ => return false,
                }
            }
        }
        true
    }

    #[cfg(feature = "gpu-3d")]
    /// Restore the initial retained 3D camera.
    pub fn orbit_reset(&mut self) {
        self.orbit.reset();
        self.orbit.update_camera(&mut self.camera);
    }

    #[cfg(feature = "gpu-3d")]
    /// Update the camera aspect ratio after a plot resize.
    pub fn set_camera_aspect(&mut self, width: f32, height: f32) {
        if width.is_finite() && height.is_finite() && height > 0.0 {
            self.camera.aspect = (width / height).max(f32::EPSILON);
        }
    }

    #[cfg(feature = "gpu-3d")]
    /// Fit the retained orbit to a mesh and make that fit the reset state.
    pub fn fit_camera_to_bounds(&mut self, bounds: MeshBounds, viewport_aspect: f32) {
        let mut fitted = OrbitControls::default();
        fitted.fit_to_bounds(bounds, viewport_aspect);
        self.orbit = OrbitControls::default()
            .with_target(fitted.target)
            .with_position(
                fitted.distance,
                fitted.azimuth.to_degrees(),
                fitted.elevation.to_degrees(),
            );
        self.orbit.update_camera(&mut self.camera);
        self.camera_fitted = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh_plot::Wireframe;
    use std::sync::Arc;

    fn pick() -> MeshPlotPick {
        MeshPlotPick {
            plot_id: Arc::from("plot"),
            mesh_id: Arc::from("mesh"),
            cell_index: 3,
            cell_id: Some(42),
            nearest_vertex_index: Some(1),
            vertex_id: Some(7),
            world_position: [0.25, 0.5, 0.0],
            displayed_value: Some(2.5),
            field_id: Some(Arc::from("field")),
        }
    }

    #[test]
    fn keyboard_navigation_preserves_selection_and_updates_view() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        state.set_selection(Some(pick()));
        let before = state.interaction.x_domain();
        assert!(state.handle_key("+") || state.handle_key("equal"));
        assert_ne!(state.interaction.x_domain(), before);
        assert_eq!(
            state.selection.as_ref().and_then(|pick| pick.cell_id),
            Some(42)
        );
        assert_eq!(state.hover_tooltip().as_deref(), None);
        state.set_hover(state.selection.clone());
        assert!(
            state
                .hover_tooltip()
                .is_some_and(|text| text.contains("42"))
        );
    }

    #[test]
    fn keyboard_navigation_honors_declared_capabilities() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        let initial_x = state.interaction.x_domain();
        assert!(!state.handle_key_with_permissions("arrowleft", false, true, false));
        assert_eq!(state.interaction.x_domain(), initial_x);
        assert!(state.handle_key_with_permissions("+", false, true, false));
        assert_ne!(state.interaction.x_domain(), initial_x);
    }

    #[test]
    fn resource_dirty_domains_only_advance_their_matching_revision() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        state.mark_resources_changed(true, true);
        assert_eq!(state.geometry_revision, 1);
        assert_eq!(state.field_revision, 1);

        state.mark_resources_changed(false, true);
        assert_eq!(state.geometry_revision, 1);
        assert_eq!(state.field_revision, 2);

        state.mark_resources_changed(true, false);
        assert_eq!(state.geometry_revision, 2);
        assert_eq!(state.field_revision, 2);
    }

    #[test]
    fn wireframe_toggle_updates_retained_style() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        assert_eq!(state.toggle_wireframe(), Wireframe::Hidden);
        assert_eq!(state.toggle_wireframe(), Wireframe::Overlay);
    }

    #[test]
    fn color_range_reset_restores_automatic_selection() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        state.color_range = crate::ColorRange::Fixed {
            min: -12.0,
            max: 3.0,
        };
        state.reset_color_range();
        assert_eq!(state.color_range, crate::ColorRange::Auto);
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn orbit_navigation_changes_camera_without_touching_2d_state() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        let initial = state.camera.position;
        state.orbit_rotate(20.0, 5.0);
        assert_ne!(state.camera.position, initial);
        state.orbit_zoom(0.5);
        assert!(state.camera.position.is_finite());
        state.orbit_reset();
        assert_eq!(state.interaction.x_domain(), (0.0, 1.0));
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn three_d_keyboard_pan_and_zoom_update_only_the_camera() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        let initial_target = state.camera.target;
        let initial_distance = state.orbit.distance;
        assert!(state.handle_3d_key("left"));
        assert_ne!(state.camera.target, initial_target);
        assert!(state.handle_3d_key("+"));
        assert!(state.orbit.distance < initial_distance);
        assert_eq!(state.interaction.x_domain(), (0.0, 1.0));
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn three_d_standard_views_and_projection_toggle_update_the_camera() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        let initial = state.camera.view_projection_matrix();
        assert!(state.handle_3d_key("5"));
        assert_ne!(state.camera.view_projection_matrix(), initial);
        assert!(matches!(
            state.camera.projection(),
            d3rs::gpu3d::Projection::Perspective { .. }
        ));
        assert!(state.handle_3d_key("p"));
        assert!(matches!(
            state.camera.projection(),
            d3rs::gpu3d::Projection::Orthographic { .. }
        ));
        assert_eq!(state.interaction.x_domain(), (0.0, 1.0));
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn fit_camera_redefines_the_subsequent_reset_view() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        let bounds = MeshBounds {
            min: [-10.0, -2.0, -1.0],
            max: [10.0, 2.0, 1.0],
        };
        state.fit_camera_to_bounds(bounds, 16.0 / 9.0);
        let fitted_position = state.camera.position;
        let fitted_target = state.camera.target;
        state.orbit_rotate(40.0, -15.0);
        assert_ne!(state.camera.position, fitted_position);
        state.orbit_reset();
        assert_eq!(state.camera.position, fitted_position);
        assert_eq!(state.camera.target, fitted_target);
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn retained_bvh_is_reused_until_geometry_changes() {
        let mesh = TriangleMesh {
            id: "mesh".into(),
            positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        state.mark_resources_changed(true, false);
        let first = state.bvh_for(&mesh);
        let second = state.bvh_for(&mesh);
        assert!(Rc::ptr_eq(&first, &second));

        state.mark_resources_changed(true, false);
        let replaced = state.bvh_for(&mesh);
        assert!(!Rc::ptr_eq(&first, &replaced));
    }

    #[test]
    fn retained_planar_index_is_reused_for_field_updates_and_replaced_for_geometry() {
        let mesh = TriangleMesh {
            id: "mesh".into(),
            positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let projected = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        state.mark_resources_changed(true, false);
        let first = state.planar_index_for(&projected, &mesh, CoordinateAxis::X, CoordinateAxis::Y);
        state.mark_resources_changed(false, true);
        let after_field_update =
            state.planar_index_for(&projected, &mesh, CoordinateAxis::X, CoordinateAxis::Y);
        assert!(Rc::ptr_eq(&first, &after_field_update));

        state.mark_resources_changed(true, false);
        let after_geometry_update =
            state.planar_index_for(&projected, &mesh, CoordinateAxis::X, CoordinateAxis::Y);
        assert!(!Rc::ptr_eq(&first, &after_geometry_update));

        let rotated_projection = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0]];
        let after_view_change = state.planar_index_for(
            &rotated_projection,
            &mesh,
            CoordinateAxis::Y,
            CoordinateAxis::X,
        );
        assert!(!Rc::ptr_eq(&after_geometry_update, &after_view_change));
    }

    #[test]
    fn retained_contours_reuse_complete_results_and_invalidate_by_field_revision() {
        let mesh = TriangleMesh {
            id: "mesh".into(),
            positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let field = ScalarField {
            id: "field".into(),
            label: "Field".into(),
            unit: None,
            values: Arc::from([0.0, 0.5, 1.0]),
            association: ScalarAssociation::Vertex,
            valid: None,
        };
        let mode = crate::mesh_plot::MeshRenderMode::Isolines {
            levels: d3rs::mesh::ContourLevels::Count(4),
        };
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        state.mark_resources_changed(true, false);
        let bands = Rc::new(Vec::new());
        let lines = Rc::new(Vec::new());
        state.store_contours(
            &mesh,
            Some(&field),
            CoordinateAxis::X,
            CoordinateAxis::Y,
            &mode,
            Some([0.0, 1.0]),
            bands.clone(),
            lines.clone(),
        );
        let (cached_bands, cached_lines) = state
            .cached_contours(
                &mesh,
                Some(&field),
                CoordinateAxis::X,
                CoordinateAxis::Y,
                &mode,
                Some([0.0, 1.0]),
            )
            .expect("complete contour result must be retained");
        assert!(Rc::ptr_eq(&cached_bands, &bands));
        assert!(Rc::ptr_eq(&cached_lines, &lines));

        state.mark_resources_changed(false, true);
        assert!(
            state
                .cached_contours(
                    &mesh,
                    Some(&field),
                    CoordinateAxis::X,
                    CoordinateAxis::Y,
                    &mode,
                    Some([0.0, 1.0]),
                )
                .is_none()
        );
    }

    #[test]
    fn stale_contour_workers_cannot_commit_after_a_newer_field_revision() {
        let mesh = TriangleMesh {
            id: "mesh".into(),
            positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let field = ScalarField {
            id: "field".into(),
            label: "Field".into(),
            unit: None,
            values: Arc::from([0.0, 0.5, 1.0]),
            association: ScalarAssociation::Vertex,
            valid: None,
        };
        let mode = crate::mesh_plot::MeshRenderMode::Isolines {
            levels: d3rs::mesh::ContourLevels::Count(4),
        };
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);

        let first = state
            .begin_contour_preparation(
                &mesh,
                Some(&field),
                CoordinateAxis::X,
                CoordinateAxis::Y,
                &mode,
                Some([0.0, 1.0]),
            )
            .expect("first contour preparation should be scheduled");
        assert!(
            state
                .begin_contour_preparation(
                    &mesh,
                    Some(&field),
                    CoordinateAxis::X,
                    CoordinateAxis::Y,
                    &mode,
                    Some([0.0, 1.0]),
                )
                .is_none(),
            "identical contour work must not be queued twice"
        );

        state.mark_resources_changed(false, true);
        assert!(
            !state.finish_contour_preparation(&first),
            "a field revision must invalidate the older worker result"
        );
        assert!(state.previous_contours().is_none());

        let newer = state
            .begin_contour_preparation(
                &mesh,
                Some(&field),
                CoordinateAxis::X,
                CoordinateAxis::Y,
                &mode,
                Some([0.0, 1.0]),
            )
            .expect("newer contour preparation should be scheduled");
        assert!(state.finish_contour_preparation(&newer));
    }

    #[test]
    fn stale_field_replacements_cannot_overwrite_newer_results() {
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        assert!(state.replace_field_values(4, &[4.0, 5.0]));
        assert!(!state.replace_field_values(3, &[3.0, 4.0]));
        assert_eq!(state.field_revision, 4);
        assert_eq!(state.field_values(), &[4.0, 5.0]);

        assert!(state.replace_field_values(4, &[4.0, 5.0]));
        assert!(!state.replace_field_values(4, &[8.0, 9.0]));
        assert_eq!(state.field_values(), &[4.0, 5.0]);
    }

    #[test]
    fn timing_stats_track_retained_planar_picks() {
        let mesh = TriangleMesh {
            id: "mesh".into(),
            positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let projected = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        let index = TriGridIndex::build(&projected, &mesh.triangles);
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);

        assert!(
            state
                .pick_at(
                    &mesh,
                    None,
                    &index,
                    CoordinateAxis::X,
                    CoordinateAxis::Y,
                    [0.25, 0.25],
                    "plot",
                    false,
                )
                .is_some()
        );

        let stats = state.timing_stats();
        assert_eq!(stats.pick_count, 1);
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn timing_stats_track_revolve_preparation() {
        let mut stats = MeshPlotTimingStats::default();
        stats.record_revolve_preparation(Duration::from_nanos(17));
        assert_eq!(stats.revolve_preparation_count, 1);
        assert_eq!(stats.revolve_preparation_ns, 17);

        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        state.record_revolve_preparation(Duration::from_nanos(23));
        assert_eq!(state.timing_stats().revolve_preparation_count, 1);
        assert_eq!(state.timing_stats().revolve_preparation_ns, 23);
    }

    #[test]
    fn contour_preparation_is_revision_keyed_and_keeps_the_last_complete_frame() {
        let mesh = TriangleMesh {
            id: "mesh".into(),
            positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let field = ScalarField {
            id: "field".into(),
            label: "Field".into(),
            unit: None,
            values: Arc::from([0.0, 0.5, 1.0]),
            association: ScalarAssociation::Vertex,
            valid: None,
        };
        let mode = crate::mesh_plot::MeshRenderMode::Isolines {
            levels: d3rs::mesh::ContourLevels::Count(4),
        };
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        state.mark_resources_changed(true, false);
        let previous_bands = Rc::new(Vec::new());
        let previous_lines = Rc::new(Vec::new());
        state.store_contours(
            &mesh,
            Some(&field),
            CoordinateAxis::X,
            CoordinateAxis::Y,
            &mode,
            Some([0.0, 1.0]),
            previous_bands.clone(),
            previous_lines.clone(),
        );
        let key = state
            .begin_contour_preparation(
                &mesh,
                Some(&field),
                CoordinateAxis::X,
                CoordinateAxis::Y,
                &mode,
                Some([0.0, 1.0]),
            )
            .expect("first revision starts a task");
        assert!(
            state
                .begin_contour_preparation(
                    &mesh,
                    Some(&field),
                    CoordinateAxis::X,
                    CoordinateAxis::Y,
                    &mode,
                    Some([0.0, 1.0]),
                )
                .is_none(),
            "a render retry must not queue duplicate work"
        );

        state.mark_resources_changed(false, true);
        let (bands, lines) = state
            .previous_contours()
            .expect("an invalidated revision keeps its complete fallback frame");
        assert!(Rc::ptr_eq(&bands, &previous_bands));
        assert!(Rc::ptr_eq(&lines, &previous_lines));
        assert!(
            !state.finish_contour_preparation(&key),
            "a late result must not replace the newer field revision"
        );
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn revolve_preparation_rejects_duplicates_and_stale_results() {
        let mesh = TriangleMesh {
            id: "profile".into(),
            positions: Arc::from([
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ]),
            triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let field = ScalarField {
            id: "pressure".into(),
            label: "Pressure".into(),
            unit: None,
            values: Arc::from([0.0, 0.5, 1.0, 0.75]),
            association: ScalarAssociation::Vertex,
            valid: None,
        };
        let spec = RevolveSpec {
            radial: CoordinateAxis::X,
            axial: CoordinateAxis::Y,
            segments: 8,
            ..RevolveSpec::default()
        };
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        state.mark_resources_changed(true, true);
        let key = state
            .begin_revolve_preparation(&mesh, &spec, Some(&field))
            .expect("first request starts preparation");
        assert!(
            state
                .begin_revolve_preparation(&mesh, &spec, Some(&field))
                .is_none(),
            "repeated frames must not queue duplicate revolve work"
        );

        state.mark_resources_changed(false, true);
        assert!(
            !state.finish_revolve_preparation(&key),
            "a field replacement must reject a late derived field"
        );

        let current_key = state
            .begin_revolve_preparation(&mesh, &spec, Some(&field))
            .expect("new field revision starts a new request");
        assert!(state.finish_revolve_preparation(&current_key));
        let revolved = d3rs::mesh::revolve(&mesh, &spec).expect("valid profile");
        let prepared = PreparedRevolve {
            bvh: MeshBvh::build(&revolved.mesh),
            field: Some(crate::mesh_plot::picking3d::revolved_field(
                &field, &revolved,
            )),
            revolved,
        };
        assert!(state.store_prepared_revolve(&current_key, &mesh, Some(&field), prepared));
        assert!(state.retained_revolve.is_some());
        assert!(state.retained_revolved_field.is_some());
        assert!(state.has_prepared_revolve(&mesh, &spec, Some(&field)));

        let replacement_mesh = TriangleMesh {
            positions: Arc::from(mesh.positions.iter().copied().collect::<Vec<_>>()),
            triangles: Arc::from(mesh.triangles.iter().copied().collect::<Vec<_>>()),
            ..mesh.clone()
        };
        let replacement_field = ScalarField {
            values: Arc::from(field.values.iter().copied().collect::<Vec<_>>()),
            ..field.clone()
        };
        assert!(
            !state.has_prepared_revolve(&replacement_mesh, &spec, Some(&replacement_field)),
            "immutable buffer replacement must not reuse a same-revision derived mesh"
        );
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn retained_revolve_and_bvh_are_reused_until_geometry_changes() {
        let mesh = TriangleMesh {
            id: "profile".into(),
            positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        state.mark_resources_changed(true, false);
        let spec = RevolveSpec::default();
        let (first_mesh, first_bvh) = state.revolved_bvh_for(&mesh, &spec).unwrap();
        let (second_mesh, second_bvh) = state.revolved_bvh_for(&mesh, &spec).unwrap();
        assert!(Rc::ptr_eq(&first_mesh, &second_mesh));
        assert!(Rc::ptr_eq(&first_bvh, &second_bvh));

        state.mark_resources_changed(true, false);
        let (replaced_mesh, replaced_bvh) = state.revolved_bvh_for(&mesh, &spec).unwrap();
        assert!(!Rc::ptr_eq(&first_mesh, &replaced_mesh));
        assert!(!Rc::ptr_eq(&first_bvh, &replaced_bvh));
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn retained_revolved_field_is_reused_and_invalidated_by_field_revision() {
        let mesh = TriangleMesh {
            id: "profile".into(),
            positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]),
            triangles: Arc::from([[0, 1, 2]]),
            vertex_ids: None,
            cell_ids: None,
        };
        let field = ScalarField {
            id: "field".into(),
            label: "Field".into(),
            unit: None,
            values: Arc::from([1.0, 2.0, 3.0]),
            association: d3rs::mesh::ScalarAssociation::Vertex,
            valid: None,
        };
        let mut state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        state.mark_resources_changed(true, true);
        let spec = RevolveSpec::default();
        let first = state.revolved_field_for(&mesh, &spec, &field).unwrap();
        let second = state.revolved_field_for(&mesh, &spec, &field).unwrap();
        assert!(Rc::ptr_eq(&first, &second));

        state.mark_resources_changed(false, true);
        let replaced = state.revolved_field_for(&mesh, &spec, &field).unwrap();
        assert!(!Rc::ptr_eq(&first, &replaced));
    }

    #[cfg(feature = "gpu-3d")]
    #[test]
    fn retained_lod_swaps_proxy_only_for_drag_and_restores_full_upload() {
        let width = 5usize;
        let positions = (0..width)
            .flat_map(|y| (0..width).map(move |x| [x as f64, y as f64, 0.0]))
            .collect::<Vec<_>>();
        let mut triangles = Vec::new();
        for y in 0..width - 1 {
            for x in 0..width - 1 {
                let a = (y * width + x) as u32;
                let b = a + 1;
                let c = a + width as u32;
                let d = c + 1;
                triangles.extend([[a, b, c], [b, d, c]]);
            }
        }
        let mesh = TriangleMesh {
            id: "grid".into(),
            positions: positions.into(),
            triangles: triangles.into(),
            vertex_ids: None,
            cell_ids: None,
        };
        let field = ScalarField {
            id: "field".into(),
            label: "Field".into(),
            unit: None,
            values: (0..mesh.positions.len())
                .map(|index| index as f64)
                .collect(),
            association: d3rs::mesh::ScalarAssociation::Vertex,
            valid: None,
        };
        let full_upload = mesh_upload_with_field(&mesh, Some(&field));
        let mut scene = MeshSceneState {
            upload: Some(full_upload.clone()),
            ..MeshSceneState::default()
        };
        let mut lod = RetainedMeshLod::with_lod_threshold(mesh, Some(&field), 1);
        assert!(lod.begin_drag(&mut scene));
        let proxy_upload = scene.upload.as_ref().unwrap();
        assert!(proxy_upload.positions_f32.len() < full_upload.positions_f32.len());
        assert_eq!(
            proxy_upload.values_f32.as_ref().map(Vec::len),
            Some(proxy_upload.positions_f32.len())
        );
        assert!(lod.end_drag(&mut scene));
        assert_eq!(
            scene.upload.as_ref().unwrap().positions_f32,
            full_upload.positions_f32
        );
        assert_eq!(scene.geometry_rev.0, 2);
        assert_eq!(
            scene.geometry_upload_count, 2,
            "a drag must upload the proxy once and restore the full mesh once"
        );
        assert!(scene.geometry_upload_bytes >= full_upload.positions_f32.len() as u64);
    }
}
