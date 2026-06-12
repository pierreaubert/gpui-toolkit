//! Parallel Sets — Observable example using d3rs::examples::parallel_sets
//!
//! Loads Titanic survival data and renders categorical flow visualization.
//! Demonstrates: `SankeyLayout`, `PathBuilder`, `ColorScheme`.
//!
//! Source: <https://observablehq.com/@d3/parallel-sets>

use crate::ShowcaseApp;
use d3rs::color::ColorScheme;
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const TITANIC_CSV: &str = include_str!("../../data/titanic.csv");

pub fn render(app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let (names, links) = d3rs::examples::parallel_sets::load_csv(TITANIC_CSV);
    let result = d3rs::examples::parallel_sets::compute(&names, &links);

    let scheme = ColorScheme::tableau10();
    let width = app.content_width as f64;
    let height = (width * 0.86).min(app.content_height as f64 * 0.8);

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // Links as filled ribbons (cubic Bézier)
    for link in &result.links {
        let source = &result.nodes[link.source];
        let target = &result.nodes[link.target];

        let sx = source.x1;
        let tx = target.x0;
        let cx = (sx + tx) / 2.0;
        let hw = link.width / 2.0;

        let path = D3PathBuilder::new()
            .move_to(sx, link.y0 - hw)
            .cubic_curve_to(cx, link.y0 - hw, cx, link.y1 - hw, tx, link.y1 - hw)
            .line_to(tx, link.y1 + hw)
            .cubic_curve_to(cx, link.y1 + hw, cx, link.y0 + hw, sx, link.y0 + hw)
            .close_path()
            .build();
        d3_paths.push(path);
        // Color based on source node index for categorical distinction
        let color = scheme.color(source.index).to_rgba();
        all_colors.push(Hsla::from(color).opacity(0.5));
    }

    // Node rectangles
    for node in &result.nodes {
        let path = D3PathBuilder::new()
            .move_to(node.x0, node.y0)
            .line_to(node.x1, node.y0)
            .line_to(node.x1, node.y1)
            .line_to(node.x0, node.y1)
            .close_path()
            .build();
        d3_paths.push(path);
        all_colors.push(hsla(0.0, 0.0, 0.2, 1.0));
    }

    // Node labels
    let label_nodes: Vec<(String, f64, f64, bool)> = result
        .nodes
        .iter()
        .map(|n| {
            let is_right = n.layer > 0;
            let lx = if is_right { n.x0 - 4.0 } else { n.x1 + 4.0 };
            let ly = (n.y0 + n.y1) / 2.0;
            (format!("{} ({:.0})", n.id, n.value), lx, ly, is_right)
        })
        .collect();

    // Category headers
    let layer_names = ["Class", "Sex", "Outcome"];

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
                .child("Parallel Sets — Titanic Survival"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/parallel-sets — {} passengers, {} nodes, {} flows",
            result.links.iter().map(|l| l.value).sum::<f64>() as usize,
            result.nodes.len(),
            result.links.len()
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
                // Layer headers
                .children(layer_names.iter().enumerate().map(|(li, name)| {
                    let x_positions: Vec<f64> = result
                        .nodes
                        .iter()
                        .filter(|n| n.layer == li)
                        .map(|n| (n.x0 + n.x1) / 2.0)
                        .collect();
                    let avg_x = if x_positions.is_empty() {
                        0.0
                    } else {
                        x_positions.iter().sum::<f64>() / x_positions.len() as f64
                    };
                    div()
                        .absolute()
                        .left(px((avg_x - 20.0) as f32))
                        .bottom(px(2.0))
                        .text_size(px(10.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(name.to_string())
                }))
                // Node labels
                .children(label_nodes.into_iter().map(|(name, lx, ly, is_right)| {
                    let mut d = div().absolute().top(px(ly as f32 - 6.0));
                    if is_right {
                        d = d.right(px((width - lx) as f32));
                    } else {
                        d = d.left(px(lx as f32));
                    }
                    d.child(div().text_size(px(10.0)).line_height(px(12.0)).child(name))
                })),
        )
}
