use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::prelude::*;
use gpui::*;

pub fn render(_app: &ShowcaseApp) -> Div {
    let theme = DefaultAxisTheme;
    let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
    let freq_scale = LogScale::new().domain(20.0, 20000.0).range(0.0, 400.0);
    let db_scale = LinearScale::new().domain(-24.0, 24.0).range(0.0, 200.0);

    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Axes Demo"),
        )
        // Bottom axis
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Bottom Axis (Linear 0-100)"),
                )
                .child(render_axis(
                    &x_scale,
                    &AxisConfig::bottom().with_ticks(10),
                    400.0,
                    &theme,
                )),
        )
        // Top axis with formatter
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Top Axis (Log 20Hz-20kHz)"),
                )
                .child(render_axis(
                    &freq_scale,
                    &AxisConfig::top().with_ticks(10).with_formatter(|f| {
                        if f >= 1000.0 {
                            format!("{:.0}k", f / 1000.0)
                        } else {
                            format!("{:.0}", f)
                        }
                    }),
                    400.0,
                    &theme,
                )),
        )
        // Left/Right axes
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Left & Right Axes (dB scale)"),
                )
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .child(render_axis(
                            &db_scale,
                            &AxisConfig::left().with_ticks(9).with_formatter(|db| {
                                if db > 0.0 {
                                    format!("+{:.0}", db)
                                } else {
                                    format!("{:.0}", db)
                                }
                            }),
                            200.0,
                            &theme,
                        ))
                        .child(
                            div()
                                .w(px(150.0))
                                .h(px(200.0))
                                .bg(rgb(0xf0f0f0))
                                .rounded_md(),
                        )
                        .child(render_axis(
                            &db_scale,
                            &AxisConfig::right().with_ticks(9),
                            200.0,
                            &theme,
                        )),
                ),
        )
}

use super::ShowcaseApp;
