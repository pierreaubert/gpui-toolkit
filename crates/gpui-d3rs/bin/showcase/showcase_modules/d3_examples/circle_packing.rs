//! Circle Packing — Observable example using d3rs::examples::circle_packing
//!
//! Source: <https://observablehq.com/@d3/pack/2>

use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::color::SequentialScheme;
use d3rs::scale::{Scale, SequentialScale};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let result = d3rs::examples::circle_packing::compute();

    // Official `scaleSequential([8, 0], interpolateMagma)` over node height,
    // approximated as max_depth - depth.
    let magma = SequentialScheme::magma();
    let color_scale = SequentialScale::new()
        .domain(8.0, 0.0)
        .interpolator(move |t| magma.get(t));
    let max_depth = result.circles.iter().map(|c| c.depth).max().unwrap_or(0);
    let width = result.width;
    let height = result.height;

    let d3_paths = result.circle_paths;
    let all_colors: Vec<Hsla> = result
        .circles
        .iter()
        .map(|c| {
            let node_height = max_depth.saturating_sub(c.depth) as f64;
            chart_colors::ink_rgba(&ui_theme, color_scale.scale(node_height).to_rgba())
        })
        .collect();

    // Labels for circles with enough radius: leaf name + value on two lines
    // like the official example, plain names for internal nodes.
    let labels: Vec<(Option<String>, String, f64, f64)> = result
        .circles
        .iter()
        .filter(|c| c.r > 15.0)
        .map(|c| {
            if c.is_leaf {
                (
                    Some(c.name.clone()),
                    format!("{:.0}", c.value),
                    c.x,
                    c.y,
                )
            } else {
                (None, c.name.clone(), c.x, c.y)
            }
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
                // Labels inside circles (white like the official example)
                .children(labels.into_iter().map(|(name, second, x, y)| {
                    let mut col = div()
                        .absolute()
                        .left(px((x - 30.0) as f32))
                        .top(px((y - 11.0) as f32))
                        .w(px(60.0))
                        .flex()
                        .flex_col()
                        .items_center()
                        .overflow_hidden()
                        .text_color(white());
                    if let Some(n) = name {
                        col = col.child(div().text_size(px(8.0)).child(n));
                    }
                    col.child(div().text_size(px(8.0)).child(second))
                })),
        )
}
