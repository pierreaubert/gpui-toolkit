use super::bar_config::BarConfig;
use super::bar_datum::BarDatum;
use super::grouped_bar_config::GroupedBarConfig;
use super::grouped_bar_datum::GroupedBarDatum;
use super::types::GroupedBarMeta;
use crate::color::D3Color;
use crate::scale::Scale;
use gpui::prelude::*;
use gpui::*;
use std::collections::BTreeMap;

/// Screen-space quad for a single bar.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct BarQuad {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// A bar quad together with its series color for grouped rendering.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct GroupedBarQuad {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: D3Color,
}

/// Render a bar chart
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::prelude::*;
/// use d3rs::shape::{render_bars, BarConfig, BarDatum};
///
/// let x_scale = LinearScale::new().domain(0.0, 5.0).range(0.0, 400.0);
/// let y_scale = LinearScale::new().domain(0.0, 100.0).range(300.0, 0.0);
///
/// let data = vec![
///     BarDatum::new("A", 50.0),
///     BarDatum::new("B", 80.0),
///     BarDatum::new("C", 30.0),
/// ];
///
/// let config = BarConfig::new().fill_color(D3Color::from_hex(0x4682b4));
/// // render_bars(&x_scale, &y_scale, &data, 400.0, 300.0, &config)
/// ```
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
    let quads = compute_bars(x_scale, y_scale, data, width, height, config);
    let fill = config.fill_color.to_rgba();
    let opacity = config.opacity;
    let stroke = config.stroke_color;
    let stroke_width = config.stroke_width;
    let border_radius = config.border_radius;

    canvas(
        move |_bounds, _window, _cx| {
            // All geometry is pre-computed; just pass it through.
            (quads, fill, stroke)
        },
        move |bounds, (quads, fill, stroke), window, _cx| {
            let origin_x: f32 = bounds.origin.x.into();
            let origin_y: f32 = bounds.origin.y.into();

            // Build one fill path containing every bar rectangle.
            let mut fill_builder = PathBuilder::fill();
            for quad in &quads {
                add_rounded_rect_to_path(
                    &mut fill_builder,
                    origin_x + quad.x,
                    origin_y + quad.y,
                    quad.width,
                    quad.height,
                    border_radius,
                );
            }
            if let Ok(path) = fill_builder.build() {
                let mut fill_color = fill;
                fill_color.a *= opacity;
                window.paint_path(path, fill_color);
            }

            // Build one stroke path if configured.
            if let Some(stroke_color) = stroke {
                let mut stroke_builder = PathBuilder::stroke(px(stroke_width));
                for quad in &quads {
                    let inset = stroke_width;
                    add_rounded_rect_to_path(
                        &mut stroke_builder,
                        origin_x + quad.x - inset,
                        origin_y + quad.y - inset,
                        quad.width + inset * 2.0,
                        quad.height + inset * 2.0,
                        border_radius + inset,
                    );
                }
                if let Ok(path) = stroke_builder.build() {
                    window.paint_path(path, stroke_color.to_rgba());
                }
            }
        },
    )
    .size_full()
    .absolute()
    .inset_0()
}

/// Render bars through the Vello scene painter.
#[cfg(all(feature = "vello-gpui", not(test)))]
pub fn render_bars_vello<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[BarDatum],
    width: f32,
    height: f32,
    config: &BarConfig,
    backend: crate::vello2d::RasterBackend,
) -> impl IntoElement
where
    XS: Scale<f64, f64> + 'static,
    YS: Scale<f64, f64> + 'static,
{
    let quads = compute_bars(x_scale, y_scale, data, width, height, config);
    let fill = config.fill_color.to_rgba();
    let opacity = config.opacity;
    let stroke = config.stroke_color;
    let stroke_width = config.stroke_width as f64;
    crate::vello2d::VelloChartElement::with_builder(move |actual_width, actual_height| {
        use crate::vello2d::kurbo::{BezPath, PathEl, Stroke};
        use crate::vello2d::peniko::{Brush, Color};
        let sx = if width.abs() > f32::EPSILON {
            actual_width / width
        } else {
            1.0
        };
        let sy = if height.abs() > f32::EPSILON {
            actual_height / height
        } else {
            1.0
        };
        let mut scene = crate::vello2d::ChartScene::new();
        let mut fill_path = BezPath::new();
        for quad in &quads {
            let x0 = quad.x * sx;
            let y0 = quad.y * sy;
            let x1 = (quad.x + quad.width) * sx;
            let y1 = (quad.y + quad.height) * sy;
            fill_path.push(PathEl::MoveTo((x0 as f64, y0 as f64).into()));
            fill_path.push(PathEl::LineTo((x1 as f64, y0 as f64).into()));
            fill_path.push(PathEl::LineTo((x1 as f64, y1 as f64).into()));
            fill_path.push(PathEl::LineTo((x0 as f64, y1 as f64).into()));
            fill_path.push(PathEl::ClosePath);
        }
        if !fill_path.is_empty() {
            scene.fill_path(
                fill_path,
                Brush::Solid(Color::new([fill.r, fill.g, fill.b, fill.a * opacity])),
            );
        }
        if let Some(stroke_color) = stroke {
            let mut stroke_path = BezPath::new();
            for quad in &quads {
                let inset = stroke_width as f32;
                let x0 = (quad.x - inset) * sx;
                let y0 = (quad.y - inset) * sy;
                let x1 = (quad.x + quad.width + inset) * sx;
                let y1 = (quad.y + quad.height + inset) * sy;
                stroke_path.push(PathEl::MoveTo((x0 as f64, y0 as f64).into()));
                stroke_path.push(PathEl::LineTo((x1 as f64, y0 as f64).into()));
                stroke_path.push(PathEl::LineTo((x1 as f64, y1 as f64).into()));
                stroke_path.push(PathEl::LineTo((x0 as f64, y1 as f64).into()));
                stroke_path.push(PathEl::ClosePath);
            }
            if !stroke_path.is_empty() {
                let color = stroke_color.to_rgba();
                scene.stroke_path(
                    stroke_path,
                    Stroke::new(stroke_width),
                    Brush::Solid(Color::new([color.r, color.g, color.b, color.a])),
                );
            }
        }
        scene
    })
    .backend(backend)
    .absolute()
}

/// Compute bar quads in a single pass.
pub(super) fn compute_bars<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[BarDatum],
    width: f32,
    height: f32,
    config: &BarConfig,
) -> Vec<BarQuad>
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    let (x_min, x_max) = x_scale.range();
    let (y_min, y_max) = y_scale.range();
    let x_range_span = x_max - x_min;
    let y_range_span = y_max - y_min;

    let bar_count = data.len() as f32;
    let available_width = width - (config.bar_gap * (bar_count - 1.0));
    let bar_width = if bar_count > 0.0 {
        available_width / bar_count
    } else {
        0.0
    };

    let (y_domain_min, y_domain_max) = y_scale.domain();
    let baseline = if y_domain_min <= 0.0 && y_domain_max >= 0.0 {
        y_scale.scale(0.0)
    } else {
        y_scale.scale(y_domain_min)
    };
    let baseline_pos = if y_range_span == 0.0 {
        0.5
    } else {
        1.0 - ((baseline - y_min) / y_range_span) as f32
    };

    data.iter()
        .enumerate()
        .map(|(i, datum)| {
            let x_value = i as f64 + 0.5;
            let x_range = x_scale.scale(x_value);
            let x_pos = if x_range_span == 0.0 {
                0.5
            } else {
                ((x_range - x_min) / x_range_span) as f32
            };

            let y_range = y_scale.scale(datum.value);
            let y_pos = if y_range_span == 0.0 {
                0.5
            } else {
                1.0 - ((y_range - y_min) / y_range_span) as f32
            };

            let bar_height_rel = (baseline_pos - y_pos).abs();
            let bar_height_px = bar_height_rel * height;
            let bar_top = if datum.value >= 0.0 {
                y_pos
            } else {
                baseline_pos
            };
            let bar_top_px = bar_top * height;

            BarQuad {
                x: x_pos * width - bar_width / 2.0,
                y: bar_top_px,
                width: bar_width,
                height: bar_height_px,
            }
        })
        .collect()
}

/// Render a grouped bar chart
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::prelude::*;
/// use d3rs::shape::{render_grouped_bars, GroupedBarConfig, GroupedBarDatum, analyze_grouped_data};
///
/// let data = vec![
///     GroupedBarDatum::new("Q1", "Product A", 50.0),
///     GroupedBarDatum::new("Q1", "Product B", 80.0),
///     GroupedBarDatum::new("Q2", "Product A", 70.0),
///     GroupedBarDatum::new("Q2", "Product B", 60.0),
/// ];
///
/// let meta = analyze_grouped_data(&data);
/// let y_scale = LinearScale::new().domain(0.0, meta.max_value).range(300.0, 0.0);
///
/// let config = GroupedBarConfig::new();
/// // render_grouped_bars(&y_scale, &data, &meta, 400.0, 300.0, &config)
/// ```
pub fn render_grouped_bars<YS>(
    y_scale: &YS,
    data: &[GroupedBarDatum],
    meta: &GroupedBarMeta,
    width: f32,
    height: f32,
    config: &GroupedBarConfig,
) -> impl IntoElement
where
    YS: Scale<f64, f64>,
{
    let quads = compute_grouped_bars(y_scale, data, meta, width, height, config);
    let opacity = config.opacity;
    let stroke = config.stroke_color;
    let stroke_width = config.stroke_width;
    let border_radius = config.border_radius;

    canvas(
        move |_bounds, _window, _cx| quads,
        move |bounds, quads, window, _cx| {
            let origin_x: f32 = bounds.origin.x.into();
            let origin_y: f32 = bounds.origin.y.into();

            // Group consecutive quads by color and emit one path per color group.
            let mut i = 0;
            while i < quads.len() {
                let color = quads[i].color;
                let mut group_end = i + 1;
                while group_end < quads.len() && quads[group_end].color == color {
                    group_end += 1;
                }

                let mut fill_builder = PathBuilder::fill();
                for quad in &quads[i..group_end] {
                    add_rounded_rect_to_path(
                        &mut fill_builder,
                        origin_x + quad.x,
                        origin_y + quad.y,
                        quad.width,
                        quad.height,
                        border_radius,
                    );
                }
                if let Ok(path) = fill_builder.build() {
                    let mut fill_color = color.to_rgba();
                    fill_color.a *= opacity;
                    window.paint_path(path, fill_color);
                }

                if let Some(stroke_color) = stroke {
                    let mut stroke_builder = PathBuilder::stroke(px(stroke_width));
                    for quad in &quads[i..group_end] {
                        let inset = stroke_width;
                        add_rounded_rect_to_path(
                            &mut stroke_builder,
                            origin_x + quad.x - inset,
                            origin_y + quad.y - inset,
                            quad.width + inset * 2.0,
                            quad.height + inset * 2.0,
                            border_radius + inset,
                        );
                    }
                    if let Ok(path) = stroke_builder.build() {
                        window.paint_path(path, stroke_color.to_rgba());
                    }
                }

                i = group_end;
            }
        },
    )
    .size_full()
    .absolute()
    .inset_0()
}

/// Render grouped bars through the Vello scene painter.
#[cfg(all(feature = "vello-gpui", not(test)))]
pub fn render_grouped_bars_vello<YS>(
    y_scale: &YS,
    data: &[GroupedBarDatum],
    meta: &GroupedBarMeta,
    width: f32,
    height: f32,
    config: &GroupedBarConfig,
    backend: crate::vello2d::RasterBackend,
) -> impl IntoElement
where
    YS: Scale<f64, f64> + 'static,
{
    let quads = compute_grouped_bars(y_scale, data, meta, width, height, config);
    let opacity = config.opacity;
    crate::vello2d::VelloChartElement::with_builder(move |actual_width, actual_height| {
        use crate::vello2d::kurbo::{BezPath, PathEl};
        use crate::vello2d::peniko::{Brush, Color};
        let sx = if width.abs() > f32::EPSILON {
            actual_width / width
        } else {
            1.0
        };
        let sy = if height.abs() > f32::EPSILON {
            actual_height / height
        } else {
            1.0
        };
        let mut scene = crate::vello2d::ChartScene::new();
        let mut start = 0usize;
        while start < quads.len() {
            let color = quads[start].color;
            let mut end = start + 1;
            while end < quads.len() && quads[end].color == color {
                end += 1;
            }
            let mut path = BezPath::new();
            for quad in &quads[start..end] {
                let x0 = quad.x * sx;
                let y0 = quad.y * sy;
                let x1 = (quad.x + quad.width) * sx;
                let y1 = (quad.y + quad.height) * sy;
                path.push(PathEl::MoveTo((x0 as f64, y0 as f64).into()));
                path.push(PathEl::LineTo((x1 as f64, y0 as f64).into()));
                path.push(PathEl::LineTo((x1 as f64, y1 as f64).into()));
                path.push(PathEl::LineTo((x0 as f64, y1 as f64).into()));
                path.push(PathEl::ClosePath);
            }
            let rgba = color.to_rgba();
            scene.fill_path(
                path,
                Brush::Solid(Color::new([rgba.r, rgba.g, rgba.b, rgba.a * opacity])),
            );
            start = end;
        }
        scene
    })
    .backend(backend)
    .absolute()
}

/// Compute grouped bar quads in a single pass, pre-computing category/series indices.
pub(super) fn compute_grouped_bars<YS>(
    y_scale: &YS,
    data: &[GroupedBarDatum],
    meta: &GroupedBarMeta,
    width: f32,
    height: f32,
    config: &GroupedBarConfig,
) -> Vec<GroupedBarQuad>
where
    YS: Scale<f64, f64>,
{
    let num_categories = meta.categories.len() as f32;
    let num_series = meta.series.len() as f32;

    if num_categories == 0.0 || num_series == 0.0 {
        return Vec::new();
    }

    let total_group_gaps = config.group_gap * (num_categories - 1.0).max(0.0);
    let available_width = width - total_group_gaps;
    let group_width = available_width / num_categories;

    let total_bar_gaps = config.bar_gap * (num_series - 1.0).max(0.0);
    let available_bar_width = group_width - total_bar_gaps;
    let bar_width = available_bar_width / num_series;

    let category_index: BTreeMap<&str, usize> = meta
        .categories
        .iter()
        .enumerate()
        .map(|(i, c)| (c.as_str(), i))
        .collect();

    let series_index: BTreeMap<&str, usize> = meta
        .series
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();

    let (y_min, y_max) = y_scale.range();
    let y_range_span = y_max - y_min;
    let (y_domain_min, y_domain_max) = y_scale.domain();
    let baseline = if y_domain_min <= 0.0 && y_domain_max >= 0.0 {
        y_scale.scale(0.0)
    } else {
        y_scale.scale(y_domain_min)
    };
    let baseline_pos = if y_range_span == 0.0 {
        0.5
    } else {
        1.0 - ((baseline - y_min) / y_range_span) as f32
    };

    let mut quads: Vec<GroupedBarQuad> = Vec::with_capacity(data.len());
    for datum in data {
        let Some(cat_idx) = category_index.get(datum.category.as_str()) else {
            continue;
        };
        let Some(ser_idx) = series_index.get(datum.series.as_str()) else {
            continue;
        };

        let group_start = *cat_idx as f32 * (group_width + config.group_gap);
        let bar_offset = *ser_idx as f32 * (bar_width + config.bar_gap);
        let x_pos = group_start + bar_offset;

        let y_range = y_scale.scale(datum.value);
        let y_pos = if y_range_span == 0.0 {
            0.5
        } else {
            1.0 - ((y_range - y_min) / y_range_span) as f32
        };

        let bar_height_rel = (baseline_pos - y_pos).abs();
        let bar_height_px = bar_height_rel * height;
        let bar_top = if datum.value >= 0.0 {
            y_pos
        } else {
            baseline_pos
        };
        let bar_top_px = bar_top * height;

        let color = config.get_series_color(*ser_idx);

        quads.push(GroupedBarQuad {
            x: x_pos,
            y: bar_top_px,
            width: bar_width,
            height: bar_height_px,
            color,
        });
    }

    // Sort by color so the renderer can batch all bars of the same series/color
    // into a single path draw.
    quads.sort_by(|a, b| {
        let a_key = (
            a.color.r.to_bits(),
            a.color.g.to_bits(),
            a.color.b.to_bits(),
            a.color.a.to_bits(),
        );
        let b_key = (
            b.color.r.to_bits(),
            b.color.g.to_bits(),
            b.color.b.to_bits(),
            b.color.a.to_bits(),
        );
        a_key.cmp(&b_key)
    });

    quads
}

/// Append a rounded rectangle outline to a path builder.
fn add_rounded_rect_to_path(
    builder: &mut PathBuilder,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
) {
    if radius <= 0.0 || width <= 0.0 || height <= 0.0 {
        builder.move_to(point(px(x), px(y)));
        builder.line_to(point(px(x + width), px(y)));
        builder.line_to(point(px(x + width), px(y + height)));
        builder.line_to(point(px(x), px(y + height)));
        builder.line_to(point(px(x), px(y)));
        return;
    }

    let r = radius.min(width / 2.0).min(height / 2.0);
    builder.move_to(point(px(x + r), px(y)));
    builder.line_to(point(px(x + width - r), px(y)));
    // Top-right corner
    builder.curve_to(point(px(x + width), px(y + r)), point(px(x + width), px(y)));
    builder.line_to(point(px(x + width), px(y + height - r)));
    // Bottom-right corner
    builder.curve_to(
        point(px(x + width - r), px(y + height)),
        point(px(x + width), px(y + height)),
    );
    builder.line_to(point(px(x + r), px(y + height)));
    // Bottom-left corner
    builder.curve_to(
        point(px(x), px(y + height - r)),
        point(px(x), px(y + height)),
    );
    builder.line_to(point(px(x), px(y + r)));
    // Top-left corner
    builder.curve_to(point(px(x + r), px(y)), point(px(x), px(y)));
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::LinearScale;

    #[::core::prelude::v1::test]
    fn render_bars_batches_by_color() {
        let x_scale = LinearScale::new().domain(0.0, 3.0).range(0.0, 300.0);
        let y_scale = LinearScale::new().domain(0.0, 100.0).range(300.0, 0.0);
        let data = vec![
            BarDatum::new("A", 50.0),
            BarDatum::new("B", 80.0),
            BarDatum::new("C", 30.0),
        ];
        let config = BarConfig::new().fill_color(D3Color::from_hex(0x4682b4));

        let quads = compute_bars(&x_scale, &y_scale, &data, 300.0, 300.0, &config);

        assert_eq!(
            quads.len(),
            data.len(),
            "all bars should be precomputed in one pass"
        );
        assert!(quads.iter().all(|q| q.width > 0.0 && q.height > 0.0));
    }

    #[::core::prelude::v1::test]
    fn render_grouped_bars_batches_by_color() {
        let data = vec![
            GroupedBarDatum::new("Q1", "A", 50.0),
            GroupedBarDatum::new("Q1", "B", 80.0),
            GroupedBarDatum::new("Q2", "A", 70.0),
            GroupedBarDatum::new("Q2", "B", 60.0),
        ];
        let meta = analyze_grouped_data(&data);
        let y_scale = LinearScale::new()
            .domain(0.0, meta.max_value)
            .range(300.0, 0.0);
        let config = GroupedBarConfig::new();

        let quads = compute_grouped_bars(&y_scale, &data, &meta, 400.0, 300.0, &config);

        assert_eq!(quads.len(), data.len());

        // Count distinct colors. There are two series, so at most two colors.
        let mut distinct_colors: Vec<D3Color> = quads.iter().map(|q| q.color).collect();
        distinct_colors.sort_by(|a, b| {
            let a_key = (a.r.to_bits(), a.g.to_bits(), a.b.to_bits(), a.a.to_bits());
            let b_key = (b.r.to_bits(), b.g.to_bits(), b.b.to_bits(), b.a.to_bits());
            a_key.cmp(&b_key)
        });
        distinct_colors.dedup_by(|a, b| {
            a.r.to_bits() == b.r.to_bits()
                && a.g.to_bits() == b.g.to_bits()
                && a.b.to_bits() == b.b.to_bits()
                && a.a.to_bits() == b.a.to_bits()
        });
        assert_eq!(distinct_colors.len(), 2);

        // Bars with the same series/color should be adjacent in the output because the
        // data is ordered by category and each category contains both series.
        // Verify the batching helper keeps series-color quads groupable.
        let mut i = 0;
        let mut batch_count = 0;
        while i < quads.len() {
            let color = quads[i].color;
            let mut j = i + 1;
            while j < quads.len() && quads[j].color == color {
                j += 1;
            }
            batch_count += 1;
            i = j;
        }
        assert_eq!(
            batch_count, 2,
            "all bars of the same color should form one batch"
        );
    }
}
*/
