//! GPUI element that paints a [`ChartScene`] via vello (GPU) or vello_cpu.

use crate::vello2d::wgpu_draw::{SharedScene, WgpuVelloDraw};
use crate::vello2d::{ChartScene, CpuRasterizer};
use gpui::{
    App, Bounds, Corners, CustomDrawId, Edges, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, Position, RenderImage, Size, Style, Window,
    px, relative,
};
use image::{Frame, RgbaImage};
use std::cell::{Cell, RefCell};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::panic::Location;
use std::rc::Rc;
use std::sync::Arc;

/// Compatibility name for the Vello backend selector.
pub type RasterBackend = crate::render2d::VelloBackend;

/// Convert vello_cpu's premultiplied RGBA pixels to GPUI's atlas format while
/// determining whether the raster contains drawable coverage. Combining those
/// operations avoids a separate full-image clear scan on every cache miss.
fn swizzle_rgba_to_bgra(pixels: &mut [u8]) -> bool {
    let mut has_coverage = false;
    for pixel in pixels.chunks_exact_mut(4) {
        has_coverage |= pixel[3] != 0;
        pixel.swap(0, 2);
    }
    has_coverage
}

type SceneBuilder = Rc<dyn Fn(f32, f32) -> ChartScene>;

/// Allocation-free cache-key builder for retained declarative scenes.
#[derive(Default)]
pub struct SceneCacheKey(DefaultHasher);

impl SceneCacheKey {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add<T: Hash>(&mut self, value: T) -> &mut Self {
        value.hash(&mut self.0);
        self
    }

    pub fn add_f32(&mut self, value: f32) -> &mut Self {
        self.add(value.to_bits())
    }

    pub fn add_f64(&mut self, value: f64) -> &mut Self {
        self.add(value.to_bits())
    }

    pub fn finish(&self) -> u64 {
        self.0.finish()
    }
}

struct CpuState {
    // Boxed: vello_cpu's RenderContext makes the bare variant ~1.2 KiB.
    rasterizer: Box<CpuRasterizer>,
    /// Atlas entry painted last frame; dropped via `Window::drop_image`
    /// before its replacement is painted so repeated repaints cannot grow
    /// the sprite atlas without bound.
    image: Option<Arc<RenderImage>>,
    /// Last scene revision and physical raster dimensions represented by `image`.
    /// Scale-factor bits are part of the key so a display/zoom change cannot
    /// reuse a low-resolution image from the previous scale.
    rendered: Option<(u64, u16, u16, u32)>,
}

enum BackendState {
    Wgpu {
        custom_id: CustomDrawId,
        shared: Rc<RefCell<SharedScene>>,
        /// Set by the draw when GPU init fails; triggers the CPU fallback.
        failed: Rc<Cell<bool>>,
    },
    Cpu(CpuState),
}

enum SceneInput<'a> {
    Borrowed(&'a ChartScene),
    Owned(ChartScene),
}

impl SceneInput<'_> {
    fn as_scene(&self) -> &ChartScene {
        match self {
            Self::Borrowed(scene) => scene,
            Self::Owned(scene) => scene,
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PainterTestStats {
    custom_registrations: u32,
    custom_unregistrations: u32,
    cpu_rasterizations: u32,
    wgpu_submissions: u32,
}

/// Reusable scene painter for custom GPUI elements.
///
/// Unlike [`VelloChartElement`], this type does not own layout or scene
/// construction. Callers can keep it in a custom element and submit a fresh
/// scene on every paint, which is useful for live audio meters.
pub struct VelloScenePainter {
    backend_pref: RasterBackend,
    state: Option<BackendState>,
    #[cfg(test)]
    test_stats: PainterTestStats,
}

impl Default for VelloScenePainter {
    fn default() -> Self {
        Self {
            backend_pref: RasterBackend::Auto,
            state: None,
            #[cfg(test)]
            test_stats: PainterTestStats::default(),
        }
    }
}

impl VelloScenePainter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Paint while retaining the resolved backend in GPUI element state.
    ///
    /// This is intended for custom elements reconstructed by their parent on
    /// every frame. The backend registration survives reconstruction and is
    /// released automatically when GPUI retires the element id.
    pub fn paint_retained(
        &mut self,
        id: Option<&GlobalElementId>,
        scene: &ChartScene,
        bounds: Bounds<Pixels>,
        window: &mut Window,
    ) {
        let retained = id.map(|id| {
            window.with_element_state::<Rc<RefCell<RetainedVelloBackend>>, _>(
                id,
                |state, _window| {
                    let state = state
                        .unwrap_or_else(|| Rc::new(RefCell::new(RetainedVelloBackend::default())));
                    (Rc::clone(&state), state)
                },
            )
        });
        if let Some(retained) = retained.as_ref() {
            self.state = retained.borrow_mut().backend.take();
        }
        self.paint(scene, bounds, window);
        if let Some(retained) = retained {
            retained.borrow_mut().backend = self.state.take();
        }
    }

    /// Set the preferred backend on a painter that is kept by a custom
    /// element.  A resolved backend is discarded so the next paint resolves
    /// the new preference; this also unregisters a previously registered
    /// custom draw.
    pub fn set_backend(&mut self, backend: RasterBackend) {
        if self.backend_pref == backend {
            return;
        }
        if let Some(BackendState::Wgpu { custom_id, .. }) = self.state.take() {
            gpui::unregister_custom_draw(custom_id);
            #[cfg(test)]
            {
                self.test_stats.custom_unregistrations += 1;
            }
        }
        self.backend_pref = backend;
    }

    pub fn backend(mut self, backend: RasterBackend) -> Self {
        self.set_backend(backend);
        self
    }

    fn resolve(&mut self) {
        if self.state.is_some() {
            return;
        }
        let backend = resolve_backend(self.backend_pref, gpui::wgpu_custom_draw_available());
        self.state = Some(match backend {
            RasterBackend::Wgpu => {
                let shared = Rc::new(RefCell::new(SharedScene {
                    // The first scene is installed by `paint_scene` below.
                    // Starting with a sentinel revision lets an owned dynamic
                    // scene move straight into the shared draw state instead
                    // of being cloned during initialization.
                    scene: ChartScene::new(),
                    revision: 0,
                    logical_size: (0.0, 0.0),
                }));
                let failed = Rc::new(Cell::new(false));
                let draw = WgpuVelloDraw::new(Rc::clone(&shared), Rc::clone(&failed));
                let custom_id = gpui::register_custom_draw(draw.into_custom_draw());
                #[cfg(test)]
                {
                    self.test_stats.custom_registrations += 1;
                }
                BackendState::Wgpu {
                    custom_id,
                    shared,
                    failed,
                }
            }
            RasterBackend::Cpu | RasterBackend::Auto => BackendState::Cpu(CpuState {
                rasterizer: Box::new(CpuRasterizer::new(1, 1)),
                image: None,
                rendered: None,
            }),
        });
    }

    fn fall_back_to_cpu_if_failed(&mut self) {
        let failed = matches!(&self.state, Some(BackendState::Wgpu { failed, .. }) if failed.get());
        if !failed {
            return;
        }
        if let Some(BackendState::Wgpu { custom_id, .. }) = self.state.take() {
            gpui::unregister_custom_draw(custom_id);
            #[cfg(test)]
            {
                self.test_stats.custom_unregistrations += 1;
            }
        }
        log::warn!("vello2d: wgpu vello init failed; falling back to CPU rasterizer");
        self.state = Some(BackendState::Cpu(CpuState {
            rasterizer: Box::new(CpuRasterizer::new(1, 1)),
            image: None,
            rendered: None,
        }));
    }

    fn clear_cpu_image(state: &mut CpuState, window: &mut Window) {
        if let Some(old) = state.image.take() {
            let _ = window.drop_image(old);
        }
        state.rendered = None;
    }

    /// Paint a borrowed scene. Prefer [`Self::paint_owned`] for dynamically
    /// built scenes so the WGPU path can retain it without a full clone.
    pub fn paint(&mut self, scene: &ChartScene, bounds: Bounds<Pixels>, window: &mut Window) {
        self.paint_scene(SceneInput::Borrowed(scene), bounds, window);
    }

    /// Paint a newly-built scene by value.
    ///
    /// The WGPU custom-draw backend retains the scene until its revision
    /// changes, so accepting ownership avoids cloning every dynamic chart
    /// scene on the UI/audio repaint path.
    pub fn paint_owned(&mut self, scene: ChartScene, bounds: Bounds<Pixels>, window: &mut Window) {
        self.paint_scene(SceneInput::Owned(scene), bounds, window);
    }

    fn paint_scene(&mut self, scene: SceneInput<'_>, bounds: Bounds<Pixels>, window: &mut Window) {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let scale_factor = window.scale_factor().max(0.01);
        if width < 1.0 || height < 1.0 || scene.as_scene().is_empty() {
            if let Some(BackendState::Cpu(state)) = self.state.as_mut() {
                Self::clear_cpu_image(state, window);
            }
            return;
        }

        self.resolve();
        self.fall_back_to_cpu_if_failed();
        match self.state.as_mut() {
            Some(BackendState::Wgpu {
                custom_id, shared, ..
            }) => {
                let mut shared = shared.borrow_mut();
                let revision = scene.as_scene().revision();
                if shared.revision != revision {
                    shared.scene = match scene {
                        SceneInput::Borrowed(scene) => scene.clone(),
                        SceneInput::Owned(scene) => scene,
                    };
                    shared.revision = revision;
                }
                shared.logical_size = (width, height);
                window.paint_custom(*custom_id, bounds);
                #[cfg(test)]
                {
                    self.test_stats.wgpu_submissions += 1;
                }
            }
            Some(BackendState::Cpu(state)) => {
                let (w, h) = physical_raster_size(width, height, scale_factor);
                let scale_bits = scale_factor.to_bits();
                if state.rendered == Some((scene.as_scene().revision(), w, h, scale_bits)) {
                    if let Some(image) = state.image.as_ref() {
                        let _ = window.paint_image(
                            bounds,
                            Corners::default(),
                            Arc::clone(image),
                            0,
                            false,
                        );
                    }
                    return;
                }
                let mut pixels = state.rasterizer.rasterize(scene.as_scene(), w, h);
                #[cfg(test)]
                {
                    self.test_stats.cpu_rasterizations += 1;
                }
                if !swizzle_rgba_to_bgra(&mut pixels) {
                    Self::clear_cpu_image(state, window);
                    return;
                }
                if let Some(rgba) = RgbaImage::from_raw(w as u32, h as u32, pixels) {
                    Self::clear_cpu_image(state, window);
                    let image = Arc::new(RenderImage::new(vec![Frame::new(rgba)]));
                    let _ = window.paint_image(
                        bounds,
                        Corners::default(),
                        Arc::clone(&image),
                        0,
                        false,
                    );
                    state.image = Some(image);
                    state.rendered = Some((scene.as_scene().revision(), w, h, scale_bits));
                }
            }
            None => {}
        }
    }
}

#[cfg(test)]
impl VelloScenePainter {
    fn test_stats(&self) -> PainterTestStats {
        self.test_stats
    }
}

/// Resolve the user-facing backend preference without touching GPUI globals.
///
/// Keeping this decision pure makes the Auto fallback contract testable on
/// every target, including feature-off and browser builds where a custom draw
/// adapter may not be available at all.
fn resolve_backend(preference: RasterBackend, custom_draw_available: bool) -> RasterBackend {
    match preference {
        RasterBackend::Auto if custom_draw_available => RasterBackend::Wgpu,
        RasterBackend::Auto => RasterBackend::Cpu,
        explicit => explicit,
    }
}

fn physical_raster_size(width: f32, height: f32, scale_factor: f32) -> (u16, u16) {
    let scale_factor = scale_factor.max(0.01);
    (
        (width * scale_factor).max(1.0).ceil().min(u16::MAX as f32) as u16,
        (height * scale_factor).max(1.0).ceil().min(u16::MAX as f32) as u16,
    )
}

impl Drop for VelloScenePainter {
    fn drop(&mut self) {
        if let Some(BackendState::Wgpu { custom_id, .. }) = &self.state {
            gpui::unregister_custom_draw(*custom_id);
        }
    }
}

/// Retained chart scene for components whose declarative element is rebuilt
/// more often than its chart data changes.
///
/// The owning component keeps this value in an `Rc<RefCell<_>>` and replaces
/// its builder only when its input changes. That preserves the painter's WGPU
/// custom-draw registration and the scene built for the current layout size.
pub struct RetainedVelloChart {
    painter: VelloScenePainter,
    builder: SceneBuilder,
    scene: ChartScene,
    scene_size: Option<(f32, f32)>,
}

impl RetainedVelloChart {
    /// Create a retained chart with a size-dependent scene builder.
    pub fn new(builder: impl Fn(f32, f32) -> ChartScene + 'static) -> Self {
        Self {
            painter: VelloScenePainter::new(),
            builder: Rc::new(builder),
            scene: ChartScene::new(),
            scene_size: None,
        }
    }

    /// Replace chart inputs and invalidate the size-specific scene.
    ///
    /// Call this only when the builder's captured chart data changes. Reusing
    /// the existing builder leaves both the Vello scene and custom-draw
    /// registration intact across GPUI element reconstruction.
    pub fn set_builder(&mut self, builder: impl Fn(f32, f32) -> ChartScene + 'static) {
        self.builder = Rc::new(builder);
        self.scene = ChartScene::new();
        self.scene_size = None;
    }

    /// Set the preferred raster backend for subsequent paints.
    pub fn set_backend(&mut self, backend: RasterBackend) {
        self.painter.set_backend(backend);
    }

    fn rebuild_for_size(&mut self, width: f32, height: f32) -> bool {
        let size = (width, height);
        if self.scene_size == Some(size) {
            return false;
        }
        self.scene = (self.builder)(width.max(1.0), height.max(1.0));
        self.scene_size = Some(size);
        true
    }

    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window) {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        self.rebuild_for_size(width, height);
        self.painter.paint(&self.scene, bounds, window);
    }
}

/// GPUI element facade for [`RetainedVelloChart`].
///
/// This element is intentionally cheap to construct: the mutable renderer and
/// size-specific chart scene live in the shared retained chart instead.
pub struct RetainedVelloChartElement {
    chart: Rc<RefCell<RetainedVelloChart>>,
    absolute: bool,
}

impl RetainedVelloChartElement {
    pub fn new(chart: Rc<RefCell<RetainedVelloChart>>) -> Self {
        Self {
            chart,
            absolute: false,
        }
    }

    pub fn absolute(mut self) -> Self {
        self.absolute = true;
        self
    }
}

impl IntoElement for RetainedVelloChartElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for RetainedVelloChartElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = if self.absolute {
            Style {
                position: Position::Absolute,
                inset: Edges {
                    top: px(0.0).into(),
                    right: px(0.0).into(),
                    bottom: px(0.0).into(),
                    left: px(0.0).into(),
                },
                size: Size {
                    width: relative(1.0).into(),
                    height: relative(1.0).into(),
                },
                ..Default::default()
            }
        } else {
            Style {
                size: Size {
                    width: relative(1.0).into(),
                    height: relative(1.0).into(),
                },
                ..Default::default()
            }
        };
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        self.chart.borrow_mut().paint(bounds, window);
    }
}

/// Element painting a [`ChartScene`]. Build it in the chart's render method;
/// `Drop` unregisters the custom draw. With `with_builder`, the scene is
/// (re)generated whenever paint bounds change size.
pub struct VelloChartElement {
    id: ElementId,
    source_location: &'static Location<'static>,
    scene: ChartScene,
    builder: Option<SceneBuilder>,
    scene_size: Option<(f32, f32)>,
    scene_key: Option<u64>,
    backend_pref: RasterBackend,
    state: Option<BackendState>,
    absolute: bool,
}

impl std::fmt::Debug for VelloChartElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("VelloChartElement");
        d.field("backend_pref", &self.backend_pref)
            .field("scene_commands", &self.scene.len())
            .field(
                "resolved",
                &match &self.state {
                    None => "no",
                    Some(BackendState::Wgpu { .. }) => "Wgpu",
                    Some(BackendState::Cpu(_)) => "Cpu",
                },
            );
        if self.builder.is_some() {
            d.field("builder", &true);
        }
        d.finish()
    }
}

impl VelloChartElement {
    /// Static scene, baked in the coordinates it will be painted at. The
    /// caller must rebuild the element when the chart's pixel size changes.
    #[track_caller]
    pub fn new(scene: ChartScene) -> Self {
        let source_location = Location::caller();
        Self {
            id: ElementId::CodeLocation(*source_location),
            source_location,
            scene,
            builder: None,
            scene_size: None,
            scene_key: None,
            backend_pref: RasterBackend::Auto,
            state: None,
            absolute: false,
        }
    }

    /// Scene is (re)built at paint time from the actual bounds size
    /// (`builder(width, height)` in element-local pixels).
    #[track_caller]
    pub fn with_builder(builder: impl Fn(f32, f32) -> ChartScene + 'static) -> Self {
        let source_location = Location::caller();
        Self {
            id: ElementId::CodeLocation(*source_location),
            source_location,
            scene: ChartScene::new(),
            builder: Some(Rc::new(builder)),
            scene_size: None,
            scene_key: None,
            backend_pref: RasterBackend::Auto,
            state: None,
            absolute: false,
        }
    }

    pub fn backend(mut self, backend: RasterBackend) -> Self {
        self.backend_pref = backend;
        self
    }

    /// Override the default call-site identity when multiple charts are built
    /// from the same source location.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Identify the declarative scene inputs across element reconstruction.
    /// Equal keys allow the encoded scene and size to be retained; callers
    /// must change the key whenever captured builder data changes.
    pub fn cache_key(mut self, key: u64) -> Self {
        self.scene_key = Some(key);
        self
    }

    pub fn absolute(mut self) -> Self {
        self.absolute = true;
        self
    }

    /// Resolve the backend on first paint and, for wgpu, register the draw.
    fn resolve(&mut self) {
        if self.state.is_some() {
            return;
        }
        let backend = match self.backend_pref {
            RasterBackend::Auto => {
                if gpui::wgpu_custom_draw_available() {
                    RasterBackend::Wgpu
                } else {
                    RasterBackend::Cpu
                }
            }
            explicit => explicit,
        };
        self.state = Some(match backend {
            RasterBackend::Wgpu => {
                let shared = Rc::new(RefCell::new(SharedScene {
                    scene: self.scene.clone(),
                    revision: self.scene.revision(),
                    logical_size: self.scene_size.unwrap_or((0.0, 0.0)),
                }));
                let failed = Rc::new(Cell::new(false));
                let draw = WgpuVelloDraw::new(Rc::clone(&shared), Rc::clone(&failed));
                let custom_id = gpui::register_custom_draw(draw.into_custom_draw());
                BackendState::Wgpu {
                    custom_id,
                    shared,
                    failed,
                }
            }
            RasterBackend::Cpu | RasterBackend::Auto => BackendState::Cpu(CpuState {
                rasterizer: Box::new(CpuRasterizer::new(1, 1)),
                image: None,
                rendered: None,
            }),
        });
    }

    /// If the wgpu draw reported an init failure, unregister it and switch to
    /// the CPU rasterizer. Returns true when a fallback happened.
    fn fall_back_to_cpu_if_failed(&mut self) -> bool {
        let failed = matches!(&self.state, Some(BackendState::Wgpu { failed, .. }) if failed.get());
        if !failed {
            return false;
        }
        if let Some(BackendState::Wgpu { custom_id, .. }) = self.state.take() {
            gpui::unregister_custom_draw(custom_id);
        }
        log::warn!("vello2d: wgpu vello init failed; falling back to CPU rasterizer");
        self.state = Some(BackendState::Cpu(CpuState {
            rasterizer: Box::new(CpuRasterizer::new(1, 1)),
            image: None,
            rendered: None,
        }));
        true
    }
}

impl Drop for VelloChartElement {
    fn drop(&mut self) {
        if let Some(BackendState::Wgpu { custom_id, .. }) = &self.state {
            gpui::unregister_custom_draw(*custom_id);
        }
    }
}

#[derive(Default)]
struct RetainedVelloBackend {
    backend: Option<BackendState>,
    scene: Option<(u64, ChartScene, Option<(f32, f32)>)>,
}

impl Drop for RetainedVelloBackend {
    fn drop(&mut self) {
        if let Some(BackendState::Wgpu { custom_id, .. }) = self.backend.take() {
            gpui::unregister_custom_draw(custom_id);
        }
    }
}

impl IntoElement for VelloChartElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for VelloChartElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        Some(self.source_location)
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = if self.absolute {
            Style {
                position: Position::Absolute,
                inset: Edges {
                    top: px(0.0).into(),
                    right: px(0.0).into(),
                    bottom: px(0.0).into(),
                    left: px(0.0).into(),
                },
                size: Size {
                    width: relative(1.0).into(),
                    height: relative(1.0).into(),
                },
                ..Default::default()
            }
        } else {
            Style {
                size: Size {
                    width: relative(1.0).into(),
                    height: relative(1.0).into(),
                },
                ..Default::default()
            }
        };
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let retained = id.map(|id| {
            window.with_element_state::<Rc<RefCell<RetainedVelloBackend>>, _>(
                id,
                |state, _window| {
                    let state = state
                        .unwrap_or_else(|| Rc::new(RefCell::new(RetainedVelloBackend::default())));
                    (Rc::clone(&state), state)
                },
            )
        });
        if let Some(retained) = retained.as_ref() {
            let mut retained = retained.borrow_mut();
            self.state = retained.backend.take();
            if let Some(key) = self.scene_key
                && retained
                    .scene
                    .as_ref()
                    .is_some_and(|(cached, _, _)| *cached == key)
                && let Some((_, scene, size)) = retained.scene.take()
            {
                self.scene = scene;
                self.scene_size = size;
            }
        }

        'paint: {
            let width: f32 = bounds.size.width.into();
            let height: f32 = bounds.size.height.into();
            let scale_factor = window.scale_factor().max(0.01);
            if width < 1.0 || height < 1.0 {
                if let Some(BackendState::Cpu(state)) = self.state.as_mut() {
                    VelloScenePainter::clear_cpu_image(state, window);
                }
                break 'paint;
            }

            // (Re)build the scene when the builder exists and the size changed.
            let mut scene_rebuilt = false;
            if let Some(builder) = self.builder.clone()
                && self.scene_size != Some((width, height))
            {
                self.scene = builder(width, height);
                self.scene_size = Some((width, height));
                scene_rebuilt = true;
            }
            if self.scene.is_empty() {
                if let Some(BackendState::Cpu(state)) = self.state.as_mut() {
                    VelloScenePainter::clear_cpu_image(state, window);
                }
                break 'paint;
            }
            self.resolve();
            self.fall_back_to_cpu_if_failed();

            match self.state.as_mut() {
                Some(BackendState::Wgpu {
                    custom_id, shared, ..
                }) => {
                    // The scene is logical; the draw receives physical bounds.
                    // Keep the logical size alongside so it can derive the scale.
                    {
                        let mut shared = shared.borrow_mut();
                        if scene_rebuilt || shared.revision != self.scene.revision() {
                            shared.scene = self.scene.clone();
                            shared.revision = self.scene.revision();
                        }
                        shared.logical_size = (width, height);
                    }
                    window.paint_custom(*custom_id, bounds);
                }
                Some(BackendState::Cpu(state)) => {
                    let (w, h) = physical_raster_size(width, height, scale_factor);
                    let scale_bits = scale_factor.to_bits();
                    if state.rendered == Some((self.scene.revision(), w, h, scale_bits)) {
                        if let Some(image) = state.image.as_ref() {
                            let _ = window.paint_image(
                                bounds,
                                Corners::default(),
                                Arc::clone(image),
                                0,
                                false,
                            );
                        }
                        break 'paint;
                    }
                    let mut pixels = state.rasterizer.rasterize(&self.scene, w, h);
                    if !swizzle_rgba_to_bgra(&mut pixels) {
                        VelloScenePainter::clear_cpu_image(state, window);
                        return;
                    }
                    // GPUI image atlases expect premultiplied BGRA (Metal uses
                    // BGRA8Unorm; the wgpu atlas prefers Bgra8Unorm and only
                    // swizzles when it falls back to Rgba8Unorm — see
                    // gpui::swap_rgba_pa_to_bgra and gpui_wgpu's
                    // swizzle_upload_data). vello_cpu yields premultiplied RGBA,
                    // so swap R<->B before handing the pixmap to paint_image.
                    if let Some(rgba) = RgbaImage::from_raw(w as u32, h as u32, pixels) {
                        // RenderImage ids are unique and paint_image caches each
                        // one in the sprite atlas; release the previous entry
                        // before inserting its replacement.
                        VelloScenePainter::clear_cpu_image(state, window);
                        let image = Arc::new(RenderImage::new(vec![Frame::new(rgba)]));
                        let _ = window.paint_image(
                            bounds,
                            Corners::default(),
                            Arc::clone(&image),
                            0,
                            false,
                        );
                        state.image = Some(image);
                        state.rendered = Some((self.scene.revision(), w, h, scale_bits));
                    }
                }
                None => {}
            }
        }

        if let Some(retained) = retained {
            let mut retained = retained.borrow_mut();
            retained.backend = self.state.take();
            if let Some(key) = self.scene_key {
                retained.scene = Some((key, std::mem::take(&mut self.scene), self.scene_size));
            } else {
                retained.scene = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vello2d::kurbo::Rect;
    use crate::vello2d::peniko::{Brush, Color};

    fn sample_scene() -> ChartScene {
        let mut scene = ChartScene::new();
        scene.fill_rect(
            Rect::new(0.0, 0.0, 4.0, 4.0),
            Brush::Solid(Color::from_rgb8(9, 9, 9)),
        );
        scene
    }

    #[test]
    fn retained_chart_rebuilds_only_for_new_size_or_builder() {
        let builds = Rc::new(Cell::new(0));
        let first_builds = Rc::clone(&builds);
        let mut chart = RetainedVelloChart::new(move |_, _| {
            first_builds.set(first_builds.get() + 1);
            sample_scene()
        });

        assert!(chart.rebuild_for_size(320.0, 180.0));
        assert!(!chart.rebuild_for_size(320.0, 180.0));
        assert_eq!(builds.get(), 1);

        let second_builds = Rc::clone(&builds);
        chart.set_builder(move |_, _| {
            second_builds.set(second_builds.get() + 1);
            sample_scene()
        });
        assert!(chart.rebuild_for_size(320.0, 180.0));
        assert!(chart.rebuild_for_size(640.0, 180.0));
        assert_eq!(builds.get(), 3);
    }

    #[test]
    fn wgpu_init_failure_falls_back_to_cpu() {
        let mut element = VelloChartElement::new(sample_scene()).backend(RasterBackend::Wgpu);
        element.resolve();
        let Some(BackendState::Wgpu { failed, .. }) = &element.state else {
            panic!("explicit Wgpu backend must resolve to the wgpu state");
        };
        failed.set(true);
        assert!(element.fall_back_to_cpu_if_failed());
        assert!(matches!(element.state, Some(BackendState::Cpu(_))));
        // Idempotent: a CPU-resolved element never "falls back" again.
        assert!(!element.fall_back_to_cpu_if_failed());
    }

    #[test]
    fn healthy_wgpu_state_does_not_fall_back() {
        let mut element = VelloChartElement::new(sample_scene()).backend(RasterBackend::Wgpu);
        element.resolve();
        assert!(!element.fall_back_to_cpu_if_failed());
        assert!(matches!(element.state, Some(BackendState::Wgpu { .. })));
    }

    #[test]
    fn painter_defaults_to_auto_and_can_switch_before_resolution() {
        let mut painter = VelloScenePainter::new();
        assert_eq!(painter.backend_pref, RasterBackend::Auto);
        assert!(painter.state.is_none());
        painter.set_backend(RasterBackend::Cpu);
        assert_eq!(painter.backend_pref, RasterBackend::Cpu);
        assert!(painter.state.is_none());
    }

    #[test]
    fn painter_backend_switch_unregisters_wgpu_state() {
        let mut painter = VelloScenePainter::new().backend(RasterBackend::Wgpu);
        painter.resolve();
        assert!(matches!(painter.state, Some(BackendState::Wgpu { .. })));
        painter.set_backend(RasterBackend::Cpu);
        assert!(painter.state.is_none());
        assert_eq!(painter.backend_pref, RasterBackend::Cpu);
        assert_eq!(painter.test_stats().custom_registrations, 1);
        assert_eq!(painter.test_stats().custom_unregistrations, 1);
    }

    #[test]
    fn retained_painter_resolves_custom_draw_once() {
        let mut painter = VelloScenePainter::new().backend(RasterBackend::Wgpu);
        // A component that keeps its painter across drag frames must retain
        // the custom draw registration rather than recreating it per frame.
        painter.resolve();
        painter.resolve();

        assert_eq!(painter.test_stats().custom_registrations, 1);
        assert_eq!(painter.test_stats().custom_unregistrations, 0);
    }

    #[test]
    fn chart_backend_switch_is_kept_until_first_paint() {
        let chart = VelloChartElement::new(sample_scene()).backend(RasterBackend::Cpu);
        assert_eq!(chart.backend_pref, RasterBackend::Cpu);
        assert!(chart.state.is_none());
    }

    #[test]
    fn cpu_state_tracks_scene_revision_and_size() {
        let state = CpuState {
            rasterizer: Box::new(CpuRasterizer::new(1, 1)),
            image: None,
            rendered: Some((7, 32, 16, 0x3f80_0000)),
        };
        assert_eq!(state.rendered, Some((7, 32, 16, 0x3f80_0000)));
    }

    #[test]
    fn auto_resolution_is_cpu_without_custom_draw_support() {
        assert_eq!(
            resolve_backend(RasterBackend::Auto, false),
            RasterBackend::Cpu
        );
    }

    #[test]
    fn auto_resolution_is_wgpu_when_custom_draw_is_available() {
        assert_eq!(
            resolve_backend(RasterBackend::Auto, true),
            RasterBackend::Wgpu
        );
    }

    #[test]
    fn explicit_backend_resolution_ignores_custom_draw_probe() {
        for preference in [RasterBackend::Cpu, RasterBackend::Wgpu] {
            assert_eq!(resolve_backend(preference, false), preference);
            assert_eq!(resolve_backend(preference, true), preference);
        }
    }

    #[test]
    fn cpu_raster_size_tracks_scale_factor_and_clamps_small_bounds() {
        assert_eq!(physical_raster_size(20.0, 10.0, 1.0), (20, 10));
        assert_eq!(physical_raster_size(20.0, 10.0, 2.0), (40, 20));
        assert_eq!(physical_raster_size(0.0, 0.0, 2.0), (1, 1));
    }

    #[test]
    fn swizzle_detects_coverage_in_the_same_pass() {
        let mut transparent = [0, 0, 0, 0];
        assert!(!swizzle_rgba_to_bgra(&mut transparent));

        let mut pixels = [10, 20, 30, 255, 0, 0, 0, 0];
        assert!(swizzle_rgba_to_bgra(&mut pixels));
        assert_eq!(pixels, [30, 20, 10, 255, 0, 0, 0, 0]);
    }
}
