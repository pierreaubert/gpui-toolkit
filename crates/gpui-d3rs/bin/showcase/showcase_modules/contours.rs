use d3rs::contour::{ContourGenerator, DensityEstimator};
use d3rs::prelude::*;
use d3rs::shape::contour::{
    ContourConfig, HeatmapData, heat_color_scale, render_contour, render_heatmap,
    viridis_color_scale,
};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_ui_kit::Slider;

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    // Get entity handle for use in slider callbacks
    let entity = cx.entity().clone();

    // Use state values
    let grid_size = app.contour_grid_size;
    let num_levels = app.contour_num_levels;
    let peak1_x = app.contour_peak1_x as f64;
    let peak1_y = app.contour_peak1_y as f64;
    let peak2_x = app.contour_peak2_x as f64;
    let peak2_y = app.contour_peak2_y as f64;
    let bandwidth = app.density_bandwidth as f64;
    let num_points = app.density_num_points;

    // Generate a 2D Gaussian surface for contour demonstration
    let mut values = vec![0.0; grid_size * grid_size];

    // Create a surface with two peaks
    for j in 0..grid_size {
        for i in 0..grid_size {
            let x = (i as f64 / grid_size as f64) * 2.0 - 1.0;
            let y = (j as f64 / grid_size as f64) * 2.0 - 1.0;

            // Two Gaussian peaks using state parameters
            let peak1 = (-((x - peak1_x).powi(2) + (y - peak1_y).powi(2)) / 0.1).exp();
            let peak2 = 0.7 * (-((x - peak2_x).powi(2) + (y - peak2_y).powi(2)) / 0.15).exp();

            values[j * grid_size + i] = peak1 + peak2;
        }
    }

    // Generate contours at various thresholds
    let generator = ContourGenerator::new(grid_size, grid_size);

    let thresholds: Vec<f64> = (1..=num_levels)
        .map(|i| i as f64 / (num_levels + 1) as f64)
        .collect();
    let contours = generator.contours(&values, &thresholds);

    // Scales for the Gaussian surface plot
    let x_scale_gaussian = LinearScale::new()
        .domain(0.0, grid_size as f64)
        .range(0.0, 400.0);
    let y_scale_gaussian = LinearScale::new()
        .domain(0.0, grid_size as f64)
        .range(0.0, 300.0);

    // Config based on render mode
    let render_mode = app.contour_render_mode;
    let gaussian_config = match render_mode {
        ContourRenderMode::Isoline => ContourConfig::new()
            .stroke_width(2.0)
            .fill(false)
            .color_scale(viridis_color_scale()),
        ContourRenderMode::Surface => ContourConfig::new()
            .stroke_width(1.5)
            .fill(true)
            .fill_opacity(0.6)
            .color_scale(viridis_color_scale()),
        ContourRenderMode::Heatmap => ContourConfig::new()
            .stroke_width(1.5)
            .fill(true)
            .fill_opacity(0.4)
            .color_scale(viridis_color_scale()),
    };

    // Generate heatmap data for the Gaussian surface
    let heatmap_x_values: Vec<f64> = (0..grid_size).map(|i| i as f64).collect();
    let heatmap_y_values: Vec<f64> = (0..grid_size).map(|i| i as f64).collect();
    let gaussian_heatmap = HeatmapData::new(heatmap_x_values, heatmap_y_values, values.clone());

    // Generate density estimation from points
    let points: Vec<(f64, f64)> = (0..num_points)
        .map(|i| {
            let angle = i as f64 * 0.1;
            let r = 0.3 + 0.2 * (i as f64 * 0.05).sin();
            (0.5 + r * angle.cos(), 0.5 + r * angle.sin())
        })
        .collect();

    let density_grid_size = 30;
    let density_estimator = DensityEstimator::new()
        .size(density_grid_size, density_grid_size)
        .x(0.0, 1.0)
        .y(0.0, 1.0)
        .bandwidth(bandwidth);

    let density_grid = density_estimator.estimate(&points);
    let density_max = density_grid.iter().cloned().fold(0.0_f64, f64::max);

    let density_generator = ContourGenerator::new(density_grid_size, density_grid_size);

    let density_thresholds: Vec<f64> = (1..=5).map(|i| density_max * (i as f64 / 6.0)).collect();
    let density_contours = density_generator.contours(&density_grid, &density_thresholds);

    // Scales for the density plot
    let x_scale_density = LinearScale::new()
        .domain(0.0, density_grid_size as f64)
        .range(0.0, 300.0);
    let y_scale_density = LinearScale::new()
        .domain(0.0, density_grid_size as f64)
        .range(0.0, 300.0);

    // Config with heat color scale
    let density_config = ContourConfig::new()
        .stroke_width(1.5)
        .fill(true)
        .fill_opacity(0.5)
        .color_scale(heat_color_scale());

    div()
        .flex()
        .gap_8()
        // Left side: Visualizations
        .child(
            div()
                .flex()
                .flex_col()
                .gap_6()
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::BOLD)
                        .child("Contours Demo"),
                )
                // Marching Squares Contours with render mode switch
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_4()
                                .child(
                                    div().text_lg().font_weight(FontWeight::SEMIBOLD).child(
                                        format!("Gaussian Surface ({})", render_mode.label()),
                                    ),
                                )
                                .child({
                                    let entity = entity.clone();
                                    div()
                                        .id("render-mode-toggle")
                                        .px_3()
                                        .py_1()
                                        .bg(rgb(0x007acc))
                                        .hover(|s| s.bg(rgb(0x005a9e)))
                                        .rounded_md()
                                        .cursor_pointer()
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child("Toggle Mode")
                                        .on_click(move |_, _window, cx| {
                                            entity.update(cx, |this, _| {
                                                this.contour_render_mode =
                                                    this.contour_render_mode.next();
                                            });
                                        })
                                }),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x666666))
                                .child(match render_mode {
                                    ContourRenderMode::Isoline => "Isoline: Contour lines only",
                                    ContourRenderMode::Surface => "Surface: Filled contour bands",
                                    ContourRenderMode::Heatmap => "Heatmap: Pixel-based rendering",
                                }),
                        )
                        .child(
                            div()
                                .w(px(400.0))
                                .h(px(300.0))
                                .bg(rgb(0xf5f5f5))
                                .border_1()
                                .border_color(rgb(0xcccccc))
                                .relative()
                                .when(render_mode != ContourRenderMode::Heatmap, |this| {
                                    this.child(
                                        render_contour(
                                            contours.clone(),
                                            &x_scale_gaussian,
                                            &y_scale_gaussian,
                                            &gaussian_config,
                                        )
                                        .height(px(300.0)),
                                    )
                                })
                                .when(render_mode == ContourRenderMode::Heatmap, |this| {
                                    let heatmap_config =
                                        ContourConfig::new().color_scale(viridis_color_scale());
                                    this.child(
                                        render_heatmap(
                                            gaussian_heatmap.clone(),
                                            &x_scale_gaussian,
                                            &y_scale_gaussian,
                                            &heatmap_config,
                                        )
                                        .height(px(300.0)),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .mt_2()
                                .text_xs()
                                .text_color(rgb(0x666666))
                                .child("Viridis color scale: low → high"),
                        ),
                )
                // Density Estimation
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Kernel Density Estimation"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x666666))
                                .child("Density contours from point data"),
                        )
                        .child(
                            div()
                                .w(px(300.0))
                                .h(px(300.0))
                                .bg(rgb(0x1a1a1a))
                                .border_1()
                                .border_color(rgb(0x333333))
                                .relative()
                                .child(
                                    render_contour(
                                        density_contours.into_iter().collect::<Vec<_>>(),
                                        &x_scale_density,
                                        &y_scale_density,
                                        &density_config,
                                    )
                                    .height(px(300.0)),
                                )
                                // Overlay the original points
                                .children(points.iter().map(|(x, y)| {
                                    div()
                                        .absolute()
                                        .left(px((*x * 300.0 - 2.0) as f32))
                                        .top(px(((1.0 - *y) * 300.0 - 2.0) as f32))
                                        .w(px(4.0))
                                        .h(px(4.0))
                                        .rounded_full()
                                        .bg(rgba(0xffffffaa))
                                })),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .mt_2()
                                .text_xs()
                                .text_color(rgb(0x666666))
                                .child("Heat color scale with point overlay"),
                        ),
                ),
        )
        // Right side: Controls
        .child(
            div()
                .w(px(280.0))
                .flex()
                .flex_col()
                .gap_4()
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
                // Gaussian Surface Controls
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(0x555555))
                                .child("Gaussian Surface"),
                        )
                        .child({
                            let entity = entity.clone();
                            Slider::new("grid-size")
                                .label("Grid Size")
                                .value(app.contour_grid_size as f32)
                                .min(20.0)
                                .max(100.0)
                                .step(10.0)
                                .show_value(true)
                                .width(220.0)
                                .on_change(move |value, _window, cx| {
                                    entity.update(cx, |this, _| {
                                        this.contour_grid_size = value as usize;
                                    });
                                })
                        })
                        .child({
                            let entity = entity.clone();
                            Slider::new("num-levels")
                                .label("Contour Levels")
                                .value(app.contour_num_levels as f32)
                                .min(2.0)
                                .max(10.0)
                                .step(1.0)
                                .show_value(true)
                                .width(220.0)
                                .on_change(move |value, _window, cx| {
                                    entity.update(cx, |this, _| {
                                        this.contour_num_levels = value as usize;
                                    });
                                })
                        })
                        .child({
                            let entity = entity.clone();
                            Slider::new("peak1-x")
                                .label("Peak 1 X")
                                .value(app.contour_peak1_x)
                                .min(-1.0)
                                .max(1.0)
                                .step(0.1)
                                .show_value(true)
                                .width(220.0)
                                .on_change(move |value, _window, cx| {
                                    entity.update(cx, |this, _| {
                                        this.contour_peak1_x = value;
                                    });
                                })
                        })
                        .child({
                            let entity = entity.clone();
                            Slider::new("peak1-y")
                                .label("Peak 1 Y")
                                .value(app.contour_peak1_y)
                                .min(-1.0)
                                .max(1.0)
                                .step(0.1)
                                .show_value(true)
                                .width(220.0)
                                .on_change(move |value, _window, cx| {
                                    entity.update(cx, |this, _| {
                                        this.contour_peak1_y = value;
                                    });
                                })
                        })
                        .child({
                            let entity = entity.clone();
                            Slider::new("peak2-x")
                                .label("Peak 2 X")
                                .value(app.contour_peak2_x)
                                .min(-1.0)
                                .max(1.0)
                                .step(0.1)
                                .show_value(true)
                                .width(220.0)
                                .on_change(move |value, _window, cx| {
                                    entity.update(cx, |this, _| {
                                        this.contour_peak2_x = value;
                                    });
                                })
                        })
                        .child({
                            let entity = entity.clone();
                            Slider::new("peak2-y")
                                .label("Peak 2 Y")
                                .value(app.contour_peak2_y)
                                .min(-1.0)
                                .max(1.0)
                                .step(0.1)
                                .show_value(true)
                                .width(220.0)
                                .on_change(move |value, _window, cx| {
                                    entity.update(cx, |this, _| {
                                        this.contour_peak2_y = value;
                                    });
                                })
                        }),
                )
                // Density Estimation Controls
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .mt_4()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(0x555555))
                                .child("Density Estimation"),
                        )
                        .child({
                            let entity = entity.clone();
                            Slider::new("bandwidth")
                                .label("Bandwidth")
                                .value(app.density_bandwidth)
                                .min(0.02)
                                .max(0.2)
                                .step(0.02)
                                .show_value(true)
                                .width(220.0)
                                .on_change(move |value, _window, cx| {
                                    entity.update(cx, |this, _| {
                                        this.density_bandwidth = value;
                                    });
                                })
                        })
                        .child({
                            let entity = entity.clone();
                            Slider::new("num-points")
                                .label("Number of Points")
                                .value(app.density_num_points as f32)
                                .min(20.0)
                                .max(200.0)
                                .step(10.0)
                                .show_value(true)
                                .width(220.0)
                                .on_change(move |value, _window, cx| {
                                    entity.update(cx, |this, _| {
                                        this.density_num_points = value as usize;
                                    });
                                })
                        }),
                )
                // Statistics
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .mt_4()
                        .p_3()
                        .bg(rgb(0xffffff))
                        .border_1()
                        .border_color(rgb(0xe0e0e0))
                        .rounded_md()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(0x888888))
                                .child("STATISTICS"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x333333))
                                .child(format!("Grid: {}x{}", grid_size, grid_size)),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x333333))
                                .child(format!("Levels: {}", num_levels)),
                        )
                        .child(div().text_sm().text_color(rgb(0x333333)).child(format!(
                            "Rings: {}",
                            contours.iter().map(|c| c.coordinates.len()).sum::<usize>()
                        )))
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x333333))
                                .child(format!("Points: {}", num_points)),
                        ),
                ),
        )
}

use super::{ContourRenderMode, ShowcaseApp};
