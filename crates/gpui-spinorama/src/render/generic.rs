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
    GenericTrace, GraphDef, PlotData, TraceFill, TraceKind, TraceMode,
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
    } else {
        nice_step(y_domain.1 - y_domain.0)
    };
    let y_ticks = ticks(y_domain, y_step);
    let fmt = formatter_for(y_domain);

    // Fill polygons (pixel space) painted under the lines.
    let x_to_px = |v: f64| -> f32 {
        if log_x {
            let (lo, hi) = (x_domain.0.ln(), x_domain.1.ln());
            ((v.ln() - lo) / (hi - lo)) as f32 * chart_width
        } else {
            ((v - x_domain.0) / (x_domain.1 - x_domain.0)) as f32 * chart_width
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

    let chart_area = div()
        .w(px(chart_width))
        .h(px(chart_height))
        .relative()
        .bg(ui_theme.surface)
        .child(fill_canvas);
    let chart_area = if log_x {
        let xs = LogScale::new()
            .domain(x_domain.0, x_domain.1)
            .range(0.0, f64::from(chart_width));
        let ys = LinearScale::new()
            .domain(y_domain.0, y_domain.1)
            .range(f64::from(chart_height), 0.0);
        let ys2 = LinearScale::new()
            .domain(y2_domain.0, y2_domain.1)
            .range(f64::from(chart_height), 0.0);
        let area = chart_area.child(render_grid(
            &xs,
            &ys,
            &GridConfig::with_lines(),
            chart_width,
            chart_height,
            &theme,
        ));
        add_lines(area, &visible_scatter, &xs, &ys, &ys2, has_secondary)
    } else {
        let xs = LinearScale::new()
            .domain(x_domain.0, x_domain.1)
            .range(0.0, f64::from(chart_width));
        let ys = LinearScale::new()
            .domain(y_domain.0, y_domain.1)
            .range(f64::from(chart_height), 0.0);
        let ys2 = LinearScale::new()
            .domain(y2_domain.0, y2_domain.1)
            .range(f64::from(chart_height), 0.0);
        let area = chart_area.child(render_grid(
            &xs,
            &ys,
            &GridConfig::with_lines(),
            chart_width,
            chart_height,
            &theme,
        ));
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
        .with_tick_values(y_ticks.clone())
        .with_formatter(fmt)
        .with_title(y_title)
        .with_label_font_size(label_size)
        .with_title_font_size(title_size)
        .with_tick_size(tick_size);
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
        // the two known secondary titles, falling back to "DI (dB)".
        let title2: &'static str = match title2.as_str() {
            "" => "DI (dB)",
            other => {
                if other.contains("DI") {
                    "DI (dB)"
                } else {
                    "Secondary"
                }
            }
        };
        top = top.child(render_axis(
            &y2_scale,
            &AxisConfig::right()
                .with_tick_values(ticks(y2_domain, nice_step(y2_domain.1 - y2_domain.0)))
                .with_formatter(fmt)
                .with_title(title2)
                .with_label_font_size(label_size)
                .with_title_font_size(title_size)
                .with_tick_size(tick_size),
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
                .child(div().flex_none().w(px(
                    AxisConfig::left()
                        .with_title(y_title)
                        .with_label_font_size(label_size)
                        .with_title_font_size(title_size)
                        .with_tick_size(tick_size)
                        .total_size(),
                )))
                .child(if log_x {
                    let xs = LogScale::new()
                        .domain(x_domain.0, x_domain.1)
                        .range(0.0, f64::from(chart_width));
                    render_axis(
                        &xs,
                        &AxisConfig::bottom()
                            .with_tick_values(bottom_ticks)
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
                        chart_width,
                        &theme,
                    )
                    .into_any_element()
                } else {
                    let xs = LinearScale::new()
                        .domain(x_domain.0, x_domain.1)
                        .range(0.0, f64::from(chart_width));
                    render_axis(
                        &xs,
                        &AxisConfig::bottom()
                            .with_tick_values(bottom_ticks)
                            .with_formatter(fmt)
                            .with_title(x_title)
                            .with_label_font_size(label_size)
                            .with_title_font_size(title_size)
                            .with_tick_size(tick_size),
                        chart_width,
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
        let points: Vec<LinePoint> = trace
            .points()
            .iter()
            .map(|&(x, y)| LinePoint::new(x, y))
            .collect();
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
            config = config.show_points(true);
        }
        let scale_y: &YS = if trace.secondary_axis && has_secondary {
            ys2
        } else {
            ys
        };
        area = area.child(render_line(xs, scale_y, &points, &config));
    }
    area
}

/// Renders a generic polar chart: every scatterpolar trace, numeric or
/// categorical (spider). No trace-count cap.
pub fn render_generic_polar(
    plot: &PlotData,
    hidden: &HashSet<String>,
    chart_size: f32,
    ui_theme: &Theme,
) -> Div {
    let center = chart_size / 2.0;
    let outer = (chart_size / 2.0 - 56.0).max(40.0);
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

    let label_cfg = GlyphTextConfig::horizontal(10.0, Hsla::from(ui_theme.text_secondary));
    let labels = div().absolute().inset_0().children(if categorical {
        categories
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let ang =
                    (i as f64 / categories.len().max(1) as f64) * std::f64::consts::TAU
                        - std::f64::consts::FRAC_PI_2;
                let r = outer + 28.0;
                div()
                    .absolute()
                    .left(px(center + r * ang.cos() as f32 - 24.0))
                    .top(px(center + r * ang.sin() as f32 - 6.0))
                    .child(render_glyph_text(label, &label_cfg))
            })
            .collect::<Vec<_>>()
    } else {
        (0..12)
            .map(|i| {
                let deg = f64::from(i) * 30.0;
                let ang = (deg - 90.0).to_radians();
                let r = outer + 22.0;
                let shown = if deg <= 180.0 { deg as i32 } else { deg as i32 - 360 };
                div()
                    .absolute()
                    .left(px(center + r * ang.cos() as f32 - 14.0))
                    .top(px(center + r * ang.sin() as f32 - 6.0))
                    .child(render_glyph_text(&format!("{shown}°"), &label_cfg))
            })
            .collect::<Vec<_>>()
    });

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
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_start()
                .child(render_axis(
                    &ys,
                    &AxisConfig::left()
                        .with_tick_values(ticks((y_min, y_max), y_step))
                        .with_formatter(formatter_for((y_min, y_max)))
                        .with_title(y_title)
                        .with_label_font_size(label_size)
                        .with_title_font_size(title_size)
                        .with_tick_size(tick_size),
                    chart_height,
                    &theme,
                ))
                .child(area),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(8.0))
                .children(categories.iter().map(|cat| {
                    div()
                        .text_size(px(label_size))
                        .text_color(ui_theme.text_secondary)
                        .child(cat.clone())
                })),
        )
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
