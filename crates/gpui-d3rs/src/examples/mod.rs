//! Complete D3.js Observable example implementations.
//!
//! Each module implements a full visualization pipeline using d3rs,
//! mirroring the corresponding Observable notebook. The output is a
//! plain data struct with all computed geometry — no rendering dependency.
//!
//! These serve three purposes:
//! 1. **Documentation** — idiomatic d3rs usage examples
//! 2. **Golden tests** — validated against D3.js output
//! 3. **Showcase** — fed into GPUI for visual rendering

pub mod box_plot;
pub mod chord;
pub mod circle_packing;
pub mod difference_chart;
pub mod donut_chart;
pub mod electric_usage;
pub mod force_directed;
pub mod hertzsprung_russell;
pub mod hexbin;
pub mod line_chart;
pub mod parallel_sets;
pub mod pie_chart;
pub mod radial_tree;
pub mod ridgeline;
pub mod sankey;
pub mod stacked_area;
pub mod stacked_bar;
pub mod star_map;
pub mod streamgraph;
pub mod sunburst;
pub mod temperature_trends;
pub mod voronoi_airports;
pub mod voronoi_labels;
pub mod voronoi_stippling;
