//! Grid rendering functions

use super::GridConfig;
use crate::axis::AxisTheme;
use crate::scale::Scale;
use gpui::prelude::*;
use gpui::*;

/// Render a grid overlay
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::prelude::*;
/// use d3rs::grid::{render_grid, GridConfig};
/// use d3rs::axis::DefaultAxisTheme;
///
/// let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
/// let y_scale = LinearScale::new().domain(0.0, 100.0).range(300.0, 0.0);
/// let config = GridConfig::with_lines();
/// let theme = DefaultAxisTheme;
///
/// // render_grid(&x_scale, &y_scale, &config, 400.0, 300.0, &theme)
/// ```
pub fn render_grid<XS, YS, T>(
    x_scale: &XS,
    y_scale: &YS,
    config: &GridConfig,
    _width: f32,
    _height: f32,
    theme: &T,
) -> impl IntoElement
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
    T: AxisTheme,
{
    // Get tick positions - use explicit values if provided, otherwise use scale ticks
    let x_ticks = config
        .vertical_line_values
        .clone()
        .unwrap_or_else(|| x_scale.ticks(10));
    let y_ticks = config
        .horizontal_line_values
        .clone()
        .unwrap_or_else(|| y_scale.ticks(10));

    let (x_range_min, x_range_max) = x_scale.range();
    let (y_range_min, y_range_max) = y_scale.range();
    let x_range_span = x_range_max - x_range_min;
    let y_range_span = y_range_max - y_range_min;

    let half_line_width = config.line_width / 2.0;

    div()
        .absolute()
        .inset_0()
        // Vertical lines
        .when(config.show_vertical_lines, |el| {
            el.children(x_ticks.iter().map(|&x| {
                let x_range = x_scale.scale(x);
                let x_pos = (x_range - x_range_min) / x_range_span;

                div()
                    .absolute()
                    .left(relative(x_pos as f32))
                    .ml(px(-half_line_width)) // Center the line on the tick position
                    .top_0()
                    .bottom_0()
                    .w(px(config.line_width))
                    .bg(theme.axis_line_color())
                    .opacity(config.line_opacity)
            }))
        })
        // Horizontal lines
        .when(config.show_horizontal_lines, |el| {
            el.children(y_ticks.iter().map(|&y| {
                let y_range = y_scale.scale(y);
                // Invert Y for screen coordinates (bottom-to-top becomes top-to-bottom)
                let y_pos = 1.0 - (y_range - y_range_min) / y_range_span;

                div()
                    .absolute()
                    .top(relative(y_pos as f32))
                    .mt(px(-half_line_width)) // Center the line on the tick position
                    .left_0()
                    .right_0()
                    .h(px(config.line_width))
                    .bg(theme.axis_line_color())
                    .opacity(config.line_opacity)
            }))
        })
        // Dots at intersections
        .when(config.show_dots, |el| {
            el.children(y_ticks.iter().flat_map(|&y| {
                let y_range = y_scale.scale(y);
                // Invert Y for screen coordinates (bottom-to-top becomes top-to-bottom)
                let y_pos = 1.0 - (y_range - y_range_min) / y_range_span;

                x_ticks.iter().map(move |&x| {
                    let x_range = x_scale.scale(x);
                    let x_pos = (x_range - x_range_min) / x_range_span;

                    div()
                        .absolute()
                        .left(relative(x_pos as f32))
                        .top(relative(y_pos as f32))
                        .w(px(config.dot_radius * 2.0))
                        .h(px(config.dot_radius * 2.0))
                        .ml(px(-config.dot_radius))
                        .mt(px(-config.dot_radius))
                        .rounded_full()
                        .bg(theme.axis_line_color())
                        .opacity(config.dot_opacity)
                })
            }))
        })
}
