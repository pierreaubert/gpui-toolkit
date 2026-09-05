use anyhow::Result;
use gpui::prelude::*;
use gpui::{AnyElement, IntoElement, Rgba, div, px};
use gpui_px::{BarTheme, ChartTheme, ScatterTheme};
use gpui_ui_kit::{Text, TextSize, theme::Theme};
use std::sync::Arc;

/// Convert a theme color to the `0xRRGGBB` hex that px chart builders take
/// for series colors. Alpha is dropped: theme data roles are opaque.
pub(super) fn theme_chart_hex(color: Rgba) -> u32 {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u32;
    (channel(color.r) << 16) | (channel(color.g) << 8) | channel(color.b)
}

/// Primary data-series color, derived from the active theme.
pub(super) fn px_primary_hex(theme: &Theme) -> u32 {
    theme_chart_hex(theme.info)
}

/// Comparison/secondary data-series color, derived from the active theme.
pub(super) fn px_secondary_hex(theme: &Theme) -> u32 {
    theme_chart_hex(theme.warning)
}

/// Categorical palette for pie/donut slices, derived from the active theme.
pub(super) fn px_category_palette(theme: &Theme) -> [u32; 8] {
    [
        theme_chart_hex(theme.info),
        theme_chart_hex(theme.warning),
        theme_chart_hex(theme.success),
        theme_chart_hex(theme.accent),
        theme_chart_hex(theme.error),
        theme_chart_hex(theme.text_secondary),
        theme_chart_hex(theme.accent_hover),
        theme_chart_hex(theme.text_muted),
    ]
}

/// Line-chart chrome (plot background, grid, axes, title, legend) derived
/// from the active theme instead of the light-mode defaults.
pub(super) fn px_line_theme(theme: &Theme) -> ChartTheme {
    ChartTheme {
        plot_background: theme.background,
        grid_color: Rgba {
            a: 0.6,
            ..theme.border
        },
        axis_line_color: theme.border,
        axis_label_color: theme.text_secondary,
        title_color: theme.text_primary,
        legend_text_color: theme.text_secondary,
    }
}

/// Bar-chart chrome derived from the active theme.
pub(super) fn px_bar_theme(theme: &Theme) -> BarTheme {
    BarTheme {
        plot_background: theme.background,
        title_color: theme.text_primary,
        legend_text_color: theme.text_secondary,
    }
}

/// Scatter-chart chrome derived from the active theme.
pub(super) fn px_scatter_theme(theme: &Theme) -> ScatterTheme {
    ScatterTheme {
        plot_background: theme.background,
        title_color: theme.text_primary,
        legend_text_color: theme.text_secondary,
    }
}

pub(super) fn render_chart_error(
    err: impl std::fmt::Display,
    theme: Arc<gpui_ui_kit::theme::Theme>,
) -> AnyElement {
    div()
        .w(px(320.0))
        .p_4()
        .bg(theme.surface)
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .child(
            Text::new(format!("Chart failed: {err}"))
                .size(TextSize::Xs)
                .color(theme.text_secondary),
        )
        .into_any_element()
}

pub(super) fn render_chart_result<E, Err>(
    result: Result<E, Err>,
    theme: Arc<gpui_ui_kit::theme::Theme>,
) -> AnyElement
where
    E: IntoElement,
    Err: std::fmt::Display,
{
    match result {
        Ok(chart) => div()
            .w_full()
            .h_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .items_center()
            .justify_center()
            .child(chart)
            .into_any_element(),
        Err(err) => render_chart_error(err, theme),
    }
}
