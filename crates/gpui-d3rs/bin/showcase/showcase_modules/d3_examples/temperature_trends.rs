//! Global Temperature Trends — scatter plot with diverging colors.
//! Source: <https://observablehq.com/@d3/global-temperature-trends>

use crate::ShowcaseApp;
use d3rs::color::DivergingScheme;
use d3rs::scale::{DivergingScale, LinearScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const TEMPERATURES_CSV: &str = include_str!("../../data/temperatures.csv");

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let values = d3rs::examples::temperature_trends::load_csv(TEMPERATURES_CSV);
    let result = d3rs::examples::temperature_trends::compute(&values);

    let width = result.width;
    let height = result.height;

    // Official diverging scale over [max, 0, min]: hot (max) -> red (t=0),
    // zero -> light, cold (min) -> blue (t=1).
    let rd_bu = DivergingScheme::rd_bu();
    let color_scale = DivergingScale::new()
        .domain(result.y_domain[1], 0.0, result.y_domain[0])
        .interpolator(move |t| rd_bu.get(t));

    let n_sides = 8;
    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // Zero reference line (thin rectangle)
    let y_zero = LinearScale::new()
        .domain(result.y_domain[0], result.y_domain[1])
        .range(height - 30.0, 20.0)
        .scale(0.0);
    d3_paths.push(
        D3PathBuilder::new()
            .move_to(40.0, y_zero)
            .line_to(width - 20.0, y_zero)
            .line_to(width - 20.0, y_zero + 0.5)
            .line_to(40.0, y_zero + 0.5)
            .close_path()
            .build(),
    );
    all_colors.push(hsla(0.0, 0.0, 0.5, 0.5));

    // Data points as small circles
    for pt in &result.points {
        let mut builder = D3PathBuilder::new();
        for v in 0..n_sides {
            let angle = std::f64::consts::TAU * v as f64 / n_sides as f64;
            let x = pt.x + result.radius * angle.cos();
            let y = pt.y + result.radius * angle.sin();
            if v == 0 {
                builder = builder.move_to(x, y);
            } else {
                builder = builder.line_to(x, y);
            }
        }
        builder = builder.close_path();
        d3_paths.push(builder.build());

        // Map value through the diverging scale (zero anomaly -> midpoint).
        let c = color_scale.scale(pt.value);
        all_colors.push(c.to_rgba().into());
    }

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
                .child("Global Temperature Trends"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/global-temperature-trends — {} monthly anomalies",
            values.len()
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
                // Y-axis: temperature anomaly labels
                .children({
                    let y_scale = LinearScale::new()
                        .domain(result.y_domain[0], result.y_domain[1])
                        .range(height - 30.0, 20.0);
                    let step = 0.2;
                    let mut ticks = Vec::new();
                    let mut v = (result.y_domain[0] / step).ceil() * step;
                    while v <= result.y_domain[1] + 0.01 {
                        ticks.push(v);
                        v += step;
                    }
                    ticks.into_iter().map(move |v| {
                        let y = y_scale.scale(v);
                        div()
                            .absolute()
                            .left(px(2.0))
                            .top(px((y - 6.0) as f32))
                            .text_size(px(9.0))
                            .child(format!("{v:+.1}°C"))
                    })
                })
                // X-axis: year labels
                .children({
                    let n = values.len();
                    let x_scale = LinearScale::new()
                        .domain(0.0, (n - 1) as f64)
                        .range(40.0, width - 20.0);
                    // ~12 months per year, label every 20 years
                    let step = 20 * 12;
                    (0..n).step_by(step).map(move |i| {
                        let x = x_scale.scale(i as f64);
                        let year = 1880 + i / 12;
                        div()
                            .absolute()
                            .left(px((x - 12.0) as f32))
                            .top(px((height - 15.0) as f32))
                            .text_size(px(9.0))
                            .child(format!("{year}"))
                    })
                })
                // Y-axis label
                .child(
                    div()
                        .absolute()
                        .left(px(2.0))
                        .top(px(2.0))
                        .text_size(px(9.0))
                        .child("Anomaly (°C)"),
                ),
        )
}
