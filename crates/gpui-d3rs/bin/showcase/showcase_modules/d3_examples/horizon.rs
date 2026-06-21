//! Horizon Chart — Observable example (Realtime)
//!
//! Renders multi-band horizon chart from realtime data using
//! d3rs LinearScale, PathBuilder, SequentialScheme, and d3rs_path_to_gpui_simple.
//!
//! Source: <https://observablehq.com/@d3/horizon-chart>

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
    let height = 100.0;
    let bands = 4;

    let data = &app.horizon_data;

    let min_val = -30.0;
    let max_val = 30.0;

    let x_scale = LinearScale::new()
        .domain(0.0, data.len() as f64 - 1.0)
        .range(0.0, width);

    let range = max_val - min_val;
    let step = range / bands as f64;

    let y_scale = LinearScale::new().domain(0.0, step).range(height, 0.0);

    // Use d3rs sequential color scheme (blues)
    let scheme = SequentialScheme::blues();

    // Generate paths using d3rs PathBuilder
    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    let y0 = y_scale.scale(0.0);

    for b in 0..bands {
        let mut builder = D3PathBuilder::new().move_to(x_scale.scale(0.0), y0);

        for (i, &v) in data.iter().enumerate() {
            let val_abs = v.abs();
            let remainder = val_abs - (b as f64 * step);
            let y = if remainder < 0.0 {
                0.0
            } else {
                remainder.min(step)
            };
            builder = builder.line_to(x_scale.scale(i as f64), y_scale.scale(y));
        }

        builder = builder.line_to(x_scale.scale((data.len() - 1) as f64), y0);
        builder = builder.close_path();
        d3_paths.push(builder.build());

        // Color: darker for higher bands (t from 0.3 to 0.9)
        let t = 0.3 + (b as f64 + 1.0) / bands as f64 * 0.6;
        all_colors.push(scheme.get(t).to_rgba().into());
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
                .mb_4()
                .child("Horizon Chart (Realtime)"),
        )
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(ui_theme.surface)
                .relative()
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
