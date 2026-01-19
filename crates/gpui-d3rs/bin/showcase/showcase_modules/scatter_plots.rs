use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::color::{ColorScheme, D3Color};
use d3rs::grid::{GridConfig, render_grid};
use d3rs::prelude::*;
use gpui::*;

pub fn render(_app: &ShowcaseApp) -> Div {
    let theme = DefaultAxisTheme;
    let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 500.0);
    let y_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 250.0);
    let scheme = ColorScheme::category10();

    let data1 = vec![
        ScatterPoint::new(10.0, 20.0),
        ScatterPoint::new(25.0, 45.0),
        ScatterPoint::new(35.0, 30.0),
        ScatterPoint::new(50.0, 75.0),
        ScatterPoint::new(65.0, 55.0),
        ScatterPoint::new(75.0, 85.0),
        ScatterPoint::new(85.0, 65.0),
        ScatterPoint::new(90.0, 90.0),
    ];

    let cluster1: Vec<_> = (0..15)
        .map(|i| {
            let angle = i as f64 * 0.4;
            ScatterPoint::new(30.0 + angle.cos() * 15.0, 30.0 + angle.sin() * 15.0)
        })
        .collect();

    let cluster2: Vec<_> = (0..15)
        .map(|i| {
            let angle = i as f64 * 0.5;
            ScatterPoint::new(70.0 + angle.cos() * 12.0, 70.0 + angle.sin() * 12.0)
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Scatter Plots Demo"),
        )
        // Simple scatter
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .mb_2()
                        .child("Simple Scatter Plot"),
                )
                .child(
                    div()
                        .flex()
                        .child(render_axis(
                            &y_scale,
                            &AxisConfig::left().with_ticks(5),
                            250.0,
                            &theme,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .w(px(500.0))
                                        .h(px(250.0))
                                        .relative()
                                        .bg(rgb(0xf8f8f8))
                                        .border_1()
                                        .border_color(rgb(0xcccccc))
                                        .child(render_grid(
                                            &x_scale,
                                            &y_scale,
                                            &GridConfig::dots_only(),
                                            500.0,
                                            250.0,
                                            &theme,
                                        ))
                                        .child(render_scatter(
                                            &x_scale,
                                            &y_scale,
                                            &data1,
                                            &ScatterConfig::new()
                                                .fill_color(scheme.color(0))
                                                .point_radius(6.0)
                                                .opacity(0.8),
                                        )),
                                )
                                .child(render_axis(
                                    &x_scale,
                                    &AxisConfig::bottom().with_ticks(5),
                                    500.0,
                                    &theme,
                                )),
                        ),
                ),
        )
        // Clusters
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .mb_2()
                        .child("Multiple Series (2 clusters)"),
                )
                .child(
                    div()
                        .flex()
                        .child(render_axis(
                            &y_scale,
                            &AxisConfig::left().with_ticks(5),
                            250.0,
                            &theme,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .w(px(500.0))
                                        .h(px(250.0))
                                        .relative()
                                        .bg(rgb(0xf8f8f8))
                                        .border_1()
                                        .border_color(rgb(0xcccccc))
                                        .child(render_grid(
                                            &x_scale,
                                            &y_scale,
                                            &GridConfig::with_lines(),
                                            500.0,
                                            250.0,
                                            &theme,
                                        ))
                                        .child(render_scatter(
                                            &x_scale,
                                            &y_scale,
                                            &cluster1,
                                            &ScatterConfig::new()
                                                .fill_color(scheme.color(4))
                                                .point_radius(5.0)
                                                .stroke_color(D3Color::from_hex(0xffffff))
                                                .stroke_width(1.5),
                                        ))
                                        .child(render_scatter(
                                            &x_scale,
                                            &y_scale,
                                            &cluster2,
                                            &ScatterConfig::new()
                                                .fill_color(scheme.color(6))
                                                .point_radius(5.0)
                                                .stroke_color(D3Color::from_hex(0xffffff))
                                                .stroke_width(1.5),
                                        )),
                                )
                                .child(render_axis(
                                    &x_scale,
                                    &AxisConfig::bottom().with_ticks(5),
                                    500.0,
                                    &theme,
                                )),
                        ),
                ),
        )
}

use super::ShowcaseApp;
