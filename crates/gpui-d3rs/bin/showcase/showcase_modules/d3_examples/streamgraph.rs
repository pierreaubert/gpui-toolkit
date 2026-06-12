//! Streamgraph -- Observable example using d3rs::examples::streamgraph
//!
//! Demonstrates idiomatic d3rs usage: `Stack` with `InsideOut` order + `Wiggle` offset,
//! `LinearScale` for axes, `PathBuilder` for area paths, `d3rs_path_to_gpui_simple` for rendering.
use crate::ShowcaseApp;
use d3rs::color::ColorScheme;
use d3rs::scale::{LinearScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use d3rs::shape::stack::{Stack, StackOffset, StackOrder};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const UNEMPLOYMENT_CSV: &str = include_str!("../../data/unemployment.csv");

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    // Load real unemployment data via d3rs CSV parser + stacked_area pivot
    let (categories, rows) =
        d3rs::examples::stacked_area::load_csv(UNEMPLOYMENT_CSV, "date", "industry", "unemployed");
    let matrix: Vec<Vec<f64>> = rows.iter().map(|r| r.values.clone()).collect();

    let scheme = ColorScheme::tableau10();
    let colors: Vec<Rgba> = (0..scheme.len())
        .map(|i| scheme.color(i).to_rgba())
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

    let x_scale = LinearScale::new()
        .domain(0.0, (n - 1) as f64)
        .range(0.0, plot_w);
    let y_scale = LinearScale::new().domain(y_min, y_max).range(plot_h, 0.0);

    // Build area paths using D3PathBuilder: top line forward + bottom line reversed + close
    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    for s in &series {
        let mut builder = D3PathBuilder::new();
        // Top line (y1 values)
        for i in 0..n {
            let x = x_scale.scale(i as f64);
            let y = y_scale.scale(s.values[i][1]);
            if i == 0 {
                builder = builder.move_to(x, y);
            } else {
                builder = builder.line_to(x, y);
            }
        }
        // Bottom line reversed (y0 values)
        for i in (0..n).rev() {
            let x = x_scale.scale(i as f64);
            let y = y_scale.scale(s.values[i][0]);
            builder = builder.line_to(x, y);
        }
        builder = builder.close_path();
        d3_paths.push(builder.build());
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

    // X-axis ticks (every 4 time steps to avoid label overlap)
    let x_step = if n > 30 {
        5
    } else if n > 15 {
        4
    } else {
        2
    };
    let x_ticks: Vec<f64> = (0..n).step_by(x_step).map(|v| v as f64).collect();

    // Y-axis ticks: 1000-unit grid lines spanning the full y range
    let y_tick_step = 1000.0;
    let y_min_tick = (y_min / y_tick_step).floor() * y_tick_step;
    let y_max_tick = (y_max / y_tick_step).ceil() * y_tick_step;
    let y_ticks: Vec<f64> = {
        let mut ticks = Vec::new();
        let mut v = y_min_tick;
        while v <= y_max_tick + 0.1 {
            ticks.push(v);
            v += y_tick_step;
        }
        ticks
    };

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
                        .child(div().text_xs().child(if val.abs() >= 1000.0 {
                            format!("{:.0}k", val / 1000.0)
                        } else {
                            format!("{:.0}", val)
                        }))
                }))
                // Y grid lines
                .children(y_ticks.iter().map(|&val| {
                    let y = y_scale.scale(val);
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + y) as f32))
                        .w(px(plot_w as f32))
                        .h(px(1.0))
                        .bg(ui_theme.surface)
                }))
                // X-axis tick labels
                .children(x_ticks.iter().map(|&val| {
                    let x = x_scale.scale(val);
                    div()
                        .absolute()
                        .left(px((margin_left + x - 10.0) as f32))
                        .top(px((margin_top + plot_h + 4.0) as f32))
                        .w(px(20.0))
                        .flex()
                        .justify_center()
                        .child(div().text_xs().child(format!("{:.0}", val)))
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
