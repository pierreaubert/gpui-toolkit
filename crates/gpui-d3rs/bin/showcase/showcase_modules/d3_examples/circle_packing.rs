//! Circle Packing — Observable example using d3rs::examples::circle_packing
//!
//! Source: <https://observablehq.com/@d3/pack/2>

use crate::ShowcaseApp;
use d3rs::color::ColorScheme;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let result = d3rs::examples::circle_packing::compute();

    let scheme = ColorScheme::tableau10();
    let width = result.width;
    let height = result.height;

    let d3_paths = result.circle_paths;
    let all_colors: Vec<Hsla> = result
        .circles
        .iter()
        .map(|c| {
            if c.is_leaf {
                Hsla::from(scheme.color(c.depth + 1).to_rgba()).opacity(0.7)
            } else {
                Hsla::from(scheme.color(c.depth).to_rgba()).opacity(0.15)
            }
        })
        .collect();

    // Labels for circles with enough radius
    let labels: Vec<(String, f64, f64, f64)> = result
        .circles
        .iter()
        .filter(|c| c.r > 15.0)
        .map(|c| {
            let label = if c.is_leaf {
                format!("{}\n{:.0}", c.name, c.value)
            } else {
                c.name.clone()
            };
            (label, c.x, c.y, c.r)
        })
        .collect();

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
                .child("Circle Packing — Flare Hierarchy"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/pack — {} circles",
            result.circles.len()
        )))
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(ui_theme.surface)
                .border_1()
                .border_color(ui_theme.border)
                .relative()
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
                )
                // Labels inside circles
                .children(labels.into_iter().map(|(label, x, y, _r)| {
                    div()
                        .absolute()
                        .left(px((x - 20.0) as f32))
                        .top(px((y - 8.0) as f32))
                        .w(px(40.0))
                        .text_size(px(8.0))
                        .overflow_hidden()
                        .flex()
                        .justify_center()
                        .child(label)
                })),
        )
}
