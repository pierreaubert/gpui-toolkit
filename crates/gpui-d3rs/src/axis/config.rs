//! Axis configuration

use super::orientation::AxisOrientation;

/// Axis configuration builder
///
/// # Example
///
/// ```
/// use d3rs::axis::{AxisConfig, AxisOrientation};
///
/// let config = AxisConfig::bottom()
///     .with_ticks(10)
///     .with_tick_size(6.0)
///     .with_formatter(|value| format!("{:.1}Hz", value))
///     .with_title("Frequency (Hz)");
/// ```
#[derive(Clone)]
pub struct AxisConfig {
    /// Axis orientation
    pub orientation: AxisOrientation,
    /// Approximate number of ticks
    pub tick_count: usize,
    /// Explicit tick values (overrides tick_count if provided)
    pub tick_values: Option<Vec<f64>>,
    /// Minor tick values (smaller ticks without labels)
    pub minor_tick_values: Option<Vec<f64>>,
    /// Minor tick size in pixels (usually smaller than main tick_size)
    pub minor_tick_size: f32,
    /// Tick size in pixels (length of tick mark)
    pub tick_size: f32,
    /// Padding between tick mark and label
    pub tick_padding: f32,
    /// Font size for labels
    pub label_font_size: f32,
    /// Custom tick formatter (return empty string to hide label)
    pub tick_format: Option<fn(f64) -> String>,
    /// Whether to show the domain line
    pub show_domain_line: bool,
    /// Domain line width
    pub domain_line_width: f32,
    /// Axis title (label)
    pub title: Option<String>,
    /// Title font size
    pub title_font_size: f32,
    /// Padding between tick labels and title
    pub title_padding: f32,
    /// Label rotation angle in degrees (0 = horizontal, -45 = angled down-left)
    /// Useful for long labels on bottom axis
    pub label_angle: f32,
}

impl Default for AxisConfig {
    fn default() -> Self {
        Self {
            orientation: AxisOrientation::Bottom,
            tick_count: 10,
            tick_values: None,
            minor_tick_values: None,
            minor_tick_size: 3.0,
            tick_size: 6.0,
            tick_padding: 4.0,
            label_font_size: 10.0,
            tick_format: None,
            show_domain_line: true,
            domain_line_width: 1.0,
            title: None,
            title_font_size: 12.0,
            title_padding: 8.0,
            label_angle: 0.0,
        }
    }
}

impl AxisConfig {
    /// Create a bottom-oriented axis
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::axis::AxisConfig;
    ///
    /// let axis = AxisConfig::bottom();
    /// ```
    pub fn bottom() -> Self {
        Self {
            orientation: AxisOrientation::Bottom,
            ..Default::default()
        }
    }

    /// Create a top-oriented axis
    pub fn top() -> Self {
        Self {
            orientation: AxisOrientation::Top,
            ..Default::default()
        }
    }

    /// Create a left-oriented axis
    pub fn left() -> Self {
        Self {
            orientation: AxisOrientation::Left,
            ..Default::default()
        }
    }

    /// Create a right-oriented axis
    pub fn right() -> Self {
        Self {
            orientation: AxisOrientation::Right,
            ..Default::default()
        }
    }

    /// Set the approximate number of ticks
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::axis::AxisConfig;
    ///
    /// let axis = AxisConfig::bottom().with_ticks(5);
    /// ```
    pub fn with_ticks(mut self, count: usize) -> Self {
        self.tick_count = count;
        self
    }

    /// Set explicit tick values (overrides tick_count)
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::axis::AxisConfig;
    ///
    /// let axis = AxisConfig::bottom()
    ///     .with_tick_values(vec![20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0]);
    /// ```
    pub fn with_tick_values(mut self, values: Vec<f64>) -> Self {
        self.tick_values = Some(values);
        self
    }

    /// Set minor tick values (smaller ticks without labels)
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::axis::AxisConfig;
    ///
    /// let axis = AxisConfig::bottom()
    ///     .with_tick_values(vec![20.0, 100.0, 1000.0, 10000.0])
    ///     .with_minor_tick_values(vec![30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0]);
    /// ```
    pub fn with_minor_tick_values(mut self, values: Vec<f64>) -> Self {
        self.minor_tick_values = Some(values);
        self
    }

    /// Set the minor tick size (length of minor tick mark)
    pub fn with_minor_tick_size(mut self, size: f32) -> Self {
        self.minor_tick_size = size;
        self
    }

    /// Set the tick size (length of tick mark)
    pub fn with_tick_size(mut self, size: f32) -> Self {
        self.tick_size = size;
        self
    }

    /// Set the padding between tick mark and label
    pub fn with_tick_padding(mut self, padding: f32) -> Self {
        self.tick_padding = padding;
        self
    }

    /// Set the label font size
    pub fn with_label_font_size(mut self, size: f32) -> Self {
        self.label_font_size = size;
        self
    }

    /// Set a custom tick formatter
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::axis::AxisConfig;
    ///
    /// let axis = AxisConfig::bottom()
    ///     .with_formatter(|v| format!("{:.0}Hz", v));
    /// ```
    pub fn with_formatter(mut self, formatter: fn(f64) -> String) -> Self {
        self.tick_format = Some(formatter);
        self
    }

    /// Hide the domain line
    pub fn hide_domain_line(mut self) -> Self {
        self.show_domain_line = false;
        self
    }

    /// Set the domain line width
    pub fn with_domain_line_width(mut self, width: f32) -> Self {
        self.domain_line_width = width;
        self
    }

    /// Set the axis title (label)
    ///
    /// For left/right axes, the title will be rendered vertically (parallel to the axis).
    /// For top/bottom axes, the title will be rendered horizontally.
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::axis::AxisConfig;
    ///
    /// let axis = AxisConfig::left()
    ///     .with_title("SPL (dB)");
    /// ```
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the title font size
    pub fn with_title_font_size(mut self, size: f32) -> Self {
        self.title_font_size = size;
        self
    }

    /// Set the padding between tick labels and title
    pub fn with_title_padding(mut self, padding: f32) -> Self {
        self.title_padding = padding;
        self
    }

    /// Set the label rotation angle in degrees
    ///
    /// Useful for long labels on bottom axis. Negative angles rotate counter-clockwise.
    /// Common values: -45 for diagonal, -90 for vertical.
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::axis::AxisConfig;
    ///
    /// let axis = AxisConfig::bottom().with_label_angle(-45.0);
    /// ```
    pub fn with_label_angle(mut self, angle_degrees: f32) -> Self {
        self.label_angle = angle_degrees;
        self
    }

    /// Calculate the total size needed for this axis
    ///
    /// For horizontal axes, this is the height.
    /// For vertical axes, this is the width.
    pub fn total_size(&self) -> f32 {
        let title_space = if self.title.is_some() {
            self.title_padding + self.title_font_size
        } else {
            0.0
        };

        match self.orientation {
            AxisOrientation::Top | AxisOrientation::Bottom => {
                // For angled labels, we need more vertical space
                let label_height = if self.label_angle.abs() > 0.1 {
                    // Approximate height for angled text: font_size * sin(angle) + some width component
                    let angle_rad = self.label_angle.abs() * std::f32::consts::PI / 180.0;
                    // Assume average label width of ~40px for frequency labels
                    let estimated_label_width = 40.0_f32;
                    estimated_label_width * angle_rad.sin() + self.label_font_size * angle_rad.cos()
                } else {
                    self.label_font_size
                };
                self.tick_size + self.tick_padding + label_height + 4.0 + title_space
            }
            AxisOrientation::Left | AxisOrientation::Right => {
                // For vertical, we need enough width for labels
                // This is an estimate - actual width depends on label content
                60.0 + title_space
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_axis_config_builder() {
        let config = AxisConfig::bottom()
            .with_ticks(5)
            .with_tick_size(8.0)
            .with_tick_padding(6.0);

        assert_eq!(config.tick_count, 5);
        assert_eq!(config.tick_size, 8.0);
        assert_eq!(config.tick_padding, 6.0);
    }

    #[test]
    fn test_axis_orientations() {
        assert_eq!(AxisConfig::bottom().orientation, AxisOrientation::Bottom);
        assert_eq!(AxisConfig::top().orientation, AxisOrientation::Top);
        assert_eq!(AxisConfig::left().orientation, AxisOrientation::Left);
        assert_eq!(AxisConfig::right().orientation, AxisOrientation::Right);
    }

    #[test]
    fn test_custom_formatter() {
        let config = AxisConfig::bottom().with_formatter(|v| format!("{:.2}", v));

        assert!(config.tick_format.is_some());
        let formatted = (config.tick_format.unwrap())(42.123);
        assert_eq!(formatted, "42.12");
    }
}
