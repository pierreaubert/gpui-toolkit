//! Scatter plot rendering

use crate::color::D3Color;
use crate::scale::Scale;
use gpui::prelude::*;
use gpui::*;

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
pub fn render_scatter<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[ScatterPoint],
    config: &ScatterConfig,
) -> impl IntoElement
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    let (x_min, x_max) = x_scale.range();
    let (y_min, y_max) = y_scale.range();
    let x_range_span = x_max - x_min;
    let y_range_span = y_max - y_min;

    let fill = config.fill_color.to_rgba();

    div()
        .absolute()
        .inset_0()
        .children(data.iter().map(|point| {
            let x_range = x_scale.scale(point.x);
            let x_pos = ((x_range - x_min) / x_range_span) as f32;

            let y_range = y_scale.scale(point.y);
            // Invert Y for screen coordinates (bottom-to-top becomes top-to-bottom)
            let y_pos = 1.0 - ((y_range - y_min) / y_range_span) as f32;

            let diameter = config.point_radius * 2.0;

            let mut circle = div()
                .absolute()
                .left(relative(x_pos))
                .top(relative(y_pos))
                .w(px(diameter))
                .h(px(diameter))
                .ml(px(-config.point_radius))
                .mt(px(-config.point_radius))
                .rounded_full()
                .bg(fill)
                .opacity(config.opacity);

            if let Some(stroke) = &config.stroke_color {
                circle = circle
                    .border_color(stroke.to_rgba())
                    .border(px(config.stroke_width));
            }

            circle
        }))
}
