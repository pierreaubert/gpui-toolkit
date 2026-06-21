//! Radial Tree — Observable example using d3rs::examples::radial_tree
//!
//! Renders a tree hierarchy in radial (polar) layout.
//! Source: <https://observablehq.com/@d3/radial-tree/2>

use crate::ShowcaseApp;
use d3rs::color::ColorScheme;
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use d3rs::text::{
    GlyphTextConfig, HorizontalTextAnchor, VerticalTextAnchor, render_glyph_text_anchored,
};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    render_radial(false, &ui_theme)
}

pub fn render_cluster(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    render_radial(true, &ui_theme)
}

fn render_radial(cluster: bool, ui_theme: &gpui_ui_kit::theme::Theme) -> Div {
    let result = d3rs::examples::radial_tree::compute(cluster);

    let scheme = ColorScheme::tableau10();
    let width = result.width;
    let height = result.height;

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // Links
    for path in &result.link_paths {
        d3_paths.push(path.clone());
        all_colors.push(hsla(0.0, 0.0, 0.75, 0.5));
    }

    // Nodes as small circles
    let n_sides = 12;
    for node in &result.nodes {
        let r = if node.is_leaf { 2.5 } else { 4.0 };
        let mut builder = D3PathBuilder::new();
        for v in 0..n_sides {
            let angle = std::f64::consts::TAU * v as f64 / n_sides as f64;
            let x = node.x + r * angle.cos();
            let y = node.y + r * angle.sin();
            if v == 0 {
                builder = builder.move_to(x, y);
            } else {
                builder = builder.line_to(x, y);
            }
        }
        builder = builder.close_path();
        d3_paths.push(builder.build());
        all_colors.push(scheme.color(node.depth).to_rgba().into());
    }

    // Node labels for internal nodes, rotated tangent to the circle.
    let label_items: Vec<Div> = result
        .nodes
        .iter()
        .filter(|n| !n.is_leaf)
        .map(|n| {
            let on_left = n.angle > std::f64::consts::PI;
            let std_angle = n.angle - std::f64::consts::FRAC_PI_2;
            let rotation = std_angle as f32 + if on_left { std::f32::consts::PI } else { 0.0 };
            let h_anchor = if on_left {
                HorizontalTextAnchor::Start
            } else {
                HorizontalTextAnchor::End
            };

            let config = GlyphTextConfig::rotated(8.0, ui_theme.text_primary, rotation);

            div()
                .absolute()
                .left(px(n.x as f32))
                .top(px(n.y as f32))
                .child(render_glyph_text_anchored(
                    &n.name,
                    &config,
                    h_anchor,
                    VerticalTextAnchor::Middle,
                ))
        })
        .collect();

    let title = if cluster {
        "Radial Cluster — Flare Hierarchy"
    } else {
        "Radial Tree — Flare Hierarchy"
    };
    let source = if cluster {
        "observablehq.com/@d3/radial-cluster/2"
    } else {
        "observablehq.com/@d3/radial-tree/2"
    };

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
                .child(title),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: {} — {} nodes, {} links",
            source,
            result.nodes.len(),
            result.link_paths.len()
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
                // Internal node labels
                .children(label_items),
        )
}
