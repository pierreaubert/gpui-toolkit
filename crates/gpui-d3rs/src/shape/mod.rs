//! Shape rendering module
//!
//! This module provides functions for rendering common chart shapes like bars, lines,
//! scatter plots, arcs, pies, areas, and more.
//!
//! # Submodules
//!
//! - `path`: SVG-like path building utilities
//! - `arc`: Arc generator for pie and donut charts
//! - `pie`: Pie layout generator
//! - `area`: Area shape generator
//! - `curve`: Curve interpolation algorithms
//! - `symbol`: Symbol generators for data markers
//! - `stack`: Stack layout for stacked charts
//! - `link`: Link generators for tree/network diagrams
//! - `radial`: Radial line/area generators for polar visualizations
//! - `bar`: Bar chart rendering
//! - `line`: Line chart rendering
//! - `scatter`: Scatter plot rendering
//!
//! # Example
//!
//! ```rust
//! use d3rs::shape::path::PathBuilder;
//! use d3rs::shape::pie::Pie;
//! use d3rs::shape::symbol::{Symbol, SymbolType};
//!
//! // Create a custom path
//! let path = PathBuilder::new()
//!     .move_to(0.0, 0.0)
//!     .line_to(100.0, 0.0)
//!     .line_to(100.0, 100.0)
//!     .close_path()
//!     .build();
//!
//! // Create pie slices
//! let values = vec![10.0, 20.0, 30.0, 40.0];
//! let slices = Pie::new().generate(&values, |v| *v);
//!
//! // Create a symbol
//! let star = Symbol::star(64.0);
//! let star_path = star.generate();
//! ```

pub mod arc;
pub mod area;
#[cfg(any(test, all(feature = "gpui", not(test))))]
pub(crate) mod contour_smoothing;
pub mod curve;
pub mod link;
pub mod path;
pub mod pie;
pub mod radial;
pub mod stack;
pub mod symbol;

#[cfg(all(feature = "gpui", not(test)))]
mod bar;
#[cfg(all(feature = "gpui", not(test)))]
pub mod contour;
pub mod line;
pub mod scatter;

// Re-export existing chart rendering functions (GPUI only)
#[cfg(all(feature = "gpui", not(test)))]
pub use bar::{
    BarConfig, BarDatum, GroupedBarConfig, GroupedBarDatum, GroupedBarMeta, analyze_grouped_data,
    render_bars, render_grouped_bars,
};
#[cfg(all(feature = "gpui", not(test)))]
pub use contour::{
    ContourBandElement, ContourConfig, ContourElement, HeatmapData, HeatmapElement,
    heat_color_scale, render_contour, render_contour_bands, render_heatmap, viridis_color_scale,
};
pub use line::{
    CurveType, LineConfig, LinePoint, LineRenderError, StrokeDashArray, validate_line_inputs,
};
#[cfg(all(feature = "gpui", not(test)))]
pub use line::{render_line, try_render_line};
pub use scatter::{ScatterConfig, ScatterPoint, ScatterRenderError, validate_scatter_inputs};
#[cfg(all(feature = "gpui", not(test)))]
pub use scatter::{render_scatter, try_render_scatter};
#[cfg(feature = "vello")]
pub use scatter::scatter_chart_scene;
#[cfg(all(feature = "vello-gpui", not(test)))]
pub use scatter::render_scatter_vello;

// Re-export new shape utilities (no GPUI dependency)
pub use arc::{Arc, ArcDatum, ArcGenerationError, arc_points, try_arc_points};
pub use area::{Area, AreaGenerationError, SimpleArea, area_points, try_area_points};
pub use curve::Curve;
pub use link::{
    Link, LinkDirection, LinkGenerationError, RadialLink, link_horizontal, link_radial, link_step,
    link_vertical, try_link_horizontal, try_link_radial, try_link_step, try_link_vertical,
};
pub use path::{Path, PathBuilder, PathCommand, Point};
pub use pie::{Pie, PieSlice, donut, half_pie, pie, try_donut, try_half_pie, try_pie};
pub use radial::{
    RadialAreaConfig, RadialGenerationError, RadialLineConfig, RadialPoint, RadialPointField,
    polar_grid_circles, polar_grid_rays, radial_area, radial_line, try_polar_grid_circles,
    try_polar_grid_rays, try_radial_area, try_radial_line,
};
pub use stack::{
    Stack, StackLayoutError, StackOffset, StackOrder, StackSeries, stack, stack_expand,
    streamgraph, try_stack, try_stack_expand, try_streamgraph,
};
pub use symbol::{Symbol, SymbolGenerationError, SymbolType, symbol_radius, try_symbol_radius};
