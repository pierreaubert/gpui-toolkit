pub mod chromatic;
mod hcl;
mod interpolate;
pub mod rgb;
pub mod scheme;

pub use chromatic::{DivergingScale, DivergingScheme, SequentialScale, SequentialScheme};
pub use hcl::{Hcl, Lab};
pub use interpolate::{interpolate_colors, sequential_color};
pub use rgb::D3Color;
pub use scheme::ColorScheme;
