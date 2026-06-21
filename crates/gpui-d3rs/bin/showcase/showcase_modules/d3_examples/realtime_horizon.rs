//! Realtime Horizon Chart — Observable example
//!
//! Uses app.horizon_data (streaming values) to render a multi-band
//! horizon chart that updates in realtime.
//! Uses d3rs LinearScale, PathBuilder, SequentialScheme.
//!
//! Source: <https://observablehq.com/@d3/realtime-horizon-chart>

use crate::ShowcaseApp;
use d3rs::color::SequentialScheme;
use d3rs::scale::{LinearScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    app.ensure_horizon_animation(cx);

    let ui_theme = cx.theme();
    let width = app.content_width as f64;
    let height = 80.0;
    let bands = 4;

    let data = &app.horizon_data;
    if data.is_empty() {
        return div().child("Waiting for realtime data...");
    }

    let max_abs = data.iter().map(|v| v.abs()).fold(0.0f64, f64::max);
    let step = max_abs / bands as f64;

    let x_scale = LinearScale::new()
        .domain(0.0, data.len() as f64 - 1.0)
        .range(0.0, width);

    let y_scale = LinearScale::new().domain(0.0, step).range(height, 0.0);

    // Positive bands (blues)
    let pos_scheme = SequentialScheme::blues();
    // Negative bands (reds/oranges)
    let neg_scheme = SequentialScheme::oranges();

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    let y0 = y_scale.scale(0.0);

    // Positive bands
    for b in 0..bands {
        let mut builder = D3PathBuilder::new().move_to(x_scale.scale(0.0), y0);
        for (i, &v) in data.iter().enumerate() {
            let remainder = v.max(0.0) - (b as f64 * step);
            let y = remainder.clamp(0.0, step);
            builder = builder.line_to(x_scale.scale(i as f64), y_scale.scale(y));
        }
        builder = builder.line_to(x_scale.scale((data.len() - 1) as f64), y0);
        builder = builder.close_path();
        d3_paths.push(builder.build());
        let t = 0.3 + (b as f64 + 1.0) / bands as f64 * 0.6;
        all_colors.push(pos_scheme.get(t).to_rgba().into());
    }

    // Negative bands (mirror)
    for b in 0..bands {
        let mut builder = D3PathBuilder::new().move_to(x_scale.scale(0.0), y0);
        for (i, &v) in data.iter().enumerate() {
            let remainder = (-v).max(0.0) - (b as f64 * step);
            let y = remainder.clamp(0.0, step);
            builder = builder.line_to(x_scale.scale(i as f64), y_scale.scale(y));
        }
        builder = builder.line_to(x_scale.scale((data.len() - 1) as f64), y0);
        builder = builder.close_path();
        d3_paths.push(builder.build());
        let t = 0.3 + (b as f64 + 1.0) / bands as f64 * 0.6;
        all_colors.push(neg_scheme.get(t).to_rgba().into());
    }

    div()
        .flex()
        .flex_col()
        .size_full()
        .p_4()
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .mb_2()
                .child("Realtime Horizon Chart"),
        )
        .child(
            div()
                .text_xs()
                .mb_2()
                .child(format!(
                    "Source: observablehq.com/@d3/realtime-horizon-chart — {} samples, ±{:.1} range, {} bands",
                    data.len(), max_abs, bands
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_4()
                .mb_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().size_3().bg(pos_scheme.get(0.7).to_rgba()).rounded_sm())
                        .child(div().text_xs().child("Positive")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().size_3().bg(neg_scheme.get(0.7).to_rgba()).rounded_sm())
                        .child(div().text_xs().child("Negative")),
                ),
        )
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(ui_theme.surface)
                .border_1()
                .border_color(ui_theme.border)
                .overflow_hidden()
                .child(
                    canvas(
                        move |bounds, _, _| {
                            d3_paths
                                .iter()
                                .map(|p| {
                                    super::path_utils::d3rs_path_to_gpui_simple(p, bounds, 0.0, 0.0)
                                })
                                .collect::<Vec<_>>()
                        },
                        move |_bounds, paths, window, _| {
                            for (i, path_opt) in paths.into_iter().enumerate() {
                                if let Some(path) = path_opt {
                                    window.paint_path(path, all_colors[i]);
                                }
                            }
                        },
                    )
                    .size_full(),
                ),
        )
}
