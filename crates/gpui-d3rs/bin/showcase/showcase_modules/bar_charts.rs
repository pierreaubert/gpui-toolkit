use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::color::ColorScheme;
use d3rs::grid::{GridConfig, render_grid};
use d3rs::render2d::{Renderer2D, VelloBackend};
use d3rs::scale::LinearScale;
use d3rs::shape::{
    BarConfig, BarDatum, GroupedBarConfig, GroupedBarDatum, GroupedBarMeta, analyze_grouped_data,
    render_bars_selected as render_bars_with_config,
    render_grouped_bars_selected as render_grouped_bars_with_config,
};
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

pub fn render(app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let theme = DefaultAxisTheme;
    let width = app.content_width * 0.7;
    let height = (width * 0.5).min(app.content_height * 0.4);
    let x_scale = LinearScale::new().domain(0.0, 6.0).range(0.0, width as f64);
    let y_scale = LinearScale::new()
        .domain(0.0, 100.0)
        .range(0.0, height as f64);
    let scheme = ColorScheme::category10();

    let data = vec![
        BarDatum::new("Mon", 45.0),
        BarDatum::new("Tue", 68.0),
        BarDatum::new("Wed", 55.0),
        BarDatum::new("Thu", 82.0),
        BarDatum::new("Fri", 70.0),
        BarDatum::new("Sat", 38.0),
    ];

    let mixed_data = vec![
        BarDatum::new("A", 30.0),
        BarDatum::new("B", -15.0),
        BarDatum::new("C", 45.0),
        BarDatum::new("D", -25.0),
        BarDatum::new("E", 60.0),
    ];
    let mixed_y_scale = LinearScale::new()
        .domain(-30.0, 70.0)
        .range(0.0, height as f64);
    let mixed_x_scale = LinearScale::new().domain(0.0, 5.0).range(0.0, width as f64);

    // Grouped bar data - quarterly sales by product
    let grouped_data = vec![
        GroupedBarDatum::new("Q1", "Product A", 45.0),
        GroupedBarDatum::new("Q1", "Product B", 60.0),
        GroupedBarDatum::new("Q1", "Product C", 35.0),
        GroupedBarDatum::new("Q2", "Product A", 55.0),
        GroupedBarDatum::new("Q2", "Product B", 48.0),
        GroupedBarDatum::new("Q2", "Product C", 52.0),
        GroupedBarDatum::new("Q3", "Product A", 70.0),
        GroupedBarDatum::new("Q3", "Product B", 65.0),
        GroupedBarDatum::new("Q3", "Product C", 45.0),
        GroupedBarDatum::new("Q4", "Product A", 85.0),
        GroupedBarDatum::new("Q4", "Product B", 78.0),
        GroupedBarDatum::new("Q4", "Product C", 68.0),
    ];
    let grouped_meta = analyze_grouped_data(&grouped_data);
    let grouped_y_scale = LinearScale::new()
        .domain(0.0, grouped_meta.max_value * 1.1)
        .range(0.0, height as f64);

    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child(format!("Bar Charts Demo · {}", app.renderer_label())),
        )
        // Simple bar chart
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .mb_2()
                        .child("Simple Bar Chart"),
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
                                        .child(render_bars_selected(
                                            &x_scale,
                                            &y_scale,
                                            &data,
                                            width,
                                            height,
                                            &BarConfig::new()
                                                .fill_color(scheme.color(0))
                                                .opacity(0.85),
                                            app.renderer_selection(),
                                        )),
                                )
                                .child(render_axis(
                                    &x_scale,
                                    &AxisConfig::bottom().with_ticks(6),
                                    width,
                                    &theme,
                                )),
                        ),
                ),
        )
        // Mixed positive/negative
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .mb_2()
                        .child("Mixed Positive/Negative Values"),
                )
                .child(
                    div()
                        .flex()
                        .child(render_axis(
                            &mixed_y_scale,
                            &AxisConfig::left().with_ticks(7).with_formatter(|v| {
                                if v > 0.0 {
                                    format!("+{:.0}", v)
                                } else {
                                    format!("{:.0}", v)
                                }
                            }),
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
                                            &mixed_x_scale,
                                            &mixed_y_scale,
                                            &GridConfig::with_lines(),
                                            width,
                                            height,
                                            &theme,
                                        ))
                                        .child(render_bars_selected(
                                            &mixed_x_scale,
                                            &mixed_y_scale,
                                            &mixed_data,
                                            width,
                                            height,
                                            &BarConfig::new()
                                                .fill_color(scheme.color(2))
                                                .bar_gap(4.0),
                                            app.renderer_selection(),
                                        )),
                                )
                                .child(render_axis(
                                    &mixed_x_scale,
                                    &AxisConfig::bottom().with_ticks(5),
                                    width,
                                    &theme,
                                )),
                        ),
                ),
        )
        // Grouped bar chart
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .mb_2()
                        .child("Grouped Bar Chart (Quarterly Sales by Product)"),
                )
                .child(
                    div()
                        .flex()
                        .child(render_axis(
                            &grouped_y_scale,
                            &AxisConfig::left().with_ticks(6),
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
                                        .child(render_grouped_bars_selected(
                                            &grouped_y_scale,
                                            &grouped_data,
                                            &grouped_meta,
                                            width,
                                            height,
                                            &GroupedBarConfig::new()
                                                .color_scheme(scheme.clone())
                                                .group_gap(16.0)
                                                .bar_gap(2.0)
                                                .opacity(0.9),
                                            app.renderer_selection(),
                                        )),
                                )
                                // Legend for grouped bars
                                .child(div().flex().gap_4().mt_2().justify_center().children(
                                    grouped_meta.series.iter().enumerate().map(|(i, name)| {
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .w(px(12.0))
                                                    .h(px(12.0))
                                                    .rounded(px(2.0))
                                                    .bg(scheme.color(i).to_rgba()),
                                            )
                                            .child(div().text_xs().child(name.clone()))
                                    }),
                                )),
                        ),
                ),
        )
}

use super::ShowcaseApp;

fn render_bars_selected<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[BarDatum],
    width: f32,
    height: f32,
    config: &BarConfig,
    selection: (Renderer2D, VelloBackend, &'static str),
) -> AnyElement
where
    XS: d3rs::scale::Scale<f64, f64> + Clone + 'static,
    YS: d3rs::scale::Scale<f64, f64> + Clone + 'static,
{
    let config = config
        .clone()
        .renderer_2d(selection.0)
        .vello_backend(selection.1);
    render_bars_with_config(x_scale, y_scale, data, width, height, &config)
}

fn render_grouped_bars_selected<YS>(
    y_scale: &YS,
    data: &[GroupedBarDatum],
    meta: &GroupedBarMeta,
    width: f32,
    height: f32,
    config: &GroupedBarConfig,
    selection: (Renderer2D, VelloBackend, &'static str),
) -> AnyElement
where
    YS: d3rs::scale::Scale<f64, f64> + Clone + 'static,
{
    let config = config
        .clone()
        .renderer_2d(selection.0)
        .vello_backend(selection.1);
    render_grouped_bars_with_config(y_scale, data, meta, width, height, &config)
}
