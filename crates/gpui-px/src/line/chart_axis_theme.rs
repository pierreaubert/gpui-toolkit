use d3rs::axis::AxisTheme;
use gpui::Rgba;

/// Axis theme adapter for d3rs
pub(super) struct ChartAxisTheme {
    pub(super) axis_line_color: Rgba,
    pub(super) axis_label_color: Rgba,
}

impl AxisTheme for ChartAxisTheme {
    fn axis_line_color(&self) -> Rgba {
        self.axis_line_color
    }

    fn axis_label_color(&self) -> Rgba {
        self.axis_label_color
    }
}
