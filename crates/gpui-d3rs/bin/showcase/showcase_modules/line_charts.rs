use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::color::ColorScheme;
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

    let data = vec![
        LinePoint::new(0.0, 20.0),
        LinePoint::new(20.0, 45.0),
        LinePoint::new(40.0, 35.0),
        LinePoint::new(60.0, 75.0),
        LinePoint::new(80.0, 60.0),
        LinePoint::new(100.0, 85.0),
    ];

    let series1 = vec![
        LinePoint::new(0.0, 25.0),
        LinePoint::new(25.0, 50.0),
        LinePoint::new(50.0, 40.0),
        LinePoint::new(75.0, 70.0),
        LinePoint::new(100.0, 65.0),
    ];

    let series2 = vec![
        LinePoint::new(0.0, 55.0),
        LinePoint::new(25.0, 30.0),
        LinePoint::new(50.0, 60.0),
        LinePoint::new(75.0, 45.0),
        LinePoint::new(100.0, 75.0),
    ];

    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Line Charts Demo"),
        )
        // Linear with points
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .mb_2()
                        .child("Linear with Points"),
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
                                        .child(render_line(
                                            &x_scale,
                                            &y_scale,
                                            &data,
                                            &LineConfig::new()
                                                .stroke_color(scheme.color(1))
                                                .curve(CurveType::Linear)
                                                .show_points(true)
                                                .point_radius(4.0),
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
        // Multiple series
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .mb_2()
                        .child("Multiple Series"),
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
                                            &GridConfig::lines_only().with_line_opacity(0.2),
                                            width,
                                            height,
                                            &theme,
                                        ))
                                        .child(render_line(
                                            &x_scale,
                                            &y_scale,
                                            &series1,
                                            &LineConfig::new()
                                                .stroke_color(scheme.color(4))
                                                .curve(CurveType::Linear)
                                                .show_points(true)
                                                .point_radius(4.0),
                                        ))
                                        .child(render_line(
                                            &x_scale,
                                            &y_scale,
                                            &series2,
                                            &LineConfig::new()
                                                .stroke_color(scheme.color(6))
                                                .curve(CurveType::Linear)
                                                .show_points(true)
                                                .point_radius(4.0),
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
