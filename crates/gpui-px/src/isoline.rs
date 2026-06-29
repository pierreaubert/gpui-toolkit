//! Isoline chart (unfilled contour lines) - Plotly Express style API.

#[cfg(not(feature = "gpu-2d"))]
use d3rs::shape::render_contour;

mod isoline_chart;

pub use isoline_chart::*;
