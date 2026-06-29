//! Surface rendering for GPUI
//!
//! Provides 3D surface visualization using 2D projection and painter's algorithm.

mod color_scale_type;
mod misc;
mod projection_impl;
mod surface_config;
mod surface_element;

pub use color_scale_type::*;
pub use surface_config::*;
pub use surface_element::*;

#[cfg(test)]
mod tests;
