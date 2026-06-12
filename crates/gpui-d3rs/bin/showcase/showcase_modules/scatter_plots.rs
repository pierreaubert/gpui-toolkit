use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::color::{ColorScheme, D3Color};
use d3rs::grid::{GridConfig, render_grid};
use d3rs::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

pub fn render(app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let theme = DefaultAxisTheme;
    let width = app.content_width * 0.7;
    let height = (width * 0.5).min(app.content_height * 0.4);
    let x_scale = LinearScale::new()
        .domain(0.0, 100.0)
        .range(0.0, width as f64);
    let y_scale = LinearScale::new()
        .domain(0.0, 100.0)
        .range(0.0, height as f64);
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
                            height,
                            &theme,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .w(px(width))
                                        .h(px(height))
                                        .relative()
                                        .bg(ui_theme.surface)
                                        .border_1()
                                        .border_color(ui_theme.border)
                                        .child(render_grid(
                                            &x_scale,
                                            &y_scale,
                                            &GridConfig::dots_only(),
                                            width,
                                            height,
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
                                    width,
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
                            height,
                            &theme,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .w(px(width))
                                        .h(px(height))
                                        .relative()
                                        .bg(ui_theme.surface)
                                        .border_1()
                                        .border_color(ui_theme.border)
                                        .child(render_grid(
                                            &x_scale,
                                            &y_scale,
                                            &GridConfig::with_lines(),
                                            width,
                                            height,
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
                                    width,
                                    &theme,
                                )),
                        ),
                ),
        )
}

use super::ShowcaseApp;
