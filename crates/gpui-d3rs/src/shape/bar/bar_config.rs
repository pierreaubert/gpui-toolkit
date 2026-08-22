use crate::color::D3Color;
use crate::render2d::{Renderer2D, VelloBackend};

/// Configuration for bar chart rendering
#[derive(Clone)]
pub struct BarConfig {
    /// Fill color for bars
    pub fill_color: D3Color,
    /// Opacity of bars (0.0 - 1.0)
    pub opacity: f32,
    /// Gap between bars in pixels
    pub bar_gap: f32,
    /// Corner radius for bars
    pub border_radius: f32,
    /// Optional stroke color
    pub stroke_color: Option<D3Color>,
    /// Stroke width in pixels
    pub stroke_width: f32,
    /// High-level renderer selection for the bar mark.
    pub renderer_2d: Renderer2D,
    /// Vello raster backend when `renderer_2d` is [`Renderer2D::Vello`].
    pub vello_backend: VelloBackend,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            fill_color: D3Color::from_hex(0x4682b4), // Steel blue
            opacity: 0.8,
            bar_gap: 2.0,
            border_radius: 2.0,
            stroke_color: None,
            stroke_width: 1.0,
            renderer_2d: Renderer2D::default(),
            vello_backend: VelloBackend::default(),
        }
    }
}

impl BarConfig {
    /// Create a new bar configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Select the renderer through the library-owned bar dispatch helper.
    pub fn renderer_2d(mut self, renderer: Renderer2D) -> Self {
        self.renderer_2d = renderer;
        self
    }

    /// Select the Vello raster backend.
    pub fn vello_backend(mut self, backend: VelloBackend) -> Self {
        self.vello_backend = backend;
        self
    }

    /// Create a bar configuration from design-system spacing and shape defaults.
    #[cfg(feature = "gpui")]
    pub fn from_design(design: &gpui_design::DesignSystem) -> Self {
        Self {
            bar_gap: design.spacing.grid_unit * 0.5,
            border_radius: design.corners.sm,
            stroke_width: design.interaction.border_width.max(1.0),
            ..Self::default()
        }
    }

    /// Apply design-system spacing and shape defaults.
    #[cfg(feature = "gpui")]
    pub fn with_design(mut self, design: &gpui_design::DesignSystem) -> Self {
        self.bar_gap = design.spacing.grid_unit * 0.5;
        self.border_radius = design.corners.sm;
        self.stroke_width = design.interaction.border_width.max(1.0);
        self
    }

    /// Set the fill color
    pub fn fill_color(mut self, color: D3Color) -> Self {
        self.fill_color = color;
        self
    }

    /// Set the opacity
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the gap between bars
    pub fn bar_gap(mut self, gap: f32) -> Self {
        self.bar_gap = gap;
        self
    }

    /// Set the border radius
    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    /// Set the stroke color
    pub fn stroke_color(mut self, color: D3Color) -> Self {
        self.stroke_color = Some(color);
        self
    }

    /// Set the stroke width
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }
}
