//! GPU-accelerated contour rendering demo
//!
//! Run with: cargo run --example gpu2d_contour_demo --features gpu-2d

use d3rs::contour::ContourGenerator;
use d3rs::gpu2d::{
    ContourConfig, HeatmapData, magma_color_scale, render_contour, render_contour_bands,
    render_heatmap, turbo_color_scale, viridis_color_scale,
};
use d3rs::scale::LinearScale;
use gpui::*;
use std::sync::Arc;

struct DemoView;

impl Render for DemoView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Generate test data: a 2D Gaussian + ripple function
        let width = 50;
        let height = 50;
        let mut values = Vec::with_capacity(width * height);

        for y in 0..height {
            for x in 0..width {
                let fx = (x as f64 / width as f64) * 2.0 - 1.0;
                let fy = (y as f64 / height as f64) * 2.0 - 1.0;

                // Gaussian peak + ripple
                let r2 = fx * fx + fy * fy;
                let gaussian = (-r2 * 3.0).exp();
                let ripple = (r2.sqrt() * 10.0).sin() * 0.3;
                let value = gaussian + ripple;

                values.push(value);
            }
        }

        // Create scales
        let x_scale = LinearScale::new()
            .domain(0.0, width as f64)
            .range(0.0, 400.0);
        let y_scale = LinearScale::new()
            .domain(0.0, height as f64)
            .range(400.0, 0.0);

        // Generate contours
        let generator = ContourGenerator::new(width, height);
        let thresholds: Vec<f64> = (-5..=10).map(|i| i as f64 * 0.1).collect();
        let contours: Arc<[_]> = generator.contours(&values, &thresholds).into();

        // Generate contour bands
        let bands: Arc<[_]> = generator.contour_bands(&values, &thresholds).into();

        // Create heatmap data
        let x_values: Vec<f64> = (0..width).map(|x| x as f64).collect();
        let y_values: Vec<f64> = (0..height).map(|y| y as f64).collect();
        let heatmap_data = HeatmapData::new(x_values, y_values, values.clone());

        // Configs
        let contour_config = ContourConfig::new()
            .stroke_width(1.5)
            .stroke_opacity(0.8)
            .fill(false)
            .color_scale(turbo_color_scale());

        let band_config = ContourConfig::new()
            .fill(true)
            .fill_opacity(0.9)
            .color_scale(viridis_color_scale());

        let heatmap_config = ContourConfig::new()
            .fill(true)
            .fill_opacity(1.0)
            .color_scale(magma_color_scale());

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1a1a1a))
            .p_4()
            .gap_4()
            .child(
                div()
                    .text_xl()
                    .text_color(rgb(0xffffff))
                    .child("GPU-Accelerated Contour Rendering Demo"),
            )
            .child(
                div()
                    .flex_row()
                    .gap_4()
                    // Contour lines
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xaaaaaa))
                                    .child("Contour Lines (GPU)"),
                            )
                            .child(
                                div()
                                    .h(px(200.0))
                                    .border_1()
                                    .border_color(rgb(0x404040))
                                    .rounded_md()
                                    .overflow_hidden()
                                    .child(render_contour(
                                        contours.clone(),
                                        &x_scale,
                                        &y_scale,
                                        &contour_config,
                                    )),
                            ),
                    )
                    // Filled bands
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xaaaaaa))
                                    .child("Filled Contour Bands (GPU)"),
                            )
                            .child(
                                div()
                                    .h(px(200.0))
                                    .border_1()
                                    .border_color(rgb(0x404040))
                                    .rounded_md()
                                    .overflow_hidden()
                                    .child(render_contour_bands(
                                        bands.clone(),
                                        &x_scale,
                                        &y_scale,
                                        &band_config,
                                    )),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_row()
                    .gap_4()
                    // Heatmap
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xaaaaaa))
                                    .child("Heatmap (GPU)"),
                            )
                            .child(
                                div()
                                    .h(px(200.0))
                                    .border_1()
                                    .border_color(rgb(0x404040))
                                    .rounded_md()
                                    .overflow_hidden()
                                    .child(render_heatmap(
                                        heatmap_data,
                                        &x_scale,
                                        &y_scale,
                                        &heatmap_config,
                                    )),
                            ),
                    )
                    // Contour lines overlay on bands
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xaaaaaa))
                                    .child("Bands + Lines Overlay (GPU)"),
                            )
                            .child({
                                let line_config = ContourConfig::new()
                                    .stroke_width(1.0)
                                    .stroke_opacity(0.6)
                                    .fill(false)
                                    .stroke_color(d3rs::color::D3Color::from_hex(0xffffff));

                                div()
                                    .h(px(200.0))
                                    .border_1()
                                    .border_color(rgb(0x404040))
                                    .rounded_md()
                                    .overflow_hidden()
                                    .relative()
                                    .child(render_contour_bands(
                                        bands.clone(),
                                        &x_scale,
                                        &y_scale,
                                        &band_config,
                                    ))
                                    .child(render_contour(
                                        contours.clone(),
                                        &x_scale,
                                        &y_scale,
                                        &line_config,
                                    ))
                            }),
                    ),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(900.0), px(550.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("GPU 2D Contour Demo".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| DemoView),
        )
        .unwrap();

        cx.activate(true);
    });
}
