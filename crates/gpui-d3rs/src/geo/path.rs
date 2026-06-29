//! GeoPath - Rendering GeoJSON to paths
//!
//! This module provides functionality for rendering GeoJSON features
//! to SVG path strings or other path representations.

#![allow(dead_code, unused_imports)]

mod clip;
mod geo_path;
mod geo_path_config;
mod preclip;
pub mod stream;
#[cfg(test)]
mod tests;
mod types;

pub use geo_path::*;
pub use geo_path_config::*;
pub use types::*;
