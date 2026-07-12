#![recursion_limit = "512"]
#![allow(
    clippy::too_many_arguments,
    reason = "chart rendering APIs mirror plotting formulas and canvas-style primitives"
)]

//! # gpui-px - High-level charting API for GPUI
//!
//! Plotly Express-style API built on top of d3rs primitives.
//!
//! ## Chart Types
//!
//! ### Scatter Charts
//! Use `scatter()` for:
//! - Displaying individual data points with x,y coordinates
//! - Exploring correlations between two continuous variables
//! - Identifying outliers or clusters in data
//! - Showing distributions in 2D space
//!
//! ### Line Charts
//! Use `line()` for:
//! - Time series or sequential data
//! - Showing trends over continuous domains
//! - Connecting related data points with smooth or linear interpolation
//! - Comparing multiple series over the same range
//!
//! ### Bar Charts
//! Use `bar()` for:
//! - Categorical data with discrete categories
//! - Comparing values across different groups
//! - Displaying counts or aggregated metrics
//! - Visualizing rankings or distributions by category
//!
//! ### Heatmaps
//! Use `heatmap()` for:
//! - Visualizing 2D scalar fields with color
//! - Spectrograms, correlation matrices, geographic data
//! - Supports log scale axes and multiple color scales
//!
//! ### Contour Charts (Filled)
//! Use `contour()` for:
//! - Filled bands between threshold values
//! - Topographic-style visualizations
//! - Density estimation results
//!
//! ### Isoline Charts (Unfilled)
//! Use `isoline()` for:
//! - Unfilled contour lines at specific levels
//! - Elevation or pressure maps
//! - Level curves of scalar fields
//!
//! ## Coordinate System
//!
//! All charts use standard mathematical coordinates:
//! - **Y-axis**: 0 at bottom, increases upward
//! - **X-axis**: 0 at left, increases rightward
//!
//! ## Color Format
//!
//! For 1D charts (scatter, line, bar, isoline), color parameters accept
//! 24-bit RGB hex values in format `0xRRGGBB`:
//! - `0x1f77b4` - Plotly blue (default)
//! - `0xff7f0e` - Plotly orange
//! - `0x2ca02c` - Plotly green
//! - `0xd62728` - Plotly red
//!
//! For 2D charts (heatmap, contour), use [`ColorScale`]:
//! - `ColorScale::Viridis` - perceptually uniform (default)
//! - `ColorScale::Plasma` - perceptually uniform
//! - `ColorScale::Inferno` - perceptually uniform
//! - `ColorScale::Magma` - perceptually uniform
//! - `ColorScale::Heat` - diverging (blue → white → red)
//! - `ColorScale::Coolwarm` - diverging
//! - `ColorScale::Greys` - sequential grayscale
//! - `ColorScale::custom(|t| ...)` - custom function
//!
//! ## Logarithmic Scales
//!
//! All chart types support logarithmic axis scaling via the `ScaleType` enum:
//!
//! ### Scatter Charts
//! - Both X and Y axes can be logarithmic independently
//! - Use `.x_scale(ScaleType::Log)` and `.y_scale(ScaleType::Log)`
//! - Ideal for power-law relationships and data spanning multiple orders of magnitude
//!
//! ### Line Charts
//! - Both X and Y axes can be logarithmic independently
//! - Perfect for frequency response plots (audio engineering)
//! - Example: frequency axis from 20 Hz to 20 kHz
//!
//! ### Bar Charts
//! - Only Y-axis (values) can be logarithmic
//! - X-axis is categorical (always linear)
//! - Use `.y_scale(ScaleType::Log)` for values spanning magnitudes
//!
//! ### Heatmaps, Contours, and Isolines
//! - Both X and Y axes support logarithmic scaling
//! - Use `.x_scale(ScaleType::Log)` and `.y_scale(ScaleType::Log)`
//!
//! **Important**: Logarithmic scales require all values to be positive.
//! Zero or negative values will cause validation errors.
//!
//! ## Example
//!
//! ```rust,ignore
//! use gpui_px::{scatter, line, bar, heatmap, contour, isoline, treemap, TreemapNode, TilingMethod, ColorScale, ScaleType};
//!
//! // Scatter plot in 3 lines
//! let chart = scatter(&x_data, &y_data)
//!     .title("My Chart")
//!     .build()?;
//!
//! // Scatter plot with logarithmic scales
//! let chart = scatter(&x_data, &y_data)
//!     .x_scale(ScaleType::Log)
//!     .y_scale(ScaleType::Log)
//!     .build()?;
//!
//! // Line chart with custom color
//! let chart = line(&x_data, &y_data)
//!     .color(0x1f77b4)  // Plotly blue
//!     .build()?;
//!
//! // Frequency response plot with log frequency axis
//! let chart = line(&frequency, &magnitude_db)
//!     .x_scale(ScaleType::Log)
//!     .build()?;
//!
//! // Bar chart
//! let chart = bar(&categories, &values)
//!     .build()?;
//!
//! // Heatmap with log scale x-axis
//! let z = vec![1.0; 12]; // 3x4 grid
//! let chart = heatmap(&z, 3, 4)
//!     .x(&[20.0, 200.0, 2000.0])
//!     .x_scale(ScaleType::Log)
//!     .color_scale(ColorScale::Inferno)
//!     .build()?;
//!
//! // Contour plot with custom thresholds
//! let chart = contour(&z, 3, 4)
//!     .thresholds(vec![0.0, 0.5, 1.0, 1.5])
//!     .color_scale(ColorScale::Viridis)
//!     .build()?;
//!
//! // Isoline plot
//! let chart = isoline(&z, 3, 4)
//!     .levels(vec![0.5, 1.0, 1.5])
//!     .color(0x333333)
//!     .stroke_width(1.5)
//!     .build()?;
//!
//! // Treemap for hierarchical data
//! let root = TreemapNode::new("Sales", 0.0)
//!     .add_child(TreemapNode::new("East", 45.0))
//!     .add_child(TreemapNode::new("West", 55.0));
//! let chart = treemap(&root)
//!     .title("Regional Sales")
//!     .tiling_method(TilingMethod::Squarify)
//!     .build()?;
//! ```

#[cfg(feature = "gpui")]
pub use area::{AreaChart, area};
#[cfg(feature = "gpui")]
pub use bar::{BarChart, BarTheme, bar};
#[cfg(feature = "gpui")]
pub use boxplot::{BoxPlotChart, boxplot};
pub use color_scale::ColorScale;
#[cfg(feature = "gpui")]
pub use contour::{ContourChart, contour};
pub use d3rs::color::D3Color;
#[cfg(feature = "gpu-3d")]
pub use d3rs::gpu3d::{Colormap, Surface3DState};
#[cfg(feature = "gpui")]
pub use d3rs::shape::{CurveType, StrokeDashArray};
pub use error::ChartError;
#[cfg(feature = "gpui")]
pub use heatmap::{HeatmapChart, heatmap};
#[cfg(feature = "gpui")]
pub use isoline::{IsolineChart, isoline};
#[cfg(feature = "gpui")]
pub use line::{ChartTheme, LegendClickCallback, LegendPosition, LineChart, line};
#[cfg(feature = "gpui")]
pub use pie::{PieChart, donut, pie};
#[cfg(feature = "gpui")]
pub use scatter::{ScatterChart, ScatterTheme, scatter};
#[cfg(feature = "gpu-3d")]
pub use surface3d::{Surface3DChart, surface3d};
#[cfg(feature = "gpui")]
pub use treemap::{TilingMethod, Treemap, TreemapNode, treemap};

#[cfg(feature = "gpui")]
mod area;
#[cfg(feature = "gpui")]
mod bar;
#[cfg(feature = "gpui")]
mod boxplot;
mod color_scale;
#[cfg(feature = "gpui")]
mod contour;
mod error;
#[cfg(feature = "gpui")]
mod heatmap;
pub mod interaction;
#[cfg(feature = "gpui")]
mod isoline;
#[cfg(feature = "gpui")]
mod line;
#[cfg(feature = "gpui")]
mod pie;
#[cfg(feature = "gpui")]
mod scatter;
#[cfg(feature = "gpu-3d")]
mod surface3d;
#[cfg(feature = "gpui")]
mod treemap;

#[cfg(feature = "gpui")]
#[path = "lib/accessibility.rs"]
mod accessibility;
#[cfg(feature = "gpui")]
#[path = "lib/annotations.rs"]
mod annotations;
#[path = "lib/chart_capabilities.rs"]
mod chart_capabilities;
#[path = "lib/chart_size.rs"]
mod chart_size;
#[path = "lib/consts.rs"]
mod consts;
#[cfg(feature = "gpui")]
#[path = "lib/legend.rs"]
mod legend;
#[path = "lib/misc.rs"]
mod misc;
#[cfg(feature = "gpui")]
#[path = "lib/static_export.rs"]
mod static_export;
#[cfg(test)]
#[path = "lib/tests.rs"]
mod tests;
#[path = "lib/types.rs"]
mod types;
#[cfg(any(feature = "gpui", test))]
#[path = "lib/validate.rs"]
mod validate;
#[path = "lib/visual_regression.rs"]
mod visual_regression;

#[cfg(feature = "gpui")]
pub use accessibility::ChartAccessibilitySummary;
#[cfg(feature = "gpui")]
pub(crate) use accessibility::{
    finite_range, finite_range_owned, format_range, format_scale, indexed_label,
};
#[cfg(feature = "gpui")]
pub use annotations::{ChartAnnotation, ChartAnnotationSummary, ChartAnnotationTarget};
pub use chart_capabilities::{
    CHART_CAPABILITY_REPORT_TYPE, CHART_CAPABILITY_SCHEMA_VERSION, ChartCapabilityEntry,
    ChartCapabilityReport, ChartCapabilityStatus, PUBLIC_CHART_STORY_IDS, chart_capability_entries,
    chart_capability_report,
};
pub use chart_size::*;
#[cfg(feature = "gpui")]
pub(crate) use consts::*;
#[cfg(feature = "gpui")]
pub use legend::{ChartLegendItem, ChartLegendMarker, ChartLegendSummary};
#[cfg(feature = "gpui")]
pub(crate) use misc::*;
#[cfg(feature = "gpui")]
pub use static_export::StaticSvgOptions;
pub use types::*;
#[cfg(feature = "gpui")]
pub(crate) use validate::*;
pub use visual_regression::{
    CHART_VISUAL_COLOR_SCHEMES, CHART_VISUAL_REGRESSION_REPORT_TYPE,
    CHART_VISUAL_REGRESSION_SCHEMA_VERSION, CHART_VISUAL_STORIES, CHART_VISUAL_VIEWPORTS,
    ChartVisualCapture, ChartVisualColorScheme, ChartVisualRegressionManifest, ChartVisualStory,
    ChartVisualViewport, chart_visual_regression_manifest, chart_visual_stories,
};
