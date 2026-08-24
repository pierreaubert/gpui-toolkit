use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::color::{ColorScheme, D3Color};
use d3rs::gpu2d::{LodScatterConfig, render_lod_scatter};
use d3rs::grid::{GridConfig, render_grid};
use d3rs::lod::LodBounds;
use d3rs::render2d::{Renderer2D, VelloBackend};
use d3rs::scale::LinearScale;
use d3rs::shape::render_scatter_selected as render_scatter_with_config;
use d3rs::shape::{ScatterConfig, ScatterPoint};
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
                .child(format!("Scatter Plots Demo · {}", app.renderer_label())),
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
                        .child("LOD Scatter: Exact Point Tier"),
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
                                        .child(render_lod_scatter(
                                            &x_scale,
                                            &y_scale,
                                            &data1,
                                            &LodScatterConfig::new()
                                                .color(scheme.color(0))
                                                .opacity(0.8)
                                                .direct_point_budget(20_000),
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
        // Large-data density scatter
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .mb_1()
                        .child("LOD Density Scatter (80,000 points)"),
                )
                .child(
                    div()
                        .text_xs()
                        .mb_2()
                        .child("The full dataset is cached once; the renderer composes a screen-sized density grid."),
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
                                        .child(app.lod_scatter.render(
                                            &LodScatterConfig::new()
                                                .color(D3Color::from_hex(0x7c3aed))
                                                .opacity(0.9)
                                                .direct_point_budget(20_000)
                                                .pyramid_dimension(512),
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
                        .child(render_scatter_selected(
                                            &x_scale,
                                            &y_scale,
                                            &cluster1,
                                            &ScatterConfig::new()
                                                .fill_color(scheme.color(4))
                                                .point_radius(5.0)
                                                .stroke_color(D3Color::from_hex(0xffffff))
                                                .stroke_width(1.5),
                            app.renderer_selection(),
                                        ))
                        .child(render_scatter_selected(
                                            &x_scale,
                                            &y_scale,
                                            &cluster2,
                                            &ScatterConfig::new()
                                                .fill_color(scheme.color(6))
                                                .point_radius(5.0)
                                                .stroke_color(D3Color::from_hex(0xffffff))
                                                .stroke_width(1.5),
                            app.renderer_selection(),
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

/// Dedicated navigation target for the screen-bounded rendering examples.
pub fn render_lod(app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let theme = DefaultAxisTheme;
    let width = app.content_width * 0.7;
    let height = (width * 0.5).min(app.content_height * 0.55);
    let (x0, x1) = app.lod_zoom.x_domain();
    let (y0, y1) = app.lod_zoom.y_domain();
    let viewport = LodBounds::new(x0, x1, y0, y1).expect("showcase zoom stays valid");
    let x_scale = LinearScale::new()
        .domain(x0 * 100.0, x1 * 100.0)
        .range(0.0, width as f64);
    let y_scale = LinearScale::new()
        .domain(y0 * 100.0, y1 * 100.0)
        .range(0.0, height as f64);

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Level of Detail / Large Data · Legacy GPU2D"),
        )
        .child(
            div()
                .text_sm()
                .child(format!(
                    "80,000 points are stored in a retained density pyramid. Scroll over the plot to zoom ({:.1}×).",
                    1.0 / (x1 - x0)
                )),
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
                                .overflow_hidden()
                                .on_scroll_wheel(cx.listener(
                                    |this, event: &ScrollWheelEvent, _window, cx| {
                                        cx.stop_propagation();
                                        let delta_y: f32 = match event.delta {
                                            ScrollDelta::Lines(lines) => lines.y,
                                            ScrollDelta::Pixels(pixels) => pixels.y.into(),
                                        };
                                        let factor = 1.15_f64.powf((f64::from(delta_y) / 30.0).clamp(-3.0, 3.0));
                                        let (x0, x1) = this.lod_zoom.x_domain();
                                        let (y0, y1) = this.lod_zoom.y_domain();
                                        let x_span = ((x1 - x0) / factor).clamp(0.002, 1.0);
                                        let y_span = ((y1 - y0) / factor).clamp(0.002, 1.0);
                                        this.lod_zoom.zoom_to(
                                            0.5 - x_span / 2.0,
                                            0.5 + x_span / 2.0,
                                            0.5 - y_span / 2.0,
                                            0.5 + y_span / 2.0,
                                        );
                                        // The scales, axes, and density viewport are
                                        // derived from `lod_zoom` during render.
                                        // Request a new frame after changing it.
                                        cx.notify();
                                    },
                                ))
                                .child(render_grid(
                                    &x_scale,
                                    &y_scale,
                                    &GridConfig::with_lines(),
                                    width,
                                    height,
                                    &theme,
                                ))
                                .child(app.lod_scatter.render(
                                    &LodScatterConfig::new()
                                        .color(D3Color::from_hex(0x7c3aed))
                                        .opacity(0.9)
                                        .direct_point_budget(20_000)
                                        .viewport(viewport),
                                )),
                        )
                        .child(render_axis(
                            &x_scale,
                            &AxisConfig::bottom().with_ticks(5),
                            width,
                            &theme,
                        )),
                ),
        )
}

use super::ShowcaseApp;

fn render_scatter_selected<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[ScatterPoint],
    config: &ScatterConfig,
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
    render_scatter_with_config(x_scale, y_scale, data, &config)
}
