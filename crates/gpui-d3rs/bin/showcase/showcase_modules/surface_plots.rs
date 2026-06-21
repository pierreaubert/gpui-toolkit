use super::ShowcaseApp;
use d3rs::prelude::*;
use d3rs::surface::{ColorScaleType, SurfaceConfig, SurfaceData, render_surface};
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

/// Parameters that determine the generated surface data.
///
/// The surface functions and ranges are fixed, so the key currently only
/// captures the grid resolutions. If the analytic functions are ever changed,
/// additional fields should be added here and to [`SURFACE_PLOT_CACHE_KEY`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfacePlotCacheKey {
    pub freq_response_res: usize,
    pub freq_2d_res: usize,
    pub spectral_res: usize,
}

/// The current surface-plot generation parameters.
pub const SURFACE_PLOT_CACHE_KEY: SurfacePlotCacheKey = SurfacePlotCacheKey {
    freq_response_res: 80,
    freq_2d_res: 40,
    spectral_res: 60,
};

/// Cached surface data for the surface-plots showcase.
///
/// The data is generated from fixed analytic functions, so it can be computed
/// once and reused across renders and resizes.
pub struct SurfacePlotCache {
    pub key: SurfacePlotCacheKey,
    pub freq_response: SurfaceData,
    pub freq_2d: SurfaceData,
    pub spectral: SurfaceData,
}

pub fn build_surface_plot_cache(key: SurfacePlotCacheKey) -> SurfacePlotCache {
    // Logarithmic frequency response surface (20 Hz to 20 kHz)
    let freq_response = SurfaceData::from_z_function_logx(
        (20.0, 20000.0),       // X: Frequency (logarithmic)
        (0.0, 1.0),            // Y: Time/Channel (linear)
        key.freq_response_res, // Resolution
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
        key.freq_2d_res,
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
        key.spectral_res,
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

    SurfacePlotCache {
        key,
        freq_response,
        freq_2d,
        spectral,
    }
}

fn camera_for_plot(this: &mut ShowcaseApp, index: usize) -> &mut d3rs::surface::SurfaceCamera {
    match index {
        0 => &mut this.surface_plot_camera_freq_response,
        1 => &mut this.surface_plot_camera_freq_2d,
        2 => &mut this.surface_plot_camera_spectral,
        _ => unreachable!(),
    }
}

fn interactive_surface_plot(
    app: &mut ShowcaseApp,
    cx: &mut Context<ShowcaseApp>,
    data: &SurfaceData,
    mut config: SurfaceConfig,
    width: f32,
    height: f32,
    plot_index: usize,
) -> impl IntoElement {
    config.camera = camera_for_plot(app, plot_index).camera.clone();

    div()
        .w(px(width))
        .h(px(height))
        .cursor_crosshair()
        .child(render_surface(data, config, width, height))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, _window, _cx| {
                this.surface_plot_drag = Some((plot_index, event.position));
            }),
        )
        .on_mouse_move(
            cx.listener(move |this, event: &MouseMoveEvent, _window, cx| {
                if let Some((idx, start)) = this.surface_plot_drag {
                    if idx == plot_index {
                        let dx: f64 = (event.position.x - start.x).into();
                        let dy: f64 = (event.position.y - start.y).into();
                        camera_for_plot(this, plot_index).apply_drag(dx, dy);
                        this.surface_plot_drag = Some((plot_index, event.position));
                        cx.notify();
                    }
                }
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |this, _event, _window, _cx| {
                if this
                    .surface_plot_drag
                    .map_or(false, |(idx, _)| idx == plot_index)
                {
                    this.surface_plot_drag = None;
                }
            }),
        )
        .on_scroll_wheel(
            cx.listener(move |this, event: &ScrollWheelEvent, _window, cx| {
                let delta_y: f32 = match event.delta {
                    ScrollDelta::Lines(lines) => lines.y,
                    ScrollDelta::Pixels(pixels) => pixels.y.into(),
                };
                camera_for_plot(this, plot_index).apply_scroll(f64::from(delta_y) / 50.0);
                cx.notify();
            }),
        )
}

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let width = app.content_width;
    let height = (width * 0.56).min(app.content_height * 0.6);

    // Ensure data is cached locally so render never falls back to generation.
    app.ensure_surface_plot_cache();
    let (freq_response, freq_2d, spectral) = {
        let cache = app.surface_plot_cache.as_ref().unwrap();
        (
            cache.freq_response.clone(),
            cache.freq_2d.clone(),
            cache.spectral.clone(),
        )
    };

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
                                .child("X-axis: 20 Hz → 20 kHz (logarithmic) | Y-axis: Time (linear)"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .child("Shows frequency response with low and high frequency rolloffs"),
                        ),
                )
                .child(
                    div()
                        .bg(ui_theme.surface)
                        .border_1()
                        .border_color(ui_theme.border)
                        .child(interactive_surface_plot(
                            app,
                            cx,
                            &freq_response,
                            SurfaceConfig::new()
                                .isometric()
                                .color_scale(ColorScaleType::Viridis)
                                .opacity(0.85)
                                .wireframe(true)
                                .wireframe_opacity(0.3)
                                .wireframe_color(D3Color::rgb(0, 0, 0))
                                .scale(1.2),
                            width,
                            height,
                            0,
                        )),
                )
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .text_xs()
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
                                .child("X-axis: 100 Hz → 10 kHz (log) | Y-axis: 100 Hz → 10 kHz (log)"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .child("Interaction between two frequency components"),
                        ),
                )
                .child(
                    div()
                        .bg(ui_theme.surface)
                        .border_1()
                        .border_color(ui_theme.border)
                        .child(interactive_surface_plot(
                            app,
                            cx,
                            &freq_2d,
                            SurfaceConfig::new()
                                .isometric()
                                .color_scale(ColorScaleType::Heat)
                                .opacity(0.9)
                                .wireframe(true)
                                .wireframe_opacity(0.2)
                                .wireframe_color(D3Color::rgb(100, 100, 100))
                                .scale(1.3),
                            width,
                            height,
                            1,
                        )),
                )
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .text_xs()
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
                                .child("X-axis: Time (linear) | Y-axis: 20 Hz → 20 kHz (logarithmic)"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .child("Simulated spectrogram showing harmonic decay at 440 Hz (A4 note)"),
                        ),
                )
                .child(
                    div()
                        .bg(ui_theme.surface)
                        .border_1()
                        .border_color(ui_theme.border)
                        .child(interactive_surface_plot(
                            app,
                            cx,
                            &spectral,
                            SurfaceConfig::new()
                                .isometric()
                                .color_scale(ColorScaleType::Spectral)
                                .opacity(0.95)
                                .wireframe(false)
                                .lighting(true)
                                .ambient(0.5)
                                .diffuse(0.5)
                                .scale(1.4),
                            width,
                            height,
                            2,
                        )),
                )
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .text_xs()
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
                .bg(ui_theme.surface)
                .border_1()
                .border_color(ui_theme.border)
                .rounded_lg()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("About Logarithmic Scales"),
                )
                .child(
                    div()
                        .text_sm()
                        .child("Logarithmic axis sampling distributes points evenly in log space, making it ideal for visualizing data that spans multiple orders of magnitude, such as audio frequency responses (20 Hz to 20 kHz)."),
                )
                .child(
                    div()
                        .text_xs()
                        .mt_2()
                        .child("Available methods: from_z_function_logx(), from_z_function_logy(), from_z_function_logxy()"),
                ),
        )
}
