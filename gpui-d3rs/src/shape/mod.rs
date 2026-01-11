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
pub mod curve;
pub mod link;
pub mod path;
pub mod pie;
pub mod radial;
pub mod stack;
pub mod symbol;

#[cfg(feature = "gpui")]
mod bar;
#[cfg(feature = "gpui")]
pub mod contour;
#[cfg(feature = "gpui")]
mod line;
#[cfg(feature = "gpui")]
mod scatter;

// Re-export existing chart rendering functions (GPUI only)
#[cfg(feature = "gpui")]
pub use bar::{
    BarConfig, BarDatum, GroupedBarConfig, GroupedBarDatum, GroupedBarMeta, analyze_grouped_data,
    render_bars, render_grouped_bars,
};
#[cfg(feature = "gpui")]
pub use contour::{
    ContourBandElement, ContourConfig, ContourElement, HeatmapData, HeatmapElement,
    heat_color_scale, render_contour, render_contour_bands, render_heatmap, viridis_color_scale,
};
#[cfg(feature = "gpui")]
pub use line::{CurveType, LineConfig, LinePoint, render_line};
#[cfg(feature = "gpui")]
pub use scatter::{ScatterConfig, ScatterPoint, render_scatter};

// Re-export new shape utilities (no GPUI dependency)
pub use arc::{Arc, ArcDatum, arc_points};
pub use area::{Area, SimpleArea, area_points};
pub use curve::Curve;
pub use link::{
    Link, LinkDirection, RadialLink, link_horizontal, link_radial, link_step, link_vertical,
};
pub use path::{Path, PathBuilder, PathCommand, Point};
pub use pie::{Pie, PieSlice, donut, half_pie, pie};
pub use radial::{
    RadialAreaConfig, RadialLineConfig, RadialPoint, polar_grid_circles, polar_grid_rays,
    radial_area, radial_line,
};
pub use stack::{Stack, StackOffset, StackOrder, StackSeries, stack, stack_expand, streamgraph};
pub use symbol::{Symbol, SymbolType, symbol_radius};
