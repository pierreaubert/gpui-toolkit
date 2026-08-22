use super::style::CurveType;
use super::style::StrokeDashArray;
use crate::color::D3Color;
use crate::render2d::{Renderer2D, VelloBackend};

/// Configuration for line chart rendering
#[derive(Clone)]
pub struct LineConfig {
    /// Stroke color for the line
    pub stroke_color: D3Color,
    /// Line width in pixels
    pub stroke_width: f32,
    /// Opacity of the line (0.0 - 1.0)
    pub opacity: f32,
    /// Curve interpolation type
    pub curve: CurveType,
    /// Whether to show points at data locations
    pub show_points: bool,
    /// Point radius if show_points is true
    pub point_radius: f32,
    /// Fill color for points
    pub point_fill_color: Option<D3Color>,
    /// Stroke dash array pattern (None = solid line)
    pub dash_array: Option<StrokeDashArray>,
    /// High-level renderer selection for the line mark.
    pub renderer_2d: Renderer2D,
    /// Vello raster backend when `renderer_2d` is [`Renderer2D::Vello`].
    pub vello_backend: VelloBackend,
}

impl Default for LineConfig {
    fn default() -> Self {
        Self {
            stroke_color: D3Color::from_hex(0x4682b4), // Steel blue
            stroke_width: 2.0,
            opacity: 1.0,
            curve: CurveType::Linear,
            show_points: false,
            point_radius: 3.0,
            point_fill_color: None,
            dash_array: None,
            renderer_2d: Renderer2D::default(),
            vello_backend: VelloBackend::default(),
        }
    }
}

impl LineConfig {
    /// Create a new line configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Select the renderer through the library-owned line dispatch helper.
    pub fn renderer_2d(mut self, renderer: Renderer2D) -> Self {
        self.renderer_2d = renderer;
        self
    }

    /// Select the Vello raster backend.
    pub fn vello_backend(mut self, backend: VelloBackend) -> Self {
        self.vello_backend = backend;
        self
    }

    /// Create a line configuration from design-system interaction defaults.
    #[cfg(feature = "gpui")]
    pub fn from_design(design: &gpui_design::DesignSystem) -> Self {
        Self {
            stroke_width: design.interaction.border_width.max(2.0),
            point_radius: (design.spacing.grid_unit * 0.75).max(2.0),
            ..Self::default()
        }
    }

    /// Apply design-system interaction defaults.
    #[cfg(feature = "gpui")]
    pub fn with_design(mut self, design: &gpui_design::DesignSystem) -> Self {
        self.stroke_width = design.interaction.border_width.max(2.0);
        self.point_radius = (design.spacing.grid_unit * 0.75).max(2.0);
        self
    }

    /// Set the stroke color
    pub fn stroke_color(mut self, color: D3Color) -> Self {
        self.stroke_color = color;
        self
    }

    /// Set the stroke width
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Set the opacity
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the curve type
    pub fn curve(mut self, curve: CurveType) -> Self {
        self.curve = curve;
        self
    }

    /// Enable point rendering
    pub fn show_points(mut self, show: bool) -> Self {
        self.show_points = show;
        self
    }

    /// Set point radius
    pub fn point_radius(mut self, radius: f32) -> Self {
        self.point_radius = radius;
        self
    }

    /// Set point fill color
    pub fn point_fill_color(mut self, color: D3Color) -> Self {
        self.point_fill_color = Some(color);
        self
    }

    /// Set the stroke dash array pattern.
    ///
    /// Use predefined patterns or a custom one:
    /// - `StrokeDashArray::Dotted` - small dots
    /// - `StrokeDashArray::Dashed` - standard dashes
    /// - `StrokeDashArray::DashDot` - alternating dash-dot
    /// - `StrokeDashArray::Custom(vec![on, off, ...])` - custom pattern
    pub fn dash_array(mut self, pattern: StrokeDashArray) -> Self {
        self.dash_array = Some(pattern);
        self
    }
}
