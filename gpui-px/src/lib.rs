#![recursion_limit = "512"]

//! # gpui-px - High-level charting API for GPUI
//!
//! Plotly Express-style API built on top of d3rs primitives.
//!
//! ## Chart Types
//!
//! ### Scatter Charts
//! Use [`scatter()`] for:
//! - Displaying individual data points with x,y coordinates
//! - Exploring correlations between two continuous variables
//! - Identifying outliers or clusters in data
//! - Showing distributions in 2D space
//!
//! ### Line Charts
//! Use [`line()`] for:
//! - Time series or sequential data
//! - Showing trends over continuous domains
//! - Connecting related data points with smooth or linear interpolation
//! - Comparing multiple series over the same range
//!
//! ### Bar Charts
//! Use [`bar()`] for:
//! - Categorical data with discrete categories
//! - Comparing values across different groups
//! - Displaying counts or aggregated metrics
//! - Visualizing rankings or distributions by category
//!
//! ### Heatmaps
//! Use [`heatmap()`] for:
//! - Visualizing 2D scalar fields with color
//! - Spectrograms, correlation matrices, geographic data
//! - Supports log scale axes and multiple color scales
//!
//! ### Contour Charts (Filled)
//! Use [`contour()`] for:
//! - Filled bands between threshold values
//! - Topographic-style visualizations
//! - Density estimation results
//!
//! ### Isoline Charts (Unfilled)
//! Use [`isoline()`] for:
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

mod area;
mod bar;
mod boxplot;
mod color_scale;
mod contour;
mod error;
mod heatmap;
pub mod interaction;
mod isoline;
mod line;
mod pie;
mod scatter;
#[cfg(feature = "gpu-3d")]
mod surface3d;
mod treemap;

pub use area::{AreaChart, area};
pub use bar::{BarChart, BarTheme, bar};
pub use boxplot::{BoxPlotChart, boxplot};
pub use color_scale::ColorScale;
pub use contour::{ContourChart, contour};
pub use error::ChartError;
pub use heatmap::{HeatmapChart, heatmap};
pub use isoline::{IsolineChart, isoline};
pub use line::{ChartTheme, LegendClickCallback, LegendPosition, LineChart, line};
pub use pie::{PieChart, donut, pie};
pub use scatter::{ScatterChart, ScatterTheme, scatter};
#[cfg(feature = "gpu-3d")]
pub use surface3d::{Surface3DChart, surface3d};
pub use treemap::{TilingMethod, Treemap, TreemapNode, treemap};

// Re-export d3rs types users might need
pub use d3rs::color::D3Color;
#[cfg(feature = "gpu-3d")]
pub use d3rs::gpu3d::{Colormap, Surface3DState};
pub use d3rs::shape::CurveType;

// ============================================================================
// Scale Types
// ============================================================================

/// Scale type for axis transformations.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ScaleType {
    /// Linear scale (default).
    #[default]
    Linear,
    /// Logarithmic scale (base 10).
    Log,
}

// ============================================================================
// Shared Constants
// ============================================================================

/// Default chart color (Plotly blue)
pub(crate) const DEFAULT_COLOR: u32 = 0x1f77b4;

/// Default chart width in pixels
pub(crate) const DEFAULT_WIDTH: f32 = 600.0;

/// Default chart height in pixels
pub(crate) const DEFAULT_HEIGHT: f32 = 400.0;

/// Default padding fraction for auto-domain calculation
pub(crate) const DEFAULT_PADDING_FRACTION: f64 = 0.05;

/// Default title font size
pub(crate) const DEFAULT_TITLE_FONT_SIZE: f32 = 16.0;

/// Title area height (font size + padding)
pub(crate) const TITLE_AREA_HEIGHT: f32 = 24.0;

// ============================================================================
// Shared Utilities
// ============================================================================

/// Calculate extent (min, max) with padding.
///
/// Returns `(min - padding, max + padding)` where padding is calculated
/// as `range * padding_fraction`.
///
/// ## Special Case: Constant Values
///
/// When all values are identical (range ≈ 0), uses a **hardcoded padding of 1.0**
/// to ensure a meaningful range for visualization. This prevents collapsed
/// axes and ensures the constant value is visible in the chart.
///
/// For example, `[5.0, 5.0, 5.0]` returns `(4.0, 6.0)` instead of `(5.0, 5.0)`.
pub(crate) fn extent_padded(values: &[f64], padding_fraction: f64) -> (f64, f64) {
    let (min, max) = values
        .iter()
        .copied()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), val| {
            (min.min(val), max.max(val))
        });

    let range = max - min;
    let padding = if range.abs() < f64::EPSILON {
        1.0 // Default padding for constant values
    } else {
        range * padding_fraction
    };
    (min - padding, max + padding)
}

/// Validate that a data array is not empty and contains only finite values.
pub(crate) fn validate_data_array(values: &[f64], field: &'static str) -> Result<(), ChartError> {
    if values.is_empty() {
        return Err(ChartError::EmptyData { field });
    }
    if values.iter().any(|x| !x.is_finite()) {
        return Err(ChartError::InvalidData {
            field,
            reason: "contains NaN or Infinity",
        });
    }
    Ok(())
}

/// Validate that two arrays have the same length.
pub(crate) fn validate_data_length(
    x_len: usize,
    y_len: usize,
    x_field: &'static str,
    y_field: &'static str,
) -> Result<(), ChartError> {
    if x_len != y_len {
        return Err(ChartError::DataLengthMismatch {
            x_field,
            y_field,
            x_len,
            y_len,
        });
    }
    Ok(())
}

/// Validate chart dimensions are positive.
pub(crate) fn validate_dimensions(width: f32, height: f32) -> Result<(), ChartError> {
    if width <= 0.0 {
        return Err(ChartError::InvalidDimension {
            field: "width",
            value: width,
        });
    }
    if height <= 0.0 {
        return Err(ChartError::InvalidDimension {
            field: "height",
            value: height,
        });
    }
    Ok(())
}

/// Validate that grid dimensions match the z array length.
pub(crate) fn validate_grid_dimensions(
    z: &[f64],
    grid_width: usize,
    grid_height: usize,
) -> Result<(), ChartError> {
    let expected = grid_width * grid_height;
    if z.len() != expected {
        return Err(ChartError::GridDimensionMismatch {
            z_len: z.len(),
            width: grid_width,
            height: grid_height,
            expected,
        });
    }
    Ok(())
}

/// Validate that axis values are strictly monotonic (increasing).
pub(crate) fn validate_monotonic(values: &[f64], field: &'static str) -> Result<(), ChartError> {
    for window in values.windows(2) {
        if window[1] <= window[0] {
            return Err(ChartError::InvalidData {
                field,
                reason: "must be strictly monotonically increasing",
            });
        }
    }
    Ok(())
}

/// Validate that all values are positive (for log scale).
pub(crate) fn validate_positive(values: &[f64], field: &'static str) -> Result<(), ChartError> {
    if values.iter().any(|&v| v <= 0.0) {
        return Err(ChartError::InvalidData {
            field,
            reason: "contains non-positive values for log scale",
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // extent_padded tests
    #[test]
    fn test_extent_padded_normal_values() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let (min, max) = extent_padded(&values, 0.05);
        // Min should be 1.0 - 0.05 * 4.0 = 0.8
        // Max should be 5.0 + 0.05 * 4.0 = 5.2
        assert!((min - 0.8).abs() < 1e-10);
        assert!((max - 5.2).abs() < 1e-10);
    }

    #[test]
    fn test_extent_padded_constant_values() {
        let values = vec![5.0, 5.0, 5.0, 5.0];
        let (min, max) = extent_padded(&values, 0.05);
        // Range is 0, so padding should be 1.0
        assert!((min - 4.0).abs() < 1e-10);
        assert!((max - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_extent_padded_single_value() {
        let values = vec![3.0];
        let (min, max) = extent_padded(&values, 0.1);
        // Range is 0, so padding should be 1.0
        assert!((min - 2.0).abs() < 1e-10);
        assert!((max - 4.0).abs() < 1e-10);
    }

    // validate_data_array tests
    #[test]
    fn test_validate_data_array_valid() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(validate_data_array(&values, "test").is_ok());
    }

    #[test]
    fn test_validate_data_array_empty() {
        let values: Vec<f64> = vec![];
        let result = validate_data_array(&values, "test");
        assert!(matches!(
            result,
            Err(ChartError::EmptyData { field: "test" })
        ));
    }

    #[test]
    fn test_validate_data_array_nan() {
        let values = vec![1.0, 2.0, f64::NAN, 4.0];
        let result = validate_data_array(&values, "test");
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "test",
                reason: "contains NaN or Infinity"
            })
        ));
    }

    #[test]
    fn test_validate_data_array_infinity() {
        let values = vec![1.0, f64::INFINITY, 3.0];
        let result = validate_data_array(&values, "test");
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "test",
                reason: "contains NaN or Infinity"
            })
        ));
    }

    #[test]
    fn test_validate_data_array_neg_infinity() {
        let values = vec![1.0, 2.0, f64::NEG_INFINITY];
        let result = validate_data_array(&values, "test");
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "test",
                reason: "contains NaN or Infinity"
            })
        ));
    }

    // validate_data_length tests
    #[test]
    fn test_validate_data_length_matching() {
        assert!(validate_data_length(5, 5, "x", "y").is_ok());
    }

    #[test]
    fn test_validate_data_length_mismatched() {
        let result = validate_data_length(3, 5, "x", "y");
        assert!(matches!(
            result,
            Err(ChartError::DataLengthMismatch {
                x_field: "x",
                y_field: "y",
                x_len: 3,
                y_len: 5,
            })
        ));
    }

    #[test]
    fn test_validate_data_length_zero() {
        assert!(validate_data_length(0, 0, "x", "y").is_ok());
    }

    // validate_dimensions tests
    #[test]
    fn test_validate_dimensions_valid() {
        assert!(validate_dimensions(600.0, 400.0).is_ok());
    }

    #[test]
    fn test_validate_dimensions_zero_width() {
        let result = validate_dimensions(0.0, 400.0);
        assert!(matches!(
            result,
            Err(ChartError::InvalidDimension {
                field: "width",
                value: 0.0
            })
        ));
    }

    #[test]
    fn test_validate_dimensions_negative_width() {
        let result = validate_dimensions(-100.0, 400.0);
        assert!(matches!(
            result,
            Err(ChartError::InvalidDimension {
                field: "width",
                value: -100.0
            })
        ));
    }

    #[test]
    fn test_validate_dimensions_zero_height() {
        let result = validate_dimensions(600.0, 0.0);
        assert!(matches!(
            result,
            Err(ChartError::InvalidDimension {
                field: "height",
                value: 0.0
            })
        ));
    }

    #[test]
    fn test_validate_dimensions_negative_height() {
        let result = validate_dimensions(600.0, -50.0);
        assert!(matches!(
            result,
            Err(ChartError::InvalidDimension {
                field: "height",
                value: -50.0
            })
        ));
    }

    // validate_grid_dimensions tests
    #[test]
    fn test_validate_grid_dimensions_valid() {
        let z = vec![1.0; 12]; // 3x4 grid
        assert!(validate_grid_dimensions(&z, 3, 4).is_ok());
    }

    #[test]
    fn test_validate_grid_dimensions_mismatch() {
        let z = vec![1.0; 10];
        let result = validate_grid_dimensions(&z, 3, 4);
        assert!(matches!(
            result,
            Err(ChartError::GridDimensionMismatch {
                z_len: 10,
                width: 3,
                height: 4,
                expected: 12,
            })
        ));
    }

    // validate_monotonic tests
    #[test]
    fn test_validate_monotonic_valid() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(validate_monotonic(&values, "x").is_ok());
    }

    #[test]
    fn test_validate_monotonic_not_increasing() {
        let values = vec![1.0, 2.0, 2.0, 4.0]; // 2.0 == 2.0
        let result = validate_monotonic(&values, "x");
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "must be strictly monotonically increasing"
            })
        ));
    }

    #[test]
    fn test_validate_monotonic_decreasing() {
        let values = vec![1.0, 3.0, 2.0, 4.0];
        let result = validate_monotonic(&values, "x");
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "must be strictly monotonically increasing"
            })
        ));
    }

    // validate_positive tests
    #[test]
    fn test_validate_positive_valid() {
        let values = vec![0.1, 1.0, 10.0, 100.0];
        assert!(validate_positive(&values, "x").is_ok());
    }

    #[test]
    fn test_validate_positive_with_zero() {
        let values = vec![0.0, 1.0, 2.0];
        let result = validate_positive(&values, "x");
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "contains non-positive values for log scale"
            })
        ));
    }

    #[test]
    fn test_validate_positive_with_negative() {
        let values = vec![-1.0, 1.0, 2.0];
        let result = validate_positive(&values, "x");
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "contains non-positive values for log scale"
            })
        ));
    }
}
