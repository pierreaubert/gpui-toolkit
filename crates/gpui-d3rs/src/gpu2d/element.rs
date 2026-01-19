//! GPUI Element wrapper for 2D chart rendering

use super::primitives::Color4;
use super::renderer::Chart2DRenderer;
use gpui::*;
use image::{Frame, RgbaImage};
use std::cell::RefCell;
use std::panic;
use std::rc::Rc;
use std::sync::Arc;

/// Draw function type for Chart2DElement
pub type DrawFn = Box<dyn Fn(&mut Chart2DRenderer, Bounds<Pixels>)>;

/// GPUI Element for GPU-accelerated 2D chart rendering
pub struct Chart2DElement {
    renderer: Rc<RefCell<Chart2DRenderer>>,
    draw_fn: DrawFn,
    background_color: Color4,
    absolute: bool,
}

impl Chart2DElement {
    /// Create a new chart element with a draw function
    ///
    /// The draw function is called during paint with a mutable reference
    /// to the renderer, allowing you to call draw_line, draw_rect, etc.
    pub fn new<F>(draw_fn: F) -> Self
    where
        F: Fn(&mut Chart2DRenderer, Bounds<Pixels>) + 'static,
    {
        Self {
            renderer: Rc::new(RefCell::new(Chart2DRenderer::new())),
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
            renderer,
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

        // Begin frame
        {
            let mut renderer = self.renderer.borrow_mut();
            renderer.begin_frame(width, height, self.background_color);

            // Call the user's draw function
            (self.draw_fn)(&mut renderer, bounds);
        }

        // End frame and get pixels
        let pixels = {
            let mut renderer = self.renderer.borrow_mut();
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
