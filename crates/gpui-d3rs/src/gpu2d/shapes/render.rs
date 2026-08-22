#[cfg(not(feature = "vello-gpui"))]
use super::super::element::Chart2DElement;
#[cfg(not(feature = "vello-gpui"))]
use super::super::primitives::Rect;
use super::gpu_axis_theme::GpuAxisTheme;
use super::gpu_grid_config::GpuGridConfig;
#[cfg(not(feature = "vello-gpui"))]
use super::misc::clip_line_segment;
#[cfg(not(feature = "vello-gpui"))]
use super::misc::format_tick;
#[cfg(not(feature = "vello-gpui"))]
use super::misc::get_contour_color;
#[cfg(not(feature = "vello-gpui"))]
use super::misc::normalize_value;
#[cfg(not(feature = "vello-gpui"))]
use super::misc::to_color4;
#[cfg(not(feature = "vello-gpui"))]
use super::types::BandDrawData;
#[cfg(not(feature = "vello-gpui"))]
use super::types::CellDrawData;
#[cfg(not(feature = "vello-gpui"))]
use super::types::ContourDrawData;
#[allow(unused_imports)]
pub use crate::axis::{AxisConfig, AxisOrientation};
pub use crate::contour::{Contour, ContourBand};
#[cfg(not(feature = "vello-gpui"))]
use crate::lod::m4_point_indices;
use crate::scale::Scale;
pub use crate::shape::contour::{ContourConfig, HeatmapData};
#[cfg(not(feature = "vello-gpui"))]
use crate::shape::contour_smoothing::{
    StrokePoint, smooth_stroke_segment as smooth_stroke_points, split_stroke_segments,
};
#[allow(unused_imports)]
pub use crate::shape::{
    BarConfig, BarDatum, CurveType, LineConfig, LinePoint, ScatterConfig, ScatterPoint,
};
use gpui::*;
use std::sync::Arc;

/// Render a scatter plot using GPU acceleration
///
/// This is a drop-in replacement for `crate::shape::render_scatter`.
#[cfg(not(feature = "vello-gpui"))]
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
#[cfg(not(feature = "vello-gpui"))]
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

/// Render a line chart using GPU acceleration
///
/// This is a drop-in replacement for `crate::shape::render_line`.
#[cfg(not(feature = "vello-gpui"))]
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
        CurveType::Basis
        | CurveType::Cardinal
        | CurveType::CatmullRom
        | CurveType::MonotoneX
        | CurveType::Natural => {
            crate::shape::line::validation::compute_line_segments(&relative_points, config.curve)
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
    let curve = config.curve;
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

        // Lines are visually bounded by horizontal screen resolution. For a
        // large linear trace retain extrema per pixel column instead of
        // creating an unbounded GPU segment batch. Curved modes retain their
        // existing exact semantics until they gain curve-aware decimation.
        let lod_indices = (curve == CurveType::Linear
            && relative_points.len() > width.ceil().max(1.0) as usize * 4)
            .then(|| m4_point_indices(&relative_points, width.ceil().max(1.0) as usize));

        // Draw line segments
        if let Some(indices) = &lod_indices {
            for pair in indices.windows(2) {
                let (x0, y0) = relative_points[pair[0]];
                let (x1, y1) = relative_points[pair[1]];
                if let Some((x0, y0, x1, y1)) = clip_line_segment(x0, y0, x1, y1) {
                    renderer.draw_line(
                        x0 * width,
                        y0 * height,
                        x1 * width,
                        y1 * height,
                        stroke_width,
                        stroke_color,
                    );
                }
            }
        } else {
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
        }

        // Draw points if enabled
        if show_points {
            let point_indices: Box<dyn Iterator<Item = usize>> = match lod_indices {
                Some(indices) => Box::new(indices.into_iter()),
                None => Box::new(0..relative_points.len()),
            };
            for index in point_indices {
                let (x_rel, y_rel) = relative_points[index];
                if (0.0..=1.0).contains(&x_rel) && (0.0..=1.0).contains(&y_rel) {
                    renderer.draw_circle(x_rel * width, y_rel * height, point_radius, point_color);
                }
            }
        }
    })
    .transparent()
    .absolute()
}

/// Render a grid overlay using GPU acceleration
#[cfg(not(feature = "vello-gpui"))]
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
    let x_positions: Vec<f32> = if x_range_span == 0.0 {
        vec![]
    } else {
        x_ticks
            .iter()
            .map(|&x| {
                let x_range = x_scale.scale(x);
                ((x_range - x_range_min) / x_range_span) as f32
            })
            .collect()
    };

    let y_positions: Vec<f32> = if y_range_span == 0.0 {
        vec![]
    } else {
        y_ticks
            .iter()
            .map(|&y| {
                let y_range = y_scale.scale(y);
                (1.0 - (y_range - y_range_min) / y_range_span) as f32
            })
            .collect()
    };

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

/// Render an axis using GPU acceleration
///
/// This is a drop-in replacement for `crate::axis::render_axis`.
#[cfg(not(feature = "vello-gpui"))]
pub fn render_axis<S>(
    scale: &S,
    config: &AxisConfig,
    size: f32,
    theme: &GpuAxisTheme,
) -> impl IntoElement
where
    S: Scale<f64, f64>,
{
    use std::borrow::Cow;

    // Borrow explicit tick values; only allocate when falling back to scale ticks.
    let ticks: Cow<[f64]> = match &config.tick_values {
        Some(values) => Cow::Borrowed(values),
        None => Cow::Owned(scale.ticks(config.tick_count)),
    };

    let (range_min, range_max) = scale.range();
    let range_span = range_max - range_min;

    // Pre-compute tick data with cached SharedString labels.
    let tick_data: Vec<(f32, SharedString)> = if range_span == 0.0 {
        vec![]
    } else {
        ticks
            .iter()
            .map(|&value| {
                let pos = ((scale.scale(value) - range_min) / range_span) as f32;
                let label = format_tick(value, &config.tick_format);
                (pos, label)
            })
            .collect()
    };

    // Minor ticks
    let minor_ticks: Vec<f32> = if range_span == 0.0 {
        vec![]
    } else {
        config
            .minor_tick_values
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|&v| ((scale.scale(v) - range_min) / range_span) as f32)
            .filter(|p| (0.0..=1.0).contains(p))
            .collect()
    };

    let orientation = config.orientation;
    let tick_size = config.tick_size;
    let minor_tick_size = config.minor_tick_size;
    let tick_padding = config.tick_padding;
    let label_font_size = config.label_font_size;
    let label_angle = config.label_angle.to_radians();
    let show_domain = config.show_domain_line;
    let domain_width = config.domain_line_width;
    let line_color = theme.line_color;
    let label_color = theme.label_color;
    let title: Option<SharedString> = config.title.clone().map(SharedString::from);
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
                    renderer.draw_text_rotated(
                        label.as_ref(),
                        x - label_width / 2.0,
                        label_y,
                        label_font_size,
                        label_color,
                        label_angle,
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
                        title_text.as_ref(),
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
                    renderer.draw_text_rotated(
                        label.as_ref(),
                        x - label_width / 2.0,
                        label_y,
                        label_font_size,
                        label_color,
                        label_angle,
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
                        label.as_ref(),
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

                if let Some(ref title_text) = title {
                    let title_width = title_text.len() as f32 * title_font_size * 0.6;
                    renderer.draw_text_rotated(
                        title_text.as_ref(),
                        2.0,
                        height / 2.0 + title_width / 2.0,
                        title_font_size,
                        label_color,
                        -std::f32::consts::FRAC_PI_2,
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
                        label.as_ref(),
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

/// Render contour lines using GPU acceleration
///
/// This is a drop-in replacement for `crate::shape::contour::render_contour`.
#[cfg(not(feature = "vello-gpui"))]
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
    let smooth_strokes = config.smooth_strokes;
    let smoothing_iterations = config.smoothing_iterations;
    let smoothing_max_deviation_px = config.smoothing_max_deviation_px;

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
                    let stroke_ring = if is_closed && ring.len() >= 2 {
                        &ring[..ring.len() - 1]
                    } else {
                        &ring[..]
                    };
                    let stroke_points: Vec<_> = stroke_ring
                        .iter()
                        .map(|(x, y)| StrokePoint::new(x * width, y * height))
                        .collect();
                    let segments = split_stroke_segments(
                        &stroke_points,
                        x_jump_threshold * width,
                        y_jump_threshold * height,
                    );
                    let closes_single_segment = is_closed && !has_jump && segments.len() == 1;

                    for segment in segments {
                        let segment_is_closed = closes_single_segment && segment.len() >= 3;
                        let draw_points = smooth_stroke_points(
                            &segment,
                            segment_is_closed,
                            smooth_strokes,
                            smoothing_iterations,
                            smoothing_max_deviation_px,
                        );
                        for pair in draw_points.windows(2) {
                            renderer.draw_line(
                                pair[0].x,
                                pair[0].y,
                                pair[1].x,
                                pair[1].y,
                                stroke_width,
                                data.stroke_color,
                            );
                        }
                        if segment_is_closed
                            && let (Some(first), Some(last)) =
                                (draw_points.first(), draw_points.last())
                        {
                            renderer.draw_line(
                                last.x,
                                last.y,
                                first.x,
                                first.y,
                                stroke_width,
                                data.stroke_color,
                            );
                        }
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
#[cfg(not(feature = "vello-gpui"))]
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
#[cfg(not(feature = "vello-gpui"))]
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

/// Compatibility entry point that delegates axis labels and ticks to GPUI's
/// retained text painter instead of the legacy readback renderer.
#[cfg(feature = "vello-gpui")]
pub fn render_axis<S>(scale: &S, config: &AxisConfig, size: f32, theme: &GpuAxisTheme) -> AnyElement
where
    S: Scale<f64, f64>,
{
    crate::axis::render_axis(scale, config, size, theme)
}

/// Vello-backed compatibility entry point for legacy grid rendering.
#[cfg(feature = "vello-gpui")]
pub fn render_grid<XS, YS>(x_scale: &XS, y_scale: &YS, config: &GpuGridConfig) -> impl IntoElement
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    use crate::vello2d::kurbo::Stroke;
    use crate::vello2d::peniko::{Brush, Color};

    let x_ticks = config
        .vertical_line_values
        .clone()
        .unwrap_or_else(|| x_scale.ticks(10));
    let y_ticks = config
        .horizontal_line_values
        .clone()
        .unwrap_or_else(|| y_scale.ticks(10));
    let (x_min, x_max) = x_scale.range();
    let (y_min, y_max) = y_scale.range();
    let x_span = x_max - x_min;
    let y_span = y_max - y_min;
    let x_positions = (x_span != 0.0)
        .then(|| {
            x_ticks
                .iter()
                .map(|&x| ((x_scale.scale(x) - x_min) / x_span) as f32)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let y_positions = (y_span != 0.0)
        .then(|| {
            y_ticks
                .iter()
                .map(|&y| (1.0 - (y_scale.scale(y) - y_min) / y_span) as f32)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
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
    let show_vertical = config.show_vertical_lines;
    let show_horizontal = config.show_horizontal_lines;
    let show_dots = config.show_dots;

    crate::vello2d::VelloChartElement::with_builder(move |width, height| {
        let mut scene = crate::vello2d::ChartScene::new();
        let line_brush = Brush::Solid(Color::new(line_color));
        if show_vertical {
            for &x in &x_positions {
                let x = x * width;
                scene.stroke_polyline(
                    &[(x as f64, 0.0), (x as f64, height as f64)],
                    Stroke::new(line_width as f64),
                    line_brush.clone(),
                );
            }
        }
        if show_horizontal {
            for &y in &y_positions {
                let y = y * height;
                scene.stroke_polyline(
                    &[(0.0, y as f64), (width as f64, y as f64)],
                    Stroke::new(line_width as f64),
                    line_brush.clone(),
                );
            }
        }
        if show_dots {
            let dot_brush = Brush::Solid(Color::new(dot_color));
            for &y in &y_positions {
                for &x in &x_positions {
                    scene.fill_circle(
                        (x * width) as f64,
                        (y * height) as f64,
                        dot_radius as f64,
                        dot_brush.clone(),
                    );
                }
            }
        }
        scene
    })
    .absolute()
}

/// Vello-backed compatibility entry point for legacy scatter rendering.
#[cfg(feature = "vello-gpui")]
pub fn render_scatter<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[ScatterPoint],
    config: &ScatterConfig,
) -> impl IntoElement
where
    XS: Scale<f64, f64> + 'static,
    YS: Scale<f64, f64> + 'static,
{
    crate::shape::render_scatter_vello(
        x_scale,
        y_scale,
        data,
        config,
        crate::vello2d::RasterBackend::Auto,
    )
}

/// Vello-backed compatibility entry point for legacy bar rendering.
#[cfg(feature = "vello-gpui")]
pub fn render_bars<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[BarDatum],
    width: f32,
    height: f32,
    config: &BarConfig,
) -> impl IntoElement
where
    XS: Scale<f64, f64> + 'static,
    YS: Scale<f64, f64> + 'static,
{
    crate::shape::render_bars_vello(
        x_scale,
        y_scale,
        data,
        width,
        height,
        config,
        crate::vello2d::RasterBackend::Auto,
    )
}

/// Vello-backed compatibility entry point for legacy line rendering.
#[cfg(feature = "vello-gpui")]
pub fn render_line<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[LinePoint],
    config: &LineConfig,
) -> impl IntoElement
where
    XS: Scale<f64, f64> + 'static,
    YS: Scale<f64, f64> + 'static,
{
    crate::shape::render_line_vello(
        x_scale,
        y_scale,
        data,
        config,
        crate::vello2d::RasterBackend::Auto,
    )
}

/// Vello-backed compatibility entry point for legacy `gpu2d::render_contour`.
#[cfg(feature = "vello-gpui")]
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
    let x_scale = x_scale.clone();
    let y_scale = y_scale.clone();
    let config = config.clone();
    crate::vello2d::VelloChartElement::with_builder(move |width, height| {
        crate::shape::contour_chart_scene(&contours, &x_scale, &y_scale, &config, width, height)
    })
    .absolute()
}

/// Vello-backed compatibility entry point for legacy contour-band rendering.
#[cfg(feature = "vello-gpui")]
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
    let x_scale = x_scale.clone();
    let y_scale = y_scale.clone();
    let config = config.clone();
    crate::vello2d::VelloChartElement::with_builder(move |width, height| {
        crate::shape::contour_bands_chart_scene(&bands, &x_scale, &y_scale, &config, width, height)
    })
    .absolute()
}

/// Vello-backed compatibility entry point for legacy heatmap rendering.
#[cfg(feature = "vello-gpui")]
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
    let x_scale = x_scale.clone();
    let y_scale = y_scale.clone();
    let config = config.clone();
    crate::vello2d::VelloChartElement::with_builder(move |width, height| {
        crate::shape::heatmap_chart_scene(&data, &x_scale, &y_scale, &config, width, height)
    })
    .absolute()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axis::{AxisConfig, DefaultAxisTheme};
    use crate::gpu2d::shapes::gpu_axis_theme::GpuAxisTheme;
    use crate::scale::LinearScale;

    #[::core::prelude::v1::test]
    fn test_gpu_render_axis_orientations() {
        let scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
        let theme = GpuAxisTheme::light();

        let _ = render_axis(&scale, &AxisConfig::bottom().with_ticks(5), 400.0, &theme);
        let _ = render_axis(&scale, &AxisConfig::top().with_ticks(5), 400.0, &theme);
        let _ = render_axis(&scale, &AxisConfig::left().with_ticks(5), 300.0, &theme);
        let _ = render_axis(&scale, &AxisConfig::right().with_ticks(5), 300.0, &theme);
    }
}
