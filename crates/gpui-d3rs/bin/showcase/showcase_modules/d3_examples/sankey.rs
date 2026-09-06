//! Sankey Diagram — Observable example using d3rs::examples::sankey
//!
//! Loads the energy flow dataset from energy.json (48 nodes, 68 links).
//! Demonstrates: `SankeyLayout`, `PathBuilder`, `ColorScheme`,
//! `d3rs_path_to_gpui_simple`.
//!
//! Source: <https://observablehq.com/@d3/sankey>

use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::color::ColorScheme;
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

/// Embedded energy.json data (48 nodes, 68 links).
const ENERGY_JSON: &str = include_str!("../../data/energy.json");

pub fn render(app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let (names, links) = d3rs::examples::sankey::load_json(ENERGY_JSON);

    let result = d3rs::examples::sankey::compute(&names, &links);

    let scheme = ColorScheme::tableau10();

    let width = app.content_width as f64;
    let height = (width * 0.71).min(app.content_height as f64 * 0.8);

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // Links as filled ribbons (cubic Bézier top + bottom)
    for link in &result.links {
        let source = &result.nodes[link.source];
        let target = &result.nodes[link.target];

        let sx = source.x1;
        let tx = target.x0;
        let cx = (sx + tx) / 2.0;
        let hw = link.width / 2.0; // half-width

        // Top edge: from source to target at y - hw
        // Bottom edge: back from target to source at y + hw
        let path = D3PathBuilder::new()
            .move_to(sx, link.y0 - hw)
            .cubic_curve_to(cx, link.y0 - hw, cx, link.y1 - hw, tx, link.y1 - hw)
            .line_to(tx, link.y1 + hw)
            .cubic_curve_to(cx, link.y1 + hw, cx, link.y0 + hw, sx, link.y0 + hw)
            .close_path()
            .build();
        d3_paths.push(path);
        // Source-colored ribbons (the official example fades source to
        // target with per-link gradients; flat fills can't do that, so use
        // a higher opacity to stay close in saturation).
        all_colors.push(chart_colors::categorical(&ui_theme, &scheme, source.layer).opacity(0.5));
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
        all_colors.push(scheme.color(node.layer).to_rgba().into());
    }

    // Node labels for nodes with enough height
    let label_nodes: Vec<(String, f64, f64, bool)> = result
        .nodes
        .iter()
        .filter(|n| n.y1 - n.y0 > 5.0)
        .map(|n| {
            let is_right = n.layer > result.nodes.iter().map(|nn| nn.layer).max().unwrap_or(0) / 2;
            let lx = if is_right { n.x0 - 4.0 } else { n.x1 + 4.0 };
            let ly = (n.y0 + n.y1) / 2.0;
            (n.id.clone(), lx, ly, is_right)
        })
        .collect();

    // The official example has no legend: nodes are labeled in place.
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
                .child("Sankey Diagram — Energy Flows"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/sankey — {} nodes, {} links (energy.json)",
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
                            // Draw links first (background), then nodes
                            for (i, path_opt) in paths.into_iter().enumerate() {
                                if let Some(path) = path_opt {
                                    window.paint_path(path, all_colors[i]);
                                }
                            }
                        },
                    )
                    .size_full(),
                )
                .children(label_nodes.into_iter().map(|(name, lx, ly, is_right)| {
                    let mut d = div().absolute().top(px(ly as f32 - 5.0));
                    if is_right {
                        d = d.right(px((width - lx) as f32));
                    } else {
                        d = d.left(px(lx as f32));
                    }
                    d.child(div().text_size(px(9.0)).line_height(px(10.0)).child(name))
                })),
        )
}
