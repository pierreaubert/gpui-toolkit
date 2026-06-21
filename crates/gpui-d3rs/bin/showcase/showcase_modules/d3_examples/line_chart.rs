//! Line Chart -- Observable example using d3rs::examples::line_chart
//!
//! Demonstrates idiomatic d3rs usage: `LinearScale` for axes, `Curve::interpolate` for
//! line interpolation, `PathBuilder` for ribbon paths, `d3rs_path_to_gpui_simple` for rendering.
use crate::ShowcaseApp;
use d3rs::color::ColorScheme;
use d3rs::scale::{LinearScale, Scale};
use d3rs::shape::path::{PathBuilder as D3PathBuilder, Point as D3Point};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let data = d3rs::examples::line_chart::default_data();
    let result = d3rs::examples::line_chart::compute(&data);

    let scheme = ColorScheme::tableau10();

    let chart_w = 700.0_f64;
    let chart_h = 400.0_f64;
    let margin_left = 50.0;
    let margin_right = 20.0;
    let margin_top = 20.0;
    let margin_bottom = 40.0;
    let plot_w = chart_w - margin_left - margin_right;
    let plot_h = chart_h - margin_top - margin_bottom;

    // Scales mapping data domain to plot area
    let x_scale = LinearScale::new()
        .domain(result.x_domain[0], result.x_domain[1])
        .range(0.0, plot_w);
    let y_scale = LinearScale::new()
        .domain(result.y_domain[0], result.y_domain[1])
        .range(plot_h, 0.0);

    // Map raw data to scaled points using d3rs types
    let points: Vec<D3Point> = data
        .iter()
        .map(|(x, y)| D3Point::new(x_scale.scale(*x), y_scale.scale(*y)))
        .collect();

    // Use d3rs Curve interpolation for each curve type, then build ribbon paths via D3PathBuilder
    let curves: Vec<(&str, d3rs::shape::curve::Curve)> = vec![
        ("linear", d3rs::shape::curve::Curve::linear()),
        ("step", d3rs::shape::curve::Curve::Step),
        ("basis", d3rs::shape::curve::Curve::basis()),
        ("cardinal", d3rs::shape::curve::Curve::cardinal(0.0)),
        ("natural", d3rs::shape::curve::Curve::natural()),
        ("monotoneX", d3rs::shape::curve::Curve::monotone_x()),
        ("catmullRom", d3rs::shape::curve::Curve::catmull_rom(0.5)),
    ];

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut curve_names: Vec<String> = Vec::new();
    for (name, curve) in &curves {
        let interpolated = curve.interpolate(&points);
        let path = points_to_ribbon_d3(&interpolated, 1.8);
        d3_paths.push(path);
        curve_names.push(name.to_string());
    }

    // Legend
    let legend_items: Vec<Div> = curve_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(div().size_3().bg(scheme.color(i).to_rgba()))
                .child(div().text_xs().child(name.clone()))
        })
        .collect();

        // Derive ticks from the scales so they match the domain
    let x_ticks = x_scale.ticks(10);
    let y_ticks = y_scale.ticks(8);

    let colors: Vec<Rgba> = (0..scheme.len())
        .map(|i| scheme.color(i).to_rgba())
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
                .child("Line Chart -- 7 Curve Types"),
        )
        .child(
            div()
                .text_xs()
                .mb_2()
                .child("Source: observablehq.com/@d3/line-chart"),
        )
        .child(
            div()
                .flex()
                .gap_3()
                .mb_3()
                .flex_wrap()
                .children(legend_items),
        )
        .child(
            div()
                .w(px(chart_w as f32))
                .h(px(chart_h as f32))
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
                // Y-axis ticks + labels + grid
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
                // Y grid lines
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
                // X-axis ticks + labels
                .children(x_ticks.iter().map(|&val| {
                    let x = x_scale.scale(val);
                    div()
                        .absolute()
                        .left(px((margin_left + x - 20.0) as f32))
                        .top(px((margin_top + plot_h + 4.0) as f32))
                        .w(px(40.0))
                        .flex()
                        .justify_center()
                        .child(div().text_xs().child(format!("{:.0}", val)))
                }))
                // X grid lines
                .children(x_ticks.iter().map(|&val| {
                    let x = x_scale.scale(val);
                    div()
                        .absolute()
                        .left(px((margin_left + x) as f32))
                        .top(px(margin_top as f32))
                        .w(px(1.0))
                        .h(px(plot_h as f32))
                        .bg(Hsla::from(ui_theme.border).opacity(0.25))
                }))
                // Plot area with curves
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
        .child(div().text_xs().mt_2().child(format!(
            "{} data points | x: [{:.0}..{:.0}] | y: [{:.1}..{:.1}]",
            data.len(),
            result.x_domain[0],
            result.x_domain[1],
            result.y_domain[0],
            result.y_domain[1]
        )))
}

/// Convert interpolated points to a thin filled ribbon d3rs Path to simulate a stroke.
fn points_to_ribbon_d3(points: &[D3Point], thickness: f64) -> d3rs::shape::path::Path {
    if points.len() < 2 {
        return D3PathBuilder::new().build();
    }

    let half = thickness / 2.0;
    let mut upper = Vec::with_capacity(points.len());
    let mut lower = Vec::with_capacity(points.len());
    for i in 0..points.len() {
        let (dx, dy) = if i == 0 {
            (points[1].x - points[0].x, points[1].y - points[0].y)
        } else if i == points.len() - 1 {
            (points[i].x - points[i - 1].x, points[i].y - points[i - 1].y)
        } else {
            (
                points[i + 1].x - points[i - 1].x,
                points[i + 1].y - points[i - 1].y,
            )
        };
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let nx = -dy / len * half;
        let ny = dx / len * half;
        upper.push((points[i].x + nx, points[i].y + ny));
        lower.push((points[i].x - nx, points[i].y - ny));
    }

    let mut builder = D3PathBuilder::new();
    builder = builder.move_to(upper[0].0, upper[0].1);
    for p in upper.iter().skip(1) {
        builder = builder.line_to(p.0, p.1);
    }
    for p in lower.iter().rev() {
        builder = builder.line_to(p.0, p.1);
    }
    builder = builder.close_path();
    builder.build()
}
