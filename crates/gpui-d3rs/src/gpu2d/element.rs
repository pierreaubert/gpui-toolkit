//! GPUI Element wrapper for 2D chart rendering

use super::primitives::Color4;
use super::renderer::Chart2DRenderer;
#[cfg(target_family = "wasm")]
use super::device::Gpu2DContext;
use gpui::*;
use image::{Frame, RgbaImage};
use std::cell::RefCell;
#[cfg(target_family = "wasm")]
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::panic;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(target_family = "wasm")]
thread_local! {
    /// Renderer cache for wasm, keyed by the plot's pixel origin.
    ///
    /// GPUI rebuilds the element tree on every re-render, so a renderer (and
    /// the deferred readback slot it carries) stored in `self` would be
    /// orphaned by the very `request_animation_frame` repaint that is meant
    /// to pick its pixels up: the new element would build a fresh renderer
    /// whose readback slot is empty, and the completed map_async callback
    /// would land in the dropped previous generation. Parking one renderer
    /// per chart position keeps the paint loop below consistent across
    /// re-renders. Charts sharing a page occupy different origins (e.g. the
    /// px-showcase gallery); a static layout keeps its origin stable, and a
    /// resize shifts origins, so stale entries are evicted by a hard cap.
    static RENDERER_CACHE: RefCell<HashMap<(u32, u32), Rc<RefCell<Chart2DRenderer>>>> =
        RefCell::new(HashMap::new());
}

/// Draw function type for Chart2DElement
pub type DrawFn = Box<dyn Fn(&mut Chart2DRenderer, Bounds<Pixels>)>;

/// GPUI Element for GPU-accelerated 2D chart rendering
///
/// The renderer is wrapped in `ManuallyDrop` to prevent TLS access-after-destruction
/// crashes. When GPUI's thread-local element Arena is cleaned up during thread exit,
/// elements are dropped — but wgpu's internal TLS may already be destroyed at that point.
/// Since the renderer holds GPU resources (buffers, pipelines) that reference the global
/// `Gpu2DContext` static via `Arc`, they are cleaned up when the static is dropped instead.
///
/// On wasm the renderer is constructed lazily during the first paint: device
/// initialization is async there, so chart construction never touches the GPU.
/// Because GPUI rebuilds elements on every re-render, the lazily built
/// renderer is parked in a thread-local cache keyed by plot origin (see
/// `RENDERER_CACHE`) so its deferred readback slot survives repaints.
/// Readbacks are deferred (`map_async` driven by the browser event loop), which
/// means painted pixels can be one frame stale during interaction/resize — a
/// size mismatch drops the stale frame instead of painting garbage. Two known
/// limitations of the origin-keyed cache: a data-only invalidation (same size)
/// can rest on the pre-change frame until the next unrelated repaint, and
/// absolutely-positioned overlay charts sharing an origin share one renderer
/// (no current wasm call site overlays). Native keeps eager construction and
/// synchronous readback, unchanged.
pub struct Chart2DElement {
    renderer: Option<ManuallyDrop<Rc<RefCell<Chart2DRenderer>>>>,
    draw_fn: DrawFn,
    background_color: Color4,
    absolute: bool,
}

impl Chart2DElement {
    /// Create a new chart element with a draw function
    ///
    /// The draw function is called during paint with a mutable reference
    /// to the renderer, allowing you to call draw_line, draw_rect, etc.
    ///
    /// Native constructs the renderer eagerly (panicking without a GPU, as
    /// before); wasm defers construction to the first paint.
    pub fn new<F>(draw_fn: F) -> Self
    where
        F: Fn(&mut Chart2DRenderer, Bounds<Pixels>) + 'static,
    {
        #[cfg(not(target_family = "wasm"))]
        let renderer = Some(ManuallyDrop::new(Rc::new(RefCell::new(
            Chart2DRenderer::new(),
        ))));
        #[cfg(target_family = "wasm")]
        let renderer = None;
        Self {
            renderer,
            draw_fn: Box::new(draw_fn),
            background_color: [1.0, 1.0, 1.0, 1.0], // White background
            absolute: false,
        }
    }

    /// Create with a specific renderer instance (for sharing state)
    pub fn with_renderer<F>(renderer: Rc<RefCell<Chart2DRenderer>>, draw_fn: F) -> Self
    where
        F: Fn(&mut Chart2DRenderer, Bounds<Pixels>) + 'static,
    {
        Self {
            renderer: Some(ManuallyDrop::new(renderer)),
            draw_fn: Box::new(draw_fn),
            background_color: [1.0, 1.0, 1.0, 1.0],
            absolute: false,
        }
    }

    /// Set the background color
    pub fn background_color(mut self, color: Color4) -> Self {
        self.background_color = color;
        self
    }

    /// Set background to transparent
    pub fn transparent(mut self) -> Self {
        self.background_color = [0.0, 0.0, 0.0, 0.0];
        self
    }

    /// Set absolute positioning (for overlaying multiple chart elements)
    pub fn absolute(mut self) -> Self {
        self.absolute = true;
        self
    }
}

impl IntoElement for Chart2DElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Chart2DElement {
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
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = if self.absolute {
            // Absolute positioning for overlay mode
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
            // Default relative positioning
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
        // Nothing to do in prepaint
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
        let width = width as u32;
        let height = height as u32;

        if width == 0 || height == 0 {
            return;
        }

        // Resolve the renderer. Native always has Some from `new()`; wasm
        // lazily builds one and parks it in RENDERER_CACHE (see above) so it
        // survives the element rebuild that every repaint triggers.
        #[cfg(not(target_family = "wasm"))]
        let renderer = (**self.renderer.as_ref().unwrap()).clone();

        #[cfg(target_family = "wasm")]
        let renderer = {
            let resolved = if let Some(renderer) = &self.renderer {
                Some((**renderer).clone())
            } else {
                let key = (
                    f32::from(bounds.origin.x).to_bits(),
                    f32::from(bounds.origin.y).to_bits(),
                );
                RENDERER_CACHE.with(|cache| {
                    let mut cache = cache.borrow_mut();
                    if let Some(renderer) = cache.get(&key) {
                        return Some(Rc::clone(renderer));
                    }
                    match Chart2DRenderer::try_new() {
                        Ok(renderer) => {
                            if cache.len() >= 32 {
                                // Origins shift on resize; drop stale entries.
                                cache.clear();
                            }
                            let renderer = Rc::new(RefCell::new(renderer));
                            cache.insert(key, Rc::clone(&renderer));
                            Some(renderer)
                        }
                        Err(_) => None,
                    }
                })
            };
            match resolved {
                Some(renderer) => renderer,
                None => {
                    if !Gpu2DContext::init_failed() {
                        // Device still initializing — keep polling until ready.
                        window.request_animation_frame();
                    }
                    // Permanent failure: paint nothing, stop rescheduling.
                    return;
                }
            }
        };

        #[cfg(target_family = "wasm")]
        {
            // 1. Paint the newest completed readback (may be one frame stale);
            //    a size mismatch drops stale pixels rather than painting garbage.
            if let Some((w, h, pixels)) = renderer.borrow_mut().take_readback() {
                if w == width && h == height {
                    if let Some(rgba_image) = RgbaImage::from_raw(w, h, pixels) {
                        let render_image = RenderImage::new(vec![Frame::new(rgba_image)]);
                        let _ = window.paint_image(
                            bounds,
                            Corners::default(),
                            Arc::new(render_image),
                            0,
                            false,
                        );
                    }
                    // Displayed content is current — stop here. Resubmitting
                    // would re-arm step 3 every vsync forever; the next redraw
                    // arrives via normal GPUI invalidation (resize/interaction).
                    return;
                }
                // Stale size: drop and fall through to redraw at the new size.
            }

            // 2. If a readback is still in flight, wait for it next frame.
            if renderer.borrow().readback_pending() {
                window.request_animation_frame();
                return;
            }

            // 3. Draw + submit + start the async readback, then repaint when it lands.
            {
                let mut renderer = renderer.borrow_mut();
                renderer.begin_frame(width, height, self.background_color);

                // Call the user's draw function
                (self.draw_fn)(&mut renderer, bounds);
                renderer.end_frame_async();
            }
            window.request_animation_frame();
        }

        #[cfg(not(target_family = "wasm"))]
        {
            // Begin frame
            {
                let mut renderer = renderer.borrow_mut();
                renderer.begin_frame(width, height, self.background_color);

                // Call the user's draw function
                (self.draw_fn)(&mut renderer, bounds);
            }

            // End frame and get pixels
            let pixels = {
                let mut renderer = renderer.borrow_mut();
                renderer.end_frame()
            };

            // Paint the rendered image
            if let Some(pixels) = pixels
                && let Some(rgba_image) = RgbaImage::from_raw(width, height, pixels)
            {
                let frame = Frame::new(rgba_image);
                let render_image = RenderImage::new(vec![frame]);

                let _ = window.paint_image(
                    bounds,
                    Corners::default(),
                    Arc::new(render_image),
                    0,
                    false,
                );
            }
        }
    }
}
