//! Color Schemes (d3-scale-chromatic)
//!
//! Sequential and diverging color schemes with perceptually uniform interpolation.

mod diverging_scale;
mod diverging_scheme;
mod sequential_scale;
mod sequential_scheme;
#[cfg(test)]
mod tests;

pub use diverging_scale::*;
pub use diverging_scheme::*;
pub use sequential_scale::*;
pub use sequential_scheme::*;
