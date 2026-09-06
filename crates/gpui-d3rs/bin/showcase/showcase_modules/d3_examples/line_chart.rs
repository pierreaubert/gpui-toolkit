//! Line Chart -- <https://observablehq.com/@d3/line-chart>
//!
//! Faithful port of the official example: a single `steelblue` 1.5px line,
//! `ticks(width / 80)` on x, `ticks(height / 40)` on y, horizontal gridlines
//! at 0.1 opacity cloned from the y ticks, x domain line kept and y domain
//! line removed.
use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::scale::{LinearScale, Scale};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let data = d3rs::examples::line_chart::default_data();
    let result = d3rs::examples::line_chart::compute(&data);

    // Official D3 margin convention and steelblue line color.
    let chart_w = 700.0_f64;
    let chart_h = 400.0_f64;
    let margin_left = 40.0;
    let margin_right = 30.0;
    let margin_top = 20.0;
    let margin_bottom = 30.0;
    let plot_w = chart_w - margin_left - margin_right;
    let plot_h = chart_h - margin_top - margin_bottom;

    // Scales mapping data domain to plot area
    let x_scale = LinearScale::new()
        .domain(result.x_domain[0], result.x_domain[1])
        .range(0.0, plot_w);
    let y_scale = LinearScale::new()
        .domain(result.y_domain[0], result.y_domain[1])
        .range(plot_h, 0.0);

    // Single linear line through the data, in plot coordinates.
    let line_points: Vec<(f32, f32)> = data
        .iter()
        .map(|(x, y)| (x_scale.scale(*x) as f32, y_scale.scale(*y) as f32))
        .collect();

    // Official tick densities: ticks(width / 80), ticks(height / 40).
    let x_ticks = x_scale.ticks((plot_w / 80.0).round().clamp(2.0, 12.0) as usize);
    let y_ticks = y_scale.ticks((plot_h / 40.0).round().clamp(2.0, 12.0) as usize);

    let line_color: Hsla = chart_colors::ink_hex(&ui_theme, 0x4682b4); // steelblue
    let grid_color: Hsla = chart_colors::grid(&ui_theme);

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
                .child("Line Chart"),
        )
        .child(
            div()
                .text_xs()
                .mb_2()
                .child("Source: observablehq.com/@d3/line-chart"),
        )
        .child(
            div()
                .w(px(chart_w as f32))
                .h(px(chart_h as f32))
                .bg(ui_theme.surface)
                .border_1()
                .border_color(ui_theme.border)
                .relative()
                // X-axis domain line (official keeps the bottom domain)
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + plot_h) as f32))
                        .w(px(plot_w as f32))
                        .h(px(1.0))
                        .bg(ui_theme.border),
                )
                // Y-axis tick labels (no y domain line: official removes it)
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
                // Y grid lines cloned from y ticks at 0.1 opacity
                .children(y_ticks.iter().map(|&val| {
                    let y = y_scale.scale(val);
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + y) as f32))
                        .w(px(plot_w as f32))
                        .h(px(1.0))
                        .bg(grid_color)
                }))
                // X-axis ticks + labels, tickSizeOuter(0): no end ticks
                .children(x_ticks.iter().map(|&val| {
                    let x = x_scale.scale(val);
                    let label_w = 40.0;
                    let left = (margin_left + x - label_w / 2.0).clamp(0.0, chart_w - label_w);
                    div()
                        .absolute()
                        .left(px(left as f32))
                        .top(px((margin_top + plot_h + 4.0) as f32))
                        .w(px(label_w as f32))
                        .flex()
                        .justify_center()
                        .child(div().text_xs().child(format!("{:.0}", val)))
                }))
                // Plot area with the single stroked line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(plot_w as f32))
                        .h(px(plot_h as f32))
                        .child(
                            canvas(
                                move |_, _, _| (),
                                move |bounds, _, window, _| {
                                    let origin = bounds.origin;
                                    let mut builder = gpui::PathBuilder::stroke(px(1.5));
                                    for (i, &(x, y)) in line_points.iter().enumerate() {
                                        let pt = origin + point(px(x), px(y));
                                        if i == 0 {
                                            builder.move_to(pt);
                                        } else {
                                            builder.line_to(pt);
                                        }
                                    }
                                    if let Ok(path) = builder.build() {
                                        window.paint_path(path, line_color);
                                    }
                                },
                            )
                            .size_full(),
                        ),
                ),
        )
        .child(div().text_xs().mt_2().child(format!(
            "{} data points | x: [{:.0}..{:.0}] | y: [{:.1}..{:.1}]",
            data.len(),
            result.x_domain[0],
            result.x_domain[1],
            result.y_domain[0],
            result.y_domain[1]
        )))
}
