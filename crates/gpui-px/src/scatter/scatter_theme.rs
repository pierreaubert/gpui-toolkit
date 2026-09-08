use gpui::{Rgba, hsla, rgb};

/// Theme for scatter chart styling
#[derive(Debug, Clone)]
pub struct ScatterTheme {
    /// Background color for plot area
    pub plot_background: Rgba,
    /// Title text color
    pub title_color: Rgba,
    /// Legend text color
    pub legend_text_color: Rgba,
    /// Axis line and grid color
    pub axis_line_color: Rgba,
    /// Axis tick label color
    pub axis_label_color: Rgba,
}

impl Default for ScatterTheme {
    fn default() -> Self {
        Self {
            plot_background: rgb(0xf8f8f8),
            title_color: hsla(0.0, 0.0, 0.2, 1.0).into(),
            legend_text_color: Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.6,
            },
            // Matches the previously hardcoded `DefaultAxisTheme`.
            axis_line_color: Rgba {
                r: 0.5,
                g: 0.5,
                b: 0.5,
                a: 1.0,
            },
            axis_label_color: Rgba {
                r: 0.3,
                g: 0.3,
                b: 0.3,
                a: 1.0,
            },
        }
    }
}
