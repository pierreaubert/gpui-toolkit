use super::line_config::LineConfig;
use super::line_point::LinePoint;
use super::style::CurveType;
use super::style::StrokeDashArray;
use super::validation::{LineRenderError, compute_line_segments, validate_line_inputs};
use crate::lod::m4_point_indices;
use crate::scale::Scale;
use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;

type LinePaintInputs = (
    Arc<[(f32, f32)]>,
    Arc<[(f32, f32, f32, f32)]>,
    f32,
    f32,
    f32,
    f32,
);

/// Render a line chart after validating data, scale outputs, and config.
pub fn try_render_line<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[LinePoint],
    config: &LineConfig,
) -> Result<impl IntoElement + use<XS, YS>, LineRenderError>
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    validate_line_inputs(x_scale, y_scale, data, config)?;
    Ok(render_line(x_scale, y_scale, data, config))
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
        let x_rel = if x_range_span == 0.0 {
            0.5_f32
        } else {
            ((x_range - x_min) / x_range_span) as f32
        };
        let y_range = y_scale.scale(point.y);
        // y_range is in screen coordinates
        // For inverted range (typical: range(height, 0)), y_min > y_max
        // y_range=0 (top) should map to y_rel=0, y_range=y_min (bottom) should map to y_rel=1
        let y_range_span = y_max - y_min;
        let y_rel = if y_range_span == 0.0 {
            0.5_f32
        } else if y_min > y_max {
            // Inverted range: y_min is at bottom, y_max is at top
            ((y_range - y_max) / (y_min - y_max)) as f32
        } else {
            // Normal range: y_min is at top (0), y_max is at bottom
            ((y_range - y_min) / y_range_span) as f32
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
    let dash_pattern: Option<Vec<f32>> = config.dash_array.as_ref().map(|da| match da {
        StrokeDashArray::Dotted => vec![2.0, 2.0],
        StrokeDashArray::Dashed => vec![6.0, 3.0],
        StrokeDashArray::DashDot => vec![6.0, 3.0, 2.0, 3.0],
        StrokeDashArray::Custom(v) => v.clone(),
    });

    // Pre-compute clipped segments once; they only depend on relative points.
    let relative_points: Arc<[(f32, f32)]> = relative_points.into();
    let segments_to_draw: Arc<[(f32, f32, f32, f32)]> =
        compute_line_segments(&relative_points, curve_type).into();

    canvas(
        // Prepaint: pass through the relative points, pre-computed segments and bounds info
        move |bounds, _window, _cx| {
            let width: f32 = bounds.size.width.into();
            let height: f32 = bounds.size.height.into();
            let origin_x: f32 = bounds.origin.x.into();
            let origin_y: f32 = bounds.origin.y.into();

            let use_lod = curve_type == CurveType::Linear
                && relative_points.len() > width.ceil().max(1.0) as usize * 4;
            let points = if use_lod {
                m4_point_indices(&relative_points, width.ceil().max(1.0) as usize)
                    .into_iter()
                    .map(|index| relative_points[index])
                    .collect::<Vec<_>>()
                    .into()
            } else {
                relative_points.clone()
            };
            let segments = if use_lod {
                compute_line_segments(&points, curve_type).into()
            } else {
                segments_to_draw.clone()
            };

            (
                points,
                segments,
                width,
                height,
                origin_x,
                origin_y,
            )
        },
        // Paint: draw clipped line segments
        move |_bounds,
              (rel_points, segments_to_draw, width, height, origin_x, origin_y): LinePaintInputs,
              window,
              _cx| {
            if segments_to_draw.is_empty() {
                return;
            }

            // Build continuous paths from clipped segments
            if !segments_to_draw.is_empty() {
                let color_with_opacity = Rgba {
                    r: stroke_color.r,
                    g: stroke_color.g,
                    b: stroke_color.b,
                    a: stroke_color.a * opacity,
                };

                if let Some(ref pattern) = dash_pattern {
                    // Dashed line: walk along segments, emitting dash sub-segments
                    let mut path_builder = PathBuilder::stroke(px(stroke_width));
                    let mut pattern_idx = 0usize; // current index into pattern
                    let mut remaining = pattern[0]; // remaining length in current dash/gap
                    let mut has_segments = false;

                    for &(rx0, ry0, rx1, ry1) in segments_to_draw.iter() {
                        let sx = origin_x + rx0 * width;
                        let sy = origin_y + ry0 * height;
                        let ex = origin_x + rx1 * width;
                        let ey = origin_y + ry1 * height;
                        let dx = ex - sx;
                        let dy = ey - sy;
                        let seg_len = (dx * dx + dy * dy).sqrt();
                        if seg_len < 0.001 {
                            continue;
                        }
                        let ux = dx / seg_len;
                        let uy = dy / seg_len;

                        let mut traveled = 0.0f32;
                        while traveled < seg_len {
                            let step = remaining.min(seg_len - traveled);
                            let is_dash = pattern_idx.is_multiple_of(2);

                            if is_dash {
                                let p0x = sx + ux * traveled;
                                let p0y = sy + uy * traveled;
                                let p1x = sx + ux * (traveled + step);
                                let p1y = sy + uy * (traveled + step);
                                path_builder.move_to(gpui::point(px(p0x), px(p0y)));
                                path_builder.line_to(gpui::point(px(p1x), px(p1y)));
                                has_segments = true;
                            }

                            traveled += step;
                            remaining -= step;
                            if remaining < 0.001 {
                                pattern_idx = (pattern_idx + 1) % pattern.len();
                                remaining = pattern[pattern_idx];
                            }
                        }
                    }

                    if has_segments && let Ok(path) = path_builder.build() {
                        window.paint_path(path, color_with_opacity);
                    }
                } else {
                    // Solid line: original path-building logic
                    let mut path_builder = PathBuilder::stroke(px(stroke_width));
                    let mut last_end: Option<(f32, f32)> = None;

                    for (x0, y0, x1, y1) in segments_to_draw.iter() {
                        let start = (origin_x + x0 * width, origin_y + y0 * height);
                        let end = (origin_x + x1 * width, origin_y + y1 * height);

                        let need_move = match last_end {
                            Some((lx, ly)) => {
                                (lx - start.0).abs() > 0.5 || (ly - start.1).abs() > 0.5
                            }
                            None => true,
                        };

                        if need_move {
                            path_builder.move_to(gpui::point(px(start.0), px(start.1)));
                        }
                        path_builder.line_to(gpui::point(px(end.0), px(end.1)));
                        last_end = Some(end);
                    }

                    if let Ok(path) = path_builder.build() {
                        window.paint_path(path, color_with_opacity);
                    }
                }
            }

            // Paint points if enabled (only for points inside the clip region)
            if show_points {
                for &(x_rel, y_rel) in rel_points.iter() {
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
