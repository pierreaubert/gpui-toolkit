//! Stacked Area Chart -- Observable example using d3rs::examples::stacked_area
//!
//! Demonstrates idiomatic d3rs usage: `Stack` for stacking, `TimeScale` (scaleUtc) for x-axis,
//! `LinearScale` for y-axis, `Area` generator with `Curve::monotone_x`,
//! `d3rs_path_to_gpui_simple` for rendering.
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

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let (categories, rows) =
        d3rs::examples::stacked_area::load_csv(UNEMPLOYMENT_CSV, "date", "industry", "unemployed");

    let scheme = ColorScheme::tableau10();
    let colors: Vec<Hsla> = (0..scheme.len())
        .map(|i| chart_colors::ink_rgba(&ui_theme, scheme.color(i).to_rgba()))
        .collect();

    let width = 700.0_f64;
    let height = 450.0_f64;
    let margin_left = 50.0_f64;
    let margin_top = 20.0_f64;
    let margin_bottom = 40.0_f64;
    let margin_right = 20.0_f64;
    let plot_w = width - margin_left - margin_right;
    let plot_h = height - margin_top - margin_bottom;
    let n = rows.len();

    // Extract values matrix for d3rs Stack
    let matrix: Vec<Vec<f64>> = rows.iter().map(|r| r.values.clone()).collect();

    // Use d3rs Stack for stacking computation
    let stack = Stack::new()
        .keys(categories.clone())
        .order(StackOrder::None)
        .offset(StackOffset::None);
    let series = stack.generate(&matrix);

    // X: TimeScale (scaleUtc) mapping dates to plot area
    let x_time = TimeScale::new()
        .domain(rows[0].date, rows[n - 1].date)
        .range(0.0, plot_w);
    let x_ticks = x_time.time_ticks(6);

    // Compute y extent
    let y_max = series
        .iter()
        .flat_map(|s| (0..n).filter_map(|i| s.get(i).map(|v| v[1])))
        .fold(0.0f64, f64::max);

    // Y: LinearScale mapping values to plot area
    let y_scale = LinearScale::new().domain(0.0, y_max).range(plot_h, 0.0);

    // Build area paths using d3rs Area generator directly in plot coordinates
    let dates: Vec<i64> = rows.iter().map(|r| r.date).collect();
    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    for s in &series {
        let data: Vec<(usize, [f64; 2])> = (0..n)
            .map(|i| (i, s.get(i).unwrap_or([0.0, 0.0])))
            .collect();

        let dates_clone = dates.clone();
        let area = Area::new()
            .x(move |d: &(usize, [f64; 2])| x_time.scale(dates_clone[d.0]))
            .y0(move |d: &(usize, [f64; 2])| y_scale.scale(d.1[0]))
            .y1(move |d: &(usize, [f64; 2])| y_scale.scale(d.1[1]))
            .curve(Curve::linear());

        d3_paths.push(area.generate(&data));
    }

    // Y-axis ticks
    let y_step = (y_max / 5.0).ceil();
    let y_ticks: Vec<f64> = (0..=8)
        .map(|i| i as f64 * y_step)
        .filter(|v| *v <= y_max + 0.1)
        .collect();

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
                .child("Stacked Area Chart"),
        )
        .child(
            div().text_xs().mb_2().child(
                "Source: observablehq.com/@d3/stacked-area-chart — uses TimeScale (scaleUtc)",
            ),
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
                // Y-axis line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(1.0))
                        .h(px(plot_h as f32))
                        .bg(ui_theme.border),
                )
                // X-axis line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + plot_h) as f32))
                        .w(px(plot_w as f32))
                        .h(px(1.0))
                        .bg(ui_theme.border),
                )
                // Y-axis tick labels
                .children(y_ticks.iter().map(|&val| {
                    let y = y_scale.scale(val);
                    div()
                        .absolute()
                        .left(px(0.0))
                        .top(px((margin_top + y - 6.0) as f32))
                        .w(px(margin_left as f32))
                        .flex()
                        .justify_end()
                        .pr_1()
                        .child(div().text_xs().child(format!("{:.0}", val)))
                }))
                // Y grid lines cloned from y ticks at low opacity
                .children(y_ticks.iter().map(|&val| {
                    let y = y_scale.scale(val);
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + y) as f32))
                        .w(px(plot_w as f32))
                        .h(px(1.0))
                        .bg(Hsla::from(ui_theme.border).opacity(0.25))
                }))
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
                // Plot area — paths are already in plot coordinates (0..plot_w, 0..plot_h)
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(plot_w as f32))
                        .h(px(plot_h as f32))
                        .overflow_hidden()
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
