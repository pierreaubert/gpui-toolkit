//! Scatter plot rendering

use crate::color::D3Color;
use crate::scale::Scale;
#[cfg(all(feature = "gpui", not(test)))]
use gpui::prelude::*;
#[cfg(all(feature = "gpui", not(test)))]
use gpui::*;
use std::fmt;

/// Configuration for scatter plot rendering
#[derive(Clone)]
pub struct ScatterConfig {
    /// Fill color for points
    pub fill_color: D3Color,
    /// Point radius in pixels
    pub point_radius: f32,
    /// Opacity of points (0.0 - 1.0)
    pub opacity: f32,
    /// Optional stroke color
    pub stroke_color: Option<D3Color>,
    /// Stroke width in pixels
    pub stroke_width: f32,
}

impl Default for ScatterConfig {
    fn default() -> Self {
        Self {
            fill_color: D3Color::from_hex(0xff6347), // Tomato
            point_radius: 4.0,
            opacity: 0.7,
            stroke_color: Some(D3Color::from_hex(0xffffff)),
            stroke_width: 1.0,
        }
    }
}

impl ScatterConfig {
    /// Create a new scatter configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a scatter configuration from design-system spacing and interaction defaults.
    #[cfg(feature = "gpui")]
    pub fn from_design(design: &gpui_design::DesignSystem) -> Self {
        Self {
            point_radius: (design.spacing.grid_unit * 0.75).max(3.0),
            stroke_width: design.interaction.border_width.max(1.0),
            ..Self::default()
        }
    }

    /// Apply design-system spacing and interaction defaults.
    #[cfg(feature = "gpui")]
    pub fn with_design(mut self, design: &gpui_design::DesignSystem) -> Self {
        self.point_radius = (design.spacing.grid_unit * 0.75).max(3.0);
        self.stroke_width = design.interaction.border_width.max(1.0);
        self
    }

    /// Set the fill color
    pub fn fill_color(mut self, color: D3Color) -> Self {
        self.fill_color = color;
        self
    }

    /// Set the point radius
    pub fn point_radius(mut self, radius: f32) -> Self {
        self.point_radius = radius;
        self
    }

    /// Set the opacity
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the stroke color
    pub fn stroke_color(mut self, color: D3Color) -> Self {
        self.stroke_color = Some(color);
        self
    }

    /// Remove stroke
    pub fn no_stroke(mut self) -> Self {
        self.stroke_color = None;
        self
    }

    /// Set the stroke width
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }
}

/// Data point for a scatter plot
#[derive(Debug, Clone, Copy)]
pub struct ScatterPoint {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
}

impl ScatterPoint {
    /// Create a new scatter point
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Recoverable errors for checked scatter rendering input validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ScatterRenderError {
    /// Checked scatter data coordinates must be finite before scaling.
    NonFiniteDataCoordinate {
        index: usize,
        coordinate: &'static str,
        value: f64,
    },
    /// Checked scatter scales must expose finite output ranges.
    NonFiniteScaleRange {
        axis: &'static str,
        endpoint: &'static str,
        value: f64,
    },
    /// Checked scatter scales must return finite outputs for finite data.
    NonFiniteScaleOutput {
        index: usize,
        axis: &'static str,
        value: f64,
    },
    /// Checked scatter numeric configuration fields must be finite.
    NonFiniteConfigField { field: &'static str, value: f32 },
    /// Checked scatter size configuration fields cannot be negative.
    NegativeConfigField { field: &'static str, value: f32 },
    /// Checked scatter opacity must stay in the normalized alpha range.
    OpacityOutOfRange { value: f32 },
}

impl fmt::Display for ScatterRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteDataCoordinate {
                index,
                coordinate,
                value,
            } => write!(
                f,
                "scatter data coordinate {coordinate} at index {index} is not finite: {value}"
            ),
            Self::NonFiniteScaleRange {
                axis,
                endpoint,
                value,
            } => write!(
                f,
                "scatter {axis}-scale range {endpoint} is not finite: {value}"
            ),
            Self::NonFiniteScaleOutput { index, axis, value } => write!(
                f,
                "scatter {axis}-scale output at index {index} is not finite: {value}"
            ),
            Self::NonFiniteConfigField { field, value } => {
                write!(f, "scatter config field {field} is not finite: {value}")
            }
            Self::NegativeConfigField { field, value } => {
                write!(f, "scatter config field {field} is negative: {value}")
            }
            Self::OpacityOutOfRange { value } => {
                write!(f, "scatter opacity is outside 0.0..=1.0: {value}")
            }
        }
    }
}

impl std::error::Error for ScatterRenderError {}

/// Pre-computed screen-space point for a scatter plot.
#[cfg(any(test, all(feature = "gpui", not(test))))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ScatterDrawPoint {
    pub x_rel: f32,
    pub y_rel: f32,
}

/// Pre-compute normalized (0-1) point positions in a single pass.
#[cfg(any(test, all(feature = "gpui", not(test))))]
pub(super) fn compute_scatter_points<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[ScatterPoint],
) -> Vec<ScatterDrawPoint>
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    let (x_min, x_max) = x_scale.range();
    let (y_min, y_max) = y_scale.range();
    let x_range_span = x_max - x_min;
    let y_range_span = y_max - y_min;

    data.iter()
        .map(|point| {
            let x_range = x_scale.scale(point.x);
            let x_pos = if x_range_span == 0.0 {
                0.5
            } else {
                ((x_range - x_min) / x_range_span) as f32
            };

            let y_range = y_scale.scale(point.y);
            let y_pos = if y_range_span == 0.0 {
                0.5
            } else {
                1.0 - ((y_range - y_min) / y_range_span) as f32
            };

            ScatterDrawPoint {
                x_rel: x_pos,
                y_rel: y_pos,
            }
        })
        .collect()
}

/// Validate scatter rendering inputs before constructing a GPUI scatter element.
pub fn validate_scatter_inputs<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[ScatterPoint],
    config: &ScatterConfig,
) -> Result<(), ScatterRenderError>
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    validate_scatter_config(config)?;

    let (x_min, x_max) = x_scale.range();
    validate_scale_range("x", "min", x_min)?;
    validate_scale_range("x", "max", x_max)?;

    let (y_min, y_max) = y_scale.range();
    validate_scale_range("y", "min", y_min)?;
    validate_scale_range("y", "max", y_max)?;

    for (index, point) in data.iter().enumerate() {
        if !point.x.is_finite() {
            return Err(ScatterRenderError::NonFiniteDataCoordinate {
                index,
                coordinate: "x",
                value: point.x,
            });
        }
        if !point.y.is_finite() {
            return Err(ScatterRenderError::NonFiniteDataCoordinate {
                index,
                coordinate: "y",
                value: point.y,
            });
        }

        let x_output = x_scale.scale(point.x);
        if !x_output.is_finite() {
            return Err(ScatterRenderError::NonFiniteScaleOutput {
                index,
                axis: "x",
                value: x_output,
            });
        }

        let y_output = y_scale.scale(point.y);
        if !y_output.is_finite() {
            return Err(ScatterRenderError::NonFiniteScaleOutput {
                index,
                axis: "y",
                value: y_output,
            });
        }
    }

    Ok(())
}

fn validate_scatter_config(config: &ScatterConfig) -> Result<(), ScatterRenderError> {
    validate_finite_f32("point_radius", config.point_radius)?;
    if config.point_radius < 0.0 {
        return Err(ScatterRenderError::NegativeConfigField {
            field: "point_radius",
            value: config.point_radius,
        });
    }

    validate_finite_f32("stroke_width", config.stroke_width)?;
    if config.stroke_width < 0.0 {
        return Err(ScatterRenderError::NegativeConfigField {
            field: "stroke_width",
            value: config.stroke_width,
        });
    }

    validate_finite_f32("opacity", config.opacity)?;
    if !(0.0..=1.0).contains(&config.opacity) {
        return Err(ScatterRenderError::OpacityOutOfRange {
            value: config.opacity,
        });
    }

    Ok(())
}

fn validate_finite_f32(field: &'static str, value: f32) -> Result<(), ScatterRenderError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ScatterRenderError::NonFiniteConfigField { field, value })
    }
}

fn validate_scale_range(
    axis: &'static str,
    endpoint: &'static str,
    value: f64,
) -> Result<(), ScatterRenderError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ScatterRenderError::NonFiniteScaleRange {
            axis,
            endpoint,
            value,
        })
    }
}

/// Render a scatter plot after validating data, scale outputs, and config.
#[cfg(all(feature = "gpui", not(test)))]
pub fn try_render_scatter<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[ScatterPoint],
    config: &ScatterConfig,
) -> Result<impl IntoElement + use<XS, YS>, ScatterRenderError>
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    validate_scatter_inputs(x_scale, y_scale, data, config)?;
    Ok(render_scatter(x_scale, y_scale, data, config))
}

/// Render a scatter plot
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::prelude::*;
/// use d3rs::shape::{render_scatter, ScatterConfig, ScatterPoint};
///
/// let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
/// let y_scale = LinearScale::new().domain(0.0, 100.0).range(300.0, 0.0);
///
/// let data = vec![
///     ScatterPoint::new(10.0, 20.0),
///     ScatterPoint::new(50.0, 80.0),
///     ScatterPoint::new(90.0, 40.0),
/// ];
///
/// let config = ScatterConfig::new()
///     .fill_color(D3Color::from_hex(0xff6347))
///     .point_radius(5.0);
/// // render_scatter(&x_scale, &y_scale, &data, &config)
/// ```
#[cfg(all(feature = "gpui", not(test)))]
pub fn render_scatter<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[ScatterPoint],
    config: &ScatterConfig,
) -> impl IntoElement + use<XS, YS>
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    let points = compute_scatter_points(x_scale, y_scale, data);
    let fill = config.fill_color.to_rgba();
    let opacity = config.opacity;
    let radius = config.point_radius;
    let stroke = config.stroke_color;
    let stroke_width = config.stroke_width;

    canvas(
        move |_bounds, _window, _cx| {
            // Prepaint is a no-op: positions are already computed.
            (points, fill, stroke)
        },
        move |bounds, (points, fill, stroke), window, _cx| {
            let width: f32 = bounds.size.width.into();
            let height: f32 = bounds.size.height.into();
            let origin_x: f32 = bounds.origin.x.into();
            let origin_y: f32 = bounds.origin.y.into();

            // Batch all points into a single fill path. If a stroke is configured,
            // build one stroke path that encircles every point.
            let mut fill_builder = PathBuilder::fill();
            let mut fill_count = 0usize;

            if let Some(stroke_color) = stroke {
                let mut stroke_builder = PathBuilder::stroke(px(stroke_width));
                // A stroked circle of radius `r + stroke_width/2` with stroke width
                // `stroke_width` produces a ring that matches the previous filled
                // stroke ring (outer radius `r + stroke_width`, inner radius `r`).
                let stroke_radius = radius + stroke_width / 2.0;
                for draw_point in &points {
                    let cx = origin_x + draw_point.x_rel * width;
                    let cy = origin_y + draw_point.y_rel * height;
                    add_circle_to_path(&mut stroke_builder, cx, cy, stroke_radius);
                    add_circle_to_path(&mut fill_builder, cx, cy, radius);
                    fill_count += 1;
                }
                if let Ok(path) = stroke_builder.build() {
                    window.paint_path(path, stroke_color.to_rgba());
                }
            } else {
                for draw_point in &points {
                    let cx = origin_x + draw_point.x_rel * width;
                    let cy = origin_y + draw_point.y_rel * height;
                    add_circle_to_path(&mut fill_builder, cx, cy, radius);
                    fill_count += 1;
                }
            }

            if fill_count > 0
                && let Ok(path) = fill_builder.build()
            {
                let mut fill_color = fill;
                fill_color.a *= opacity;
                window.paint_path(path, fill_color);
            }
        },
    )
    .size_full()
    .absolute()
    .inset_0()
}

/// Append a circle outline to a GPUI path builder.
#[cfg(all(feature = "gpui", not(test)))]
fn add_circle_to_path(builder: &mut PathBuilder, cx: f32, cy: f32, r: f32) {
    if r <= 0.0 {
        return;
    }

    // Start at the rightmost point and draw four quadratic arcs.
    builder.move_to(point(px(cx + r), px(cy)));
    // Top-right quadrant
    builder.curve_to(point(px(cx + r), px(cy - r)), point(px(cx), px(cy - r)));
    // Top-left quadrant
    builder.curve_to(point(px(cx - r), px(cy - r)), point(px(cx - r), px(cy)));
    // Bottom-left quadrant
    builder.curve_to(point(px(cx - r), px(cy + r)), point(px(cx), px(cy + r)));
    // Bottom-right quadrant
    builder.curve_to(point(px(cx + r), px(cy + r)), point(px(cx + r), px(cy)));
    builder.close();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::LinearScale;

    #[test]
    fn render_scatter_uses_single_precomputed_pass() {
        let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
        let y_scale = LinearScale::new().domain(0.0, 100.0).range(300.0, 0.0);
        let data = vec![
            ScatterPoint::new(10.0, 20.0),
            ScatterPoint::new(50.0, 80.0),
            ScatterPoint::new(90.0, 40.0),
        ];

        let points = compute_scatter_points(&x_scale, &y_scale, &data);

        assert_eq!(
            points.len(),
            data.len(),
            "all points should be precomputed in one pass"
        );
        assert_eq!(points[0].x_rel, 0.1);
        assert_eq!(points[2].x_rel, 0.9);
    }

    #[test]
    fn validate_scatter_inputs_accepts_valid_points() {
        let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
        let y_scale = LinearScale::new().domain(0.0, 100.0).range(300.0, 0.0);
        let data = vec![ScatterPoint::new(10.0, 20.0), ScatterPoint::new(90.0, 40.0)];
        let config = ScatterConfig::new().point_radius(5.0);

        validate_scatter_inputs(&x_scale, &y_scale, &data, &config).unwrap();
    }

    #[test]
    fn validate_scatter_inputs_rejects_non_finite_data_coordinates() {
        let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
        let y_scale = LinearScale::new().domain(0.0, 100.0).range(300.0, 0.0);
        let data = vec![
            ScatterPoint::new(10.0, 20.0),
            ScatterPoint::new(90.0, f64::NAN),
        ];
        let config = ScatterConfig::new();

        let error = validate_scatter_inputs(&x_scale, &y_scale, &data, &config).unwrap_err();
        match error {
            ScatterRenderError::NonFiniteDataCoordinate {
                index,
                coordinate,
                value,
            } => {
                assert_eq!(index, 1);
                assert_eq!(coordinate, "y");
                assert!(value.is_nan());
            }
            error => panic!("unexpected error: {error:?}"),
        }
    }

    #[test]
    fn validate_scatter_inputs_rejects_non_finite_scale_range() {
        let x_scale = LinearScale::new()
            .domain(0.0, 100.0)
            .range(f64::NEG_INFINITY, 400.0);
        let y_scale = LinearScale::new().domain(0.0, 100.0).range(300.0, 0.0);
        let data = vec![ScatterPoint::new(10.0, 20.0), ScatterPoint::new(90.0, 40.0)];
        let config = ScatterConfig::new();

        assert_eq!(
            validate_scatter_inputs(&x_scale, &y_scale, &data, &config).unwrap_err(),
            ScatterRenderError::NonFiniteScaleRange {
                axis: "x",
                endpoint: "min",
                value: f64::NEG_INFINITY,
            }
        );
    }

    #[test]
    fn validate_scatter_inputs_rejects_invalid_config() {
        let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
        let y_scale = LinearScale::new().domain(0.0, 100.0).range(300.0, 0.0);
        let data = vec![ScatterPoint::new(10.0, 20.0), ScatterPoint::new(90.0, 40.0)];

        let mut config = ScatterConfig::new();
        config.point_radius = -1.0;
        assert_eq!(
            validate_scatter_inputs(&x_scale, &y_scale, &data, &config).unwrap_err(),
            ScatterRenderError::NegativeConfigField {
                field: "point_radius",
                value: -1.0,
            }
        );

        let mut config = ScatterConfig::new();
        config.stroke_width = f32::INFINITY;
        assert_eq!(
            validate_scatter_inputs(&x_scale, &y_scale, &data, &config).unwrap_err(),
            ScatterRenderError::NonFiniteConfigField {
                field: "stroke_width",
                value: f32::INFINITY,
            }
        );

        let mut config = ScatterConfig::new();
        config.opacity = 1.5;
        assert_eq!(
            validate_scatter_inputs(&x_scale, &y_scale, &data, &config).unwrap_err(),
            ScatterRenderError::OpacityOutOfRange { value: 1.5 }
        );
    }
}
