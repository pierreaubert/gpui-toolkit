//! Line chart rendering

use crate::color::D3Color;
use crate::scale::Scale;
use gpui::prelude::*;
use gpui::*;

/// Curve interpolation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveType {
    /// Linear interpolation (straight lines between points)
    Linear,
    /// Step function (horizontal then vertical)
    Step,
    /// Step before (vertical then horizontal)
    StepBefore,
    /// Step after (horizontal then vertical)
    StepAfter,
}

/// Configuration for line chart rendering
#[derive(Clone)]
pub struct LineConfig {
    /// Stroke color for the line
    pub stroke_color: D3Color,
    /// Line width in pixels
    pub stroke_width: f32,
    /// Opacity of the line (0.0 - 1.0)
    pub opacity: f32,
    /// Curve interpolation type
    pub curve: CurveType,
    /// Whether to show points at data locations
    pub show_points: bool,
    /// Point radius if show_points is true
    pub point_radius: f32,
    /// Fill color for points
    pub point_fill_color: Option<D3Color>,
}

impl Default for LineConfig {
    fn default() -> Self {
        Self {
            stroke_color: D3Color::from_hex(0x4682b4), // Steel blue
            stroke_width: 2.0,
            opacity: 1.0,
            curve: CurveType::Linear,
            show_points: false,
            point_radius: 3.0,
            point_fill_color: None,
        }
    }
}

impl LineConfig {
    /// Create a new line configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the stroke color
    pub fn stroke_color(mut self, color: D3Color) -> Self {
        self.stroke_color = color;
        self
    }

    /// Set the stroke width
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Set the opacity
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the curve type
    pub fn curve(mut self, curve: CurveType) -> Self {
        self.curve = curve;
        self
    }

    /// Enable point rendering
    pub fn show_points(mut self, show: bool) -> Self {
        self.show_points = show;
        self
    }

    /// Set point radius
    pub fn point_radius(mut self, radius: f32) -> Self {
        self.point_radius = radius;
        self
    }

    /// Set point fill color
    pub fn point_fill_color(mut self, color: D3Color) -> Self {
        self.point_fill_color = Some(color);
        self
    }
}

/// Data point for a line chart
#[derive(Debug, Clone, Copy)]
pub struct LinePoint {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
}

impl LinePoint {
    /// Create a new line point
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Clip a line segment to the unit rectangle [0,1] x [0,1] using Cohen-Sutherland algorithm
/// Returns Some((x0, y0, x1, y1)) if the clipped segment is visible, None if entirely outside
fn clip_line_segment(x0: f32, y0: f32, x1: f32, y1: f32) -> Option<(f32, f32, f32, f32)> {
    const INSIDE: u8 = 0;
    const LEFT: u8 = 1;
    const RIGHT: u8 = 2;
    const BOTTOM: u8 = 4;
    const TOP: u8 = 8;

    fn compute_outcode(x: f32, y: f32) -> u8 {
        let mut code = INSIDE;
        if x < 0.0 {
            code |= LEFT;
        } else if x > 1.0 {
            code |= RIGHT;
        }
        if y < 0.0 {
            code |= TOP;
        } else if y > 1.0 {
            code |= BOTTOM;
        }
        code
    }

    let mut x0 = x0;
    let mut y0 = y0;
    let mut x1 = x1;
    let mut y1 = y1;
    let mut outcode0 = compute_outcode(x0, y0);
    let mut outcode1 = compute_outcode(x1, y1);

    loop {
        if (outcode0 | outcode1) == 0 {
            // Both points inside
            return Some((x0, y0, x1, y1));
        } else if (outcode0 & outcode1) != 0 {
            // Both points share an outside zone
            return None;
        } else {
            // Calculate intersection
            let outcode_out = if outcode0 != 0 { outcode0 } else { outcode1 };
            let (x, y);

            if (outcode_out & TOP) != 0 {
                x = x0 + (x1 - x0) * (0.0 - y0) / (y1 - y0);
                y = 0.0;
            } else if (outcode_out & BOTTOM) != 0 {
                x = x0 + (x1 - x0) * (1.0 - y0) / (y1 - y0);
                y = 1.0;
            } else if (outcode_out & RIGHT) != 0 {
                y = y0 + (y1 - y0) * (1.0 - x0) / (x1 - x0);
                x = 1.0;
            } else {
                // LEFT
                y = y0 + (y1 - y0) * (0.0 - x0) / (x1 - x0);
                x = 0.0;
            }

            if outcode_out == outcode0 {
                x0 = x;
                y0 = y;
                outcode0 = compute_outcode(x0, y0);
            } else {
                x1 = x;
                y1 = y;
                outcode1 = compute_outcode(x1, y1);
            }
        }
    }
}

/// Render a line chart using GPUI's PathBuilder for proper vector line rendering
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::prelude::*;
/// use d3rs::shape::{render_line, LineConfig, LinePoint, CurveType};
///
/// let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
/// let y_scale = LinearScale::new().domain(0.0, 100.0).range(300.0, 0.0);
///
/// let data = vec![
///     LinePoint::new(0.0, 20.0),
///     LinePoint::new(25.0, 50.0),
///     LinePoint::new(50.0, 30.0),
///     LinePoint::new(75.0, 80.0),
///     LinePoint::new(100.0, 60.0),
/// ];
///
/// let config = LineConfig::new()
///     .stroke_color(D3Color::from_hex(0x4682b4))
///     .curve(CurveType::Linear)
///     .show_points(true);
/// // render_line(&x_scale, &y_scale, &data, &config)
/// ```
pub fn render_line<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[LinePoint],
    config: &LineConfig,
) -> impl IntoElement + use<XS, YS>
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    let (x_min, x_max) = x_scale.range();
    let (y_min, y_max) = y_scale.range();
    let x_range_span = x_max - x_min;

    // Pre-calculate relative positions for the line (in 0..1 range)
    // The scale maps domain values to range values (screen coordinates)
    // We need to normalize to 0..1 where 0 is the top of the plot area
    let mut relative_points: Vec<(f32, f32)> = Vec::with_capacity(data.len());
    for point in data {
        let x_range = x_scale.scale(point.x);
        let x_rel = ((x_range - x_min) / x_range_span) as f32;
        let y_range = y_scale.scale(point.y);
        // y_range is in screen coordinates
        // For inverted range (typical: range(height, 0)), y_min > y_max
        // y_range=0 (top) should map to y_rel=0, y_range=y_min (bottom) should map to y_rel=1
        let y_rel = if y_min > y_max {
            // Inverted range: y_min is at bottom, y_max (0) is at top
            (y_range / y_min) as f32
        } else {
            // Normal range: y_min is at top (0), y_max is at bottom
            ((y_range - y_min) / (y_max - y_min)) as f32
        };
        relative_points.push((x_rel, y_rel));
    }

    let stroke_color = config.stroke_color.to_rgba();
    let stroke_width = config.stroke_width;
    let opacity = config.opacity;
    let curve_type = config.curve;
    let show_points = config.show_points;
    let point_radius = config.point_radius;
    let point_fill = config
        .point_fill_color
        .as_ref()
        .unwrap_or(&config.stroke_color)
        .to_rgba();

    canvas(
        // Prepaint: pass through the relative points and bounds info
        move |bounds, _window, _cx| {
            let width: f32 = bounds.size.width.into();
            let height: f32 = bounds.size.height.into();
            let origin_x: f32 = bounds.origin.x.into();
            let origin_y: f32 = bounds.origin.y.into();

            (relative_points.clone(), width, height, origin_x, origin_y)
        },
        // Paint: draw clipped line segments
        move |_bounds,
              (rel_points, width, height, origin_x, origin_y): (
            Vec<(f32, f32)>,
            f32,
            f32,
            f32,
            f32,
        ),
              window,
              _cx| {
            if rel_points.len() < 2 {
                return;
            }

            // Build segments to draw based on curve type, applying clipping
            let segments_to_draw: Vec<(f32, f32, f32, f32)> = match curve_type {
                CurveType::Linear => {
                    let mut segments = Vec::new();
                    for i in 1..rel_points.len() {
                        let (x0, y0) = rel_points[i - 1];
                        let (x1, y1) = rel_points[i];
                        if let Some(clipped) = clip_line_segment(x0, y0, x1, y1) {
                            segments.push(clipped);
                        }
                    }
                    segments
                }
                CurveType::Step | CurveType::StepAfter => {
                    let mut segments = Vec::new();
                    for i in 1..rel_points.len() {
                        let (x0, y0) = rel_points[i - 1];
                        let (x1, y1) = rel_points[i];
                        // Horizontal then vertical: (x0,y0) -> (x1,y0) -> (x1,y1)
                        if let Some(clipped) = clip_line_segment(x0, y0, x1, y0) {
                            segments.push(clipped);
                        }
                        if let Some(clipped) = clip_line_segment(x1, y0, x1, y1) {
                            segments.push(clipped);
                        }
                    }
                    segments
                }
                CurveType::StepBefore => {
                    let mut segments = Vec::new();
                    for i in 1..rel_points.len() {
                        let (x0, y0) = rel_points[i - 1];
                        let (x1, y1) = rel_points[i];
                        // Vertical then horizontal: (x0,y0) -> (x0,y1) -> (x1,y1)
                        if let Some(clipped) = clip_line_segment(x0, y0, x0, y1) {
                            segments.push(clipped);
                        }
                        if let Some(clipped) = clip_line_segment(x0, y1, x1, y1) {
                            segments.push(clipped);
                        }
                    }
                    segments
                }
            };

            // Build continuous paths from clipped segments
            if !segments_to_draw.is_empty() {
                let mut path_builder = PathBuilder::stroke(px(stroke_width));
                let mut last_end: Option<(f32, f32)> = None;

                for (x0, y0, x1, y1) in &segments_to_draw {
                    let start = (origin_x + x0 * width, origin_y + y0 * height);
                    let end = (origin_x + x1 * width, origin_y + y1 * height);

                    // Check if we need to start a new path segment
                    let need_move = match last_end {
                        Some((lx, ly)) => (lx - start.0).abs() > 0.5 || (ly - start.1).abs() > 0.5,
                        None => true,
                    };

                    if need_move {
                        path_builder.move_to(gpui::point(px(start.0), px(start.1)));
                    }
                    path_builder.line_to(gpui::point(px(end.0), px(end.1)));
                    last_end = Some(end);
                }

                if let Ok(path) = path_builder.build() {
                    let color_with_opacity = Rgba {
                        r: stroke_color.r,
                        g: stroke_color.g,
                        b: stroke_color.b,
                        a: stroke_color.a * opacity,
                    };
                    window.paint_path(path, color_with_opacity);
                }
            }

            // Paint points if enabled (only for points inside the clip region)
            if show_points {
                for &(x_rel, y_rel) in &rel_points {
                    // Only draw points inside the chart area
                    if (0.0..=1.0).contains(&x_rel) && (0.0..=1.0).contains(&y_rel) {
                        let px_x = origin_x + x_rel * width;
                        let px_y = origin_y + y_rel * height;
                        let point_bounds = Bounds {
                            origin: gpui::point(px(px_x - point_radius), px(px_y - point_radius)),
                            size: gpui::size(px(point_radius * 2.0), px(point_radius * 2.0)),
                        };
                        let color_with_opacity = Rgba {
                            r: point_fill.r,
                            g: point_fill.g,
                            b: point_fill.b,
                            a: point_fill.a * opacity,
                        };
                        window.paint_quad(PaintQuad {
                            bounds: point_bounds,
                            corner_radii: Corners::all(px(point_radius)),
                            background: color_with_opacity.into(),
                            border_widths: Edges::default(),
                            border_color: transparent_black(),
                            border_style: BorderStyle::default(),
                        });
                    }
                }
            }
        },
    )
    .size_full()
    .absolute()
    .inset_0()
}
