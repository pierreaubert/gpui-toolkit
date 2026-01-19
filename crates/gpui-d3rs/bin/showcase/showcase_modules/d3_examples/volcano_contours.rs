//! Volcano Contours - D3.js Example Port
//!
//! This example demonstrates filled contour rendering of volcanic terrain,
//! ported from: https://observablehq.com/@d3/volcano-contours/2
//!
//! The example shows both:
//! 1. **Low-level API**: Direct use of ContourGenerator, scales, and manual rendering
//! 2. **High-level API**: Using render_contour() and render_heatmap() helper functions

use super::volcano_data::{VOLCANO_HEIGHT, VOLCANO_WIDTH, generate_volcano_data};
use crate::ShowcaseApp;
use d3rs::contour::ContourGenerator;
use d3rs::prelude::*;
use d3rs::shape::contour::{
    ContourConfig, HeatmapData, render_contour, render_heatmap, turbo_color_scale,
    viridis_color_scale,
};
use gpui::*;
use gpui_ui_kit::Slider;

/// Color scale options for the visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VolcanoColorScale {
    #[default]
    Turbo,
    Viridis,
}

impl VolcanoColorScale {
    fn label(&self) -> &'static str {
        match self {
            Self::Turbo => "Turbo",
            Self::Viridis => "Viridis",
        }
    }

    fn next(&self) -> Self {
        match self {
            Self::Turbo => Self::Viridis,
            Self::Viridis => Self::Turbo,
        }
    }
}

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let entity = cx.entity().clone();

    // Get volcano data
    let values = generate_volcano_data();
    let (min_elev, max_elev) = {
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        (min, max)
    };

    // State values (you'd add these to ShowcaseApp)
    let num_thresholds = app.volcano_num_thresholds;
    let color_scale_type = app.volcano_color_scale;
    let show_stroke = app.volcano_show_stroke;

    // Generate thresholds evenly spaced across the elevation range
    let thresholds: Vec<f64> = (0..num_thresholds)
        .map(|i| min_elev + (max_elev - min_elev) * (i as f64 / num_thresholds as f64))
        .collect();

    // Create contour generator
    let generator = ContourGenerator::new(VOLCANO_WIDTH, VOLCANO_HEIGHT);
    let contours = generator.contours(&values, &thresholds);

    // Scales for positioning
    let plot_width = 400.0;
    let plot_height = (VOLCANO_HEIGHT as f64 / VOLCANO_WIDTH as f64 * plot_width).round();

    let x_scale = LinearScale::new()
        .domain(0.0, VOLCANO_WIDTH as f64)
        .range(0.0, plot_width);
    let y_scale = LinearScale::new()
        .domain(0.0, VOLCANO_HEIGHT as f64)
        .range(0.0, plot_height);

    // Color configuration
    let config = match color_scale_type {
        VolcanoColorScale::Turbo => ContourConfig::new()
            .stroke_width(if show_stroke { 0.5 } else { 0.0 })
            .stroke_opacity(if show_stroke { 0.3 } else { 0.0 })
            .fill(true)
            .fill_opacity(1.0)
            .color_scale(turbo_color_scale()),
        VolcanoColorScale::Viridis => ContourConfig::new()
            .stroke_width(if show_stroke { 0.5 } else { 0.0 })
            .stroke_opacity(if show_stroke { 0.3 } else { 0.0 })
            .fill(true)
            .fill_opacity(1.0)
            .color_scale(viridis_color_scale()),
    };

    // Heatmap data for high-level API demo
    let heatmap_x: Vec<f64> = (0..VOLCANO_WIDTH).map(|i| i as f64).collect();
    let heatmap_y: Vec<f64> = (0..VOLCANO_HEIGHT).map(|i| i as f64).collect();
    let heatmap_data = HeatmapData::new(heatmap_x, heatmap_y, values.clone());

    div()
        .flex()
        .flex_col()
        .gap_6()
        // Title
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::BOLD)
                        .child("Volcano Contours"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x666666))
                        .child("Ported from Observable: d3/volcano-contours"),
                ),
        )
        // Main content: two columns
        .child(
            div()
                .flex()
                .gap_8()
                // Left column: Visualizations
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_6()
                        // High-level API: render_contour()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_3()
                                        .child(
                                            div()
                                                .text_lg()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child("High-level API: render_contour()"),
                                        )
                                        .child(
                                            div()
                                                .px_2()
                                                .py_1()
                                                .bg(rgb(0x28a745))
                                                .rounded_md()
                                                .text_xs()
                                                .text_color(rgb(0xffffff))
                                                .child("Recommended"),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x888888))
                                        .font_family("monospace")
                                        .child(
                                            "render_contour(contours, &x_scale, &y_scale, &config)",
                                        ),
                                )
                                .child(
                                    div()
                                        .w(px(plot_width as f32))
                                        .h(px(plot_height as f32))
                                        .bg(rgb(0x1a1a1a))
                                        .border_1()
                                        .border_color(rgb(0x333333))
                                        .rounded_md()
                                        .overflow_hidden()
                                        .child(
                                            render_contour(
                                                contours.clone(),
                                                &x_scale,
                                                &y_scale,
                                                &config,
                                            )
                                            .value_range(min_elev, max_elev)
                                            .height(px(plot_height as f32)),
                                        ),
                                ),
                        )
                        // High-level API: render_heatmap()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("High-level API: render_heatmap()"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x888888))
                                        .font_family("monospace")
                                        .child("render_heatmap(data, &x_scale, &y_scale, &config)"),
                                )
                                .child(
                                    div()
                                        .w(px(plot_width as f32))
                                        .h(px(plot_height as f32))
                                        .bg(rgb(0x1a1a1a))
                                        .border_1()
                                        .border_color(rgb(0x333333))
                                        .rounded_md()
                                        .overflow_hidden()
                                        .child(
                                            render_heatmap(
                                                heatmap_data,
                                                &x_scale,
                                                &y_scale,
                                                &config,
                                            )
                                            .value_range(min_elev, max_elev)
                                            .height(px(plot_height as f32)),
                                        ),
                                ),
                        ),
                )
                // Right column: Controls and code
                .child(
                    div()
                        .w(px(320.0))
                        .flex()
                        .flex_col()
                        .gap_4()
                        // Controls panel
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .p_4()
                                .bg(rgb(0xf8f8f8))
                                .border_1()
                                .border_color(rgb(0xe0e0e0))
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(0x333333))
                                        .child("Controls"),
                                )
                                // Number of contour levels
                                .child({
                                    let entity = entity.clone();
                                    Slider::new("volcano-thresholds")
                                        .label("Contour Levels")
                                        .value(num_thresholds as f32)
                                        .min(5.0)
                                        .max(30.0)
                                        .step(1.0)
                                        .show_value(true)
                                        .width(250.0)
                                        .on_change(move |value, _window, cx| {
                                            entity.update(cx, |this, _| {
                                                this.volcano_num_thresholds = value as usize;
                                            });
                                        })
                                })
                                // Color scale toggle
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(0x555555))
                                                .child("Color Scale"),
                                        )
                                        .child({
                                            let entity = entity.clone();
                                            div()
                                                .id("color-scale-toggle")
                                                .px_3()
                                                .py_1()
                                                .bg(rgb(0x007acc))
                                                .hover(|s| s.bg(rgb(0x005a9e)))
                                                .rounded_md()
                                                .cursor_pointer()
                                                .text_sm()
                                                .text_color(rgb(0xffffff))
                                                .child(color_scale_type.label())
                                                .on_click(move |_, _window, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.volcano_color_scale =
                                                            this.volcano_color_scale.next();
                                                    });
                                                })
                                        }),
                                )
                                // Show stroke toggle
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(0x555555))
                                                .child("Show Contour Lines"),
                                        )
                                        .child({
                                            let entity = entity.clone();
                                            let bg = if show_stroke {
                                                rgb(0x28a745)
                                            } else {
                                                rgb(0xcccccc)
                                            };
                                            div()
                                                .id("stroke-toggle")
                                                .px_3()
                                                .py_1()
                                                .bg(bg)
                                                .hover(|s| s.opacity(0.8))
                                                .rounded_md()
                                                .cursor_pointer()
                                                .text_sm()
                                                .text_color(rgb(0xffffff))
                                                .child(if show_stroke { "On" } else { "Off" })
                                                .on_click(move |_, _window, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.volcano_show_stroke =
                                                            !this.volcano_show_stroke;
                                                    });
                                                })
                                        }),
                                ),
                        )
                        // Statistics panel
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .p_4()
                                .bg(rgb(0xffffff))
                                .border_1()
                                .border_color(rgb(0xe0e0e0))
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(0x888888))
                                        .child("DATA INFO"),
                                )
                                .child(div().text_sm().text_color(rgb(0x333333)).child(format!(
                                    "Grid: {}x{} ({} points)",
                                    VOLCANO_WIDTH,
                                    VOLCANO_HEIGHT,
                                    VOLCANO_WIDTH * VOLCANO_HEIGHT
                                )))
                                .child(div().text_sm().text_color(rgb(0x333333)).child(format!(
                                    "Elevation: {:.0}m - {:.0}m",
                                    min_elev, max_elev
                                )))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(0x333333))
                                        .child(format!("Contour levels: {}", num_thresholds)),
                                )
                                .child(div().text_sm().text_color(rgb(0x333333)).child(format!(
                                    "Total contours: {}",
                                    contours.iter().map(|c| c.coordinates.len()).sum::<usize>()
                                ))),
                        )
                        // Low-level API code sample
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .p_4()
                                .bg(rgb(0x1e1e1e))
                                .border_1()
                                .border_color(rgb(0x333333))
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(0x888888))
                                        .child("LOW-LEVEL API USAGE"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .font_family("monospace")
                                        .text_color(rgb(0xd4d4d4))
                                        .child(
                                            r#"// 1. Generate data
let values = generate_volcano_data();

// 2. Create contour generator
let gen = ContourGenerator::new(W, H);
let contours = gen.contours(&values, &thresholds);

// 3. Set up scales
let x_scale = LinearScale::new()
    .domain(0.0, W as f64)
    .range(0.0, plot_width);

// 4. Configure rendering
let config = ContourConfig::new()
    .fill(true)
    .color_scale(turbo_color_scale());

// 5. Render
render_contour(contours, &x_scale, &y_scale, &config)"#,
                                        ),
                                ),
                        ),
                ),
        )
        // Color scale legend
        .child(render_color_legend(color_scale_type, min_elev, max_elev))
}

/// Render a color scale legend
fn render_color_legend(scale_type: VolcanoColorScale, min_val: f64, max_val: f64) -> Div {
    let num_steps = 20;
    let step_width = 20.0;

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(0x555555))
                .child(format!("Color Legend ({} scale)", scale_type.label())),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x666666))
                        .child(format!("{:.0}m", min_val)),
                )
                .child(
                    div()
                        .flex()
                        .h(px(20.0))
                        .rounded_sm()
                        .overflow_hidden()
                        .children((0..num_steps).map(|i| {
                            let t = i as f64 / (num_steps - 1) as f64;
                            let color = match scale_type {
                                VolcanoColorScale::Turbo => {
                                    d3rs::shape::contour::turbo_color_scale()(t)
                                }
                                VolcanoColorScale::Viridis => {
                                    d3rs::shape::contour::viridis_color_scale()(t)
                                }
                            };
                            div().w(px(step_width as f32)).h_full().bg(gpui::rgba(
                                ((color.r * 255.0) as u32) << 24
                                    | ((color.g * 255.0) as u32) << 16
                                    | ((color.b * 255.0) as u32) << 8
                                    | 0xff,
                            ))
                        })),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x666666))
                        .child(format!("{:.0}m", max_val)),
                ),
        )
}
