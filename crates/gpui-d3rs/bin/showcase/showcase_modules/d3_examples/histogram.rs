//! Histogram — Observable example
//!
//! Loads diamonds.csv and bins the carat column into a histogram.
//! Uses d3rs LinearScale, PathBuilder, ColorScheme, and d3rs_path_to_gpui_simple.
//!
//! Source: <https://observablehq.com/@d3/histogram>

use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::scale::{LinearScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const DIAMONDS_CSV: &str = include_str!("../../data/diamonds.csv");

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let width = 640.0;
    let height = 400.0;
    let margin_left = 50.0;
    let margin_right = 20.0;
    let margin_top = 20.0;
    let margin_bottom = 56.0;
    let chart_width = width - margin_left - margin_right;
    let chart_height = height - margin_top - margin_bottom;

    // Parse diamonds.csv: extract carat column (first numeric column)
    let carats: Vec<f64> = DIAMONDS_CSV
        .lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<&str> = line.split(',').collect();
            // carat is the first column
            cols.first()?.parse::<f64>().ok()
        })
        .collect();

    let min_val = carats.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_val = carats.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    // d3.bin-style thresholds from scale ticks so bin edges align with ticks,
    // like the official example (`bin.thresholds(40)` over the x domain).
    let probe = LinearScale::new().domain(min_val, max_val);
    let mut edges = probe.ticks(40);
    if edges.len() < 2 {
        edges = vec![min_val, max_val];
    }
    let bin_count = edges.len() - 1;
    let mut bins = vec![0usize; bin_count];
    for &c in &carats {
        // Bisect right: last bin is inclusive of the max edge.
        let mut idx = bin_count - 1;
        for i in 0..bin_count {
            if c < edges[i + 1] || i == bin_count - 1 {
                idx = i;
                break;
            }
        }
        bins[idx] += 1;
    }

    let max_bin = *bins.iter().max().unwrap_or(&0) as f64;

    let x_scale = LinearScale::new()
        .domain(min_val, max_val)
        .range(margin_left, margin_left + chart_width);

    let y_scale = LinearScale::new()
        .domain(0.0, max_bin)
        .range(margin_top + chart_height, margin_top);

    // Official bars are steelblue and contiguous (no gaps).
    let bar_color: Hsla = chart_colors::ink_hex(&ui_theme, 0x4682b4);

    // Build bar paths
    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    for (i, &count) in bins.iter().enumerate() {
        let x0 = x_scale.scale(edges[i]);
        let x1 = x_scale.scale(edges[i + 1]);
        let y0 = y_scale.scale(count as f64);
        let y_base = y_scale.scale(0.0);

        let path = D3PathBuilder::new()
            .move_to(x0, y0)
            .line_to(x1, y0)
            .line_to(x1, y_base)
            .line_to(x0, y_base)
            .close_path()
            .build();
        d3_paths.push(path);
        all_colors.push(bar_color);
    }

    // X-axis domain line only: the official example removes the y domain.
    // (Thin closed rectangle to avoid fill-triangle artifacts.)
    let axis_w = 1.0;
    d3_paths.push(
        D3PathBuilder::new()
            .move_to(margin_left, margin_top + chart_height)
            .line_to(margin_left + chart_width, margin_top + chart_height)
            .line_to(
                margin_left + chart_width,
                margin_top + chart_height + axis_w,
            )
            .line_to(margin_left, margin_top + chart_height + axis_w)
            .close_path()
            .build(),
    );
    all_colors.push(chart_colors::axis_line(&ui_theme));

    // Y grid lines cloned from y ticks at 0.1 opacity
    let y_tick_step = (max_bin / 5.0).ceil().max(1.0);
    let y_ticks: Vec<f64> = (0..)
        .map(|i| i as f64 * y_tick_step)
        .take_while(|&v| v <= max_bin)
        .collect();

    for &tick_val in &y_ticks {
        let y = y_scale.scale(tick_val);
        d3_paths.push(
            D3PathBuilder::new()
                .move_to(margin_left, y)
                .line_to(margin_left + chart_width, y)
                .line_to(margin_left + chart_width, y + 0.5)
                .line_to(margin_left, y + 0.5)
                .close_path()
                .build(),
        );
        all_colors.push(chart_colors::grid(&ui_theme));
    }

    // X tick values
    let x_tick_step = ((max_val - min_val) / 8.0 * 10.0).round() / 10.0;
    let x_ticks: Vec<f64> = (0..)
        .map(|i| min_val + i as f64 * x_tick_step)
        .take_while(|&v| v <= max_val + 0.01)
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
                .child("Histogram — Diamond Carat Distribution"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/histogram — {} diamonds from diamonds.csv",
            carats.len()
        )))
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(ui_theme.surface)
                .border_1()
                .border_color(ui_theme.border)
                .relative()
                .child(
                    canvas(
                        move |bounds, _, _| {
                            d3_paths
                                .iter()
                                .map(|p| {
                                    super::path_utils::d3rs_path_to_gpui_simple(p, bounds, 0.0, 0.0)
                                })
                                .collect::<Vec<_>>()
                        },
                        move |_bounds, paths, window, _| {
                            for (i, path_opt) in paths.into_iter().enumerate() {
                                if let Some(path) = path_opt {
                                    window.paint_path(path, all_colors[i]);
                                }
                            }
                        },
                    )
                    .size_full(),
                )
                // Y-axis labels
                .children(y_ticks.iter().map(|&tick_val| {
                    let y = y_scale.scale(tick_val);
                    div()
                        .absolute()
                        .left(px(0.0))
                        .top(px((y - 6.0) as f32))
                        .w(px((margin_left - 6.0) as f32))
                        .flex()
                        .justify_end()
                        .pr_1()
                        .text_size(px(9.0))
                        .child(format!("{:.0}", tick_val))
                }))
                // X-axis labels
                .children(x_ticks.iter().map(|&tick_val| {
                    let x = x_scale.scale(tick_val);
                    let label_w = 40.0;
                    let left = (x - label_w / 2.0).clamp(0.0, width - label_w);
                    div()
                        .absolute()
                        .left(px(left as f32))
                        .top(px((margin_top + chart_height + 5.0) as f32))
                        .w(px(label_w as f32))
                        .flex()
                        .justify_center()
                        .text_size(px(9.0))
                        .child(format!("{tick_val:.1}"))
                }))
                // X-axis label
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((height - 24.0) as f32))
                        .w(px(chart_width as f32))
                        .flex()
                        .justify_center()
                        .text_size(px(10.0))
                        .child("Carat"),
                ),
        )
}
