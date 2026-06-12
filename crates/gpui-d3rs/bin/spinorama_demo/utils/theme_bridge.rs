//! Bridge between gpui-ui-kit Theme and d3rs AxisTheme.

use d3rs::axis::AxisTheme;
use gpui::Rgba;
use gpui_ui_kit::theme::Theme;

/// Wraps a gpui-ui-kit Theme to implement d3rs AxisTheme.
pub struct ChartTheme {
    pub line_color: Rgba,
    pub label_color: Rgba,
    pub bg: Option<Rgba>,
}

impl ChartTheme {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            line_color: theme.text_secondary,
            label_color: theme.text_primary,
            bg: Some(theme.surface),
        }
    }
}

impl AxisTheme for ChartTheme {
    fn axis_line_color(&self) -> Rgba {
        self.line_color
    }

    fn axis_label_color(&self) -> Rgba {
        self.label_color
    }

    fn background_color(&self) -> Option<Rgba> {
        self.bg
    }
}
