/// Default chart color (Plotly blue)
#[cfg(feature = "gpui")]
pub(crate) const DEFAULT_COLOR: u32 = 0x1f77b4;

/// Default chart width in pixels
pub(crate) const DEFAULT_WIDTH: f32 = 600.0;

/// Default chart height in pixels
pub(crate) const DEFAULT_HEIGHT: f32 = 400.0;

/// Default padding fraction for auto-domain calculation
#[cfg(feature = "gpui")]
pub(crate) const DEFAULT_PADDING_FRACTION: f64 = 0.05;

/// Default title font size
#[cfg(feature = "gpui")]
pub(crate) const DEFAULT_TITLE_FONT_SIZE: f32 = 16.0;

/// Title area height (font size + padding)
#[cfg(feature = "gpui")]
pub(crate) const TITLE_AREA_HEIGHT: f32 = 24.0;
