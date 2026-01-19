use d3rs::prelude::*;
use gpui::*;

pub fn render(_app: &ShowcaseApp) -> Div {
    let linear = LinearScale::new().domain(0.0, 100.0).range(0.0, 500.0);
    let log_scale = LogScale::new().domain(20.0, 20000.0).range(0.0, 1.0);

    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Scales Demo"),
        )
        // Linear scale
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Linear Scale (0-100 -> 0-500)"),
                )
                .child(scale_table(&[0.0, 25.0, 50.0, 75.0, 100.0], |v| {
                    format!("{:.0}", linear.scale(v))
                })),
        )
        // Log scale
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Logarithmic Scale (20Hz-20kHz -> 0-1)"),
                )
                .child(scale_table(&[20.0, 100.0, 1000.0, 10000.0, 20000.0], |v| {
                    format!("{:.3}", log_scale.scale(v))
                })),
        )
        // Ticks
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Generated Ticks"),
                )
                .child(
                    div().p_3().bg(rgb(0xf5f5f5)).rounded_md().child(
                        div()
                            .text_sm()
                            .child(format!("Linear ticks: {:?}", linear.ticks(5))),
                    ),
                ),
        )
}

pub fn scale_table<F>(values: &[f64], transform: F) -> Div
where
    F: Fn(f64) -> String,
{
    div()
        .p_3()
        .bg(rgb(0xf5f5f5))
        .rounded_md()
        .flex()
        .flex_col()
        .gap_1()
        .children(values.iter().map(|v| {
            div()
                .flex()
                .gap_4()
                .text_sm()
                .child(div().w(px(80.0)).child(format!("{:.0}", v)))
                .child(div().text_color(rgb(0x666666)).child("->"))
                .child(div().font_weight(FontWeight::MEDIUM).child(transform(*v)))
        }))
}

use super::ShowcaseApp;
