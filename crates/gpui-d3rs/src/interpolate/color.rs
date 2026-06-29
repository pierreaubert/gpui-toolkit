//! Color interpolation functions
//!
//! Provides interpolation in various color spaces including RGB, HSL, LAB, HCL,
//! and Cubehelix.

mod cubehelix;
mod hcl;
mod hsl;
mod interpolate;
mod lab;
#[cfg(test)]
mod tests;

pub use cubehelix::*;
pub use hcl::*;
pub use hsl::*;
pub use interpolate::*;
pub use lab::*;
