use d3rs::axis::{AxisConfig, render_axis};
use d3rs::grid::{GridConfig, render_grid};
use d3rs::prelude::*;
use d3rs::shape::{CurveType, LineConfig, render_line};
use gpui::prelude::FluentBuilder;
use gpui::{Div, ParentElement, Styled, div, px};
use gpui_ui_kit::theme::Theme;

use super::types::{BrushOverlay, PlotCurve, SecondaryAxisConfig};
use super::utils::ChartTheme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FreqSplPlotMargins {
    pub left_axis_width: f32,
    pub right_axis_width: f32,
}

fn freq_spl_plot_axis_sizes(chart_width: f32) -> (f32, f32, f32) {
    let scale = (chart_width / 800.0).clamp(0.7, 1.2);
    let label_size = (10.0 * scale).round();
    let title_size = (12.0 * scale).round();
    let tick_size = (6.0 * scale).round().max(4.0);

    (label_size, title_size, tick_size)
}

pub fn freq_spl_plot_margins(chart_width: f32, has_secondary_axis: bool) -> FreqSplPlotMargins {
    let (label_size, title_size, tick_size) = freq_spl_plot_axis_sizes(chart_width);
    let left_axis_width = AxisConfig::left()
        .with_title("SPL (dB)")
        .with_label_font_size(label_size)
        .with_title_font_size(title_size)
        .with_tick_size(tick_size)
        .total_size();
    let right_axis_width = if has_secondary_axis {
        AxisConfig::right()
            .with_title("DI (dB)")
            .with_label_font_size(label_size)
            .with_title_font_size(title_size)
            .with_tick_size(tick_size)
            .total_size()
    } else {
        0.0
    };

    FreqSplPlotMargins {
        left_axis_width,
        right_axis_width,
    }
}

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
    ui_theme: &Theme,
) -> Div {
    let theme = ChartTheme::from_theme(ui_theme);

    // Scale font sizes relative to a 800px reference width, clamped to [8, 14] range
    let scale = (chart_width / 800.0).clamp(0.7, 1.2);
    let (label_size, title_size, tick_size) = freq_spl_plot_axis_sizes(chart_width);
    let margins = freq_spl_plot_margins(chart_width, secondary_axis.is_some());

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
    let left_axis_config = AxisConfig::left()
        .with_tick_values(spl_ticks)
        .with_formatter(|v| format!("{:.0}", v))
        .with_title("SPL (dB)")
        .with_label_font_size(label_size)
        .with_title_font_size(title_size)
        .with_tick_size(tick_size);

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
                    &left_axis_config,
                    chart_height,
                    &theme,
                ))
                // Chart area
                .child(
                    div()
                        .w(px(chart_width))
                        .h(px(chart_height))
                        .relative()
                        .bg(ui_theme.surface)
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
                            let mut line_config = LineConfig::new()
                                .stroke_color(curve.color)
                                .stroke_width(curve.stroke_width)
                                .curve(CurveType::Linear);
                            if let Some(dash_array) = curve.dash_array.clone() {
                                line_config = line_config.dash_array(dash_array);
                            }
                            Some(render_line(
                                &freq_scale,
                                &spl_scale,
                                &curve.points,
                                &line_config,
                            ))
                        }))
                        // Render secondary axis curves
                        .children(secondary_curves.iter().filter_map(|curve| {
                            let sec_scale = secondary_scale.as_ref()?;
                            if curve.points.is_empty() {
                                return None;
                            }
                            let mut line_config = LineConfig::new()
                                .stroke_color(curve.color)
                                .stroke_width(curve.stroke_width)
                                .curve(CurveType::Linear);
                            if let Some(dash_array) = curve.dash_array.clone() {
                                line_config = line_config.dash_array(dash_array);
                            }
                            Some(render_line(
                                &freq_scale,
                                sec_scale,
                                &curve.points,
                                &line_config,
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
                                    .bg(ui_theme.accent_muted)
                                    .border_1()
                                    .border_color(ui_theme.accent),
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
                        .with_title(cfg.title)
                        .with_label_font_size(label_size)
                        .with_title_font_size(title_size)
                        .with_tick_size(tick_size);
                    el.child(render_axis(&sec_scale, &axis_config, chart_height, &theme))
                }),
        )
        // Bottom axis
        .child(
            div()
                .flex()
                .child(div().flex_none().w(px(margins.left_axis_width)))
                .child(render_axis(
                    &freq_scale,
                    &AxisConfig::bottom()
                        .with_tick_values(major_freq_ticks)
                        .with_minor_tick_values(minor_freq_ticks)
                        .with_minor_tick_size((3.0 * scale).max(2.0))
                        .with_formatter(|f| {
                            if f >= 1000.0 {
                                format!("{:.0}k", f / 1000.0)
                            } else {
                                format!("{:.0}", f)
                            }
                        })
                        .with_title("Frequency (Hz)")
                        .with_label_font_size(label_size)
                        .with_title_font_size(title_size)
                        .with_tick_size(tick_size),
                    chart_width,
                    &theme,
                ))
                .when(margins.right_axis_width > 0.0, |el| {
                    el.child(div().flex_none().w(px(margins.right_axis_width)))
                }),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freq_spl_plot_margins_track_axis_config() {
        let without_secondary = freq_spl_plot_margins(800.0, false);
        let with_secondary = freq_spl_plot_margins(800.0, true);

        assert!(without_secondary.left_axis_width > 0.0);
        assert_eq!(without_secondary.right_axis_width, 0.0);
        assert_eq!(
            with_secondary.left_axis_width,
            without_secondary.left_axis_width
        );
        assert!(with_secondary.right_axis_width > 0.0);
    }

    #[test]
    fn freq_spl_plot_margins_scale_with_chart_width() {
        let narrow = freq_spl_plot_margins(400.0, true);
        let wide = freq_spl_plot_margins(1200.0, true);

        assert!(wide.left_axis_width > narrow.left_axis_width);
        assert!(wide.right_axis_width > narrow.right_axis_width);
    }
}
