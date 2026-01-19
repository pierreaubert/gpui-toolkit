//! # d3rs - D3.js-inspired plotting library for GPUI
//!
//! A Rust plotting library that brings D3.js concepts to GPUI using idiomatic Rust patterns.
//!
//! ## Features
//!
//! - **Scales**: Linear, log, power, symlog, quantize, quantile, threshold scales
//! - **Axes**: Four orientations (Top, Right, Bottom, Left) with customizable formatting
//! - **Colors**: RGB/HSL with interpolation and categorical schemes
//! - **Shapes**: Bars, lines, areas, scatter plots, arcs, pies, symbols, stacks
//! - **Curves**: Linear, step, basis, cardinal, catmull-rom, monotone, natural
//! - **Grids**: Dots and lines at tick intersections
//! - **Legends**: Configurable position and formatting
//! - **Arrays**: Statistics, search, binning, transformations (d3-array)
//! - **Interpolation**: Numeric, color (HSL/LAB/HCL/Cubehelix), transform, string, zoom (d3-interpolate)
//! - **Contours**: Marching squares, density estimation (d3-contour)
//! - **Fetch**: CSV/TSV/JSON parsing utilities (d3-fetch)
//! - **Format**: Number formatting with SI prefixes, locales (d3-format)
//!
//! ## Example
//!
//! ```rust,no_run
//! use d3rs::scale::{LinearScale, Scale};
//!
//! let scale = LinearScale::new()
//!     .domain(0.0, 100.0)
//!     .range(0.0, 500.0);
//!
//! let output = scale.scale(50.0); // 250.0
//! ```

#![cfg_attr(feature = "gpui", recursion_limit = "512")]

pub mod array;
pub mod brush;
pub mod chord;
pub mod color;
pub mod ease;
pub mod force;
pub mod format;
pub mod hierarchy;
pub mod interpolate;
pub mod scale;
pub mod time;
pub mod zoom;

// Note: axis, grid, and text modules are excluded from test builds due to
// a known gpui_macros proc macro stack overflow issue in debug compilation.
// See: https://github.com/rust-lang/rust - the proc macro crashes with SIGBUS
// when parsing complex closures in the canvas! macro during debug builds.
// Release builds work fine. Tests can be run with --no-default-features.
#[cfg(all(feature = "gpui", not(test)))]
pub mod axis;
pub mod contour;
pub mod delaunay;
pub mod fetch;
pub mod geo;
#[cfg(all(feature = "gpu-2d", not(test)))]
pub mod gpu2d;
#[cfg(feature = "gpu-3d")]
pub mod gpu3d;
#[cfg(all(feature = "gpui", not(test)))]
pub mod grid;
pub mod legend;
pub mod polygon;
pub mod quadtree;
pub mod random;
pub mod shape;
#[cfg(all(feature = "gpui", not(test)))]
pub mod surface;
#[cfg(feature = "gpui")]
pub mod text;
pub mod timer;
pub mod transition;

/// Prelude module for convenient imports
pub mod prelude {
    #[cfg(all(feature = "gpui", not(test)))]
    pub use crate::axis::{AxisConfig, AxisOrientation, AxisTheme, DefaultAxisTheme, render_axis};
    pub use crate::color::{ColorScheme, D3Color};
    #[cfg(all(feature = "gpui", not(test)))]
    pub use crate::grid::{GridConfig, render_grid};
    pub use crate::scale::{LinearScale, LogScale, Scale};
    #[cfg(all(feature = "gpui", not(test)))]
    pub use crate::shape::{
        BarConfig, BarDatum, CurveType, GroupedBarConfig, GroupedBarDatum, GroupedBarMeta,
        LineConfig, LinePoint, ScatterConfig, ScatterPoint, analyze_grouped_data, render_bars,
        render_grouped_bars, render_line, render_scatter,
    };
    #[cfg(all(feature = "gpui", not(test)))]
    pub use crate::surface::{
        ColorScaleType, SurfaceConfig, SurfaceData, SurfaceElement, render_surface,
    };
}
