//! GPUI element that paints a [`ChartScene`] via vello (GPU) or vello_cpu.

use crate::vello2d::wgpu_draw::WgpuVelloDraw;
use crate::vello2d::{ChartScene, CpuRasterizer};
use gpui::{
    App, Bounds, Corners, CustomDrawId, Edges, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, Position, RenderImage, Size, Style,
    Window, px, relative,
};
use image::{Frame, RgbaImage};
use std::cell::RefCell;
use std::panic::Location;
use std::rc::Rc;
use std::sync::Arc;

/// Which rasterizer paints the scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RasterBackend {
    /// Probe `gpui::wgpu_custom_draw_available()` at first paint.
    Auto,
    /// Zero-copy GPU path through `WgpuCustomDraw` (requires the wgpu renderer).
    Wgpu,
    /// `vello_cpu` pixmap + `paint_image`. Works on every renderer.
    Cpu,
}

type SceneBuilder = Rc<dyn Fn(f32, f32) -> ChartScene>;

enum BackendState {
    Wgpu {
        custom_id: CustomDrawId,
        shared: Rc<RefCell<ChartScene>>,
    },
    // Boxed: vello_cpu's RenderContext makes the bare variant ~1.2 KiB.
    Cpu(Box<CpuRasterizer>),
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
                let shared = Rc::new(RefCell::new(self.scene.clone()));
                let draw = WgpuVelloDraw::new(Rc::clone(&shared));
                let custom_id = gpui::register_custom_draw(draw.into_custom_draw());
                BackendState::Wgpu { custom_id, shared }
            }
            RasterBackend::Cpu | RasterBackend::Auto => {
                BackendState::Cpu(Box::new(CpuRasterizer::new(1, 1)))
            }
        });
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
        if width < 1.0 || height < 1.0 {
            return;
        }

        // (Re)build the scene when the builder exists and the size changed.
        if let Some(builder) = self.builder.clone()
            && self.scene_size != Some((width, height))
        {
            self.scene = builder(width, height);
            self.scene_size = Some((width, height));
            if let Some(BackendState::Wgpu { shared, .. }) = &self.state {
                *shared.borrow_mut() = self.scene.clone();
            }
        }
        if self.scene.is_empty() {
            return;
        }
        self.resolve();

        match self.state.as_mut() {
            Some(BackendState::Wgpu { custom_id, .. }) => {
                window.paint_custom(*custom_id, bounds);
            }
            Some(BackendState::Cpu(rasterizer)) => {
                let (w, h) = (width as u16, height as u16);
                let mut pixels = rasterizer.rasterize(&self.scene, w, h);
                if pixels.iter().all(|&b| b == 0) {
                    return;
                }
                // GPUI image atlases expect premultiplied BGRA (Metal uses
                // BGRA8Unorm; the wgpu atlas prefers Bgra8Unorm and only
                // swizzles when it falls back to Rgba8Unorm — see
                // gpui::swap_rgba_pa_to_bgra and gpui_wgpu's
                // swizzle_upload_data). vello_cpu yields premultiplied RGBA,
                // so swap R<->B before handing the pixmap to paint_image.
                for px in pixels.chunks_exact_mut(4) {
                    px.swap(0, 2);
                }
                if let Some(rgba) = RgbaImage::from_raw(w as u32, h as u32, pixels) {
                    let image = RenderImage::new(vec![Frame::new(rgba)]);
                    let _ = window.paint_image(bounds, Corners::default(), Arc::new(image), 0, false);
                }
            }
            None => {}
        }
    }
}
