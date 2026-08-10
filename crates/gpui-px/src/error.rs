//! Error types for gpui-px charts.

use thiserror::Error;

/// Errors that can occur when building or rendering charts.
#[derive(Debug, Error)]
pub enum ChartError {
    #[error("mesh validation failed: {0}")]
    MeshValidation(#[from] d3rs::mesh::MeshValidationError),
    /// X and Y data arrays have different lengths.
    #[error("{x_field} has {x_len} elements but {y_field} has {y_len} elements")]
    DataLengthMismatch {
        x_field: &'static str,
        y_field: &'static str,
        x_len: usize,
        y_len: usize,
    },

    /// Data array is empty.
    #[error("empty data: {field} array is empty")]
    EmptyData { field: &'static str },

    /// Data contains invalid values (NaN or Infinity).
    #[error("invalid data in {field}: {reason}")]
    InvalidData {
        field: &'static str,
        reason: &'static str,
    },

    /// Invalid dimension specified.
    #[error("invalid dimension: {field} must be positive, got {value}")]
    InvalidDimension { field: &'static str, value: f32 },

    /// Grid dimension mismatch for 2D data.
    #[error(
        "grid dimension mismatch: z has {z_len} values but expected {width} x {height} = {expected}"
    )]
    GridDimensionMismatch {
        z_len: usize,
        width: usize,
        height: usize,
        expected: usize,
    },

    /// The requested color range is not finite or has no positive extent.
    #[error("invalid color range: {reason}")]
    InvalidColorRange { reason: &'static str },

    /// The requested view needs an optional rendering backend that is not
    /// enabled for this build.
    #[error("unsupported mesh plot view {view}: {reason}")]
    UnsupportedView {
        view: &'static str,
        reason: &'static str,
    },
}
