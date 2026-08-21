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

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::axis::{
        AxisConfig, AxisLayout, AxisLayoutError, AxisLine, AxisOrientation, AxisPoint, AxisTick,
        AxisTitle, axis_layout,
    };
    #[cfg(all(feature = "gpui", not(test)))]
    pub use crate::axis::{AxisTheme, DefaultAxisTheme, render_axis};
    pub use crate::color::{ColorScheme, D3Color};
    pub use crate::drag::{
        DragConfig, DragDelta, DragError, DragExtent, DragPhase, DragPoint, DragState, DragUpdate,
    };
    pub use crate::feature_parity::{
        D3_BENCHMARK_COVERAGE_REPORT_TYPE, D3_BENCHMARK_COVERAGE_SCHEMA_VERSION,
        D3BenchmarkCoverageCase, D3BenchmarkCoverageReport, D3BenchmarkCoverageStatus,
        FEATURE_PARITY_REPORT_TYPE, FEATURE_PARITY_SCHEMA_VERSION, FeatureParityEntry,
        FeatureParityReport, FeatureParityStatus, d3_benchmark_coverage_cases,
        d3_benchmark_coverage_report, feature_parity_entries, feature_parity_report,
    };
    #[cfg(all(feature = "gpui", not(test)))]
    pub use crate::grid::render_grid;
    pub use crate::grid::{
        GridConfig, GridDot, GridLayout, GridLayoutError, GridLine, GridPoint, grid_layout,
    };
    pub use crate::legend::{
        LegendConfig, LegendItem, LegendItemLayout, LegendLayout, LegendLayoutError,
        LegendOrientation, LegendPoint, LegendPosition, LegendRect, LegendSymbol,
        LegendTitleLayout, legend_layout,
    };
    pub use crate::lod::{DensityGrid, DensityPyramid, LodBounds, LodError, m4_indices};
    pub use crate::scale::{LinearScale, LogScale, Scale};
    pub use crate::selection::{
        SelectionEnter, SelectionExit, SelectionJoin, SelectionJoinError, SelectionUpdate,
        index_data_join, keyed_data_join,
    };
    #[cfg(all(feature = "gpui", not(test)))]
    pub use crate::shape::{
        BarConfig, BarDatum, CurveType, GroupedBarConfig, GroupedBarDatum, GroupedBarMeta,
        LineConfig, LinePoint, ScatterConfig, ScatterPoint, StrokeDashArray, analyze_grouped_data,
        render_bars, render_grouped_bars, render_line, render_scatter,
    };
    #[cfg(all(feature = "gpui", not(test)))]
    pub use crate::surface::{
        ColorScaleType, SurfaceConfig, SurfaceData, SurfaceElement, render_surface,
    };
    pub use crate::text_layout::{
        TextAnchorX, TextAnchorY, TextBounds, TextLayout, TextLayoutConfig, TextLayoutError,
        TextLineLayout, TextPoint, measure_text_width, text_layout,
    };
    pub use crate::tile::{Tile, TileError, TileLayout, TileSet, tiles_for_viewport};
}
