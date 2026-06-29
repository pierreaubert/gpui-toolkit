use super::super::primitives::Color4;

/// GPU-accelerated grid configuration
#[derive(Clone)]
pub struct GpuGridConfig {
    /// Width of grid lines in pixels
    pub line_width: f32,
    /// Opacity of grid lines (0.0 - 1.0)
    pub line_opacity: f32,
    /// Radius of dots at intersections
    pub dot_radius: f32,
    /// Opacity of dots (0.0 - 1.0)
    pub dot_opacity: f32,
    /// Show vertical grid lines
    pub show_vertical_lines: bool,
    /// Show horizontal grid lines
    pub show_horizontal_lines: bool,
    /// Show dots at grid intersections
    pub show_dots: bool,
    /// Custom vertical line positions (if None, uses scale ticks)
    pub vertical_line_values: Option<Vec<f64>>,
    /// Custom horizontal line positions (if None, uses scale ticks)
    pub horizontal_line_values: Option<Vec<f64>>,
    /// Grid line color
    pub line_color: Color4,
}

impl Default for GpuGridConfig {
    fn default() -> Self {
        Self {
            line_width: 1.0,
            line_opacity: 0.2,
            dot_radius: 2.0,
            dot_opacity: 0.3,
            show_vertical_lines: true,
            show_horizontal_lines: true,
            show_dots: false,
            vertical_line_values: None,
            horizontal_line_values: None,
            line_color: [0.5, 0.5, 0.5, 1.0], // Gray
        }
    }
}

impl GpuGridConfig {
    /// Create a GPU grid configuration from design-system interaction defaults.
    #[cfg(feature = "gpui")]
    pub fn from_design(design: &gpui_design::DesignSystem) -> Self {
        Self {
            line_width: design.interaction.border_width.max(1.0),
            dot_radius: (design.spacing.grid_unit * 0.5).max(1.0),
            ..Self::default()
        }
    }

    /// Create a grid with only lines
    pub fn with_lines() -> Self {
        Self::default()
    }

    /// Create a grid with only dots
    pub fn with_dots() -> Self {
        Self {
            show_vertical_lines: false,
            show_horizontal_lines: false,
            show_dots: true,
            ..Default::default()
        }
    }

    /// Create a grid with both lines and dots
    pub fn with_lines_and_dots() -> Self {
        Self {
            show_dots: true,
            ..Default::default()
        }
    }

    /// Set the line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    /// Set the line opacity
    pub fn line_opacity(mut self, opacity: f32) -> Self {
        self.line_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the line color
    pub fn line_color(mut self, color: Color4) -> Self {
        self.line_color = color;
        self
    }
}
