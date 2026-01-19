use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::prelude::*;
use d3rs::quadtree::{QuadNode, QuadTree};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_ui_kit::Slider;

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let entity = cx.entity().clone();
    let theme = DefaultAxisTheme;

    // Generate random points for demonstration
    let points: Vec<(f64, f64)> = (0..50)
        .map(|i| {
            let angle = i as f64 * 0.15;
            let r = 20.0 + 30.0 * (i as f64 * 0.07).sin();
            (50.0 + r * angle.cos(), 50.0 + r * angle.sin())
        })
        .collect();

    // Build quadtree - store coordinates as data for easy retrieval
    let mut quadtree: QuadTree<(f64, f64)> = QuadTree::new();
    for &(x, y) in &points {
        quadtree.add(x, y, (x, y));
    }

    // Query parameters from state
    let query_x = app.quadtree_query_x as f64;
    let query_y = app.quadtree_query_y as f64;
    let search_radius = app.quadtree_search_radius as f64;

    // Find nearest point
    let nearest = quadtree.find(query_x, query_y, None);

    // Find all points within radius
    let within_radius = quadtree.find_all(query_x, query_y, search_radius);

    // Scales for the visualization
    let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
    let y_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);

    // Collect quadtree bounds for visualization
    let mut bounds_list: Vec<(f64, f64, f64, f64)> = Vec::new();
    if let Some(_ext) = quadtree.extent() {
        // Visit quadtree nodes to collect bounds
        quadtree.visit(|nx0, ny0, nx1, ny1, node| {
            bounds_list.push((nx0, ny0, nx1, ny1));
            match node {
                QuadNode::Internal(_) => false, // Continue visiting children
                QuadNode::Leaf(_) => true,      // Stop at leaves
            }
        });
    }

    // Build a set of points within radius for efficient lookup
    let within_radius_set: std::collections::HashSet<(i64, i64)> = {
        let mut set = std::collections::HashSet::new();
        for &(pt_x, pt_y) in &points {
            let dist_sq = (pt_x - query_x).powi(2) + (pt_y - query_y).powi(2);
            if dist_sq <= search_radius * search_radius {
                // Store as integer keys to avoid floating point comparison
                set.insert(((pt_x * 1000.0) as i64, (pt_y * 1000.0) as i64));
            }
        }
        set
    };

    div()
        .flex()
        .gap_8()
        // Left side: Visualization
        .child(
            div()
                .flex()
                .flex_col()
                .gap_6()
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::BOLD)
                        .child("QuadTree Demo"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x666666))
                        .max_w(px(500.0))
                        .child("QuadTree is a 2D spatial index for efficient nearest-neighbor queries. Move the query point and adjust the search radius to see how the quadtree partitions space."),
                )
                // Main visualization
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .items_start()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Spatial Partitioning"),
                        )
                        .child(
                            div()
                                .flex()
                                .items_start()
                                // Left axis
                                .child(render_axis(
                                    &y_scale,
                                    &AxisConfig::left().with_ticks(5),
                                    400.0,
                                    &theme,
                                ))
                                // Plot area
                                .child(
                                    div()
                                        .w(px(400.0))
                                        .h(px(400.0))
                                        .bg(rgb(0xf8f8f8))
                                        .border_1()
                                        .border_color(rgb(0xcccccc))
                                        .relative()
                                        // Draw quadtree partitions
                                        .children(bounds_list.iter().map(|&(bx0, by0, bx1, by1)| {
                                            let px_x = x_scale.scale(bx0) as f32;
                                            let px_y = (400.0 - y_scale.scale(by1)) as f32;
                                            let px_w = (x_scale.scale(bx1) - x_scale.scale(bx0)) as f32;
                                            let px_h = (y_scale.scale(by1) - y_scale.scale(by0)) as f32;
                                            div()
                                                .absolute()
                                                .left(px(px_x))
                                                .top(px(px_y))
                                                .w(px(px_w))
                                                .h(px(px_h))
                                                .border_1()
                                                .border_color(rgba(0x0066cc40))
                                        }))
                                        // Draw search radius circle
                                        .child(
                                            div()
                                                .absolute()
                                                .left(px((x_scale.scale(query_x) - x_scale.scale(search_radius) + x_scale.scale(0.0)) as f32))
                                                .top(px((400.0 - y_scale.scale(query_y) - y_scale.scale(search_radius) + y_scale.scale(0.0)) as f32))
                                                .w(px((2.0 * (x_scale.scale(search_radius) - x_scale.scale(0.0))) as f32))
                                                .h(px((2.0 * (y_scale.scale(search_radius) - y_scale.scale(0.0))) as f32))
                                                .rounded_full()
                                                .bg(rgba(0x00aa0020))
                                                .border_2()
                                                .border_color(rgba(0x00aa0080))
                                        )
                                        // Draw all points
                                        .children(points.iter().map(|&(pt_x, pt_y)| {
                                            let key = ((pt_x * 1000.0) as i64, (pt_y * 1000.0) as i64);
                                            let is_in_radius = within_radius_set.contains(&key);
                                            let color = if is_in_radius { rgb(0x00aa00) } else { rgb(0x666666) };
                                            div()
                                                .absolute()
                                                .left(gpui::px((x_scale.scale(pt_x) - 4.0) as f32))
                                                .top(gpui::px((400.0 - y_scale.scale(pt_y) - 4.0) as f32))
                                                .w(gpui::px(8.0))
                                                .h(gpui::px(8.0))
                                                .rounded_full()
                                                .bg(color)
                                        }))
                                        // Draw query point
                                        .child(
                                            div()
                                                .absolute()
                                                .left(px((x_scale.scale(query_x) - 6.0) as f32))
                                                .top(px((400.0 - y_scale.scale(query_y) - 6.0) as f32))
                                                .w(px(12.0))
                                                .h(px(12.0))
                                                .rounded_full()
                                                .bg(rgb(0xff0000))
                                                .border_2()
                                                .border_color(rgb(0xffffff))
                                        )
                                        // Draw nearest point highlight
                                        .when_some(nearest.cloned(), |this, (nx, ny)| {
                                            this.child(
                                                div()
                                                    .absolute()
                                                    .left(px((x_scale.scale(nx) - 8.0) as f32))
                                                    .top(px((400.0 - y_scale.scale(ny) - 8.0) as f32))
                                                    .w(px(16.0))
                                                    .h(px(16.0))
                                                    .rounded_full()
                                                    .border_3()
                                                    .border_color(rgb(0xff6600))
                                            )
                                        }),
                                ),
                        )
                        // Bottom axis
                        .child(
                            div()
                                .flex()
                                // Spacer for left axis
                                .child(div().w(px(60.0)))
                                .child(render_axis(
                                    &x_scale,
                                    &AxisConfig::bottom().with_ticks(5),
                                    400.0,
                                    &theme,
                                )),
                        ),
                )
                // Legend
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .mt_2()
                        .text_xs()
                        .child(
                            div()
                                .flex()
                                .gap_1()
                                .items_center()
                                .child(div().w(px(12.0)).h(px(12.0)).rounded_full().bg(rgb(0xff0000)))
                                .child("Query Point"),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_1()
                                .items_center()
                                .child(div().w(px(12.0)).h(px(12.0)).rounded_full().border_2().border_color(rgb(0xff6600)))
                                .child("Nearest"),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_1()
                                .items_center()
                                .child(div().w(px(12.0)).h(px(12.0)).rounded_full().bg(rgb(0x00aa00)))
                                .child("Within Radius"),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_1()
                                .items_center()
                                .child(div().w(px(12.0)).h(px(12.0)).border_1().border_color(rgba(0x0066cc80)))
                                .child("QuadTree Cell"),
                        ),
                ),
        )
        // Right side: Controls
        .child(
            div()
                .w(px(280.0))
                .flex()
                .flex_col()
                .gap_4()
                .p_4()
                .bg(rgb(0xf8f8f8))
                .border_1()
                .border_color(rgb(0xe0e0e0))
                .rounded_lg()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x333333))
                        .child("Controls"),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(0x555555))
                                .child("Query Point"),
                        )
                        .child({
                            let entity = entity.clone();
                            Slider::new("query-x")
                                .label("X")
                                .value(app.quadtree_query_x)
                                .min(0.0)
                                .max(100.0)
                                .step(1.0)
                                .show_value(true)
                                .width(220.0)
                                .on_change(move |value, _window, cx| {
                                    entity.update(cx, |this, _| {
                                        this.quadtree_query_x = value;
                                    });
                                })
                        })
                        .child({
                            let entity = entity.clone();
                            Slider::new("query-y")
                                .label("Y")
                                .value(app.quadtree_query_y)
                                .min(0.0)
                                .max(100.0)
                                .step(1.0)
                                .show_value(true)
                                .width(220.0)
                                .on_change(move |value, _window, cx| {
                                    entity.update(cx, |this, _| {
                                        this.quadtree_query_y = value;
                                    });
                                })
                        })
                        .child({
                            let entity = entity.clone();
                            Slider::new("search-radius")
                                .label("Search Radius")
                                .value(app.quadtree_search_radius)
                                .min(5.0)
                                .max(50.0)
                                .step(1.0)
                                .show_value(true)
                                .width(220.0)
                                .on_change(move |value, _window, cx| {
                                    entity.update(cx, |this, _| {
                                        this.quadtree_search_radius = value;
                                    });
                                })
                        }),
                )
                // Statistics
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .mt_4()
                        .p_3()
                        .bg(rgb(0xffffff))
                        .border_1()
                        .border_color(rgb(0xe0e0e0))
                        .rounded_md()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(0x888888))
                                .child("STATISTICS"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x333333))
                                .child(format!("Total Points: {}", quadtree.size())),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x333333))
                                .child(format!("Within Radius: {}", within_radius.len())),
                        )
                        .when_some(nearest.cloned(), |this, (nx, ny)| {
                            this.child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x333333))
                                    .child(format!("Nearest: ({:.1}, {:.1})", nx, ny)),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x333333))
                                    .child(format!(
                                        "Distance: {:.2}",
                                        ((nx - query_x).powi(2) + (ny - query_y).powi(2)).sqrt()
                                    )),
                            )
                        })
                        .when_some(quadtree.extent(), |this, ext| {
                            this.child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x333333))
                                    .child(format!("Extent: [{:.0},{:.0}]-[{:.0},{:.0}]", ext.x0, ext.y0, ext.x1, ext.y1)),
                            )
                        })
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x333333))
                                .child(format!("Cells: {}", bounds_list.len())),
                        ),
                )
                // API Examples
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .mt_4()
                        .p_3()
                        .bg(rgb(0x2d2d2d))
                        .rounded_md()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(0x888888))
                                .child("API USAGE"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x9cdcfe))
                                .font_family("Monaco")
                                .child("let mut qt = QuadTree::new();"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x9cdcfe))
                                .font_family("Monaco")
                                .child("qt.add(x, y, data);"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x9cdcfe))
                                .font_family("Monaco")
                                .child("qt.find(x, y, radius);"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x9cdcfe))
                                .font_family("Monaco")
                                .child("qt.find_all(x, y, radius);"),
                        ),
                ),
        )
}

use super::ShowcaseApp;
