//! Streamgraph -- Observable example using d3rs::examples::streamgraph
//!
//! Demonstrates idiomatic d3rs usage: `Stack` with `InsideOut` order + `Wiggle` offset,
//! `LinearScale` for axes, `PathBuilder` for area paths, `d3rs_path_to_gpui_simple` for rendering.
use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::color::ColorScheme;
use d3rs::scale::{LinearScale, Scale};
use d3rs::shape::area::Area;
use d3rs::shape::curve::Curve;
use d3rs::shape::stack::{Stack, StackOffset, StackOrder};
use d3rs::time::TimeScale;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const UNEMPLOYMENT_CSV: &str = include_str!("../../data/unemployment.csv");

/// Format an epoch timestamp as a calendar year (official x-axis shows years).
fn format_epoch_year(epoch: i64) -> String {
    // Days since Unix epoch -> civil year (Howard Hinnant's algorithm).
    let days = epoch.div_euclid(86_400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    (yoe + era * 400).to_string()
}

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    // Load real unemployment data via d3rs CSV parser + stacked_area pivot
    let (categories, rows) =
        d3rs::examples::stacked_area::load_csv(UNEMPLOYMENT_CSV, "date", "industry", "unemployed");
    let matrix: Vec<Vec<f64>> = rows.iter().map(|r| r.values.clone()).collect();

    let scheme = ColorScheme::tableau10();
    let colors: Vec<Hsla> = (0..scheme.len())
        .map(|i| chart_colors::categorical(&ui_theme, &scheme, i))
        .collect();

    let width = 700.0_f64;
    let height = 450.0_f64;
    let margin_left = 50.0_f64;
    let margin_top = 20.0_f64;
    let margin_bottom = 40.0_f64;
    let margin_right = 20.0_f64;
    let plot_w = width - margin_left - margin_right;
    let plot_h = height - margin_top - margin_bottom;
    let n = matrix.len();

    // Use d3rs Stack with InsideOut order and Wiggle offset for streamgraph layout
    let stack = Stack::new()
        .keys(categories.clone())
        .order(StackOrder::InsideOut)
        .offset(StackOffset::Wiggle);
    let series = stack.generate(&matrix);

    // Compute y extent from stacked values
    let mut y_min = f64::MAX;
    let mut y_max = f64::MIN;
    for s in &series {
        for v in &s.values {
            y_min = y_min.min(v[0]);
            y_max = y_max.max(v[1]);
        }
    }

    // X: TimeScale over the real row dates like the official example.
    let dates: Vec<i64> = rows.iter().map(|r| r.date).collect();
    let x_time = TimeScale::new()
        .domain(dates[0], dates[n - 1])
        .range(0.0, plot_w);
    let x_ticks = x_time.time_ticks(6);
    let y_scale = LinearScale::new().domain(y_min, y_max).range(plot_h, 0.0);

    // Build smooth area paths with the d3rs Area generator and a basis
    // curve, matching the official streamgraph's rounded shapes.
    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    for s in &series {
        let data: Vec<(usize, [f64; 2])> = (0..n).map(|i| (i, s.values[i])).collect();
        let dates_clone = dates.clone();
        let area = Area::new()
            .x(move |d: &(usize, [f64; 2])| x_time.scale(dates_clone[d.0]))
            .y0(move |d: &(usize, [f64; 2])| y_scale.scale(d.1[0]))
            .y1(move |d: &(usize, [f64; 2])| y_scale.scale(d.1[1]))
            .curve(Curve::basis());
        d3_paths.push(area.generate(&data));
    }

    let legend_items: Vec<Div> = categories
        .iter()
        .enumerate()
        .map(|(i, name)| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(div().size_3().bg(colors[i % colors.len()]))
                .child(div().text_xs().child(name.clone()))
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .size_full()
        .p_4()
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .mb_2()
                .child("Streamgraph"),
        )
        .child(
            div()
                .text_xs()
                .mb_2()
                .child("Source: observablehq.com/@d3/streamgraph"),
        )
        .child(
            div()
                .flex()
                .gap_4()
                .mb_2()
                .flex_wrap()
                .children(legend_items),
        )
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(ui_theme.surface)
                .border_1()
                .border_color(ui_theme.border)
                .relative()
                // X-axis line (the official example shows only the year axis;
                // wiggle offsets are relative, so there is no y-axis)
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + plot_h) as f32))
                        .w(px(plot_w as f32))
                        .h(px(1.0))
                        .bg(ui_theme.border),
                )
                // X-axis tick labels from the actual tick dates (years)
                .children(x_ticks.iter().map(|&epoch| {
                    let x = x_time.scale(epoch);
                    let label = format_epoch_year(epoch);
                    div()
                        .absolute()
                        .left(px((margin_left + x - 20.0) as f32))
                        .top(px((margin_top + plot_h + 4.0) as f32))
                        .w(px(40.0))
                        .flex()
                        .justify_center()
                        .child(div().text_xs().child(label))
                }))
                // Plot area with streamgraph
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(plot_w as f32))
                        .h(px(plot_h as f32))
                        .child(
                            canvas(
                                move |bounds, _, _| {
                                    d3_paths
                                        .iter()
                                        .map(|p| {
                                            super::path_utils::d3rs_path_to_gpui_simple(
                                                p, bounds, 0.0, 0.0,
                                            )
                                        })
                                        .collect::<Vec<_>>()
                                },
                                move |_bounds, paths, window, _| {
                                    for (i, path_opt) in paths.into_iter().enumerate() {
                                        if let Some(path) = path_opt {
                                            window.paint_path(path, colors[i % colors.len()]);
                                        }
                                    }
                                },
                            )
                            .size_full(),
                        ),
                ),
        )
}
