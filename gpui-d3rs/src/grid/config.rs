//! Grid configuration

/// Grid configuration builder
///
/// # Example
///
/// ```
/// use d3rs::grid::GridConfig;
///
/// let config = GridConfig::new()
///     .with_vertical_lines(true)
///     .with_horizontal_lines(true)
///     .with_dots(true);
/// ```
#[derive(Debug, Clone)]
pub struct GridConfig {
    /// Show vertical grid lines
    pub show_vertical_lines: bool,
    /// Show horizontal grid lines
    pub show_horizontal_lines: bool,
    /// Show dots at grid intersections
    pub show_dots: bool,
    /// Line width for grid lines
    pub line_width: f32,
    /// Dot radius
    pub dot_radius: f32,
    /// Line opacity (0.0 - 1.0)
    pub line_opacity: f32,
    /// Dot opacity (0.0 - 1.0)
    pub dot_opacity: f32,
    /// Explicit vertical line positions (overrides scale ticks if provided)
    pub vertical_line_values: Option<Vec<f64>>,
    /// Explicit horizontal line positions (overrides scale ticks if provided)
    pub horizontal_line_values: Option<Vec<f64>>,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            show_vertical_lines: false,
            show_horizontal_lines: false,
            show_dots: true,
            line_width: 1.0,
            dot_radius: 2.0,
            line_opacity: 0.2,
            dot_opacity: 0.4,
            vertical_line_values: None,
            horizontal_line_values: None,
        }
    }
}

impl GridConfig {
    /// Create a new grid configuration with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a grid with only dots (no lines)
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::grid::GridConfig;
    ///
    /// let config = GridConfig::dots_only();
    /// ```
    pub fn dots_only() -> Self {
        Self {
            show_dots: true,
            show_vertical_lines: false,
            show_horizontal_lines: false,
            ..Default::default()
        }
    }

    /// Create a grid with lines and dots
    pub fn with_lines() -> Self {
        Self {
            show_dots: true,
            show_vertical_lines: true,
            show_horizontal_lines: true,
            ..Default::default()
        }
    }

    /// Create a grid with only lines (no dots)
    pub fn lines_only() -> Self {
        Self {
            show_dots: false,
            show_vertical_lines: true,
            show_horizontal_lines: true,
            ..Default::default()
        }
    }

    /// Set whether to show vertical lines
    pub fn with_vertical_lines(mut self, show: bool) -> Self {
        self.show_vertical_lines = show;
        self
    }

    /// Set whether to show horizontal lines
    pub fn with_horizontal_lines(mut self, show: bool) -> Self {
        self.show_horizontal_lines = show;
        self
    }

    /// Set whether to show dots
    pub fn with_dots(mut self, show: bool) -> Self {
        self.show_dots = show;
        self
    }

    /// Set the line width
    pub fn with_line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    /// Set the dot radius
    pub fn with_dot_radius(mut self, radius: f32) -> Self {
        self.dot_radius = radius;
        self
    }

    /// Set the line opacity
    pub fn with_line_opacity(mut self, opacity: f32) -> Self {
        self.line_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the dot opacity
    pub fn with_dot_opacity(mut self, opacity: f32) -> Self {
        self.dot_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set explicit vertical line positions (overrides scale ticks)
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::grid::GridConfig;
    ///
    /// let config = GridConfig::with_lines()
    ///     .with_vertical_values(vec![50.0, 500.0, 5000.0]);
    /// ```
    pub fn with_vertical_values(mut self, values: Vec<f64>) -> Self {
        self.vertical_line_values = Some(values);
        self
    }

    /// Set explicit horizontal line positions (overrides scale ticks)
    pub fn with_horizontal_values(mut self, values: Vec<f64>) -> Self {
        self.horizontal_line_values = Some(values);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_config_dots_only() {
        let config = GridConfig::dots_only();
        assert!(config.show_dots);
        assert!(!config.show_vertical_lines);
        assert!(!config.show_horizontal_lines);
    }

    #[test]
    fn test_grid_config_with_lines() {
        let config = GridConfig::with_lines();
        assert!(config.show_dots);
        assert!(config.show_vertical_lines);
        assert!(config.show_horizontal_lines);
    }

    #[test]
    fn test_grid_config_builder() {
        let config = GridConfig::new()
            .with_vertical_lines(true)
            .with_line_width(2.0)
            .with_dot_radius(3.0);

        assert!(config.show_vertical_lines);
        assert_eq!(config.line_width, 2.0);
        assert_eq!(config.dot_radius, 3.0);
    }
}
