//! Generic renderers for every spinorama.org graph family.
//!
//! Frequency charts support arbitrary axis titles/ticks, log or linear
//! x axes, filled traces, a secondary y axis, markers, and annotations.
//! Polar charts draw every scatterpolar trace (no frequency cap) and
//! categorical spider traces. Bar charts draw grouped categorical bars
//! with scatter overlays (histograms and `filters_eq`).

// Rust guideline compliant 2026-02-21

use std::collections::HashSet;

use d3rs::axis::{AxisConfig, render_axis};
use d3rs::grid::{GridConfig, render_grid};
use d3rs::scale::{LinearScale, LogScale, Scale};
use d3rs::shape::{CurveType, LineConfig, LinePoint, StrokeDashArray, render_line};
use d3rs::text::{GlyphTextConfig, render_glyph_text};
use gpui::prelude::*;
use gpui::{Div, Hsla, PathBuilder, canvas, div, point, px};
use gpui_ui_kit::theme::{Theme, ThemeExt};

use crate::types::{
    GenericTrace, GraphDef, PlotData, PlotLayout, TraceFill, TraceKind, TraceMode,
};
use crate::utils::ChartTheme;

fn dash_for(name: Option<&str>) -> Option<StrokeDashArray> {
    match name {
        Some("dash") => Some(StrokeDashArray::Dashed),
        Some("dot") => Some(StrokeDashArray::Dotted),
        Some("dashdot") => Some(StrokeDashArray::DashDot),
        Some("longdash") => Some(StrokeDashArray::Custom(vec![12.0, 4.0])),
        Some("longdashdot") => Some(StrokeDashArray::Custom(vec![12.0, 4.0, 2.0, 4.0])),
        _ => None,
    }
}

fn nice_step(range: f64) -> f64 {
    let raw = range / 5.0;
    if raw <= 0.0 {
        return 1.0;
    }
    let mag = 10f64.powf(raw.log10().floor());
    for step in [1.0, 2.0, 5.0, 10.0] {
        if mag * step >= raw {
            return mag * step;
        }
    }
    mag * 10.0
}

fn ticks(domain: (f64, f64), step: f64) -> Vec<f64> {
    if step <= 0.0 {
        return Vec::new();
    }
    let start = (domain.0 / step).ceil() as i64;
    let end = (domain.1 / step).floor() as i64;
    (start..=end).map(|i| i as f64 * step).collect()
}

fn fmt0(v: f64) -> String {
    format!("{v:.0}")
}
fn fmt1(v: f64) -> String {
    format!("{v:.1}")
}
fn fmt2(v: f64) -> String {
    format!("{v:.2}")
}

fn formatter_for(domain: (f64, f64)) -> fn(f64) -> String {
    let span = (domain.1 - domain.0).abs();
    if span <= 2.0 {
        fmt2
    } else if span <= 20.0 {
        fmt1
    } else {
        fmt0
    }
}

fn domain_for(
    layout_range: Option<(f64, f64)>,
    log: bool,
    extent: Option<(f64, f64)>,
    fallback: (f64, f64),
) -> (f64, f64) {
    if let Some((a, b)) = layout_range {
        // Plotly log axes store exponents (e.g. [1.3, 4.3] for 20 Hz–20 kHz).
        if log && a.abs() < 10.0 && b.abs() < 10.0 {
            return (10f64.powf(a), 10f64.powf(b));
        }
        if a < b {
            return (a, b);
        }
    }
    if let Some((a, b)) = extent {
        if log {
            return (a.max(1.0), b);
        }
        let pad = ((b - a) * 0.05).max(0.5);
        return (a - pad, b + pad);
    }
    fallback
}

fn grid_config(plot: &PlotData, log_x: bool, x_domain: (f64, f64)) -> GridConfig {
    // Thin grid lines, no dots at intersections (Python layout style).
    let mut config = GridConfig::with_lines().with_dots(false);
    if !plot.layout.x_axis.tick_values.is_empty() {
        config = config.with_vertical_values(plot.layout.x_axis.tick_values.clone());
    } else if log_x {
        // Grid at every log tick, 2..9 per decade (Plotly dtick="D1").
        let log_grid: Vec<f64> = (0..=3)
            .flat_map(|decade| {
                let base = 10f64.powi(decade + 1);
                (1..=9).map(move |m| f64::from(m) * base)
            })
            .filter(|f| *f >= x_domain.0 && *f <= x_domain.1)
            .collect();
        config = config.with_vertical_values(log_grid);
    }
    if !plot.layout.y_axis.tick_values.is_empty() {
        config = config.with_horizontal_values(plot.layout.y_axis.tick_values.clone());
    }
    config
}

/// Renders a generic frequency chart from a parsed Plotly payload.
pub fn render_generic_freq_chart(
    plot: &PlotData,
    def: &GraphDef,
    hidden: &HashSet<String>,
    chart_width: f32,
    chart_height: f32,
    ui_theme: &Theme,
) -> Div {
    let theme = ChartTheme::from_theme(ui_theme);
    let scale = (chart_width / 800.0).clamp(0.7, 1.2);
    let label_size = (10.0 * scale).round();
    let title_size = (12.0 * scale).round();
    let tick_size = (6.0 * scale).round().max(4.0);

    let log_x = plot.layout.x_axis.log
        || plot
            .traces
            .iter()
            .any(|t| t.kind == TraceKind::Scatter && t.x.iter().any(|&v| v > 100.0));
    let x_domain = domain_for(
        plot.layout.x_axis.range,
        log_x,
        plot.extent(false, false),
        (20.0, 20_000.0),
    );
    let y_domain = domain_for(
        plot.layout.y_axis.range,
        false,
        plot.extent(false, true),
        (-40.0, 10.0),
    );
    let y2_domain = domain_for(
        plot.layout.y_axis2.as_ref().and_then(|a| a.range),
        false,
        plot.extent(true, true),
        y_domain,
    );
    let has_secondary = plot.layout.y_axis2.is_some()
        && plot
            .traces
            .iter()
            .any(|t| t.secondary_axis && t.visible);
    let y_title: &str = if def.y_title.is_empty() {
        if plot.layout.y_axis.title.is_empty() {
            "SPL (dB)"
        } else {
            plot.layout.y_axis.title.as_str()
        }
    } else {
        def.y_title
    };
    let y_step = if def.y_step > 0.0 {
        def.y_step
    } else if y_domain.1 - y_domain.0 <= 60.0 {
        // SPL graphs: a label every 5 dB, as in the Python layout.
        5.0
    } else {
        nice_step(y_domain.1 - y_domain.0)
    };
    let y_ticks = ticks(y_domain, y_step);
    let fmt = formatter_for(y_domain);
    // `chart_width` is the total width including the Y axes (callers
    // pass the available content width). The plot area is what remains;
    // using the total for the plot area makes the chart row overflow
    // and flex-shrink out of sync with the bottom-axis row, drifting
    // the frequency labels away from the grid and curves.
    let left_axis_width = AxisConfig::left()
        .with_title(y_title)
        .with_label_font_size(label_size)
        .with_title_font_size(title_size)
        .with_tick_size(tick_size)
        .with_title_padding(super::AXIS_TITLE_PADDING)
        .total_size();
    // Always reserve the DI axis width (see freq_spl_plot_margins):
    // every frequency graph gets the same plot width.
    let right_axis_width = AxisConfig::right()
        .with_title("DI (dB)")
        .with_label_font_size(label_size)
        .with_title_font_size(title_size)
        .with_tick_size(tick_size)
        .with_title_padding(super::AXIS_TITLE_PADDING)
        .total_size();
    let plot_width = (chart_width - left_axis_width - right_axis_width).max(1.0);

    // Fill polygons (pixel space) painted under the lines.
    let x_to_px = |v: f64| -> f32 {
        if log_x {
            let (lo, hi) = (x_domain.0.ln(), x_domain.1.ln());
            ((v.ln() - lo) / (hi - lo)) as f32 * plot_width
        } else {
            ((v - x_domain.0) / (x_domain.1 - x_domain.0)) as f32 * plot_width
        }
    };
    let y_to_px = |v: f64| -> f32 {
        ((y_domain.1 - v) / (y_domain.1 - y_domain.0)) as f32 * chart_height
    };
    type FillPoly = (Vec<(f32, f32)>, gpui::Rgba, f32);
    let mut fills: Vec<FillPoly> = Vec::new();
    let visible_scatter: Vec<&GenericTrace> = plot
        .traces
        .iter()
        .filter(|t| t.visible && t.kind == TraceKind::Scatter && !hidden.contains(&t.name))
        .collect();
    for (idx, trace) in visible_scatter.iter().enumerate() {
        let Some(fill) = trace.fill else { continue };
        let color = trace.fill_color.unwrap_or(trace.color).to_rgba();
        let mut poly: Vec<(f32, f32)> = trace
            .points()
            .iter()
            .map(|&(x, y)| (x_to_px(x), y_to_px(y)))
            .collect();
        match fill {
            TraceFill::ToSelf => {}
            TraceFill::ToZeroY => {
                if let (Some(&(fx, _)), Some(&(lx, _))) = (poly.first(), poly.last()) {
                    poly.push((lx, y_to_px(0.0_f64.clamp(y_domain.0, y_domain.1))));
                    poly.push((fx, y_to_px(0.0_f64.clamp(y_domain.0, y_domain.1))));
                }
            }
            TraceFill::ToNextY => {
                if idx > 0 {
                    let prev: Vec<(f32, f32)> = visible_scatter[idx - 1]
                        .points()
                        .iter()
                        .rev()
                        .map(|&(x, y)| (x_to_px(x), y_to_px(y)))
                        .collect();
                    poly.extend(prev);
                }
            }
        }
        // Fill traces are closed data polygons (e.g. the ±3 dB zone is
        // built from frequencies beyond the axis range and a sloped
        // regression line); GPUI does not clip canvas paint to the chart
        // area, so clip here or the zone spills past the axes instead of
        // reading as a rectangle inside the plot.
        let poly = clip_poly_to_rect(&poly, plot_width, chart_height);
        if poly.len() >= 3 {
            fills.push((poly, color, trace.opacity));
        }
    }

    let fill_canvas = canvas(
        move |_b, _w, _c| fills.clone(),
        move |bounds, polys, window, _cx| {
            let ox: f32 = bounds.origin.x.into();
            let oy: f32 = bounds.origin.y.into();
            for (poly, color, opacity) in &polys {
                let mut builder = PathBuilder::fill();
                for (i, &(x, y)) in poly.iter().enumerate() {
                    if i == 0 {
                        builder.move_to(point(px(ox + x), px(oy + y)));
                    } else {
                        builder.line_to(point(px(ox + x), px(oy + y)));
                    }
                }
                builder.close();
                if let Ok(path) = builder.build() {
                    let mut c = *color;
                    c.a *= opacity;
                    window.paint_path(path, c);
                }
            }
        },
    )
    .absolute()
    .inset_0();

    // 0 dB reference line: black, slightly thicker than the grid,
    // painted above the grid but below the curves.
    let make_zero_line = || {
        (!plot.layout.hide_zero_line && y_domain.0 <= 0.0 && 0.0 <= y_domain.1).then(|| {
            let zero_y = ((y_domain.1 / (y_domain.1 - y_domain.0)) as f32) * chart_height;
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px(zero_y - 0.75))
                .h(px(1.5))
                .bg(gpui::black())
        })
    };
    let chart_area = div()
        .w(px(plot_width))
        .h(px(chart_height))
        .relative()
        .bg(ui_theme.surface)
        .child(fill_canvas);
    let chart_area = if log_x {
        let xs = LogScale::new()
            .domain(x_domain.0, x_domain.1)
            .range(0.0, f64::from(plot_width));
        let ys = LinearScale::new()
            .domain(y_domain.0, y_domain.1)
            .range(f64::from(chart_height), 0.0);
        let ys2 = LinearScale::new()
            .domain(y2_domain.0, y2_domain.1)
            .range(f64::from(chart_height), 0.0);
        let area = chart_area.child(render_grid(
            &xs,
            &ys,
            &grid_config(plot, log_x, x_domain),
            plot_width,
            chart_height,
            &theme,
        ));
        let area = if let Some(line) = make_zero_line() {
            area.child(line)
        } else {
            area
        };
        add_lines(area, &visible_scatter, &xs, &ys, &ys2, has_secondary)
    } else {
        let xs = LinearScale::new()
            .domain(x_domain.0, x_domain.1)
            .range(0.0, f64::from(plot_width));
        let ys = LinearScale::new()
            .domain(y_domain.0, y_domain.1)
            .range(f64::from(chart_height), 0.0);
        let ys2 = LinearScale::new()
            .domain(y2_domain.0, y2_domain.1)
            .range(f64::from(chart_height), 0.0);
        let area = chart_area.child(render_grid(
            &xs,
            &ys,
            &grid_config(plot, log_x, x_domain),
            plot_width,
            chart_height,
            &theme,
        ));
        let area = if let Some(line) = make_zero_line() {
            area.child(line)
        } else {
            area
        };
        add_lines(area, &visible_scatter, &xs, &ys, &ys2, has_secondary)
    };
    // Annotations as positioned glyph text.
    let annotation_layer = div()
        .absolute()
        .inset_0()
        .children(plot.layout.annotations.iter().filter_map(|a| {
            if a.x < x_domain.0 || a.x > x_domain.1 || a.y < y_domain.0 || a.y > y_domain.1 {
                return None;
            }
            let cfg = GlyphTextConfig::horizontal(
                (9.0 * scale).round(),
                Hsla::from(ui_theme.text_secondary),
            );
            Some(
                div()
                    .absolute()
                    .left(px(x_to_px(a.x)))
                    .top(px(y_to_px(a.y) - 12.0))
                    .child(render_glyph_text(&a.text, &cfg)),
            )
        }));
    let chart_area = chart_area.child(annotation_layer);

    let left_axis = AxisConfig::left()
        .with_ticks_inside(true)
        .with_tick_values(y_ticks.clone())
        .with_formatter(fmt)
        .with_title(y_title)
        .with_label_font_size(label_size)
        .with_title_font_size(title_size)
        .with_tick_size(tick_size)
        .with_title_padding(super::AXIS_TITLE_PADDING);
    let y_scale_axis = LinearScale::new()
        .domain(y_domain.0, y_domain.1)
        .range(f64::from(chart_height), 0.0);

    let mut bottom_ticks: Vec<f64> = if log_x {
        [
            20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0,
        ]
        .iter()
        .copied()
        .filter(|&f| f >= x_domain.0 && f <= x_domain.1)
        .collect()
    } else if !plot.layout.x_axis.tick_values.is_empty() {
        plot.layout.x_axis.tick_values.clone()
    } else {
        ticks(x_domain, nice_step(x_domain.1 - x_domain.0))
    };
    if bottom_ticks.is_empty() {
        bottom_ticks = ticks(x_domain, nice_step(x_domain.1 - x_domain.0));
    }

    let mut top = div()
        .flex()
        .items_start()
        .child(render_axis(&y_scale_axis, &left_axis, chart_height, &theme))
        .child(chart_area);
    if has_secondary {
        let y2_scale = LinearScale::new()
            .domain(y2_domain.0, y2_domain.1)
            .range(f64::from(chart_height), 0.0);
        let title2 = plot
            .layout
            .y_axis2
            .as_ref()
            .map(|a| a.title.clone())
            .unwrap_or_default();
        // Title must be 'static for AxisConfig; leak is avoided by using
        // the known secondary titles, falling back to "DI (dB)".
        let title2: &'static str = match title2.as_str() {
            "" => "DI (dB)",
            other => {
                if other.starts_with("Phase") {
                    "Phase (deg)"
                } else if other.contains("DI") {
                    "DI (dB)"
                } else {
                    "Secondary"
                }
            }
        };
        top = top.child(render_axis(
            &y2_scale,
            &AxisConfig::right()
                .with_ticks_inside(true)
                .with_tick_values(ticks(y2_domain, if title2 == "Phase (deg)" { 90.0 } else { nice_step(y2_domain.1 - y2_domain.0) }))
                .with_formatter(fmt)
                .with_title(title2)
                .with_label_font_size(label_size)
                .with_title_font_size(title_size)
                .with_tick_size(tick_size)
                .with_title_padding(super::AXIS_TITLE_PADDING),
            chart_height,
            &theme,
        ));
    }

    let x_title: &'static str = if log_x || plot.layout.x_axis.title.contains("Hz") || plot.layout.x_axis.title.is_empty() {
        "Frequency (Hz)"
    } else {
        "X"
    };
    div()
        .flex()
        .flex_col()
        .child(top)
        .child(
            div()
                .flex()
                .child(div().flex_none().w(px(left_axis_width)))
                .child(if log_x {
                    let xs = LogScale::new()
                        .domain(x_domain.0, x_domain.1)
                        .range(0.0, f64::from(plot_width));
                    render_axis(
                        &xs,
                        &AxisConfig::bottom()
                            .with_ticks_inside(true)
                            .with_tick_values(bottom_ticks)
                            .with_minor_tick_values(
                                (0..=3)
                                    .flat_map(|decade| {
                                        let base = 10f64.powi(decade + 1);
                                        (2..=9).map(move |m| f64::from(m) * base)
                                    })
                                    .filter(|f| *f >= x_domain.0 && *f <= x_domain.1)
                                    .collect::<Vec<f64>>(),
                            )
                            .with_formatter(|f| {
                                if f >= 1000.0 {
                                    format!("{:.0}k", f / 1000.0)
                                } else {
                                    format!("{f:.0}")
                                }
                            })
                            .with_title(x_title)
                            .with_label_font_size(label_size)
                            .with_title_font_size(title_size)
                            .with_tick_size(tick_size),
                        plot_width,
                        &theme,
                    )
                    .into_any_element()
                } else {
                    let xs = LinearScale::new()
                        .domain(x_domain.0, x_domain.1)
                        .range(0.0, f64::from(plot_width));
                    render_axis(
                        &xs,
                        &AxisConfig::bottom()
                            .with_ticks_inside(true)
                            .with_tick_values(bottom_ticks)
                            .with_formatter(fmt)
                            .with_title(x_title)
                            .with_label_font_size(label_size)
                            .with_title_font_size(title_size)
                            .with_tick_size(tick_size),
                        plot_width,
                        &theme,
                    )
                    .into_any_element()
                }),
        )
}

fn add_lines<XS: Scale<f64, f64>, YS: Scale<f64, f64>>(
    mut area: Div,
    traces: &[&GenericTrace],
    xs: &XS,
    ys: &YS,
    ys2: &YS,
    has_secondary: bool,
) -> Div {
    for trace in traces {
        if trace.mode == TraceMode::None {
            continue;
        }
        // Null/NaN samples break wrapped phase traces into finite runs.
        let points: Vec<LinePoint> = trace.x.iter().zip(&trace.y)
            .map(|(&x, &y)| LinePoint::new(x, y)).collect();
        if points.is_empty() {
            continue;
        }
        let mut config = LineConfig::new()
            .stroke_color(trace.color)
            .stroke_width(trace.stroke_width)
            .curve(CurveType::Linear);
        if let Some(dash) = dash_for(trace.dash.as_deref()) {
            config = config.dash_array(dash);
        }
        if matches!(trace.mode, TraceMode::Markers | TraceMode::LinesMarkers) {
            config = config
                .show_points(true)
                .point_radius((trace.marker_size / 2.0).max(1.5))
                .point_symbol(trace.marker_symbol.unwrap_or_default());
        }
        let scale_y: &YS = if trace.secondary_axis && has_secondary {
            ys2
        } else {
            ys
        };
        for segment in points.split(|p| !p.x.is_finite() || !p.y.is_finite()) {
            if !segment.is_empty() {
                area = area.child(super::render_marked_line(xs, scale_y, segment, &config));
            }
        }
    }
    area
}

/// Formats a radial tick value without trailing zeros (`0.5`, `-30`).
fn format_radial_value(value: f64) -> String {
    if value == value.trunc() && value.abs() < 1e15 {
        #[expect(clippy::cast_possible_truncation, reason = "checked integral and bounded")]
        let int = value as i64;
        format!("{int}")
    } else {
        format!("{value:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

/// Renders a generic polar chart: every scatterpolar trace, numeric or
/// categorical (spider). No trace-count cap.
pub fn render_generic_polar(
    plot: &PlotData,
    hidden: &HashSet<String>,
    chart_size: f32,
    ui_theme: &Theme,
) -> Div {
    // Category and radial labels scale with the chart so published
    // pictures (rendered large) keep readable text.
    let label_px = (chart_size * 0.028).clamp(15.0, 26.0);
    let radial_px = label_px * 0.78;
    let center = chart_size / 2.0;
    let outer = (chart_size / 2.0 - label_px * 3.4).max(40.0);
    let traces: Vec<&GenericTrace> = plot
        .traces
        .iter()
        .filter(|t| {
            t.visible
                && matches!(
                    t.kind,
                    TraceKind::ScatterPolar | TraceKind::ScatterPolarCategorical
                )
                && !hidden.contains(&t.name)
        })
        .collect();
    let categorical = traces
        .first()
        .is_some_and(|t| t.kind == TraceKind::ScatterPolarCategorical);
    let categories: Vec<String> = if categorical {
        traces
            .first()
            .map(|t| t.categories.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let (r_min, r_max) = plot.layout.polar_radial_range.unwrap_or_else(|| {
        let mut lo = 0.0f64;
        let mut hi = 1.0f64;
        for t in &traces {
            for &v in &t.r {
                if v.is_finite() {
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
            }
        }
        if hi <= lo {
            (lo, lo + 1.0)
        } else {
            (lo, hi)
        }
    });
    type PolarSeries = (Vec<(f32, f32)>, gpui::Rgba, f32);
    let mut series: Vec<PolarSeries> = Vec::new();
    for trace in &traces {
        let pts: Vec<(f32, f32)> = if categorical {
            let n = categories.len().max(1) as f64;
            trace
                .r
                .iter()
                .enumerate()
                .filter(|(_, v)| v.is_finite())
                .map(|(i, &v)| {
                    let ang = (i as f64 / n) * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
                    let radius = ((v - r_min) / (r_max - r_min)).clamp(0.0, 1.0) as f32 * outer;
                    (
                        center + radius * ang.cos() as f32,
                        center + radius * ang.sin() as f32,
                    )
                })
                .collect()
        } else {
            trace
                .polar_points()
                .iter()
                .map(|&(deg, v)| {
                    let ang = (deg - 90.0).to_radians();
                    let radius = ((v - r_min) / (r_max - r_min)).clamp(0.0, 1.0) as f32 * outer;
                    (
                        center + radius * ang.cos() as f32,
                        center + radius * ang.sin() as f32,
                    )
                })
                .collect()
        };
        if pts.len() >= 2 {
            series.push((pts, trace.color.to_rgba(), trace.stroke_width));
        }
    }
    let grid_radii: Vec<f32> = [0.25, 0.5, 0.75, 1.0]
        .iter()
        .map(|&t| t * outer)
        .collect();
    let spokes: Vec<f64> = if categorical && !categories.is_empty() {
        (0..categories.len())
            .map(|i| (i as f64 / categories.len() as f64) * std::f64::consts::TAU)
            .collect()
    } else {
        (0..12).map(|i| f64::from(i) * 30.0_f64.to_radians()).collect()
    };
    // Radial scale: grid circles carry value labels along the top
    // spoke. Spiders use the grid fractions of their (usually
    // normalised) range; numeric radars get nice round steps.
    let radial_ticks: Vec<(f32, String)> = if categorical {
        [0.25, 0.5, 0.75, 1.0]
            .iter()
            .map(|&t| {
                let value = r_min + t * (r_max - r_min);
                #[expect(clippy::cast_possible_truncation, reason = "radius fits in f32")]
                let radius = (t * f64::from(outer)) as f32;
                (radius, format_radial_value(value))
            })
            .collect()
    } else {
        let span = r_max - r_min;
        let step = [1.0, 2.0, 5.0, 10.0, 20.0, 25.0, 50.0]
            .into_iter()
            .find(|step| span / step <= 5.0)
            .unwrap_or(50.0);
        let mut ticks = Vec::new();
        let mut value = (r_min / step).ceil() * step;
        while value <= r_max + 1e-9 {
            if value > r_min {
                #[expect(clippy::cast_possible_truncation, reason = "radius fits in f32")]
                let radius = ((value - r_min) / span) as f32 * outer;
                ticks.push((radius, format_radial_value(value)));
            }
            value += step;
        }
        ticks
    };

    let polar_canvas = canvas(
        move |_b, _w, _c| (series.clone(), grid_radii.clone(), spokes.clone()),
        move |bounds, (series, radii, spokes), window, cx| {
            let theme = cx.theme();
            let grid = Hsla::from(theme.border);
            let ox: f32 = bounds.origin.x.into();
            let oy: f32 = bounds.origin.y.into();
            for &r in &radii {
                let mut b = PathBuilder::stroke(px(1.0));
                for i in 0..=72 {
                    let a = (i as f32 / 72.0) * std::f32::consts::TAU;
                    let (x, y) = (ox + center + r * a.cos(), oy + center + r * a.sin());
                    if i == 0 {
                        b.move_to(point(px(x), px(y)));
                    } else {
                        b.line_to(point(px(x), px(y)));
                    }
                }
                if let Ok(p) = b.build() {
                    window.paint_path(p, grid);
                }
            }
            for &a in &spokes {
                let mut b = PathBuilder::stroke(px(1.0));
                b.move_to(point(px(ox + center), px(oy + center)));
                b.line_to(point(
                    px(ox + center + outer * a.cos() as f32),
                    px(oy + center + outer * a.sin() as f32),
                ));
                if let Ok(p) = b.build() {
                    window.paint_path(p, grid);
                }
            }
            for (pts, color, width) in &series {
                let mut b = PathBuilder::stroke(px(*width));
                for (i, &(x, y)) in pts.iter().enumerate() {
                    if i == 0 {
                        b.move_to(point(px(ox + x), px(oy + y)));
                    } else {
                        b.line_to(point(px(ox + x), px(oy + y)));
                    }
                }
                if let Some(&(x, y)) = pts.first() {
                    b.line_to(point(px(ox + x), px(oy + y)));
                }
                if let Ok(p) = b.build() {
                    window.paint_path(p, *color);
                }
            }
        },
    )
    .w(px(chart_size))
    .h(px(chart_size));

    let label_cfg = GlyphTextConfig::horizontal(label_px, Hsla::from(ui_theme.text_primary));
    let radial_cfg = GlyphTextConfig::horizontal(radial_px, Hsla::from(ui_theme.text_secondary));
    // Glyph advance is roughly half the font size; centre each label
    // on its spoke point with that estimate.
    let centred = |text: &str, x: f32, y: f32, font: f32| {
        #[expect(clippy::cast_precision_loss, reason = "label length fits in f32")]
        let est_width = text.chars().count() as f32 * font * 0.52;
        (x - est_width / 2.0, y - font * 0.55)
    };
    let mut label_divs: Vec<Div> = if categorical {
        categories
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let ang =
                    (i as f64 / categories.len().max(1) as f64) * std::f64::consts::TAU
                        - std::f64::consts::FRAC_PI_2;
                let r = outer + label_px * 1.1;
                let (left, top) = centred(
                    label,
                    center + r * ang.cos() as f32,
                    center + r * ang.sin() as f32,
                    label_px,
                );
                div()
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .child(render_glyph_text(label, &label_cfg))
            })
            .collect()
    } else {
        (0..12)
            .map(|i| {
                let deg = f64::from(i) * 30.0;
                let ang = (deg - 90.0).to_radians();
                let r = outer + label_px * 0.9;
                let shown = if deg <= 180.0 { deg as i32 } else { deg as i32 - 360 };
                let text = format!("{shown}°");
                let (left, top) = centred(
                    &text,
                    center + r * ang.cos() as f32,
                    center + r * ang.sin() as f32,
                    label_px,
                );
                div()
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .child(render_glyph_text(&text, &label_cfg))
            })
            .collect()
    };
    // Radial scale labels along the top spoke, right of the line.
    for (radius, text) in &radial_ticks {
        label_divs.push(
            div()
                .absolute()
                .left(px(center + radial_px * 0.35))
                .top(px(center - radius - radial_px * 0.55))
                .child(render_glyph_text(text, &radial_cfg)),
        );
    }
    let labels = div().absolute().inset_0().children(label_divs);

    div()
        .relative()
        .w(px(chart_size))
        .h(px(chart_size))
        .child(polar_canvas)
        .child(labels)
}

/// Renders grouped categorical bars with optional scatter overlays.
///
/// Categories are the union of bar-trace categories in payload order;
/// each bar trace is one series. Scatter traces (e.g. in `filters_eq`)
/// are drawn as lines over the category index axis.
pub fn render_generic_bars(
    plot: &PlotData,
    hidden: &HashSet<String>,
    chart_width: f32,
    chart_height: f32,
    ui_theme: &Theme,
) -> Div {
    let theme = ChartTheme::from_theme(ui_theme);
    let scale = (chart_width / 800.0).clamp(0.7, 1.2);
    let label_size = (10.0 * scale).round();
    let title_size = (12.0 * scale).round();
    let tick_size = (6.0 * scale).round().max(4.0);

    let bar_traces: Vec<&GenericTrace> = plot
        .traces
        .iter()
        .filter(|t| t.visible && t.kind == TraceKind::Bar && !hidden.contains(&t.name))
        .collect();
    let mut categories: Vec<String> = Vec::new();
    for trace in &bar_traces {
        for cat in &trace.categories {
            if !categories.contains(cat) {
                categories.push(cat.clone());
            }
        }
    }
    if categories.is_empty() {
        categories = vec![String::new()];
    }
    let (mut y_min, mut y_max) = (0.0f64, 1.0f64);
    for trace in &bar_traces {
        for &v in &trace.y {
            if v.is_finite() {
                y_min = y_min.min(v);
                y_max = y_max.max(v);
            }
        }
    }
    if let Some((a, b)) = plot.layout.y_axis.range {
        y_min = a;
        y_max = b;
    }
    if y_max <= y_min {
        y_max = y_min + 1.0;
    }
    let slot = chart_width / categories.len() as f32;
    let series_n = bar_traces.len().max(1) as f32;
    let bar_w = (slot * 0.8 / series_n).max(1.0);
    type BarRect = (f32, f32, f32, f32, gpui::Rgba);
    let mut rects: Vec<BarRect> = Vec::new();
    for (si, trace) in bar_traces.iter().enumerate() {
        for (ci, cat) in categories.iter().enumerate() {
            let Some(pos) = trace.categories.iter().position(|c| c == cat) else {
                continue;
            };
            let Some(&v) = trace.y.get(pos) else { continue };
            if !v.is_finite() {
                continue;
            }
            let x = ci as f32 * slot + slot * 0.1 + si as f32 * bar_w;
            let y0 = ((y_max - 0.0) / (y_max - y_min)) as f32 * chart_height;
            let y1 = ((y_max - v) / (y_max - y_min)) as f32 * chart_height;
            rects.push((
                x,
                y0.min(y1),
                bar_w * 0.9,
                (y0 - y1).abs().max(1.0),
                trace.color.to_rgba(),
            ));
        }
    }
    let bar_canvas = canvas(
        move |_b, _w, _c| rects.clone(),
        move |bounds, rects, window, _cx| {
            let ox: f32 = bounds.origin.x.into();
            let oy: f32 = bounds.origin.y.into();
            for (x, y, w, h, color) in &rects {
                let mut b = PathBuilder::fill();
                b.move_to(point(px(ox + x), px(oy + y)));
                b.line_to(point(px(ox + x + w), px(oy + y)));
                b.line_to(point(px(ox + x + w), px(oy + y + h)));
                b.line_to(point(px(ox + x), px(oy + y + h)));
                b.close();
                if let Ok(p) = b.build() {
                    window.paint_path(p, *color);
                }
            }
        },
    )
    .absolute()
    .inset_0();

    let xs = LinearScale::new()
        .domain(0.0, categories.len() as f64)
        .range(0.0, f64::from(chart_width));
    let ys = LinearScale::new()
        .domain(y_min, y_max)
        .range(f64::from(chart_height), 0.0);
    let mut area = div()
        .w(px(chart_width))
        .h(px(chart_height))
        .relative()
        .bg(ui_theme.surface)
        .child(render_grid(
            &xs,
            &ys,
            &GridConfig::with_lines(),
            chart_width,
            chart_height,
            &theme,
        ))
        .child(bar_canvas);
    // Scatter overlays (filters_eq combines bars and lines).
    for trace in plot
        .traces
        .iter()
        .filter(|t| t.visible && t.kind == TraceKind::Scatter && !hidden.contains(&t.name))
    {
        let points: Vec<LinePoint> = if trace.categories.is_empty() {
            trace
                .points()
                .iter()
                .map(|&(x, y)| LinePoint::new(x, y))
                .collect()
        } else {
            trace
                .categories
                .iter()
                .zip(&trace.y)
                .filter_map(|(cat, &y)| {
                    categories
                        .iter()
                        .position(|c| c == cat)
                        .map(|i| LinePoint::new(i as f64 + 0.5, y))
                })
                .collect()
        };
        if points.is_empty() {
            continue;
        }
        let mut config = LineConfig::new()
            .stroke_color(trace.color)
            .stroke_width(trace.stroke_width)
            .curve(CurveType::Linear);
        if let Some(dash) = dash_for(trace.dash.as_deref()) {
            config = config.dash_array(dash);
        }
        area = area.child(render_line(&xs, &ys, &points, &config));
    }

    let y_step = nice_step(y_max - y_min);
    let y_title: &'static str = if plot.layout.y_axis.title.contains("dB") {
        "SPL (dB)"
    } else {
        "Value"
    };
    let y_axis_config = AxisConfig::left()
        .with_tick_values(ticks((y_min, y_max), y_step))
        .with_formatter(formatter_for((y_min, y_max)))
        .with_title(y_title)
        .with_label_font_size(label_size)
        .with_title_font_size(title_size)
        .with_tick_size(tick_size);
    let axis_w = y_axis_config.total_size();
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_start()
                .child(render_axis(&ys, &y_axis_config, chart_height, &theme))
                .child(area),
        )
        // Category labels sit in slots of the same width as the bar
        // slots above, offset by the y-axis width, so each label stays
        // centred under its bar group.
        .child(
            div()
                .flex()
                .child(div().w(px(axis_w)).flex_none())
                .child(div().flex().children(categories.iter().map(|cat| {
                    div()
                        .w(px(slot))
                        .flex_none()
                        .text_center()
                        .text_size(px(label_size))
                        .text_color(ui_theme.text_secondary)
                        .child(cat.clone())
                }))),
        )
}

/// Renders a `make_subplots` figure: one mini chart per subplot cell.
///
/// Each cell reuses the frequency-chart or bar renderer with the
/// cell's own axes, positioned by the cell's paper domain. This draws
/// the combined `filters_auto_eq` sheet (8x2 cells of PEQ details,
/// spin curves, per-curve regressions and error histograms).
pub fn render_generic_subplots(
    plot: &PlotData,
    def: &GraphDef,
    hidden: &HashSet<String>,
    chart_width: f32,
    chart_height: f32,
    ui_theme: &Theme,
) -> Div {
    let theme = ChartTheme::from_theme(ui_theme);
    let title_size = (chart_width / 800.0).clamp(0.7, 1.2) * 10.0;
    let mut container = div()
        .relative()
        .w(px(chart_width))
        .h(px(chart_height));
    for (index, cell) in plot.layout.subplots.iter().enumerate() {
        let cell_plot = PlotData {
            traces: plot
                .traces
                .iter()
                .filter(|trace| trace.subplot == index)
                .cloned()
                .collect(),
            layout: PlotLayout {
                title: None,
                x_axis: cell.x_axis.clone(),
                y_axis: cell.y_axis.clone(),
                y_axis2: None,
                annotations: Vec::new(),
                polar_radial_range: None,
                hide_zero_line: false,
                subplots: Vec::new(),
            },
        };
        if cell_plot.traces.is_empty() {
            continue;
        }
        let left = cell.x_domain.0 as f32 * chart_width;
        let top = (1.0 - cell.y_domain.1) as f32 * chart_height;
        let width = ((cell.x_domain.1 - cell.x_domain.0) as f32 * chart_width).max(40.0);
        let height = ((cell.y_domain.1 - cell.y_domain.0) as f32 * chart_height).max(40.0);
        let title_h = if cell.title.is_some() { 15.0 } else { 0.0 };
        let has_bars = cell_plot
            .traces
            .iter()
            .any(|trace| trace.visible && trace.kind == TraceKind::Bar);
        let chart = if has_bars {
            render_generic_bars(
                &cell_plot,
                hidden,
                width,
                (height - title_h).max(30.0),
                ui_theme,
            )
        } else {
            render_generic_freq_chart(
                &cell_plot,
                def,
                hidden,
                width,
                (height - title_h).max(30.0),
                ui_theme,
            )
        };
        let mut cell_div = div()
            .absolute()
            .left(px(left))
            .top(px(top))
            .w(px(width))
            .h(px(height))
            .flex()
            .flex_col();
        if let Some(title) = &cell.title {
            cell_div = cell_div.child(
                div()
                    .pl(px(16.0))
                    .text_size(px(title_size))
                    .text_color(theme.label_color)
                    .child(title.clone()),
            );
        }
        container = container.child(cell_div.child(chart));
    }
    container
}

/// Clips a pixel-space polygon to the chart rectangle
/// (`0..=width`, `0..=height`) with Sutherland–Hodgman clipping.
///
/// Fill polygons are built from data coordinates that can exceed the
/// axis domain (frequencies above 20 kHz, band edges beyond the y
/// range); without clipping they paint outside the plot area.
fn clip_poly_to_rect(poly: &[(f32, f32)], width: f32, height: f32) -> Vec<(f32, f32)> {
    // One clip edge: (point inside test, intersection with the edge).
    fn clip_edge(
        poly: &[(f32, f32)],
        inside: impl Fn((f32, f32)) -> bool,
        intersect: impl Fn((f32, f32), (f32, f32)) -> (f32, f32),
    ) -> Vec<(f32, f32)> {
        let mut out = Vec::with_capacity(poly.len());
        for (index, &current) in poly.iter().enumerate() {
            let previous = poly[(index + poly.len() - 1) % poly.len()];
            let (current_in, previous_in) = (inside(current), inside(previous));
            if current_in {
                if !previous_in {
                    out.push(intersect(previous, current));
                }
                out.push(current);
            } else if previous_in {
                out.push(intersect(previous, current));
            }
        }
        out
    }
    // Intersection factor along `a -> b` for one coordinate; the caller
    // only invokes this when the segment crosses the edge, so the
    // denominator is non-zero.
    fn cross(a: (f32, f32), b: (f32, f32), edge: f32, horizontal: bool) -> (f32, f32) {
        let (a_edge, b_edge) = if horizontal { (a.1, b.1) } else { (a.0, b.0) };
        let t = (edge - a_edge) / (b_edge - a_edge);
        (a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1))
    }
    let mut clipped = poly.to_vec();
    clipped = clip_edge(&clipped, |p| p.0 >= 0.0, |a, b| cross(a, b, 0.0, false));
    clipped = clip_edge(&clipped, |p| p.0 <= width, |a, b| cross(a, b, width, false));
    clipped = clip_edge(&clipped, |p| p.1 >= 0.0, |a, b| cross(a, b, 0.0, true));
    clipped = clip_edge(&clipped, |p| p.1 <= height, |a, b| {
        cross(a, b, height, true)
    });
    clipped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_keeps_polygon_inside_the_chart() {
        let poly = vec![(10.0, 10.0), (90.0, 10.0), (90.0, 90.0), (10.0, 90.0)];
        assert_eq!(clip_poly_to_rect(&poly, 100.0, 100.0), poly);
    }

    #[test]
    fn clip_cuts_band_polygon_at_the_chart_edges() {
        // ±dB zone built from out-of-domain points: wider than the
        // chart on both sides and taller than the y range at the top.
        let poly = vec![(-20.0, -10.0), (120.0, -30.0), (120.0, 60.0), (-20.0, 60.0)];
        let clipped = clip_poly_to_rect(&poly, 100.0, 50.0);
        assert!(clipped.len() >= 3);
        assert!(
            clipped
                .iter()
                .all(|&(x, y)| (0.0..=100.0).contains(&x) && (0.0..=50.0).contains(&y)),
            "clipped polygon escapes the chart: {clipped:?}"
        );
    }

    #[test]
    fn clip_drops_polygon_fully_outside() {
        let poly = vec![(200.0, 10.0), (300.0, 10.0), (300.0, 90.0)];
        assert!(clip_poly_to_rect(&poly, 100.0, 100.0).len() < 3);
    }
}

/// Legend for generic traces, in payload order with toggle support data.
#[allow(dead_code, reason = "public legend helper for batch/image consumers")]
pub fn generic_legend_entries(plot: &PlotData) -> Vec<(String, d3rs::color::D3Color)> {
    plot.traces
        .iter()
        .filter(|t| t.visible && !t.name.is_empty())
        .map(|t| (t.name.clone(), t.fill_color.unwrap_or(t.color)))
        .collect()
}
