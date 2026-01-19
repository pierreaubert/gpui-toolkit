pub mod chromatic;
mod interpolate;
pub mod rgb;
pub mod scheme;

pub use interpolate::{interpolate_colors, sequential_color};
pub use rgb::D3Color;
pub use scheme::ColorScheme;
