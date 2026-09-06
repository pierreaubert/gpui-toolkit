//! Stacked Bar Chart -- Observable example using d3rs::examples::stacked_bar
//!
//! Demonstrates idiomatic d3rs usage: `Stack` with `Diverging` offset, `BandScale` for x-axis,
//! `LinearScale` for y-axis, `PathBuilder` for rectangle paths, `d3rs_path_to_gpui_simple`.
use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::color::ColorScheme;
use d3rs::scale::{BandScale, LinearScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use d3rs::shape::stack::{Stack, StackOffset, StackOrder};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let (states, ages, matrix) = d3rs::examples::stacked_bar::default_data();

    let scheme = ColorScheme::tableau10();
    let colors: Vec<Hsla> = (0..scheme.len())
        .map(|i| chart_colors::ink_rgba(&ui_theme, scheme.color(i).to_rgba()))
        .collect();

    let width = 700.0_f64;
    let height = 450.0_f64;
    let margin_top = 20.0_f64;
    let margin_right = 20.0_f64;
    let margin_bottom = 40.0_f64;
    let margin_left = 50.0_f64;
    let chart_width = width - margin_left - margin_right;
    let chart_height = height - margin_top - margin_bottom;

    // Use d3rs Stack with Diverging offset
    let stack = Stack::new()
        .keys(ages.clone())
        .order(StackOrder::None)
        .offset(StackOffset::Diverging);
    let series = stack.generate(&matrix);

    // Use d3rs BandScale for state positions
    let band = BandScale::new()
        .domain(states.clone())
        .range(0.0, chart_width)
        .padding_inner(0.1);
    let bw = band.bandwidth();

    // Compute y extent from stacked values
    let mut y_min = f64::MAX;
    let mut y_max = f64::MIN;
    for s in &series {
        for i in 0..matrix.len() {
            if let Some(v) = s.get(i) {
                y_min = y_min.min(v[0]);
                y_max = y_max.max(v[1]);
            }
        }
    }

    let y_scale = LinearScale::new()
        .domain(y_min, y_max)
        .range(chart_height, 0.0);

    // Build rectangle d3rs Paths for each series segment using D3PathBuilder
    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut rect_colors: Vec<Hsla> = Vec::new();
    for (si, s) in series.iter().enumerate() {
        for (gi, state) in states.iter().enumerate() {
            if let Some(v) = s.get(gi) {
                let x = band.scale(state).unwrap_or(0.0);
                let y0 = y_scale.scale(v[0]);
                let y1 = y_scale.scale(v[1]);
                let top = y0.min(y1);
                let bottom = y0.max(y1);
                let h = (bottom - top).max(0.5);
                let path = D3PathBuilder::new()
                    .move_to(x, top)
                    .line_to(x + bw, top)
                    .line_to(x + bw, top + h)
                    .line_to(x, top + h)
                    .close_path()
                    .build();
                d3_paths.push(path);
                rect_colors.push(colors[si % colors.len()]);
            }
        }
    }

    let legend_items: Vec<Div> = ages
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

    // Y-axis ticks
    let y_range = y_max - y_min;
    let y_step = (y_range / 5.0).ceil();
    let y_min_tick = (y_min / y_step).floor() * y_step;
    let y_ticks: Vec<f64> = (0..=6)
        .map(|i| y_min_tick + i as f64 * y_step)
        .filter(|v| *v >= y_min - 0.1 && *v <= y_max + 0.1)
        .collect();

    // State labels along x-axis
    let state_labels: Vec<Div> = states
        .iter()
        .map(|state| {
            let x = margin_left + band.scale(state).unwrap_or(0.0);
            div()
                .absolute()
                .left(px(x as f32))
                .top(px((margin_top + chart_height + 4.0) as f32))
                .w(px(bw as f32))
                .text_xs()
                .child(state.clone())
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
                .child("Stacked Bar Chart"),
        )
        .child(
            div()
                .text_xs()
                .mb_2()
                .child("Source: observablehq.com/@d3/stacked-bar-chart"),
        )
        .child(
            div()
                .flex()
                .gap_3()
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
                .children(state_labels)
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(chart_width as f32))
                        .h(px(chart_height as f32))
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
                                            window.paint_path(path, rect_colors[i]);
                                        }
                                    }
                                },
                            )
                            .size_full(),
                        ),
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
                        .w(px(chart_width as f32))
                        .h(px(1.0))
                        .bg(Hsla::from(ui_theme.border).opacity(0.25))
                }))
                // X-axis line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + chart_height) as f32))
                        .w(px(chart_width as f32))
                        .h(px(1.0))
                        .bg(ui_theme.border),
                )
                // Y-axis line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(1.0))
                        .h(px(chart_height as f32))
                        .bg(ui_theme.border),
                ),
        )
}
