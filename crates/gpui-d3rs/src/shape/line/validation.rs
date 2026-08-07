use super::line_config::LineConfig;
use super::line_point::LinePoint;
#[cfg(any(test, all(feature = "gpui", not(test))))]
use super::misc::clip_line_segment;
#[cfg(any(test, all(feature = "gpui", not(test))))]
use super::style::CurveType;
use super::style::StrokeDashArray;
use crate::scale::Scale;
#[cfg(any(test, all(feature = "gpui", not(test))))]
use crate::shape::curve::Curve;
#[cfg(any(test, all(feature = "gpui", not(test))))]
use crate::shape::path::Point;
use std::fmt;

/// Recoverable errors for checked line rendering input validation.
#[derive(Debug, Clone, PartialEq)]
pub enum LineRenderError {
    /// Checked line data coordinates must be finite before scaling.
    NonFiniteDataCoordinate {
        index: usize,
        coordinate: &'static str,
        value: f64,
    },
    /// Checked line scales must expose finite output ranges.
    NonFiniteScaleRange {
        axis: &'static str,
        endpoint: &'static str,
        value: f64,
    },
    /// Checked line scales must return finite outputs for finite data.
    NonFiniteScaleOutput {
        index: usize,
        axis: &'static str,
        value: f64,
    },
    /// Checked line numeric configuration fields must be finite.
    NonFiniteConfigField { field: &'static str, value: f32 },
    /// Checked line size configuration fields cannot be negative.
    NegativeConfigField { field: &'static str, value: f32 },
    /// Checked line opacity must stay in the normalized alpha range.
    OpacityOutOfRange { value: f32 },
    /// Checked custom dash arrays must contain at least one dash/gap length.
    EmptyDashPattern,
    /// Checked custom dash lengths must be finite and positive.
    InvalidDashLength { index: usize, value: f32 },
}

impl fmt::Display for LineRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteDataCoordinate {
                index,
                coordinate,
                value,
            } => write!(
                f,
                "line data coordinate {coordinate} at index {index} is not finite: {value}"
            ),
            Self::NonFiniteScaleRange {
                axis,
                endpoint,
                value,
            } => write!(
                f,
                "line {axis}-scale range {endpoint} is not finite: {value}"
            ),
            Self::NonFiniteScaleOutput { index, axis, value } => write!(
                f,
                "line {axis}-scale output at index {index} is not finite: {value}"
            ),
            Self::NonFiniteConfigField { field, value } => {
                write!(f, "line config field {field} is not finite: {value}")
            }
            Self::NegativeConfigField { field, value } => {
                write!(f, "line config field {field} is negative: {value}")
            }
            Self::OpacityOutOfRange { value } => {
                write!(f, "line opacity is outside 0.0..=1.0: {value}")
            }
            Self::EmptyDashPattern => write!(f, "line custom dash pattern is empty"),
            Self::InvalidDashLength { index, value } => write!(
                f,
                "line custom dash length at index {index} must be finite and positive: {value}"
            ),
        }
    }
}

impl std::error::Error for LineRenderError {}

/// Compute clipped line segments from relative points based on the curve type.
#[cfg(any(test, all(feature = "gpui", not(test))))]
pub(crate) fn compute_line_segments(
    relative_points: &[(f32, f32)],
    curve_type: CurveType,
) -> Vec<(f32, f32, f32, f32)> {
    if relative_points.len() < 2 {
        return Vec::new();
    }

    match curve_type {
        CurveType::Linear => {
            let mut segments = Vec::with_capacity(relative_points.len() - 1);
            for i in 1..relative_points.len() {
                let (x0, y0) = relative_points[i - 1];
                let (x1, y1) = relative_points[i];
                if let Some(clipped) = clip_line_segment(x0, y0, x1, y1) {
                    segments.push(clipped);
                }
            }
            segments
        }
        CurveType::Step | CurveType::StepAfter => {
            let mut segments = Vec::with_capacity((relative_points.len() - 1) * 2);
            for i in 1..relative_points.len() {
                let (x0, y0) = relative_points[i - 1];
                let (x1, y1) = relative_points[i];
                if let Some(clipped) = clip_line_segment(x0, y0, x1, y0) {
                    segments.push(clipped);
                }
                if let Some(clipped) = clip_line_segment(x1, y0, x1, y1) {
                    segments.push(clipped);
                }
            }
            segments
        }
        CurveType::Basis => smooth_line_segments(relative_points, Curve::Basis),
        CurveType::Cardinal => {
            smooth_line_segments(relative_points, Curve::Cardinal { tension: 0.0 })
        }
        CurveType::CatmullRom => {
            smooth_line_segments(relative_points, Curve::CatmullRom { alpha: 0.5 })
        }
        CurveType::MonotoneX => smooth_line_segments(relative_points, Curve::MonotoneX),
        CurveType::Natural => smooth_line_segments(relative_points, Curve::Natural),
        CurveType::StepBefore => {
            let mut segments = Vec::with_capacity((relative_points.len() - 1) * 2);
            for i in 1..relative_points.len() {
                let (x0, y0) = relative_points[i - 1];
                let (x1, y1) = relative_points[i];
                if let Some(clipped) = clip_line_segment(x0, y0, x0, y1) {
                    segments.push(clipped);
                }
                if let Some(clipped) = clip_line_segment(x0, y1, x1, y1) {
                    segments.push(clipped);
                }
            }
            segments
        }
    }
}

#[cfg(any(test, all(feature = "gpui", not(test))))]
fn smooth_line_segments(relative_points: &[(f32, f32)], curve: Curve) -> Vec<(f32, f32, f32, f32)> {
    let points = relative_points
        .iter()
        .map(|&(x, y)| Point::new(f64::from(x), f64::from(y)))
        .collect::<Vec<_>>();
    let interpolated = curve.interpolate(&points);
    let mut segments = Vec::with_capacity(interpolated.len().saturating_sub(1));

    for pair in interpolated.windows(2) {
        let [start, end] = pair else { continue };
        if let Some(clipped) =
            clip_line_segment(start.x as f32, start.y as f32, end.x as f32, end.y as f32)
        {
            segments.push(clipped);
        }
    }

    segments
}

/// Validate line rendering inputs before constructing a GPUI line element.
pub fn validate_line_inputs<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[LinePoint],
    config: &LineConfig,
) -> Result<(), LineRenderError>
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    validate_config(config)?;

    let (x_min, x_max) = x_scale.range();
    validate_scale_range("x", "min", x_min)?;
    validate_scale_range("x", "max", x_max)?;

    let (y_min, y_max) = y_scale.range();
    validate_scale_range("y", "min", y_min)?;
    validate_scale_range("y", "max", y_max)?;

    for (index, point) in data.iter().enumerate() {
        if !point.x.is_finite() {
            return Err(LineRenderError::NonFiniteDataCoordinate {
                index,
                coordinate: "x",
                value: point.x,
            });
        }
        if !point.y.is_finite() {
            return Err(LineRenderError::NonFiniteDataCoordinate {
                index,
                coordinate: "y",
                value: point.y,
            });
        }

        let x_output = x_scale.scale(point.x);
        if !x_output.is_finite() {
            return Err(LineRenderError::NonFiniteScaleOutput {
                index,
                axis: "x",
                value: x_output,
            });
        }

        let y_output = y_scale.scale(point.y);
        if !y_output.is_finite() {
            return Err(LineRenderError::NonFiniteScaleOutput {
                index,
                axis: "y",
                value: y_output,
            });
        }
    }

    Ok(())
}

fn validate_config(config: &LineConfig) -> Result<(), LineRenderError> {
    validate_finite_f32("stroke_width", config.stroke_width)?;
    if config.stroke_width < 0.0 {
        return Err(LineRenderError::NegativeConfigField {
            field: "stroke_width",
            value: config.stroke_width,
        });
    }

    validate_finite_f32("point_radius", config.point_radius)?;
    if config.point_radius < 0.0 {
        return Err(LineRenderError::NegativeConfigField {
            field: "point_radius",
            value: config.point_radius,
        });
    }

    validate_finite_f32("opacity", config.opacity)?;
    if !(0.0..=1.0).contains(&config.opacity) {
        return Err(LineRenderError::OpacityOutOfRange {
            value: config.opacity,
        });
    }

    if let Some(StrokeDashArray::Custom(lengths)) = &config.dash_array {
        if lengths.is_empty() {
            return Err(LineRenderError::EmptyDashPattern);
        }
        for (index, &value) in lengths.iter().enumerate() {
            if !value.is_finite() || value <= 0.0 {
                return Err(LineRenderError::InvalidDashLength { index, value });
            }
        }
    }

    Ok(())
}

fn validate_finite_f32(field: &'static str, value: f32) -> Result<(), LineRenderError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(LineRenderError::NonFiniteConfigField { field, value })
    }
}

fn validate_scale_range(
    axis: &'static str,
    endpoint: &'static str,
    value: f64,
) -> Result<(), LineRenderError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(LineRenderError::NonFiniteScaleRange {
            axis,
            endpoint,
            value,
        })
    }
}
