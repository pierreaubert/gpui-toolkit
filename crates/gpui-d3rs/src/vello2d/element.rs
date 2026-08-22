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

    fn resolve(&mut self, scene: &ChartScene) {
        if self.state.is_some() {
            return;
        }
        let backend = resolve_backend(self.backend_pref, gpui::wgpu_custom_draw_available());
        self.state = Some(match backend {
            RasterBackend::Wgpu => {
                let shared = Rc::new(RefCell::new(SharedScene {
                    scene: scene.clone(),
                    revision: scene.revision(),
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

    pub fn paint(&mut self, scene: &ChartScene, bounds: Bounds<Pixels>, window: &mut Window) {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let scale_factor = window.scale_factor().max(0.01);
        if width < 1.0 || height < 1.0 || scene.is_empty() {
            if let Some(BackendState::Cpu(state)) = self.state.as_mut() {
                Self::clear_cpu_image(state, window);
            }
            return;
        }

        self.resolve(scene);
        self.fall_back_to_cpu_if_failed();
        match self.state.as_mut() {
            Some(BackendState::Wgpu {
                custom_id, shared, ..
            }) => {
                let mut shared = shared.borrow_mut();
                if shared.revision != scene.revision() {
                    shared.scene = scene.clone();
                    shared.revision = scene.revision();
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
                if state.rendered == Some((scene.revision(), w, h, scale_bits)) {
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
                let mut pixels = state.rasterizer.rasterize(scene, w, h);
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
                    state.rendered = Some((scene.revision(), w, h, scale_bits));
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

/// Element painting a [`ChartScene`]. Build it in the chart's render method;
/// `Drop` unregisters the custom draw. With `with_builder`, the scene is
/// (re)generated whenever paint bounds change size.
pub struct VelloChartElement {
    scene: ChartScene,
    builder: Option<SceneBuilder>,
    scene_size: Option<(f32, f32)>,
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
    pub fn new(scene: ChartScene) -> Self {
        Self {
            scene,
            builder: None,
            scene_size: None,
            backend_pref: RasterBackend::Auto,
            state: None,
            absolute: false,
        }
    }

    /// Scene is (re)built at paint time from the actual bounds size
    /// (`builder(width, height)` in element-local pixels).
    pub fn with_builder(builder: impl Fn(f32, f32) -> ChartScene + 'static) -> Self {
        Self {
            scene: ChartScene::new(),
            builder: Some(Rc::new(builder)),
            scene_size: None,
            backend_pref: RasterBackend::Auto,
            state: None,
            absolute: false,
        }
    }

    pub fn backend(mut self, backend: RasterBackend) -> Self {
        self.backend_pref = backend;
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
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let scale_factor = window.scale_factor().max(0.01);
        if width < 1.0 || height < 1.0 {
            if let Some(BackendState::Cpu(state)) = self.state.as_mut() {
                VelloScenePainter::clear_cpu_image(state, window);
            }
            return;
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
            return;
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
                    return;
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
        painter.resolve(&sample_scene());
        assert!(matches!(painter.state, Some(BackendState::Wgpu { .. })));
        painter.set_backend(RasterBackend::Cpu);
        assert!(painter.state.is_none());
        assert_eq!(painter.backend_pref, RasterBackend::Cpu);
        assert_eq!(painter.test_stats().custom_registrations, 1);
        assert_eq!(painter.test_stats().custom_unregistrations, 1);
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
