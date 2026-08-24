#![allow(
    clippy::too_many_arguments,
    reason = "chart geometry APIs mirror plotting formulas and canvas-style primitives"
)]

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

#![cfg_attr(feature = "gpui", recursion_limit = "1024")]

pub(crate) mod util;

pub mod array;
pub mod brush;
pub mod chord;
pub mod color;
pub mod dispatch;
pub mod drag;
pub mod ease;
pub mod examples;
pub mod feature_parity;
pub mod force;
pub mod format;
pub mod hierarchy;
pub mod interpolate;
pub mod lod;
pub mod scale;
pub mod time;
pub mod zoom;

// Note: text and the GPUI-backed axis/grid renderers are excluded from test
// builds due to a known gpui_macros proc macro stack overflow issue in debug
// compilation. Renderer-independent axis/grid layout remains available everywhere.
pub mod axis;
pub mod contour;
pub mod delaunay;
pub mod fetch;
pub mod geo;
#[cfg(all(feature = "gpu-2d", not(test)))]
pub mod gpu2d;
#[cfg(all(feature = "gpu-3d", not(test)))]
pub mod gpu3d;
pub mod grid;
pub mod hexbin;
pub mod legend;
pub mod mesh;
pub mod polygon;
pub mod quadtree;
pub mod random;
pub mod render2d;
pub mod sankey;
pub mod selection;
pub mod shape;
#[cfg(all(feature = "gpu-3d", not(test)))]
pub mod sphere_gallery;
#[cfg(any(test, feature = "gpui"))]
pub mod surface;
#[cfg(all(feature = "gpui", not(test)))]
pub mod text;
pub mod text_layout;
pub mod tile;
pub mod timer;
pub mod transition;
#[cfg(feature = "vello")]
pub mod vello2d;
