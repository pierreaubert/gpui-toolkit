//! Force-Directed Graph -- Observable example using d3rs::examples::force_directed
//!
//! Loads the full Les Miserables dataset from miserables.json (77 nodes, 254 links).
//! Demonstrates: `Simulation`, `ForceLink`, `ForceManyBody`, `ForceCenter`,
//! `LinearScale`, `ColorScheme`, `PathBuilder`, `d3rs_path_to_gpui_simple`.
use crate::ShowcaseApp;
use d3rs::color::ColorScheme;
use d3rs::scale::{LinearScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;
use std::collections::HashMap;
use std::rc::Rc;

/// Embedded miserables.json data (77 nodes, 254 links).
const MISERABLES_JSON: &str = include_str!("../../data/miserables.json");

/// Cached force-directed graph data so the full simulation and path generation
/// run only once.
pub struct ForceDirectedCache {
    pub node_count: usize,
    pub link_count: usize,
    pub d3_paths: Rc<[d3rs::shape::path::Path]>,
    pub all_colors: Rc<[Hsla]>,
    pub groups: Rc<[usize]>,
    pub scheme: ColorScheme,
}

fn build_cache() -> Rc<ForceDirectedCache> {
    // Load the full Les Miserables dataset from JSON
    let (node_data, links) = d3rs::examples::force_directed::load_json(MISERABLES_JSON);

    // Run simulation with all three forces
    let result = d3rs::examples::force_directed::compute(&node_data, &links, 300);

    let scheme = ColorScheme::tableau10();

    // Observable default: 640×400, viewBox centered at [-w/2, -h/2, w, h]
    let width = 640.0_f64;
    let height = 400.0_f64;
    let margin = 20.0_f64;

    // Map simulation positions (centered at 0,0) to rendering area
    let x_ext = result
        .nodes
        .iter()
        .fold((f64::MAX, f64::MIN), |a, n| (a.0.min(n.x), a.1.max(n.x)));
    let y_ext = result
        .nodes
        .iter()
        .fold((f64::MAX, f64::MIN), |a, n| (a.0.min(n.y), a.1.max(n.y)));

    let x_scale = LinearScale::new()
        .domain(x_ext.0, x_ext.1)
        .range(margin, width - margin);
    let y_scale = LinearScale::new()
        .domain(y_ext.0, y_ext.1)
        .range(margin, height - margin);

    let node_positions: Vec<(f64, f64, usize)> = result
        .nodes
        .iter()
        .map(|n| (x_scale.scale(n.x), y_scale.scale(n.y), n.group))
        .collect();

    // Fast id -> index lookup for link resolution.
    let node_id_to_index: HashMap<String, usize> = result
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect();

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // Links as thin lines (Observable: stroke="#999", stroke-opacity=0.6, stroke-width=sqrt(value)*1.5)
    for link in &result.links {
        if let (Some(&si), Some(&ti)) = (
            node_id_to_index.get(&link.source),
            node_id_to_index.get(&link.target),
        ) {
            let (sx, sy, _) = node_positions[si];
            let (tx, ty, _) = node_positions[ti];
            let dx = tx - sx;
            let dy = ty - sy;
            let len = (dx * dx + dy * dy).sqrt().max(1e-6);
            let half_w = (link.value as f64).sqrt() * 0.75;
            let nx = -dy / len * half_w;
            let ny = dx / len * half_w;
            let path = D3PathBuilder::new()
                .move_to(sx + nx, sy + ny)
                .line_to(tx + nx, ty + ny)
                .line_to(tx - nx, ty - ny)
                .line_to(sx - nx, sy - ny)
                .close_path()
                .build();
            d3_paths.push(path);
            // Observable: #999 at 0.6 opacity
            all_colors.push(hsla(0.0, 0.0, 0.6, 0.6));
        }
    }

    // Nodes as filled circles — Observable: constant 5px radius
    let node_radius = 5.0;
    let n_sides = 16;
    for (px_val, py_val, group) in &node_positions {
        let mut builder = D3PathBuilder::new();
        for v in 0..n_sides {
            let angle = std::f64::consts::TAU * v as f64 / n_sides as f64;
            let x = px_val + node_radius * angle.cos();
            let y = py_val + node_radius * angle.sin();
            if v == 0 {
                builder = builder.move_to(x, y);
            } else {
                builder = builder.line_to(x, y);
            }
        }
        builder = builder.close_path();
        d3_paths.push(builder.build());
        all_colors.push(scheme.color(*group).to_rgba().into());
    }

    // Legend: unique groups
    let mut groups: Vec<usize> = result.nodes.iter().map(|n| n.group).collect();
    groups.sort();
    groups.dedup();

    Rc::new(ForceDirectedCache {
        node_count: result.nodes.len(),
        link_count: result.links.len(),
        d3_paths: d3_paths.into(),
        all_colors: all_colors.into(),
        groups: groups.into(),
        scheme,
    })
}

fn ensure_cache(app: &mut ShowcaseApp) -> Rc<ForceDirectedCache> {
    if let Some(cache) = app.force_directed_cache.clone() {
        return cache;
    }
    let cache = build_cache();
    app.force_directed_cache = Some(cache.clone());
    cache
}

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let cache = ensure_cache(app);

    let width = 640.0_f64;
    let height = 400.0_f64;

    let scheme = cache.scheme.clone();
    let legend_items = cache
        .groups
        .iter()
        .map(|&g| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(div().size_3().bg(scheme.color(g).to_rgba()))
                .child(div().text_xs().child(format!("Group {}", g)))
        })
        .collect::<Vec<_>>();

    let cache_for_paths = cache.clone();
    let cache_for_paint = cache.clone();

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
                .child("Force-Directed Graph — Les Miserables"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/force-directed-graph — {} nodes, {} links",
            cache.node_count, cache.link_count
        )))
        .child(
            div()
                .flex()
                .gap_2()
                .mb_2()
                .flex_wrap()
                .children(legend_items),
        )
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
                            cache_for_paths
                                .d3_paths
                                .iter()
                                .map(|p| {
                                    super::path_utils::d3rs_path_to_gpui_simple(p, bounds, 0.0, 0.0)
                                })
                                .collect::<Vec<_>>()
                        },
                        move |_bounds, paths, window, _| {
                            for (i, path_opt) in paths.into_iter().enumerate() {
                                if let Some(path) = path_opt {
                                    window.paint_path(path, cache_for_paint.all_colors[i]);
                                }
                            }
                        },
                    )
                    .size_full(),
                ),
        )
}
