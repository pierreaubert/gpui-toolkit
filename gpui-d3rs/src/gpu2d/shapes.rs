//! GPU-accelerated shape rendering functions
//!
//! These functions mirror the API of src/shape/*.rs but use GPU rendering.

use super::element::Chart2DElement;
use super::primitives::{Color4, Rect};
use crate::color::D3Color;
use crate::scale::Scale;
use gpui::*;

use std::sync::Arc;

// Re-export existing types from shape module
pub use crate::axis::{AxisConfig, AxisOrientation};
// Re-export contour types
pub use crate::contour::{Contour, ContourBand};
pub use crate::shape::contour::{ContourConfig, HeatmapData};
pub use crate::shape::contour::{
    heat_color_scale, inferno_color_scale, magma_color_scale, plasma_color_scale,
    turbo_color_scale, viridis_color_scale,
};
pub use crate::shape::{
    BarConfig, BarDatum, CurveType, LineConfig, LinePoint, ScatterConfig, ScatterPoint,
};

/// Convert D3Color + opacity to Color4
fn to_color4(color: &D3Color, opacity: f32) -> Color4 {
    // Use D3Color fields directly (they're already f32 in [0,1] range)
    [color.r, color.g, color.b, color.a * opacity]
}

/// Render a scatter plot using GPU acceleration
///
/// This is a drop-in replacement for `crate::shape::render_scatter`.
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

    // Pre-compute screen positions (relative coordinates 0-1)
    let points: Vec<(f32, f32)> = data
        .iter()
        .map(|point| {
            let x_range = x_scale.scale(point.x);
            let x_pos = ((x_range - x_min) / x_range_span) as f32;

            let y_range = y_scale.scale(point.y);
            let y_pos = 1.0 - ((y_range - y_min) / y_range_span) as f32;

            (x_pos, y_pos)
        })
        .collect();

    let fill_color = to_color4(&config.fill_color, config.opacity);
    let stroke_color = config.stroke_color.as_ref().map(|c| to_color4(c, 1.0));
    let stroke_width = config.stroke_width;
    let radius = config.point_radius;

    Chart2DElement::new(move |renderer, bounds| {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();

        for &(x_rel, y_rel) in &points {
            let cx = x_rel * width;
            let cy = y_rel * height;

            // Draw stroke circle first (larger, behind)
            if let Some(stroke) = stroke_color {
                renderer.draw_circle(cx, cy, radius + stroke_width, stroke);
            }

            // Draw fill circle
            renderer.draw_circle(cx, cy, radius, fill_color);
        }
    })
    .transparent()
    .absolute()
}

/// Render a bar chart using GPU acceleration
///
/// This is a drop-in replacement for `crate::shape::render_bars`.
pub fn render_bars<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[BarDatum],
    width: f32,
    height: f32,
    config: &BarConfig,
) -> impl IntoElement
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    let (x_min, x_max) = x_scale.range();
    let (y_min, y_max) = y_scale.range();
    let x_range_span = x_max - x_min;
    let y_range_span = y_max - y_min;

    // Calculate bar width
    let bar_count = data.len() as f32;
    let available_width = width - (config.bar_gap * (bar_count - 1.0));
    let bar_width = if bar_count > 0.0 {
        available_width / bar_count
    } else {
        0.0
    };

    // Get baseline
    let (y_domain_min, y_domain_max) = y_scale.domain();
    let baseline = if y_domain_min <= 0.0 && y_domain_max >= 0.0 {
        y_scale.scale(0.0)
    } else {
        y_scale.scale(y_domain_min)
    };
    let baseline_pos = 1.0 - ((baseline - y_min) / y_range_span) as f32;

    // Pre-compute bar positions and sizes
    let bars: Vec<(f32, f32, f32, f32)> = data
        .iter()
        .enumerate()
        .map(|(i, datum)| {
            let x_value = i as f64 + 0.5;
            let x_range = x_scale.scale(x_value);
            let x_pos = ((x_range - x_min) / x_range_span) as f32;

            let y_range = y_scale.scale(datum.value);
            let y_pos = 1.0 - ((y_range - y_min) / y_range_span) as f32;

            let bar_height_rel = (baseline_pos - y_pos).abs();
            let bar_top = if datum.value >= 0.0 {
                y_pos
            } else {
                baseline_pos
            };

            (x_pos, bar_top, bar_height_rel, bar_width)
        })
        .collect();

    let fill_color = to_color4(&config.fill_color, config.opacity);
    let stroke_color = config.stroke_color.as_ref().map(|c| to_color4(c, 1.0));
    let stroke_width = config.stroke_width;
    let border_radius = config.border_radius;
    let chart_height = height;

    Chart2DElement::new(move |renderer, bounds| {
        let w: f32 = bounds.size.width.into();
        let _h: f32 = bounds.size.height.into();

        for &(x_rel, y_rel, h_rel, bar_w) in &bars {
            let x = x_rel * w - bar_w / 2.0;
            let y = y_rel * chart_height;
            let bar_h = h_rel * chart_height;

            // Draw stroke rectangle first (slightly larger, behind)
            if let Some(stroke) = stroke_color {
                let stroke_rect = Rect::new(
                    x - stroke_width,
                    y - stroke_width,
                    bar_w + stroke_width * 2.0,
                    bar_h + stroke_width * 2.0,
                );
                renderer.draw_rect(stroke_rect, stroke, border_radius + stroke_width);
            }

            // Draw fill rectangle
            let rect = Rect::new(x, y, bar_w, bar_h);
            renderer.draw_rect(rect, fill_color, border_radius);
        }
    })
    .transparent()
    .absolute()
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
            return Some((x0, y0, x1, y1));
        } else if (outcode0 & outcode1) != 0 {
            return None;
        } else {
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

/// Render a line chart using GPU acceleration
///
/// This is a drop-in replacement for `crate::shape::render_line`.
pub fn render_line<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[LinePoint],
    config: &LineConfig,
) -> impl IntoElement
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    let (x_min, x_max) = x_scale.range();
    let (y_min, y_max) = y_scale.range();
    let x_range_span = x_max - x_min;
    let y_range_span = y_max - y_min;

    // Pre-calculate relative positions (0-1 range)
    let relative_points: Vec<(f32, f32)> = data
        .iter()
        .map(|point| {
            let x_range = x_scale.scale(point.x);
            let x_rel = ((x_range - x_min) / x_range_span) as f32;
            let y_range = y_scale.scale(point.y);
            let y_rel = 1.0 - ((y_range - y_min) / y_range_span) as f32;
            (x_rel, y_rel)
        })
        .collect();

    // Build segments based on curve type, applying clipping
    let segments: Vec<(f32, f32, f32, f32)> = match config.curve {
        CurveType::Linear => {
            let mut segs = Vec::new();
            for i in 1..relative_points.len() {
                let (x0, y0) = relative_points[i - 1];
                let (x1, y1) = relative_points[i];
                if let Some(clipped) = clip_line_segment(x0, y0, x1, y1) {
                    segs.push(clipped);
                }
            }
            segs
        }
        CurveType::Step | CurveType::StepAfter => {
            let mut segs = Vec::new();
            for i in 1..relative_points.len() {
                let (x0, y0) = relative_points[i - 1];
                let (x1, y1) = relative_points[i];
                // Horizontal then vertical
                if let Some(clipped) = clip_line_segment(x0, y0, x1, y0) {
                    segs.push(clipped);
                }
                if let Some(clipped) = clip_line_segment(x1, y0, x1, y1) {
                    segs.push(clipped);
                }
            }
            segs
        }
        CurveType::StepBefore => {
            let mut segs = Vec::new();
            for i in 1..relative_points.len() {
                let (x0, y0) = relative_points[i - 1];
                let (x1, y1) = relative_points[i];
                // Vertical then horizontal
                if let Some(clipped) = clip_line_segment(x0, y0, x0, y1) {
                    segs.push(clipped);
                }
                if let Some(clipped) = clip_line_segment(x0, y1, x1, y1) {
                    segs.push(clipped);
                }
            }
            segs
        }
    };

    let stroke_color = to_color4(&config.stroke_color, config.opacity);
    let stroke_width = config.stroke_width;
    let show_points = config.show_points;
    let point_radius = config.point_radius;
    let point_color = config
        .point_fill_color
        .as_ref()
        .map(|c| to_color4(c, config.opacity))
        .unwrap_or(stroke_color);

    Chart2DElement::new(move |renderer, bounds| {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();

        // Draw line segments
        for &(x0, y0, x1, y1) in &segments {
            renderer.draw_line(
                x0 * width,
                y0 * height,
                x1 * width,
                y1 * height,
                stroke_width,
                stroke_color,
            );
        }

        // Draw points if enabled
        if show_points {
            for &(x_rel, y_rel) in &relative_points {
                if x_rel >= 0.0 && x_rel <= 1.0 && y_rel >= 0.0 && y_rel <= 1.0 {
                    renderer.draw_circle(x_rel * width, y_rel * height, point_radius, point_color);
                }
            }
        }
    })
    .transparent()
    .absolute()
}

/// GPU-accelerated grid configuration
#[derive(Clone)]
pub struct GpuGridConfig {
    /// Width of grid lines in pixels
    pub line_width: f32,
    /// Opacity of grid lines (0.0 - 1.0)
    pub line_opacity: f32,
    /// Radius of dots at intersections
    pub dot_radius: f32,
    /// Opacity of dots (0.0 - 1.0)
    pub dot_opacity: f32,
    /// Show vertical grid lines
    pub show_vertical_lines: bool,
    /// Show horizontal grid lines
    pub show_horizontal_lines: bool,
    /// Show dots at grid intersections
    pub show_dots: bool,
    /// Custom vertical line positions (if None, uses scale ticks)
    pub vertical_line_values: Option<Vec<f64>>,
    /// Custom horizontal line positions (if None, uses scale ticks)
    pub horizontal_line_values: Option<Vec<f64>>,
    /// Grid line color
    pub line_color: Color4,
}

impl Default for GpuGridConfig {
    fn default() -> Self {
        Self {
            line_width: 1.0,
            line_opacity: 0.2,
            dot_radius: 2.0,
            dot_opacity: 0.3,
            show_vertical_lines: true,
            show_horizontal_lines: true,
            show_dots: false,
            vertical_line_values: None,
            horizontal_line_values: None,
            line_color: [0.5, 0.5, 0.5, 1.0], // Gray
        }
    }
}

impl GpuGridConfig {
    /// Create a grid with only lines
    pub fn with_lines() -> Self {
        Self::default()
    }

    /// Create a grid with only dots
    pub fn with_dots() -> Self {
        Self {
            show_vertical_lines: false,
            show_horizontal_lines: false,
            show_dots: true,
            ..Default::default()
        }
    }

    /// Create a grid with both lines and dots
    pub fn with_lines_and_dots() -> Self {
        Self {
            show_dots: true,
            ..Default::default()
        }
    }

    /// Set the line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    /// Set the line opacity
    pub fn line_opacity(mut self, opacity: f32) -> Self {
        self.line_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the line color
    pub fn line_color(mut self, color: Color4) -> Self {
        self.line_color = color;
        self
    }
}

/// Render a grid overlay using GPU acceleration
pub fn render_grid<XS, YS>(x_scale: &XS, y_scale: &YS, config: &GpuGridConfig) -> impl IntoElement
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    // Get tick positions
    let x_ticks = config
        .vertical_line_values
        .clone()
        .unwrap_or_else(|| x_scale.ticks(10));
    let y_ticks = config
        .horizontal_line_values
        .clone()
        .unwrap_or_else(|| y_scale.ticks(10));

    let (x_range_min, x_range_max) = x_scale.range();
    let (y_range_min, y_range_max) = y_scale.range();
    let x_range_span = x_range_max - x_range_min;
    let y_range_span = y_range_max - y_range_min;

    // Pre-compute relative positions
    let x_positions: Vec<f32> = x_ticks
        .iter()
        .map(|&x| {
            let x_range = x_scale.scale(x);
            ((x_range - x_range_min) / x_range_span) as f32
        })
        .collect();

    let y_positions: Vec<f32> = y_ticks
        .iter()
        .map(|&y| {
            let y_range = y_scale.scale(y);
            (1.0 - (y_range - y_range_min) / y_range_span) as f32
        })
        .collect();

    let line_width = config.line_width;
    let line_color = [
        config.line_color[0],
        config.line_color[1],
        config.line_color[2],
        config.line_color[3] * config.line_opacity,
    ];
    let dot_radius = config.dot_radius;
    let dot_color = [
        config.line_color[0],
        config.line_color[1],
        config.line_color[2],
        config.line_color[3] * config.dot_opacity,
    ];
    let show_v = config.show_vertical_lines;
    let show_h = config.show_horizontal_lines;
    let show_dots = config.show_dots;

    Chart2DElement::new(move |renderer, bounds| {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();

        // Draw vertical lines
        if show_v {
            for &x_rel in &x_positions {
                let x = x_rel * width;
                renderer.draw_line(x, 0.0, x, height, line_width, line_color);
            }
        }

        // Draw horizontal lines
        if show_h {
            for &y_rel in &y_positions {
                let y = y_rel * height;
                renderer.draw_line(0.0, y, width, y, line_width, line_color);
            }
        }

        // Draw dots at intersections
        if show_dots {
            for &y_rel in &y_positions {
                for &x_rel in &x_positions {
                    let x = x_rel * width;
                    let y = y_rel * height;
                    renderer.draw_circle(x, y, dot_radius, dot_color);
                }
            }
        }
    })
    .transparent()
    .absolute()
}

/// GPU-accelerated axis theme configuration
#[derive(Clone)]
pub struct GpuAxisTheme {
    /// Line color for domain line and ticks
    pub line_color: Color4,
    /// Text color for labels
    pub label_color: Color4,
}

impl Default for GpuAxisTheme {
    fn default() -> Self {
        Self {
            line_color: [1.0, 1.0, 1.0, 1.0],
            label_color: [0.9, 0.9, 0.9, 1.0],
        }
    }
}

impl GpuAxisTheme {
    /// Create with custom colors
    pub fn new(line_color: Color4, label_color: Color4) -> Self {
        Self {
            line_color,
            label_color,
        }
    }

    /// Light theme (dark text on light background)
    pub fn light() -> Self {
        Self {
            line_color: [0.2, 0.2, 0.2, 1.0],
            label_color: [0.1, 0.1, 0.1, 1.0],
        }
    }

    /// Dark theme (light text on dark background)
    pub fn dark() -> Self {
        Self::default()
    }
}

/// Format a tick value using the optional custom formatter
fn format_tick(value: f64, formatter: &Option<fn(f64) -> String>) -> String {
    match formatter {
        Some(f) => f(value),
        None => {
            if value.abs() < 1e-10 {
                "0".to_string()
            } else if value.abs() >= 1000.0 || value.abs() < 0.01 {
                format!("{:.1e}", value)
            } else if value.fract().abs() < 1e-10 {
                format!("{:.0}", value)
            } else {
                format!("{:.1}", value)
            }
        }
    }
}

/// Render an axis using GPU acceleration
///
/// This is a drop-in replacement for `crate::axis::render_axis`.
pub fn render_axis<S>(
    scale: &S,
    config: &AxisConfig,
    size: f32,
    theme: &GpuAxisTheme,
) -> impl IntoElement
where
    S: Scale<f64, f64>,
{
    let ticks = match &config.tick_values {
        Some(values) => values.clone(),
        None => scale.ticks(config.tick_count),
    };

    let (range_min, range_max) = scale.range();
    let range_span = range_max - range_min;

    // Pre-compute tick data
    let tick_data: Vec<(f32, String)> = ticks
        .iter()
        .map(|&value| {
            let pos = ((scale.scale(value) - range_min) / range_span) as f32;
            let label = format_tick(value, &config.tick_format);
            (pos, label)
        })
        .collect();

    // Minor ticks
    let minor_ticks: Vec<f32> = config
        .minor_tick_values
        .as_ref()
        .map(|values| {
            values
                .iter()
                .map(|&v| ((scale.scale(v) - range_min) / range_span) as f32)
                .filter(|&p| p >= 0.0 && p <= 1.0)
                .collect()
        })
        .unwrap_or_default();

    let orientation = config.orientation.clone();
    let tick_size = config.tick_size;
    let minor_tick_size = config.minor_tick_size;
    let tick_padding = config.tick_padding;
    let label_font_size = config.label_font_size;
    let show_domain = config.show_domain_line;
    let domain_width = config.domain_line_width;
    let line_color = theme.line_color;
    let label_color = theme.label_color;
    let title = config.title.clone();
    let title_font_size = config.title_font_size;
    let _size = size; // Not used directly, bounds come from element

    Chart2DElement::new(move |renderer, bounds| {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();

        match orientation {
            AxisOrientation::Bottom => {
                // Domain line at top
                if show_domain {
                    renderer.draw_line(0.0, 0.0, width, 0.0, domain_width, line_color);
                }

                // Ticks and labels
                for (rel_pos, label) in &tick_data {
                    let x = *rel_pos * width;

                    // Tick mark
                    renderer.draw_line(x, 0.0, x, tick_size, domain_width, line_color);

                    // Label (centered below tick)
                    let label_y = tick_size + tick_padding + label_font_size * 0.8;
                    let label_width = label.len() as f32 * label_font_size * 0.6;
                    renderer.draw_text(
                        label,
                        x - label_width / 2.0,
                        label_y,
                        label_font_size,
                        label_color,
                    );
                }

                // Minor ticks
                for &rel_pos in &minor_ticks {
                    let x = rel_pos * width;
                    renderer.draw_line(x, 0.0, x, minor_tick_size, domain_width, line_color);
                }

                // Title
                if let Some(ref title_text) = title {
                    let title_y = tick_size
                        + tick_padding
                        + label_font_size
                        + tick_padding
                        + title_font_size * 0.8;
                    let title_width = title_text.len() as f32 * title_font_size * 0.6;
                    renderer.draw_text(
                        title_text,
                        width / 2.0 - title_width / 2.0,
                        title_y,
                        title_font_size,
                        label_color,
                    );
                }
            }
            AxisOrientation::Top => {
                let base_y = height;

                // Domain line at bottom
                if show_domain {
                    renderer.draw_line(0.0, base_y, width, base_y, domain_width, line_color);
                }

                // Ticks and labels
                for (rel_pos, label) in &tick_data {
                    let x = *rel_pos * width;

                    // Tick mark (pointing up)
                    renderer.draw_line(x, base_y - tick_size, x, base_y, domain_width, line_color);

                    // Label (centered above tick)
                    let label_y = base_y - tick_size - tick_padding;
                    let label_width = label.len() as f32 * label_font_size * 0.6;
                    renderer.draw_text(
                        label,
                        x - label_width / 2.0,
                        label_y,
                        label_font_size,
                        label_color,
                    );
                }

                // Minor ticks
                for &rel_pos in &minor_ticks {
                    let x = rel_pos * width;
                    renderer.draw_line(
                        x,
                        base_y - minor_tick_size,
                        x,
                        base_y,
                        domain_width,
                        line_color,
                    );
                }
            }
            AxisOrientation::Left => {
                let base_x = width;

                // Domain line at right
                if show_domain {
                    renderer.draw_line(base_x, 0.0, base_x, height, domain_width, line_color);
                }

                // Ticks and labels
                for (rel_pos, label) in &tick_data {
                    let y = (1.0 - *rel_pos) * height; // Invert for screen coords

                    // Tick mark (pointing left)
                    renderer.draw_line(base_x - tick_size, y, base_x, y, domain_width, line_color);

                    // Label (right-aligned to the left of tick)
                    let label_width = label.len() as f32 * label_font_size * 0.6;
                    let label_x = base_x - tick_size - tick_padding - label_width;
                    renderer.draw_text(
                        label,
                        label_x,
                        y + label_font_size * 0.3,
                        label_font_size,
                        label_color,
                    );
                }

                // Minor ticks
                for &rel_pos in &minor_ticks {
                    let y = (1.0 - rel_pos) * height;
                    renderer.draw_line(
                        base_x - minor_tick_size,
                        y,
                        base_x,
                        y,
                        domain_width,
                        line_color,
                    );
                }

                // Title (note: rotation not yet supported, using horizontal for now)
                if let Some(ref title_text) = title {
                    let title_width = title_text.len() as f32 * title_font_size * 0.6;
                    renderer.draw_text(
                        title_text,
                        2.0,
                        height / 2.0 + title_width / 2.0,
                        title_font_size,
                        label_color,
                    );
                }
            }
            AxisOrientation::Right => {
                // Domain line at left
                if show_domain {
                    renderer.draw_line(0.0, 0.0, 0.0, height, domain_width, line_color);
                }

                // Ticks and labels
                for (rel_pos, label) in &tick_data {
                    let y = (1.0 - *rel_pos) * height;

                    // Tick mark (pointing right)
                    renderer.draw_line(0.0, y, tick_size, y, domain_width, line_color);

                    // Label (left-aligned to the right of tick)
                    let label_x = tick_size + tick_padding;
                    renderer.draw_text(
                        label,
                        label_x,
                        y + label_font_size * 0.3,
                        label_font_size,
                        label_color,
                    );
                }

                // Minor ticks
                for &rel_pos in &minor_ticks {
                    let y = (1.0 - rel_pos) * height;
                    renderer.draw_line(0.0, y, minor_tick_size, y, domain_width, line_color);
                }
            }
        }
    })
    .transparent()
    .absolute()
}

// ============================================================================
// Contour rendering
// ============================================================================

/// Render contour lines using GPU acceleration
///
/// This is a drop-in replacement for `crate::shape::contour::render_contour`.
pub fn render_contour<XS, YS>(
    contours: impl Into<Arc<[Contour]>>,
    x_scale: &XS,
    y_scale: &YS,
    config: &ContourConfig,
) -> impl IntoElement
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    let contours = contours.into();

    // Calculate value range for color normalization
    let value_range = if contours.is_empty() {
        (0.0, 1.0)
    } else {
        let min = contours
            .iter()
            .map(|c| c.value)
            .fold(f64::INFINITY, f64::min);
        let max = contours
            .iter()
            .map(|c| c.value)
            .fold(f64::NEG_INFINITY, f64::max);
        (min, max)
    };

    let (x_range_min, x_range_max) = x_scale.range();
    let (y_range_min, y_range_max) = y_scale.range();
    let x_range_span = x_range_max - x_range_min;
    let y_range_span = y_range_max - y_range_min;

    // Pre-process contours into drawable data
    let contour_data: Vec<ContourDrawData> = contours
        .iter()
        .map(|contour| {
            let t = normalize_value(contour.value, value_range.0, value_range.1);
            let stroke_color = get_contour_color(
                t,
                &config.color_scale,
                &config.stroke_color,
                config.stroke_opacity,
            );
            let fill_color = get_contour_color(
                t,
                &config.color_scale,
                &config.fill_color,
                config.fill_opacity,
            );

            let rings: Vec<Vec<(f32, f32)>> = contour
                .coordinates
                .iter()
                .filter(|ring| ring.points.len() >= 3)
                .map(|ring| {
                    ring.points
                        .iter()
                        .map(|p| {
                            let x_scaled = x_scale.scale(p.x);
                            let y_scaled = y_scale.scale(p.y);
                            let x_rel = ((x_scaled - x_range_min) / x_range_span) as f32;
                            let y_rel = 1.0 - ((y_scaled - y_range_min) / y_range_span) as f32;
                            (x_rel, y_rel)
                        })
                        .collect()
                })
                .collect();

            ContourDrawData {
                rings,
                stroke_color,
                fill_color,
            }
        })
        .collect();

    let stroke_width = config.stroke_width;
    let do_fill = config.fill;

    Chart2DElement::new(move |renderer, bounds| {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let x_jump_threshold = 0.15;
        let y_jump_threshold = 0.15;

        for data in &contour_data {
            for ring in &data.rings {
                if ring.len() < 3 {
                    continue;
                }

                // Check if ring is closed and has no large jumps
                let is_closed = {
                    let first = ring[0];
                    let last = ring[ring.len() - 1];
                    (first.0 - last.0).abs() < 0.01 && (first.1 - last.1).abs() < 0.01
                };

                let has_jump = ring.windows(2).any(|pair| {
                    (pair[1].0 - pair[0].0).abs() > x_jump_threshold
                        || (pair[1].1 - pair[0].1).abs() > y_jump_threshold
                });

                // Draw fill (only for closed rings without jumps)
                if do_fill && is_closed && !has_jump {
                    let polygon: Vec<[f32; 2]> =
                        ring.iter().map(|(x, y)| [x * width, y * height]).collect();
                    renderer.draw_polygon(&polygon, data.fill_color);
                }

                // Draw stroke
                if stroke_width > 0.0 {
                    let mut i = 0;
                    while i < ring.len() - 1 {
                        let (x0, y0) = ring[i];
                        let (x1, y1) = ring[i + 1];

                        // Skip if jump
                        if (x1 - x0).abs() > x_jump_threshold || (y1 - y0).abs() > y_jump_threshold
                        {
                            i += 1;
                            continue;
                        }

                        // Clip and draw
                        if let Some((cx0, cy0, cx1, cy1)) = clip_line_segment(x0, y0, x1, y1) {
                            renderer.draw_line(
                                cx0 * width,
                                cy0 * height,
                                cx1 * width,
                                cy1 * height,
                                stroke_width,
                                data.stroke_color,
                            );
                        }
                        i += 1;
                    }
                }
            }
        }
    })
    .transparent()
    .absolute()
}

/// Render filled contour bands using GPU acceleration
///
/// This is a drop-in replacement for `crate::shape::contour::render_contour_bands`.
pub fn render_contour_bands<XS, YS>(
    bands: impl Into<Arc<[ContourBand]>>,
    x_scale: &XS,
    y_scale: &YS,
    config: &ContourConfig,
) -> impl IntoElement
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    let bands = bands.into();

    // Calculate value range
    let value_range = if bands.is_empty() {
        (0.0, 1.0)
    } else {
        let min = bands.iter().map(|b| b.lower).fold(f64::INFINITY, f64::min);
        let max = bands
            .iter()
            .map(|b| b.upper)
            .fold(f64::NEG_INFINITY, f64::max);
        (min, max)
    };

    let (x_range_min, x_range_max) = x_scale.range();
    let (y_range_min, y_range_max) = y_scale.range();
    let x_range_span = x_range_max - x_range_min;
    let y_range_span = y_range_max - y_range_min;

    // Pre-process bands into drawable data
    let band_data: Vec<BandDrawData> = bands
        .iter()
        .map(|band| {
            let t = normalize_value(band.mid_value(), value_range.0, value_range.1);
            let fill_color = get_contour_color(
                t,
                &config.color_scale,
                &config.fill_color,
                config.fill_opacity,
            );

            let polygons: Vec<Vec<(f32, f32)>> = band
                .polygons
                .iter()
                .filter(|ring| ring.points.len() >= 3)
                .map(|ring| {
                    ring.points
                        .iter()
                        .map(|p| {
                            let x_scaled = x_scale.scale(p.x);
                            let y_scaled = y_scale.scale(p.y);
                            let x_rel = ((x_scaled - x_range_min) / x_range_span) as f32;
                            let y_rel = 1.0 - ((y_scaled - y_range_min) / y_range_span) as f32;
                            (x_rel, y_rel)
                        })
                        .collect()
                })
                .collect();

            BandDrawData {
                polygons,
                fill_color,
            }
        })
        .collect();

    let stroke_width = 2.0; // For gap elimination

    Chart2DElement::new(move |renderer, bounds| {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();

        for data in &band_data {
            for polygon in &data.polygons {
                if polygon.len() < 3 {
                    continue;
                }

                let pts: Vec<[f32; 2]> = polygon
                    .iter()
                    .map(|(x, y)| [x * width, y * height])
                    .collect();

                // Draw fill
                renderer.draw_polygon(&pts, data.fill_color);

                // Draw stroke to eliminate anti-aliasing gaps
                for i in 0..pts.len() {
                    let j = (i + 1) % pts.len();
                    renderer.draw_line(
                        pts[i][0],
                        pts[i][1],
                        pts[j][0],
                        pts[j][1],
                        stroke_width,
                        data.fill_color,
                    );
                }
            }
        }
    })
    .transparent()
    .absolute()
}

/// Render a heatmap (2D grid of colored cells) using GPU acceleration
///
/// This is a drop-in replacement for `crate::shape::contour::render_heatmap`.
pub fn render_heatmap<XS, YS>(
    data: HeatmapData,
    x_scale: &XS,
    y_scale: &YS,
    config: &ContourConfig,
) -> impl IntoElement
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    // Calculate value range
    let value_range = if data.values.is_empty() {
        (0.0, 1.0)
    } else {
        let min = data.values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = data
            .values
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        (min, max)
    };

    let (x_range_min, x_range_max) = x_scale.range();
    let (y_range_min, y_range_max) = y_scale.range();
    let x_range_span = x_range_max - x_range_min;
    let y_range_span = y_range_max - y_range_min;

    // Pre-process cells
    let mut cells: Vec<CellDrawData> = Vec::with_capacity(data.width * data.height);

    for yi in 0..data.height {
        for xi in 0..data.width {
            let value = match data.get(xi, yi) {
                Some(v) if v.is_finite() => v,
                _ => continue,
            };

            let t = normalize_value(value, value_range.0, value_range.1);
            let fill_color = get_contour_color(
                t,
                &config.color_scale,
                &config.fill_color,
                config.fill_opacity,
            );

            // Cell boundaries in data coordinates
            let x0_data = data.x_values[xi];
            let x1_data = if xi + 1 < data.width {
                data.x_values[xi + 1]
            } else if xi > 0 {
                x0_data + (x0_data - data.x_values[xi - 1])
            } else {
                x0_data * 1.1
            };

            let y0_data = data.y_values[yi];
            let y1_data = if yi + 1 < data.height {
                data.y_values[yi + 1]
            } else if yi > 0 {
                y0_data + (y0_data - data.y_values[yi - 1])
            } else {
                y0_data * 1.1
            };

            // Transform to relative coordinates
            let x0_scaled = x_scale.scale(x0_data);
            let x1_scaled = x_scale.scale(x1_data);
            let y0_scaled = y_scale.scale(y0_data);
            let y1_scaled = y_scale.scale(y1_data);

            let x0_rel = ((x0_scaled - x_range_min) / x_range_span) as f32;
            let x1_rel = ((x1_scaled - x_range_min) / x_range_span) as f32;
            let y0_rel = 1.0 - ((y0_scaled - y_range_min) / y_range_span) as f32;
            let y1_rel = 1.0 - ((y1_scaled - y_range_min) / y_range_span) as f32;

            cells.push(CellDrawData {
                x_min: x0_rel.min(x1_rel),
                x_max: x0_rel.max(x1_rel),
                y_min: y0_rel.min(y1_rel),
                y_max: y0_rel.max(y1_rel),
                fill_color,
            });
        }
    }

    Chart2DElement::new(move |renderer, bounds| {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();

        for cell in &cells {
            let x = cell.x_min * width;
            let y = cell.y_min * height;
            let w = (cell.x_max - cell.x_min) * width + 0.5; // Slight overlap
            let h = (cell.y_max - cell.y_min) * height + 0.5;

            let rect = Rect::new(x, y, w.max(1.0), h.max(1.0));
            renderer.draw_rect(rect, cell.fill_color, 0.0);
        }
    })
    .transparent()
    .absolute()
}

// Helper structs for pre-processed contour data
struct ContourDrawData {
    rings: Vec<Vec<(f32, f32)>>,
    stroke_color: Color4,
    fill_color: Color4,
}

struct BandDrawData {
    polygons: Vec<Vec<(f32, f32)>>,
    fill_color: Color4,
}

struct CellDrawData {
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    fill_color: Color4,
}

fn normalize_value(value: f64, min: f64, max: f64) -> f64 {
    if (max - min).abs() < 1e-10 {
        0.5
    } else {
        (value - min) / (max - min)
    }
}

fn get_contour_color(
    t: f64,
    color_scale: &Option<Arc<dyn Fn(f64) -> D3Color + Send + Sync>>,
    default_color: &D3Color,
    opacity: f32,
) -> Color4 {
    let color = if let Some(scale) = color_scale {
        scale(t)
    } else {
        *default_color
    };
    to_color4(&color, opacity)
}
