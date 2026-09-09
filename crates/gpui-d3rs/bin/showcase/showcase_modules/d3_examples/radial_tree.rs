//! Radial Tree / Radial Cluster — Observable examples
//!
//! Renders the full Flare hierarchy with `d3rs::examples::radial_tree`,
//! mirroring the official layouts:
//! tree <https://observablehq.com/@d3/radial-tree/2>,
//! cluster <https://observablehq.com/@d3/radial-cluster/2>.
//!
//! Labels follow the official placement rule for every node (offset 6px,
//! leaves outward, internal nodes inward, baseline flipped on the left
//! half), shaped with the bundled DejaVu Sans through the GPU-text engine
//! and replayed as rotated scene runs (Phase 3 of
//! `reviews/20260906-gpu-text.md`).

use super::flare_data;
use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::examples::radial_tree::{
    FlareNode, RadialTreeResult, compute_with_root, labels as radial_labels,
};
use d3rs::gputext::{FAMILY_SANS, FontEngine, TextWeight};
use d3rs::hierarchy::HierarchyNode as D3HierarchyNode;
use d3rs::shape::path::{Path, PathCommand};
use d3rs::vello2d::kurbo::{Affine, BezPath, PathEl, Stroke};
use d3rs::vello2d::peniko::{Brush, Color};
use d3rs::vello2d::{ChartScene, VelloChartElement};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;
use std::cell::RefCell;
use std::rc::Rc;

fn convert(node: &flare_data::HierarchyNode) -> Rc<RefCell<D3HierarchyNode<FlareNode>>> {
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

    // Links as kurbo paths for scene strokes. Official is #555 at 0.4 over
    // white; the adapted lightness is kept but alpha lifts to 0.6 — 1.5px
    // hairlines at 0.4 dissolve over the dark surface while staying subtle.
    let link_ink = chart_colors::ink(ui_theme, hsla(0.0, 0.0, 0.33, 0.4));
    let link_brush = ink_brush(hsla(link_ink.h, link_ink.s, link_ink.l, 0.6));
    let links: Vec<BezPath> = result.link_paths.iter().map(d3_path_to_kurbo).collect();

    // Nodes as center points (official: #555 internal, #999 leaves, r=2.5).
    let internal_brush = ink_brush(chart_colors::ink(ui_theme, hsla(0.0, 0.0, 0.33, 1.0)));
    let leaf_brush = ink_brush(chart_colors::ink(ui_theme, hsla(0.0, 0.0, 0.6, 1.0)));
    let nodes: Vec<(f64, f64, bool)> = result
        .nodes
        .iter()
        .map(|node| (node.x, node.y, node.is_leaf))
        .collect();

    // Labels keep the official placement rule (offset 6px, leaves outward,
    // baseline flipped on the left half); only the width source changes from
    // Hershey metrics to shaped advances.
    let labels = radial_labels(&result);
    let label_brush = ink_brush(ui_theme.text_primary);

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
                    VelloChartElement::with_builder(move |_, _| {
                        let mut engine = FontEngine::new();
                        let mut scene = ChartScene::new();
                        let link_stroke = Stroke::new(1.5);
                        for path in &links {
                            scene.stroke_path(
                                path.clone(),
                                link_stroke.clone(),
                                link_brush.clone(),
                            );
                        }
                        for (x, y, is_leaf) in &nodes {
                            scene.fill_circle(
                                *x,
                                *y,
                                2.5,
                                if *is_leaf {
                                    leaf_brush.clone()
                                } else {
                                    internal_brush.clone()
                                },
                            );
                        }
                        let font_size = 10.0f32;
                        for label in &labels {
                            let shaped_width = FontEngine::line_width(&engine.shape(
                                &label.name,
                                font_size,
                                FAMILY_SANS,
                                TextWeight::NORMAL,
                            ));
                            let spoke = label.angle - std::f64::consts::FRAC_PI_2;
                            let (ux, uy) = (spoke.cos(), spoke.sin());
                            let side = if label.outward { 1.0 } else { -1.0 };
                            let dist = shaped_width as f64 / 2.0 + 6.0;
                            let cx = label.x + side * ux * dist;
                            let cy = label.y + side * uy * dist;
                            // Anchor the run's cap-middle center on the label
                            // point, matching the old Hershey centering.
                            let anchor = Affine::translate((cx, cy))
                                * Affine::rotate(label.rotation)
                                * Affine::translate((
                                    -shaped_width as f64 / 2.0,
                                    0.35 * font_size as f64,
                                ));
                            scene.fill_text(
                                &mut engine,
                                &label.name,
                                font_size,
                                FAMILY_SANS,
                                TextWeight::NORMAL,
                                anchor,
                                label_brush.clone(),
                            );
                        }
                        scene
                    })
                    .absolute(),
                ),
        )
}

/// Theme color (`Hsla` ink or `Rgba` field) into a solid scene brush.
fn ink_brush(color: impl Into<Rgba>) -> Brush {
    let rgba: Rgba = color.into();
    Brush::Solid(Color::new([rgba.r, rgba.g, rgba.b, rgba.a]))
}

/// Map a d3 path onto kurbo, preserving subpaths. Radial links only emit
/// `MoveTo` + `CubicCurveTo`; arcs fall back to endpoint lines (never
/// exercised here) and rects map corner to corner.
fn d3_path_to_kurbo(path: &Path) -> BezPath {
    let mut out = BezPath::new();
    let (mut cx, mut cy) = (0.0, 0.0);
    for cmd in path.commands() {
        match *cmd {
            PathCommand::MoveTo { x, y } => {
                out.push(PathEl::MoveTo((x, y).into()));
                cx = x;
                cy = y;
            }
            PathCommand::LineTo { x, y } => {
                out.push(PathEl::LineTo((x, y).into()));
                cx = x;
                cy = y;
            }
            PathCommand::HorizontalLineTo { x } => {
                cx = x;
                out.push(PathEl::LineTo((cx, cy).into()));
            }
            PathCommand::VerticalLineTo { y } => {
                cy = y;
                out.push(PathEl::LineTo((cx, cy).into()));
            }
            PathCommand::ClosePath => out.push(PathEl::ClosePath),
            PathCommand::QuadraticCurveTo { x1, y1, x, y } => {
                out.push(PathEl::QuadTo((x1, y1).into(), (x, y).into()));
                cx = x;
                cy = y;
            }
            PathCommand::CubicCurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                out.push(PathEl::CurveTo(
                    (x1, y1).into(),
                    (x2, y2).into(),
                    (x, y).into(),
                ));
                cx = x;
                cy = y;
            }
            PathCommand::Arc { x, y, .. } | PathCommand::EllipticalArc { x, y, .. } => {
                out.push(PathEl::LineTo((x, y).into()));
                cx = x;
                cy = y;
            }
            PathCommand::Rect {
                x,
                y,
                width,
                height,
            } => {
                out.push(PathEl::MoveTo((x, y).into()));
                out.push(PathEl::LineTo(((x + width), y).into()));
                out.push(PathEl::LineTo(((x + width), (y + height)).into()));
                out.push(PathEl::LineTo((x, (y + height)).into()));
                out.push(PathEl::ClosePath);
                cx = x;
                cy = y;
            }
        }
    }
    out
}
