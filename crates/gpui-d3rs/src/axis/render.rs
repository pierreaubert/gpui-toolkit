//! Axis rendering functions

use super::{AxisConfig, AxisOrientation, AxisTheme};
use crate::scale::Scale;
use crate::text::{VectorFontConfig, measure_text_width, render_vector_text};
use gpui::prelude::*;
use gpui::*;

/// Render an axis with the given scale
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::prelude::*;
/// use d3rs::axis::{render_axis, AxisConfig, DefaultAxisTheme};
/// use gpui::*;
///
/// let scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
/// let config = AxisConfig::bottom().with_ticks(10);
/// let theme = DefaultAxisTheme;
///
/// // In a GPUI render context:
/// // render_axis(&scale, &config, 400.0, &theme)
/// ```
pub fn render_axis<S, T>(scale: &S, config: &AxisConfig, size: f32, theme: &T) -> AnyElement
where
    S: Scale<f64, f64>,
    T: AxisTheme,
{
    // Use explicit tick values if provided, otherwise generate from scale
    let ticks = match &config.tick_values {
        Some(values) => values.clone(),
        None => scale.ticks(config.tick_count),
    };

    match config.orientation {
        AxisOrientation::Bottom => {
            render_bottom_axis(scale, &ticks, config, size, theme).into_any_element()
        }
        AxisOrientation::Top => {
            render_top_axis(scale, &ticks, config, size, theme).into_any_element()
        }
        AxisOrientation::Left => {
            render_left_axis(scale, &ticks, config, size, theme).into_any_element()
        }
        AxisOrientation::Right => {
            render_right_axis(scale, &ticks, config, size, theme).into_any_element()
        }
    }
}

/// Render a bottom-oriented horizontal axis
fn render_bottom_axis<S, T>(
    scale: &S,
    ticks: &[f64],
    config: &AxisConfig,
    width: f32,
    theme: &T,
) -> impl IntoElement
where
    S: Scale<f64, f64>,
    T: AxisTheme,
{
    let height = config.total_size();
    let (range_min, range_max) = scale.range();
    let range_span = range_max - range_min;
    let tick_top = config.domain_line_width; // Top of tick area (below domain line)

    div()
        .w(px(width))
        .h(px(height))
        .relative()
        // Domain line
        .when(config.show_domain_line, |el| {
            el.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top_0()
                    .h(px(config.domain_line_width))
                    .bg(theme.axis_line_color()),
            )
        })
        // Ticks and labels - position each independently
        .children(ticks.iter().flat_map(|&tick_value| {
            let range_value = scale.scale(tick_value);
            let x_pos = (range_value - range_min) / range_span;
            let label = format_tick(tick_value, &config.tick_format);
            let half_tick_width = config.domain_line_width / 2.0;

            // Convert angle from degrees to radians
            let angle_rad = config.label_angle * std::f32::consts::PI / 180.0;
            let font_config = VectorFontConfig {
                font_size: config.label_font_size,
                stroke_width: 1.2,
                color: theme.axis_label_color().into(),
                rotation: angle_rad,
                letter_spacing: 0.1,
            };

            // Tick mark - positioned absolutely and centered on the x position
            let tick_mark = div()
                .absolute()
                .left(relative(x_pos as f32))
                .ml(px(-half_tick_width))
                .top(px(tick_top))
                .w(px(config.domain_line_width))
                .h(px(config.tick_size))
                .bg(theme.axis_line_color());

            // Label - positioned absolutely
            let label_top = tick_top + config.tick_size + config.tick_padding;
            let label_width = measure_text_width(&label, config.label_font_size);

            // For angled labels, adjust positioning
            // Negative angle rotates counter-clockwise, so text goes down-left
            let (ml_offset, mt_offset) = if config.label_angle.abs() > 0.1 {
                // For angled text, anchor at the right end of the text
                // so it appears to hang from the tick mark
                (-label_width * angle_rad.cos().abs() * 0.1, 0.0)
            } else {
                // For horizontal text, center it
                (-label_width / 2.0, 0.0)
            };

            let label_div = div()
                .absolute()
                .left(relative(x_pos as f32))
                .ml(px(ml_offset))
                .top(px(label_top + mt_offset))
                .child(render_vector_text(&label, &font_config));

            [tick_mark.into_any_element(), label_div.into_any_element()]
        }))
        // Minor ticks (no labels, shorter)
        .children(
            config
                .minor_tick_values
                .as_ref()
                .map(|minor_ticks| {
                    minor_ticks
                        .iter()
                        .filter_map(|&tick_value| {
                            let range_value = scale.scale(tick_value);
                            let x_pos = (range_value - range_min) / range_span;
                            // Only render if within visible range
                            if !(0.0..=1.0).contains(&x_pos) {
                                return None;
                            }
                            let half_tick_width = config.domain_line_width / 2.0;

                            Some(
                                div()
                                    .absolute()
                                    .left(relative(x_pos as f32))
                                    .ml(px(-half_tick_width))
                                    .top(px(tick_top))
                                    .w(px(config.domain_line_width))
                                    .h(px(config.minor_tick_size))
                                    .bg(theme.axis_line_color())
                                    .into_any_element(),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        )
        // Title (horizontal for bottom axis)
        .when(config.title.is_some(), |el| {
            let title = config.title.clone().unwrap_or_default();

            // Calculate label height accounting for angle
            let label_height = if config.label_angle.abs() > 0.1 {
                // Approximate height for angled text: font_size * sin(angle) + some width component
                let angle_rad = config.label_angle.abs() * std::f32::consts::PI / 180.0;
                // Assume average label width of ~40px for frequency labels
                let estimated_label_width = 40.0_f32;
                estimated_label_width * angle_rad.sin() + config.label_font_size * angle_rad.cos()
            } else {
                config.label_font_size
            };

            let title_top =
                config.tick_size + config.tick_padding + label_height + config.title_padding;
            let font_config = VectorFontConfig::horizontal(
                config.title_font_size,
                theme.axis_label_color().into(),
            );
            el.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(title_top))
                    .flex()
                    .justify_center()
                    .child(render_vector_text(&title, &font_config)),
            )
        })
}

/// Render a top-oriented horizontal axis
fn render_top_axis<S, T>(
    scale: &S,
    ticks: &[f64],
    config: &AxisConfig,
    width: f32,
    theme: &T,
) -> impl IntoElement
where
    S: Scale<f64, f64>,
    T: AxisTheme,
{
    let height = config.total_size();
    let (range_min, range_max) = scale.range();
    let range_span = range_max - range_min;
    let domain_line_y = height - config.domain_line_width;
    let tick_bottom = domain_line_y; // Bottom of tick area (above domain line)

    div()
        .w(px(width))
        .h(px(height))
        .relative()
        // Domain line (at the bottom of the axis area)
        .when(config.show_domain_line, |el| {
            el.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .h(px(config.domain_line_width))
                    .bg(theme.axis_line_color()),
            )
        })
        // Ticks and labels - position each independently (ticks point UP, labels ABOVE)
        .children(ticks.iter().flat_map(|&tick_value| {
            let range_value = scale.scale(tick_value);
            let x_pos = (range_value - range_min) / range_span;
            let label = format_tick(tick_value, &config.tick_format);
            let half_tick_width = config.domain_line_width / 2.0;
            let font_config = VectorFontConfig::horizontal(
                config.label_font_size,
                theme.axis_label_color().into(),
            );

            // Tick mark - positioned absolutely, pointing upward from domain line
            let tick_mark = div()
                .absolute()
                .left(relative(x_pos as f32))
                .ml(px(-half_tick_width))
                .top(px(tick_bottom - config.tick_size))
                .w(px(config.domain_line_width))
                .h(px(config.tick_size))
                .bg(theme.axis_line_color());

            // Label - positioned absolutely, above the tick
            let label_bottom = tick_bottom - config.tick_size - config.tick_padding;
            let half_label_width = measure_text_width(&label, config.label_font_size) / 2.0;
            let label_div = div()
                .absolute()
                .left(relative(x_pos as f32))
                .ml(px(-half_label_width))
                .top(px(label_bottom - config.label_font_size))
                .child(render_vector_text(&label, &font_config));

            [tick_mark.into_any_element(), label_div.into_any_element()]
        }))
        // Minor ticks (no labels, shorter)
        .children(
            config
                .minor_tick_values
                .as_ref()
                .map(|minor_ticks| {
                    minor_ticks
                        .iter()
                        .filter_map(|&tick_value| {
                            let range_value = scale.scale(tick_value);
                            let x_pos = (range_value - range_min) / range_span;
                            // Only render if within visible range
                            if !(0.0..=1.0).contains(&x_pos) {
                                return None;
                            }
                            let half_tick_width = config.domain_line_width / 2.0;

                            Some(
                                div()
                                    .absolute()
                                    .left(relative(x_pos as f32))
                                    .ml(px(-half_tick_width))
                                    .top(px(tick_bottom - config.minor_tick_size))
                                    .w(px(config.domain_line_width))
                                    .h(px(config.minor_tick_size))
                                    .bg(theme.axis_line_color())
                                    .into_any_element(),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        )
        // Title (horizontal for top axis, at the very top)
        .when(config.title.is_some(), |el| {
            let title = config.title.clone().unwrap_or_default();
            let font_config = VectorFontConfig::horizontal(
                config.title_font_size,
                theme.axis_label_color().into(),
            );
            el.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top_0()
                    .flex()
                    .justify_center()
                    .child(render_vector_text(&title, &font_config)),
            )
        })
}

/// Render a left-oriented vertical axis
fn render_left_axis<S, T>(
    scale: &S,
    ticks: &[f64],
    config: &AxisConfig,
    height: f32,
    theme: &T,
) -> impl IntoElement
where
    S: Scale<f64, f64>,
    T: AxisTheme,
{
    let width = config.total_size();
    let (range_min, range_max) = scale.range();
    let range_span = range_max - range_min;
    let domain_line_x = width - config.domain_line_width;
    let tick_right = config.domain_line_width; // Right edge of tick area (where domain line starts)

    div()
        .w(px(width))
        .h(px(height))
        .relative()
        // Title (rotated text for left axis - reading bottom-to-top)
        .when(config.title.is_some(), |el| {
            let title = config.title.clone().unwrap_or_default();
            let font_config = VectorFontConfig::vertical_bottom_to_top(
                config.title_font_size,
                theme.axis_label_color().into(),
            );
            el.child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(px(config.title_font_size + 4.0))
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(render_vector_text(&title, &font_config)),
            )
        })
        // Domain line
        .when(config.show_domain_line, |el| {
            el.child(
                div()
                    .absolute()
                    .left(px(domain_line_x))
                    .top_0()
                    .bottom_0()
                    .w(px(config.domain_line_width))
                    .bg(theme.axis_line_color()),
            )
        })
        // Ticks and labels - position each independently
        .children(ticks.iter().flat_map(|&tick_value| {
            let range_value = scale.scale(tick_value);
            // Invert Y for screen coordinates (bottom-to-top becomes top-to-bottom)
            let y_pos = 1.0 - (range_value - range_min) / range_span;
            let label = format_tick(tick_value, &config.tick_format);
            let half_tick_height = config.domain_line_width / 2.0;
            let font_config = VectorFontConfig::horizontal(
                config.label_font_size,
                theme.axis_label_color().into(),
            );

            // Tick mark - positioned absolutely and centered on the y position
            let tick_mark = div()
                .absolute()
                .right(px(tick_right))
                .top(relative(y_pos as f32))
                .mt(px(-half_tick_height))
                .w(px(config.tick_size))
                .h(px(config.domain_line_width))
                .bg(theme.axis_line_color());

            // Label - positioned absolutely, vertically centered on the y position
            // We estimate half the label height as label_font_size / 2 for centering
            let half_label_height = config.label_font_size / 2.0;
            let label_div = div()
                .absolute()
                .right(px(tick_right + config.tick_size + config.tick_padding))
                .top(relative(y_pos as f32))
                .mt(px(-half_label_height))
                .child(render_vector_text(&label, &font_config));

            [tick_mark.into_any_element(), label_div.into_any_element()]
        }))
        // Minor ticks (no labels, shorter)
        .children(
            config
                .minor_tick_values
                .as_ref()
                .map(|minor_ticks| {
                    minor_ticks
                        .iter()
                        .filter_map(|&tick_value| {
                            let range_value = scale.scale(tick_value);
                            let y_pos = 1.0 - (range_value - range_min) / range_span;
                            // Only render if within visible range
                            if !(0.0..=1.0).contains(&y_pos) {
                                return None;
                            }
                            let half_tick_height = config.domain_line_width / 2.0;

                            Some(
                                div()
                                    .absolute()
                                    .right(px(tick_right))
                                    .top(relative(y_pos as f32))
                                    .mt(px(-half_tick_height))
                                    .w(px(config.minor_tick_size))
                                    .h(px(config.domain_line_width))
                                    .bg(theme.axis_line_color())
                                    .into_any_element(),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        )
}

/// Render a right-oriented vertical axis
fn render_right_axis<S, T>(
    scale: &S,
    ticks: &[f64],
    config: &AxisConfig,
    height: f32,
    theme: &T,
) -> impl IntoElement
where
    S: Scale<f64, f64>,
    T: AxisTheme,
{
    let width = config.total_size();
    let (range_min, range_max) = scale.range();
    let range_span = range_max - range_min;
    let tick_left = config.domain_line_width; // Left edge of tick area (where domain line ends)

    div()
        .w(px(width))
        .h(px(height))
        .relative()
        // Domain line
        .when(config.show_domain_line, |el| {
            el.child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(px(config.domain_line_width))
                    .bg(theme.axis_line_color()),
            )
        })
        // Ticks and labels - position each independently
        .children(ticks.iter().flat_map(|&tick_value| {
            let range_value = scale.scale(tick_value);
            // Invert Y for screen coordinates (bottom-to-top becomes top-to-bottom)
            let y_pos = 1.0 - (range_value - range_min) / range_span;
            let label = format_tick(tick_value, &config.tick_format);
            let half_tick_height = config.domain_line_width / 2.0;
            let font_config = VectorFontConfig::horizontal(
                config.label_font_size,
                theme.axis_label_color().into(),
            );

            // Tick mark - positioned absolutely and centered on the y position
            let tick_mark = div()
                .absolute()
                .left(px(tick_left))
                .top(relative(y_pos as f32))
                .mt(px(-half_tick_height))
                .w(px(config.tick_size))
                .h(px(config.domain_line_width))
                .bg(theme.axis_line_color());

            // Label - positioned absolutely, vertically centered on the y position
            let half_label_height = config.label_font_size / 2.0;
            let label_div = div()
                .absolute()
                .left(px(tick_left + config.tick_size + config.tick_padding))
                .top(relative(y_pos as f32))
                .mt(px(-half_label_height))
                .child(render_vector_text(&label, &font_config));

            [tick_mark.into_any_element(), label_div.into_any_element()]
        }))
        // Title (rotated text for right axis - reading bottom-to-top)
        .when(config.title.is_some(), |el| {
            let title = config.title.clone().unwrap_or_default();
            let font_config = VectorFontConfig::vertical_bottom_to_top(
                config.title_font_size,
                theme.axis_label_color().into(),
            );
            el.child(
                div()
                    .absolute()
                    .right_0()
                    .top_0()
                    .bottom_0()
                    .w(px(config.title_font_size + 4.0))
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(render_vector_text(&title, &font_config)),
            )
        })
}

/// Format a tick value using the optional custom formatter
fn format_tick(value: f64, formatter: &Option<fn(f64) -> String>) -> String {
    match formatter {
        Some(f) => f(value),
        None => {
            // Default formatting: remove trailing zeros
            if value.abs() < 1e-10 {
                "0".to_string()
            } else if value.abs() >= 1000.0 || value.abs() < 0.01 {
                format!("{:.1e}", value)
            } else if value.fract().abs() < 1e-10 {
                format!("{:.0}", value)
            } else {
                format!("{:.1}", value)
            }
        }
    }
}

// Tests for render functions require GPUI runtime, see examples instead
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tick_default() {
        assert_eq!(format_tick(0.0, &None), "0");
        assert_eq!(format_tick(10.0, &None), "10");
        assert_eq!(format_tick(10.5, &None), "10.5");
        assert_eq!(format_tick(1000.5, &None), "1.0e3");
    }

    #[test]
    fn test_format_tick_custom() {
        let formatter = |v: f64| format!("{:.2}Hz", v);
        assert_eq!(format_tick(440.0, &Some(formatter)), "440.00Hz");
    }
}
