use anyhow::Result;
use gpui::prelude::*;
use gpui::{AnyElement, IntoElement, div, px};
use gpui_ui_kit::{Text, TextSize};
use std::sync::Arc;

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
            .child(chart)
            .into_any_element(),
        Err(err) => render_chart_error(err, theme),
    }
}
