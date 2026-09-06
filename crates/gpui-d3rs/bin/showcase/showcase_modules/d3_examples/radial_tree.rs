//! Radial Tree / Radial Cluster — Observable examples
//!
//! Renders the full Flare hierarchy with `d3rs::examples::radial_tree`,
//! mirroring the official layouts:
//! tree <https://observablehq.com/@d3/radial-tree/2>,
//! cluster <https://observablehq.com/@d3/radial-cluster/2>.
//!
//! Labels follow the official placement rule for every node (offset 6px,
//! leaves outward, internal nodes inward, baseline flipped on the left
//! half), painted with the repo's rotatable Hershey vector font since filled
//! GPUI text cannot rotate.

use super::flare_data;
use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::examples::radial_tree::{
    FlareNode, RadialTreeResult, compute_with_root, labels as radial_labels,
};
use d3rs::hierarchy::HierarchyNode as D3HierarchyNode;
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use d3rs::text::vector_font::{measure_text_width, paint_vector_text_at};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;
use std::cell::RefCell;
use std::rc::Rc;

fn convert(
    node: &flare_data::HierarchyNode,
) -> Rc<RefCell<D3HierarchyNode<FlareNode>>> {
    let d3 = D3HierarchyNode::new(FlareNode {
        name: node.name.clone(),
        value: node.value.unwrap_or(0) as f64,
    });
    if !node.children.is_empty() {
        let kids = node.children.iter().map(convert).collect();
        d3.borrow_mut().set_children(&d3, kids);
    }
    d3
}

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    render_radial(false, &ui_theme)
}

pub fn render_cluster(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    render_radial(true, &ui_theme)
}

fn render_radial(cluster: bool, ui_theme: &gpui_ui_kit::theme::Theme) -> Div {
    let flare = flare_data::flare_hierarchy();
    let root = convert(&flare);
    let result: RadialTreeResult = compute_with_root(root, cluster);

    let width = result.width;
    let height = result.height;

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // Links (official: #555 at 0.4 opacity, 1.5px)
    for path in &result.link_paths {
        d3_paths.push(path.clone());
        all_colors.push(chart_colors::ink(ui_theme, hsla(0.0, 0.0, 0.33, 0.4)));
    }

    // Nodes as small circles (official: #555 internal, #999 leaves, r=2.5)
    let n_sides = 12;
    for node in &result.nodes {
        let r = 2.5;
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
        let shade = if node.is_leaf { 0.6 } else { 0.33 };
        all_colors.push(chart_colors::ink(ui_theme, hsla(0.0, 0.0, shade, 1.0)));
    }

    // Labels for every node, painted as rotated Hershey vector text along the
    // spokes (the repo's rotatable-text primitive; filled GPUI text cannot
    // rotate). Placement follows the official rule: the text center sits half
    // a measured width plus 6px from the node, outward for leaves and inward
    // for internal nodes, with the baseline flipped on the left half.
    let font_size = 10.0f32;
    let label_specs: Vec<(String, f32, f32, f32)> = radial_labels(&result)
        .iter()
        .map(|label| {
            let width = measure_text_width(&label.name, font_size);
            let spoke = label.angle - std::f64::consts::FRAC_PI_2;
            let (ux, uy) = (spoke.cos(), spoke.sin());
            let side = if label.outward { 1.0 } else { -1.0 };
            let dist = width as f64 / 2.0 + 6.0;
            let cx = label.x + side * ux * dist;
            let cy = label.y + side * uy * dist;
            (
                label.name.clone(),
                cx as f32,
                cy as f32,
                label.rotation as f32,
            )
        })
        .collect();
    let label_color = ui_theme.text_primary;

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
                        move |bounds, paths, window, _| {
                            for (i, path_opt) in paths.into_iter().enumerate() {
                                if let Some(path) = path_opt {
                                    window.paint_path(path, all_colors[i]);
                                }
                            }
                            let ox: f32 = bounds.origin.x.into();
                            let oy: f32 = bounds.origin.y.into();
                            for (name, cx, cy, rotation) in &label_specs {
                                paint_vector_text_at(
                                    window,
                                    name,
                                    ox + cx,
                                    oy + cy,
                                    font_size,
                                    1.0,
                                    label_color,
                                    *rotation,
                                );
                            }
                        },
                    )
                    .size_full(),
                ),
        )
}
