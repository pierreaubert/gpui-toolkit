//! Heatmap chart - Plotly Express style API.

#[cfg(not(feature = "gpu-2d"))]
use d3rs::shape::render_heatmap;

mod heatmap_chart;

pub use heatmap_chart::*;
