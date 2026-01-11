use d3rs::prelude::*;
use d3rs::surface::{ColorScaleType, SurfaceConfig, SurfaceData, render_surface};
use gpui::*;

pub fn render(_app: &mut ShowcaseApp, _cx: &mut Context<ShowcaseApp>) -> Div {
    // Logarithmic frequency response surface (20 Hz to 20 kHz)
    let freq_response = SurfaceData::from_z_function_logx(
        (20.0, 20000.0), // X: Frequency (logarithmic)
        (0.0, 1.0),      // Y: Time/Channel (linear)
        80,              // Resolution
        |freq, time| {
            // Simulated frequency response with rolloffs and time variation
            let base_response = if freq < 100.0 {
                -12.0 * (100.0 - freq) / 80.0 // Low frequency rolloff
            } else if freq > 10000.0 {
                -6.0 * (freq - 10000.0) / 10000.0 // High frequency rolloff
            } else {
                0.0 // Flat response
            };

            // Add time-varying component (simulated transient response)
            let transient = -2.0 * (1.0 - time).powi(2);

            base_response + transient
        },
    );

    // 2D frequency domain surface (both axes logarithmic)
    let freq_2d = SurfaceData::from_z_function_logxy(
        (100.0, 10000.0), // X: Frequency 1 (log)
        (100.0, 10000.0), // Y: Frequency 2 (log)
        40,
        |fx, fy| {
            // Interaction between two frequency components
            let product = (fx * fy).sqrt();
            let z = if product < 1000.0 {
                -8.0 * (1000.0 - product) / 900.0
            } else if product > 3000.0 {
                -4.0 * (product - 3000.0) / 7000.0
            } else {
                0.0
            };

            // Add some ripple
            z + 0.5 * ((fx / 1000.0).ln() * (fy / 1000.0).ln()).sin()
        },
    );

    // Spectral analysis surface (log frequency Y-axis)
    let spectral = SurfaceData::from_z_function_logy(
        (0.0, 1.0),      // X: Time (linear)
        (20.0, 20000.0), // Y: Frequency (log)
        60,
        |time, freq| {
            // Simulated spectrogram data
            let fundamental = 440.0; // A4 note
            let harmonic1 = (freq - fundamental).abs();
            let harmonic2 = (freq - fundamental * 2.0).abs();
            let harmonic3 = (freq - fundamental * 3.0).abs();

            let energy = (-harmonic1 / 50.0).exp() * 0.8
                + (-harmonic2 / 50.0).exp() * 0.4
                + (-harmonic3 / 50.0).exp() * 0.2;

            // Decay over time
            energy * (1.0 - 0.7 * time)
        },
    );

    div()
        .flex()
        .flex_col()
        .gap_8()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("3D Surface Plots with Logarithmic Scales"),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(0x666666))
                .child("Demonstrating logarithmic axis sampling for frequency domain visualizations"),
        )
        // First row: Frequency response (log X)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Frequency Response (Logarithmic X-axis)"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x666666))
                                .child("X-axis: 20 Hz → 20 kHz (logarithmic) | Y-axis: Time (linear)"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x888888))
                                .child("Shows frequency response with low and high frequency rolloffs"),
                        ),
                )
                .child(
                    div()
                        .w(px(800.0))
                        .h(px(450.0))
                        .bg(rgb(0xf5f5f5))
                        .border_1()
                        .border_color(rgb(0xcccccc))
                        .child(
                            render_surface(
                                &freq_response,
                                SurfaceConfig::new()
                                    .isometric()
                                    .rotation(30.0, 45.0)
                                    .color_scale(ColorScaleType::Viridis)
                                    .opacity(0.85)
                                    .wireframe(true)
                                    .wireframe_opacity(0.3)
                                    .wireframe_color(D3Color::rgb(0, 0, 0))
                                    .scale(1.2),
                                800.0,
                                450.0,
                            ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .text_xs()
                        .text_color(rgb(0x666666))
                        .child("Method: SurfaceData::from_z_function_logx()")
                        .child("•")
                        .child("Color scale: Viridis (magnitude in dB)")
                        .child("•")
                        .child("Wireframe enabled"),
                ),
        )
        // Second row: 2D frequency domain (log X and Y)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("2D Frequency Domain (Both Axes Logarithmic)"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x666666))
                                .child("X-axis: 100 Hz → 10 kHz (log) | Y-axis: 100 Hz → 10 kHz (log)"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x888888))
                                .child("Interaction between two frequency components"),
                        ),
                )
                .child(
                    div()
                        .w(px(800.0))
                        .h(px(450.0))
                        .bg(rgb(0xf5f5f5))
                        .border_1()
                        .border_color(rgb(0xcccccc))
                        .child(
                            render_surface(
                                &freq_2d,
                                SurfaceConfig::new()
                                    .isometric()
                                    .rotation(35.0, 50.0)
                                    .color_scale(ColorScaleType::Heat)
                                    .opacity(0.9)
                                    .wireframe(true)
                                    .wireframe_opacity(0.2)
                                    .wireframe_color(D3Color::rgb(100, 100, 100))
                                    .scale(1.3),
                                800.0,
                                450.0,
                            ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .text_xs()
                        .text_color(rgb(0x666666))
                        .child("Method: SurfaceData::from_z_function_logxy()")
                        .child("•")
                        .child("Color scale: Heat (blue → white → red)")
                        .child("•")
                        .child("Resolution: 40×40"),
                ),
        )
        // Third row: Spectrogram (log Y)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Spectral Analysis (Logarithmic Y-axis)"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x666666))
                                .child("X-axis: Time (linear) | Y-axis: 20 Hz → 20 kHz (logarithmic)"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x888888))
                                .child("Simulated spectrogram showing harmonic decay at 440 Hz (A4 note)"),
                        ),
                )
                .child(
                    div()
                        .w(px(800.0))
                        .h(px(450.0))
                        .bg(rgb(0xf5f5f5))
                        .border_1()
                        .border_color(rgb(0xcccccc))
                        .child(
                            render_surface(
                                &spectral,
                                SurfaceConfig::new()
                                    .isometric()
                                    .rotation(25.0, 40.0)
                                    .color_scale(ColorScaleType::Spectral)
                                    .opacity(0.95)
                                    .wireframe(false)
                                    .lighting(true)
                                    .ambient(0.5)
                                    .diffuse(0.5)
                                    .scale(1.4),
                                800.0,
                                450.0,
                            ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .text_xs()
                        .text_color(rgb(0x666666))
                        .child("Method: SurfaceData::from_z_function_logy()")
                        .child("•")
                        .child("Color scale: Spectral (rainbow)")
                        .child("•")
                        .child("Lighting enabled"),
                ),
        )
        // Info section
        .child(
            div()
                .mt_6()
                .p_4()
                .bg(rgb(0xf0f8ff))
                .border_1()
                .border_color(rgb(0xb0d4f1))
                .rounded_lg()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x0066cc))
                        .child("About Logarithmic Scales"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x333333))
                        .child("Logarithmic axis sampling distributes points evenly in log space, making it ideal for visualizing data that spans multiple orders of magnitude, such as audio frequency responses (20 Hz to 20 kHz)."),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x666666))
                        .mt_2()
                        .child("Available methods: from_function_logx(), from_function_logy(), from_function_logxy()"),
                ),
        )
}

use super::ShowcaseApp;
