//! Logarithmic Scale Demo - demonstrates log scale support in gpui-px
//!
//! Run with: cargo run --example logscale_demo --features gpui

use gpui::*;
use gpui_px::*;
use gpui_ui_kit::{MiniApp, MiniAppConfig};

fn main() {
    MiniApp::run(
        MiniAppConfig::new("gpui-px Logarithmic Scale Demo").size(900.0, 1200.0),
        |cx| cx.new(|_| LogScaleDemo),
    );
}

struct LogScaleDemo;

impl Render for LogScaleDemo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Generate logarithmic data for scatter plot
        let log_x: Vec<f64> = vec![10.0, 100.0, 1000.0, 10000.0];
        let log_y: Vec<f64> = vec![1.0, 10.0, 100.0, 1000.0];

        // Generate frequency response data (20 Hz to 20 kHz)
        let freq_x: Vec<f64> = (0..60)
            .map(|i| 20.0 * 10_f64.powf(i as f64 / 20.0))
            .collect();
        let freq_y: Vec<f64> = freq_x
            .iter()
            .map(|&f| {
                // Simulated frequency response with rolloffs
                if f < 100.0 {
                    -12.0 * (100.0 - f) / 80.0 // Low frequency rolloff
                } else if f > 5000.0 {
                    -6.0 * (f - 5000.0) / 15000.0 // High frequency rolloff
                } else {
                    0.0 // Flat response
                }
            })
            .collect();

        // Bar chart data
        let bar_categories: Vec<&str> = vec!["10", "100", "1K", "10K", "100K"];
        let bar_values: Vec<f64> = vec![10.0, 100.0, 1000.0, 10000.0, 100000.0];

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .p_8()
            .gap_8()
            // Header
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_3xl()
                            .font_weight(FontWeight::BOLD)
                            .child("Logarithmic Scale Support"),
                    )
                    .child(
                        div()
                            .text_base()
                            .text_color(rgb(0x666666))
                            .child("Demonstrating logarithmic axis scaling for scatter, line, and bar charts"),
                    ),
            )
            // Example 1: Scatter Plot with Log-Log Scale
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("1. Scatter Plot - Log-Log Scale"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child("Both X and Y axes use logarithmic scaling. Ideal for power-law relationships."),
                    )
                    .child(
                        scatter(&log_x, &log_y)
                            .title("Log-Log Scatter Plot")
                            .color(0xe377c2)
                            .x_scale(ScaleType::Log)
                            .y_scale(ScaleType::Log)
                            .point_radius(10.0)
                            .opacity(0.8)
                            .size(800.0, 400.0)
                            .build()
                            .unwrap(),
                    )
                    .child(
                        div()
                            .p_3()
                            .bg(rgb(0xf5f5f5))
                            .rounded_md()
                            .font_family("Monaco")
                            .text_sm()
                            .text_color(rgb(0x333333))
                            .child("scatter(&x, &y)\n    .x_scale(ScaleType::Log)\n    .y_scale(ScaleType::Log)\n    .build()?"),
                    ),
            )
            // Example 2: Line Chart with Log X-Axis
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("2. Line Chart - Logarithmic X-Axis"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child("Frequency response plot with logarithmic frequency axis (20 Hz to 20 kHz) - standard in audio engineering."),
                    )
                    .child(
                        line(&freq_x, &freq_y)
                            .title("Frequency Response")
                            .color(0x1f77b4)
                            .x_scale(ScaleType::Log)
                            .stroke_width(2.5)
                            .show_points(false)
                            .size(800.0, 400.0)
                            .build()
                            .unwrap(),
                    )
                    .child(
                        div()
                            .p_3()
                            .bg(rgb(0xf5f5f5))
                            .rounded_md()
                            .font_family("Monaco")
                            .text_sm()
                            .text_color(rgb(0x333333))
                            .child("line(&frequency, &magnitude_db)\n    .x_scale(ScaleType::Log)\n    .build()?"),
                    ),
            )
            // Example 3: Bar Chart with Log Y-Axis
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("3. Bar Chart - Logarithmic Y-Axis"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child("Bar chart with logarithmic value axis - useful for comparing values across multiple orders of magnitude."),
                    )
                    .child(
                        bar(&bar_categories, &bar_values)
                            .title("Logarithmic Values")
                            .color(0x2ca02c)
                            .y_scale(ScaleType::Log)
                            .opacity(0.85)
                            .size(800.0, 400.0)
                            .build()
                            .unwrap(),
                    )
                    .child(
                        div()
                            .p_3()
                            .bg(rgb(0xf5f5f5))
                            .rounded_md()
                            .font_family("Monaco")
                            .text_sm()
                            .text_color(rgb(0x333333))
                            .child("bar(&categories, &values)\n    .y_scale(ScaleType::Log)\n    .build()?"),
                    ),
            )
            // Notes section
            .child(
                div()
                    .mt_4()
                    .p_4()
                    .bg(rgb(0xfef3c7))
                    .border_1()
                    .border_color(rgb(0xfbbf24))
                    .rounded_md()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0x92400e))
                            .child("Important Notes about Logarithmic Scales:"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x78350f))
                            .child("• All values must be positive - zero and negative values will cause validation errors"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x78350f))
                            .child("• Each decade (10x increase) gets equal spacing on the axis"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x78350f))
                            .child("• Logarithmic scales are ideal for data spanning multiple orders of magnitude"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x78350f))
                            .child("• Common use cases: frequency domain (audio/RF), power measurements, exponential growth"),
                    ),
            )
            // Supported chart types
            .child(
                div()
                    .mt_4()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Supported Chart Types:"),
                    )
                    .child(
                        div()
                            .ml_4()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .child("• Scatter: Both X and Y axes can be logarithmic"),
                            )
                            .child(div().text_sm().child("• Line: Both X and Y axes can be logarithmic"))
                            .child(div().text_sm().child("• Bar: Only Y-axis (values) can be logarithmic"))
                            .child(div().text_sm().child("• Heatmap: Both X and Y axes support logarithmic scaling"))
                            .child(div().text_sm().child("• Contour/Isoline: Both X and Y axes support logarithmic scaling")),
                    ),
            )
    }
}
