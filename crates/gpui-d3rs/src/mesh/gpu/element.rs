//! GPUI element for a retained mesh scene.

use super::{FieldRevision, GeometryRevision, MeshColorConfig, MeshSceneState, render_offscreen};
use gpui::{
    App, Bounds, Corners, CustomDrawId, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, RenderImage, Size, Style, Window, relative,
};
use image::Frame;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::panic;
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// Maximum fallback images retained at once.
const OFFSCREEN_CACHE_CAPACITY: usize = 32;
/// Maximum retained RGBA bytes across all fallback images (64 MiB).
///
/// The count cap alone admitted 32 x 2048x2048 RGBA8 images (~512 MiB);
/// the byte budget is the binding constraint for large viewports.
const OFFSCREEN_CACHE_MAX_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, PartialEq)]
struct OffscreenCacheKey {
    geometry_revision: GeometryRevision,
    field_revision: FieldRevision,
    width: u32,
    height: u32,
    view_transform: [[f32; 4]; 4],
    color: MeshColorConfig,
}

struct OffscreenCacheEntry {
    state: Weak<RefCell<MeshSceneState>>,
    key: OffscreenCacheKey,
    image: Arc<RenderImage>,
}

/// Retained-byte and activity counters for the offscreen fallback cache.
#[cfg(test)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct OffscreenCacheStats {
    pub entries: usize,
    pub retained_bytes: usize,
    pub high_water_bytes: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

/// Byte-budgeted LRU of software fallback images (pretext pattern):
/// hits move to the back, admission evicts least-recently-used entries
/// until count and byte budgets hold, and a single image larger than
/// the byte budget is returned without retention.
#[derive(Default)]
struct OffscreenCache {
    entries: HashMap<usize, OffscreenCacheEntry>,
    order: VecDeque<usize>,
    retained_bytes: usize,
    high_water_bytes: usize,
    hits: u64,
    misses: u64,
    evictions: u64,
}

fn image_bytes(key: &OffscreenCacheKey) -> usize {
    (key.width as usize)
        .saturating_mul(key.height as usize)
        .saturating_mul(4)
}

impl OffscreenCache {
    fn get(&mut self, scene_id: usize, key: &OffscreenCacheKey) -> Option<Arc<RenderImage>> {
        let entry = self.entries.get(&scene_id)?;
        if entry.key != *key {
            return None;
        }
        let image = Arc::clone(&entry.image);
        self.hits = self.hits.saturating_add(1);
        self.order.retain(|id| *id != scene_id);
        self.order.push_back(scene_id);
        Some(image)
    }

    fn insert(&mut self, scene_id: usize, entry: OffscreenCacheEntry) {
        let bytes = image_bytes(&entry.key);
        if bytes > OFFSCREEN_CACHE_MAX_BYTES {
            self.evictions = self.evictions.saturating_add(1);
            return;
        }
        if let Some(old) = self.entries.remove(&scene_id) {
            self.retained_bytes = self.retained_bytes.saturating_sub(image_bytes(&old.key));
            self.order.retain(|id| *id != scene_id);
        }
        while !self.entries.is_empty()
            && (self.entries.len() >= OFFSCREEN_CACHE_CAPACITY
                || self.retained_bytes.saturating_add(bytes) > OFFSCREEN_CACHE_MAX_BYTES)
        {
            let Some(victim) = self.order.pop_front() else {
                break;
            };
            if let Some(evicted) = self.entries.remove(&victim) {
                self.retained_bytes = self
                    .retained_bytes
                    .saturating_sub(image_bytes(&evicted.key));
                self.evictions = self.evictions.saturating_add(1);
            }
        }
        self.retained_bytes = self.retained_bytes.saturating_add(bytes);
        self.high_water_bytes = self.high_water_bytes.max(self.retained_bytes);
        self.entries.insert(scene_id, entry);
        self.order.push_back(scene_id);
    }

    fn retain_live(&mut self) {
        let dead: Vec<usize> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.state.upgrade().is_none())
            .map(|(id, _)| *id)
            .collect();
        for id in dead {
            if let Some(entry) = self.entries.remove(&id) {
                self.retained_bytes = self.retained_bytes.saturating_sub(image_bytes(&entry.key));
                self.evictions = self.evictions.saturating_add(1);
            }
            self.order.retain(|candidate| *candidate != id);
        }
    }

    #[cfg(test)]
    fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.retained_bytes = 0;
    }

    #[cfg(test)]
    fn trim(&mut self) {
        self.clear();
        self.entries.shrink_to_fit();
        self.order.shrink_to_fit();
    }

    #[cfg(test)]
    fn stats(&self) -> OffscreenCacheStats {
        OffscreenCacheStats {
            entries: self.entries.len(),
            retained_bytes: self.retained_bytes,
            high_water_bytes: self.high_water_bytes,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
        }
    }
}

thread_local! {
    /// Element values are rebuilt by GPUI, while their retained scene state is
    /// stable. Keep fallback images beside that state without extending its
    /// lifetime or retaining stale scenes indefinitely.
    static OFFSCREEN_CACHE: RefCell<OffscreenCache> = RefCell::default();
}

/// Current retained-byte/activity counters of the fallback cache.
#[cfg(test)]
pub(crate) fn offscreen_cache_stats() -> OffscreenCacheStats {
    OFFSCREEN_CACHE.with(|cache| cache.borrow().stats())
}

/// Release every retained fallback image (memory-pressure hook).
#[cfg(test)]
pub(crate) fn trim_offscreen_cache() {
    OFFSCREEN_CACHE.with(|cache| cache.borrow_mut().trim());
}

fn cached_offscreen_image(
    scene: &Rc<RefCell<MeshSceneState>>,
    width: u32,
    height: u32,
) -> Arc<RenderImage> {
    let key = {
        let state = scene.borrow();
        OffscreenCacheKey {
            geometry_revision: state.geometry_rev,
            field_revision: state.field_rev,
            width,
            height,
            view_transform: state.view_transform,
            color: state.color.clone(),
        }
    };
    let scene_id = Rc::as_ptr(scene) as usize;

    OFFSCREEN_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.retain_live();
        if let Some(image) = cache.get(scene_id, &key) {
            return image;
        }
        cache.misses = cache.misses.saturating_add(1);

        let image = {
            let state = scene.borrow();
            Arc::new(RenderImage::new(vec![Frame::new(render_offscreen(
                state.upload.as_ref(),
                &state,
                width,
                height,
            ))]))
        };
        cache.insert(
            scene_id,
            OffscreenCacheEntry {
                state: Rc::downgrade(scene),
                key,
                image: Arc::clone(&image),
            },
        );
        image
    })
}

#[derive(Clone)]
pub struct MeshSceneElement {
    state: Rc<RefCell<MeshSceneState>>,
    custom_id: Option<CustomDrawId>,
}

impl MeshSceneElement {
    pub fn new(state: Rc<RefCell<MeshSceneState>>) -> Self {
        Self {
            state,
            custom_id: None,
        }
    }

    pub fn with_custom_id(mut self, custom_id: CustomDrawId) -> Self {
        self.custom_id = Some(custom_id);
        self
    }

    pub fn state(&self) -> Rc<RefCell<MeshSceneState>> {
        self.state.clone()
    }
}

impl IntoElement for MeshSceneElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for MeshSceneElement {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (
            window.request_layout(
                Style {
                    size: Size {
                        width: relative(1.0).into(),
                        height: relative(1.0).into(),
                    },
                    ..Default::default()
                },
                [],
                cx,
            ),
            (),
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut (),
        (): &mut (),
        window: &mut Window,
        _: &mut App,
    ) {
        if let Some(id) = self.custom_id {
            window.paint_custom(id, bounds);
        } else {
            // Keep the element useful on platforms without a registered
            // zero-copy backend. The fallback is deliberately rendered from
            // the retained upload and painted as one GPUI image.
            let width = f32::from(bounds.size.width).max(1.0) as u32;
            let height = f32::from(bounds.size.height).max(1.0) as u32;
            let image = cached_offscreen_image(&self.state, width, height);
            let _ = window.paint_image(bounds, bounds, Corners::default(), image, 0, false);
        }
    }
}

/// Default triangle count above which the element keeps an interactive proxy.
pub const DEFAULT_LOD_THRESHOLD: usize = 2_000_000;

/// Retained full/proxy mesh selection for an interactive GPU element.
#[derive(Debug, Clone)]
pub struct MeshLodController {
    full_mesh: crate::mesh::TriangleMesh,
    proxy_mesh: Option<crate::mesh::MeshDecimation>,
    lod_threshold: usize,
    camera_dragging: bool,
}

impl MeshLodController {
    pub fn new(mesh: crate::mesh::TriangleMesh) -> Self {
        Self::with_lod_threshold(mesh, DEFAULT_LOD_THRESHOLD)
    }
    pub fn with_lod_threshold(mesh: crate::mesh::TriangleMesh, lod_threshold: usize) -> Self {
        let proxy_mesh = make_proxy(&mesh, lod_threshold);
        Self {
            full_mesh: mesh,
            proxy_mesh,
            lod_threshold,
            camera_dragging: false,
        }
    }
    pub fn set_mesh(&mut self, mesh: crate::mesh::TriangleMesh) {
        self.proxy_mesh = make_proxy(&mesh, self.lod_threshold);
        self.full_mesh = mesh;
        self.camera_dragging = false;
    }
    pub fn set_lod_threshold(&mut self, lod_threshold: usize) {
        self.proxy_mesh = make_proxy(&self.full_mesh, lod_threshold);
        self.lod_threshold = lod_threshold;
    }
    pub fn begin_camera_drag(&mut self) {
        self.camera_dragging = true;
    }
    pub fn end_camera_drag(&mut self) {
        self.camera_dragging = false;
    }
    pub fn set_camera_dragging(&mut self, dragging: bool) {
        self.camera_dragging = dragging;
    }
    pub fn active_mesh(&self) -> &crate::mesh::TriangleMesh {
        if self.camera_dragging {
            self.proxy_mesh
                .as_ref()
                .map_or(&self.full_mesh, |proxy| &proxy.mesh)
        } else {
            &self.full_mesh
        }
    }
    pub fn full_mesh(&self) -> &crate::mesh::TriangleMesh {
        &self.full_mesh
    }
    pub fn proxy_mesh(&self) -> Option<&crate::mesh::TriangleMesh> {
        self.proxy_mesh.as_ref().map(|proxy| &proxy.mesh)
    }
    /// Return the active proxy provenance while a drag is in progress.
    /// Callers use this to map vertex/cell scalar fields without changing the
    /// public full-resolution picking and selection contract.
    pub fn active_proxy(&self) -> Option<&crate::mesh::MeshDecimation> {
        self.camera_dragging
            .then_some(self.proxy_mesh.as_ref())
            .flatten()
    }
    /// Return a scalar field whose samples match the mesh currently rendered
    /// by this controller. Selection remains against `full_mesh`; this is
    /// only for the temporary drag-time visual proxy.
    pub fn active_field(
        &self,
        field: &crate::mesh::ScalarField,
    ) -> Result<crate::mesh::ScalarField, crate::mesh::MeshValidationError> {
        self.active_proxy().map_or_else(
            || Ok(field.clone()),
            |proxy| proxy.map_field(&self.full_mesh, field),
        )
    }
    pub fn lod_threshold(&self) -> usize {
        self.lod_threshold
    }
    pub fn camera_dragging(&self) -> bool {
        self.camera_dragging
    }
    pub fn uses_proxy(&self) -> bool {
        self.camera_dragging && self.proxy_mesh.is_some()
    }
}

fn make_proxy(
    mesh: &crate::mesh::TriangleMesh,
    threshold: usize,
) -> Option<crate::mesh::MeshDecimation> {
    if threshold == 0 || mesh.triangles.len() <= threshold {
        None
    } else {
        Some(crate::mesh::decimate_vertex_clustering_with_mapping(
            mesh, threshold,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FieldRevision, GeometryRevision, MeshColorConfig, MeshLodController, MeshSceneState,
        OFFSCREEN_CACHE, OFFSCREEN_CACHE_MAX_BYTES, OffscreenCache, OffscreenCacheEntry,
        OffscreenCacheKey, cached_offscreen_image, offscreen_cache_stats, trim_offscreen_cache,
    };
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;

    #[test]
    fn offscreen_fallback_reuses_only_an_unchanged_scene_image() {
        OFFSCREEN_CACHE.with(|cache| cache.borrow_mut().clear());
        let state = Rc::new(RefCell::new(MeshSceneState::default()));
        let first = cached_offscreen_image(&state, 8, 8);
        let second = cached_offscreen_image(&state, 8, 8);
        assert!(Arc::ptr_eq(&first, &second));

        state.borrow_mut().field_rev = FieldRevision(1);
        let updated = cached_offscreen_image(&state, 8, 8);
        assert!(!Arc::ptr_eq(&first, &updated));
    }

    fn mesh() -> crate::mesh::TriangleMesh {
        crate::mesh::TriangleMesh {
            id: "grid".into(),
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ]
            .into(),
            triangles: vec![[0, 1, 2], [0, 2, 3]].into(),
            vertex_ids: None,
            cell_ids: None,
        }
    }
    fn cache_entry(
        state: &Rc<RefCell<MeshSceneState>>,
        width: u32,
        height: u32,
    ) -> OffscreenCacheEntry {
        OffscreenCacheEntry {
            state: Rc::downgrade(state),
            key: OffscreenCacheKey {
                geometry_revision: GeometryRevision(0),
                field_revision: FieldRevision(0),
                width,
                height,
                view_transform: [[0.0; 4]; 4],
                color: MeshColorConfig::default(),
            },
            image: Arc::new(gpui::RenderImage::new(vec![image::Frame::new(
                image::RgbaImage::new(1, 1),
            )])),
        }
    }

    #[test]
    fn offscreen_cache_plateaus_under_churn_and_bypasses_oversized() {
        let state = Rc::new(RefCell::new(MeshSceneState::default()));
        let mut cache = OffscreenCache::default();
        // 512x512 RGBA = 1 MiB per entry; 200 inserts must plateau at
        // the byte budget, not grow to 200 MiB.
        for id in 0..200 {
            cache.insert(id, cache_entry(&state, 512, 512));
        }
        let stats = cache.stats();
        assert!(
            stats.retained_bytes <= OFFSCREEN_CACHE_MAX_BYTES,
            "{stats:?}"
        );
        assert!(stats.entries <= 32, "{stats:?}");
        assert!(stats.evictions > 0, "{stats:?}");
        assert_eq!(stats.high_water_bytes, stats.retained_bytes);
        assert!(stats.high_water_bytes <= OFFSCREEN_CACHE_MAX_BYTES);
        // LRU: the oldest ids are gone, the newest survives and hits.
        let key = cache_entry(&state, 512, 512).key;
        assert!(cache.get(0, &key).is_none());
        assert!(cache.get(199, &key).is_some());
        assert_eq!(cache.stats().hits, 1);
        // A single image larger than the whole budget is not retained.
        let before = cache.stats().entries;
        cache.insert(10_000, cache_entry(&state, 8192, 8192));
        assert_eq!(cache.stats().entries, before);
        assert!(
            cache
                .get(10_000, &cache_entry(&state, 8192, 8192).key)
                .is_none()
        );
        cache.trim();
        assert_eq!(cache.stats().retained_bytes, 0);
        assert_eq!(cache.stats().entries, 0);
    }

    #[test]
    fn global_offscreen_cache_stats_and_trim() {
        trim_offscreen_cache();
        let state = Rc::new(RefCell::new(MeshSceneState::default()));
        let _image = cached_offscreen_image(&state, 16, 16);
        let stats = offscreen_cache_stats();
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.retained_bytes, 16 * 16 * 4);
        trim_offscreen_cache();
        assert_eq!(offscreen_cache_stats().entries, 0);
    }

    #[test]
    fn lod_controller_restores_full_mesh_after_drag() {
        let mesh = mesh();
        let count = mesh.triangles.len();
        let mut controller = MeshLodController::with_lod_threshold(mesh, 1);
        controller.begin_camera_drag();
        assert!(controller.uses_proxy());
        controller.end_camera_drag();
        assert_eq!(controller.active_mesh().triangles.len(), count);
    }
    #[test]
    fn small_mesh_does_not_allocate_proxy() {
        let mut controller = MeshLodController::with_lod_threshold(mesh(), 100);
        controller.begin_camera_drag();
        assert!(!controller.uses_proxy());
    }

    #[test]
    fn lod_controller_maps_the_active_proxy_field() {
        let mut controller = MeshLodController::with_lod_threshold(mesh(), 1);
        let field = crate::mesh::ScalarField {
            id: "field".into(),
            label: "Field".into(),
            unit: None,
            values: vec![0.0, 1.0, 2.0, 3.0].into(),
            association: crate::mesh::ScalarAssociation::Vertex,
            valid: None,
        };
        assert_eq!(
            controller.active_field(&field).unwrap().values,
            field.values
        );
        controller.begin_camera_drag();
        let proxy_field = controller.active_field(&field).unwrap();
        assert_eq!(
            proxy_field.values.len(),
            controller.active_mesh().positions.len()
        );
        assert_eq!(
            proxy_field.association,
            crate::mesh::ScalarAssociation::Vertex
        );
    }
}
