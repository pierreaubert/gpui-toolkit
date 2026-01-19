//! Theme integration for axes

use gpui::Rgba;

/// Theme trait for axis rendering
///
/// This trait can be implemented for any theme type to provide
/// colors for axis elements.
pub trait AxisTheme {
    /// Color for axis lines and tick marks
    fn axis_line_color(&self) -> Rgba;

    /// Color for axis labels
    fn axis_label_color(&self) -> Rgba;

    /// Background color (optional, for context)
    fn background_color(&self) -> Option<Rgba> {
        None
    }
}

/// Default theme with neutral colors
#[derive(Debug, Clone, Copy)]
pub struct DefaultAxisTheme;

impl AxisTheme for DefaultAxisTheme {
    fn axis_line_color(&self) -> Rgba {
        Rgba {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        }
    }

    fn axis_label_color(&self) -> Rgba {
        Rgba {
            r: 0.3,
            g: 0.3,
            b: 0.3,
            a: 1.0,
        }
    }
}
