//! D3.js Observable Examples
//!
//! This module contains ports of D3.js examples from Observable.
//! Each example demonstrates both:
//! 1. Low-level API usage (direct scale/generator manipulation)
//! 2. High-level API usage (ready-made render functions)
//!
//! Source examples from: https://observablehq.com/@d3

pub mod choropleth;
pub mod faithful_data;
pub mod flare_data;
pub mod histogram;
pub mod horizon;
pub mod kernel_density_estimation;
pub mod path_utils;
pub mod revenue;
pub mod stacked_grouped_bars;
pub mod treemap;
pub mod versor;
pub mod volcano_contours;
pub mod volcano_data;

pub use kernel_density_estimation::KernelType;
pub use stacked_grouped_bars::BarLayout;
pub use treemap::TilingMethod;
pub use volcano_contours::render;
