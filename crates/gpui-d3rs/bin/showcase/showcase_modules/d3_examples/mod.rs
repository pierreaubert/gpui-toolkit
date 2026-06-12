//! D3.js Observable Examples
//!
//! This module contains ports of D3.js examples from Observable.
//! Each example demonstrates both:
//! 1. Low-level API usage (direct scale/generator manipulation)
//! 2. High-level API usage (ready-made render functions)
//!
//! Source examples from: <https://observablehq.com/@d3>

pub mod calendar;
pub mod choropleth;
pub mod circle_packing;
pub mod difference_chart;
pub mod faithful_data;
pub mod flare_data;
pub mod histogram;
pub mod horizon;
pub mod kernel_density_estimation;
pub mod parallel_coordinates;
pub mod parallel_sets;
pub mod path_utils;
pub mod radial_line;
pub mod radial_tree;
pub mod realtime_horizon;
pub mod revenue;
pub mod ridgeline;
pub mod sankey;
pub mod stacked_grouped_bars;
pub mod treemap;
pub mod versor;
pub mod volcano_contours;
pub mod volcano_data;
pub mod voronoi_airports;

pub mod box_plot;
pub mod chord;
pub mod donut_chart;
pub mod electric_usage;
pub mod force_directed;
pub mod hertzsprung_russell;
pub mod hexbin;
pub mod line_chart;
pub mod pie_chart;
pub mod stacked_area;
pub mod stacked_bar;
pub mod star_map;
pub mod streamgraph;
pub mod sunburst;
pub mod temperature_trends;
pub mod voronoi_labels;
pub mod voronoi_stippling;

pub use kernel_density_estimation::KernelType;
pub use stacked_grouped_bars::BarLayout;
pub use treemap::TilingMethod;
pub use volcano_contours::render;
