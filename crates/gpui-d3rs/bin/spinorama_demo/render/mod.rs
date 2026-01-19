use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::grid::{GridConfig, render_grid};
use d3rs::prelude::*;
use d3rs::shape::{CurveType, LineConfig, render_line};
use gpui::prelude::FluentBuilder;
use gpui::*;

use super::types::{BrushOverlay, PlotCurve, SecondaryAxisConfig};

/// Renders a reusable frequency/SPL plot with optional secondary Y-axis
///
/// This is the common chart used for CEA2034, horizontal SPL, and vertical SPL plots.
/// All use a log frequency X-axis and linear SPL Y-axis.
pub fn render_freq_spl_plot(
    curves: Vec<PlotCurve>,
    freq_domain: (f64, f64),
    spl_domain: (f64, f64),
    secondary_axis: Option<SecondaryAxisConfig>,
    chart_width: f32,
    chart_height: f32,
    brush_overlay: Option<BrushOverlay>,
) -> Div {
    let theme = DefaultAxisTheme;

    // Create log frequency scale with zoom support
    let freq_scale = LogScale::new()
        .domain(freq_domain.0, freq_domain.1)
        .range(0.0, chart_width as f64);
    // Create linear SPL scale for main curves
    let spl_scale = LinearScale::new()
        .domain(spl_domain.0, spl_domain.1)
        .range(chart_height as f64, 0.0);

    // All possible major frequency ticks
    let all_major_ticks = [
        20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0,
    ];

    // Filter ticks to those within the current domain
    let major_freq_ticks: Vec<f64> = all_major_ticks
        .iter()
        .copied()
        .filter(|&f| f >= freq_domain.0 && f <= freq_domain.1)
        .collect();

    // All possible minor frequency ticks
    let all_minor_ticks: Vec<f64> = vec![
        // 20-100 range
        30.0, 40.0, 60.0, 70.0, 80.0, 90.0, // 100-1000 range
        300.0, 400.0, 600.0, 700.0, 800.0, 900.0, // 1000-10000 range
        3000.0, 4000.0, 6000.0, 7000.0, 8000.0, 9000.0,
    ];

    // Filter minor ticks to those within the current domain
    let minor_freq_ticks: Vec<f64> = all_minor_ticks
        .iter()
        .copied()
        .filter(|&f| f >= freq_domain.0 && f <= freq_domain.1)
        .collect();

    // Grid lines - filter to current domain
    let grid_freq_values: Vec<f64> = vec![
        50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0,
    ]
    .into_iter()
    .filter(|&f| f >= freq_domain.0 && f <= freq_domain.1)
    .collect();

    // Generate SPL tick values
    let spl_step = 10.0;
    let spl_ticks: Vec<f64> = {
        let start = (spl_domain.0 / spl_step).ceil() as i32;
        let end = (spl_domain.1 / spl_step).floor() as i32;
        (start..=end).map(|i| i as f64 * spl_step).collect()
    };

    // Create secondary scale if needed
    let secondary_scale = secondary_axis.as_ref().map(|cfg| {
        LinearScale::new()
            .domain(cfg.domain.0, cfg.domain.1)
            .range(chart_height as f64, 0.0)
    });

    // Separate curves by axis
    let primary_curves: Vec<&PlotCurve> = curves.iter().filter(|c| !c.use_secondary_axis).collect();
    let secondary_curves: Vec<&PlotCurve> =
        curves.iter().filter(|c| c.use_secondary_axis).collect();

    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_start()
                // Left Y-axis (SPL)
                .child(render_axis(
                    &spl_scale,
                    &AxisConfig::left()
                        .with_tick_values(spl_ticks)
                        .with_formatter(|v| format!("{:.0}", v))
                        .with_title("SPL (dB)"),
                    chart_height,
                    &theme,
                ))
                // Chart area
                .child(
                    div()
                        .w(px(chart_width))
                        .h(px(chart_height))
                        .relative()
                        .bg(rgb(0xf8f8f8))
                        .child(render_grid(
                            &freq_scale,
                            &spl_scale,
                            &GridConfig::with_lines()
                                .with_vertical_values(grid_freq_values.clone()),
                            chart_width,
                            chart_height,
                            &theme,
                        ))
                        // Render primary axis curves
                        .children(primary_curves.iter().filter_map(|curve| {
                            if curve.points.is_empty() {
                                return None;
                            }
                            Some(render_line(
                                &freq_scale,
                                &spl_scale,
                                &curve.points,
                                &LineConfig::new()
                                    .stroke_color(curve.color)
                                    .stroke_width(curve.stroke_width)
                                    .curve(CurveType::Linear),
                            ))
                        }))
                        // Render secondary axis curves
                        .children(secondary_curves.iter().filter_map(|curve| {
                            let sec_scale = secondary_scale.as_ref()?;
                            if curve.points.is_empty() {
                                return None;
                            }
                            Some(render_line(
                                &freq_scale,
                                sec_scale,
                                &curve.points,
                                &LineConfig::new()
                                    .stroke_color(curve.color)
                                    .stroke_width(curve.stroke_width)
                                    .curve(CurveType::Linear),
                            ))
                        }))
                        // Brush selection overlay (when dragging)
                        .when_some(brush_overlay, |el, overlay| {
                            let sel = overlay.selection;
                            el.child(
                                div()
                                    .absolute()
                                    .left(px(sel.x0 as f32))
                                    .top(px(sel.y0 as f32))
                                    .w(px(sel.width() as f32))
                                    .h(px(sel.height() as f32))
                                    .bg(rgba(0x6496c850)) // Semi-transparent blue
                                    .border_1()
                                    .border_color(rgb(0x4682b4)), // Steel blue
                            )
                        }),
                )
                // Right Y-axis (optional, for DI curves)
                .when_some(secondary_axis, |el, cfg| {
                    let sec_scale = LinearScale::new()
                        .domain(cfg.domain.0, cfg.domain.1)
                        .range(chart_height as f64, 0.0);
                    // Note: with_formatter takes a fn pointer, so we can't capture max_label_value
                    // For DI axis, we use the tick values directly and filter with max_label_value
                    // by passing only tick values up to max_label_value that should have labels
                    let axis_config = AxisConfig::right()
                        .with_tick_values(cfg.tick_values)
                        .with_formatter(|v| format!("{:.0}", v))
                        .with_title(cfg.title);
                    el.child(render_axis(&sec_scale, &axis_config, chart_height, &theme))
                }),
        )
        // Bottom axis
        .child(
            div()
                .flex()
                .child(
                    // Spacer for left axis
                    div().w(px(80.0)),
                )
                .child(render_axis(
                    &freq_scale,
                    &AxisConfig::bottom()
                        .with_tick_values(major_freq_ticks)
                        .with_minor_tick_values(minor_freq_ticks)
                        .with_minor_tick_size(3.0)
                        .with_formatter(|f| {
                            if f >= 1000.0 {
                                format!("{:.0}k", f / 1000.0)
                            } else {
                                format!("{:.0}", f)
                            }
                        })
                        .with_title("Frequency (Hz)"),
                    chart_width,
                    &theme,
                )),
        )
}
