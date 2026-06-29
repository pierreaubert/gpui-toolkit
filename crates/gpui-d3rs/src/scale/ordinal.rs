//! Ordinal and band scales for categorical data
//!
//! Ordinal scales map discrete domain values to discrete range values.
//! Band scales are a variant that divide a continuous range into uniform bands.

mod band_scale;
mod ordinal_scale;
mod point_scale;
#[cfg(test)]
mod tests;

pub use band_scale::*;
pub use ordinal_scale::*;
pub use point_scale::*;
