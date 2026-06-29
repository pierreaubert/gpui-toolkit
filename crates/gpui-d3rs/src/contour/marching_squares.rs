//! Marching Squares algorithm for contour generation
//!
//! Implements the marching squares algorithm for generating contour lines
//! from a 2D scalar field.

mod contour;
mod contour_band;
mod contour_generator;
mod contour_ring;
mod contour_segment;
mod misc;
#[cfg(test)]
mod tests;

pub use contour::*;
pub use contour_band::*;
pub use contour_generator::*;
pub use contour_ring::*;
