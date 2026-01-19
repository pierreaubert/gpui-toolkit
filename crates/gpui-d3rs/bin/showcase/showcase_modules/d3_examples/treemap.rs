//! Treemap Visualization - D3.js Example Port
//!
//! This example demonstrates treemap layout for hierarchical data,
//! ported from: https://observablehq.com/@d3/treemap/2
//!
//! The example shows:
//! 1. Multiple tiling algorithms (Squarify, Binary, Slice, Dice, SliceDice)
//! 2. Color coding by top-level category
//! 3. Interactive controls for tiling method

use super::flare_data::{HierarchyNode, flare_hierarchy, top_level_categories};
use crate::ShowcaseApp;
use d3rs::color::D3Color;
use gpui::prelude::FluentBuilder;
use gpui::*;

/// Tiling algorithm for treemap layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TilingMethod {
    #[default]
    Squarify,
    Binary,
    Slice,
    Dice,
    SliceDice,
}

impl TilingMethod {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Squarify => "Squarify",
            Self::Binary => "Binary",
            Self::Slice => "Slice",
            Self::Dice => "Dice",
            Self::SliceDice => "Slice-Dice",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Squarify => Self::Binary,
            Self::Binary => Self::Slice,
            Self::Slice => Self::Dice,
            Self::Dice => Self::SliceDice,
            Self::SliceDice => Self::Squarify,
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Squarify,
            Self::Binary,
            Self::Slice,
            Self::Dice,
            Self::SliceDice,
        ]
    }
}

/// A rectangle in the treemap layout
#[derive(Debug, Clone)]
pub struct TreemapRect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub name: String,
    #[allow(dead_code)] // Reserved for tooltip display
    pub value: u64,
    #[allow(dead_code)] // Reserved for depth-based styling
    pub depth: usize,
    pub category_index: usize, // Index of top-level category for coloring
}

impl TreemapRect {
    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> f64 {
        self.y1 - self.y0
    }
}

/// Compute treemap layout for a hierarchy
pub fn compute_treemap(
    node: &HierarchyNode,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    method: TilingMethod,
    padding: f64,
    depth: usize,
    category_index: usize,
    results: &mut Vec<TreemapRect>,
) {
    let total_value = node.total_value() as f64;
    if total_value == 0.0 {
        return;
    }

    // Apply padding
    let px0 = x0 + padding;
    let py0 = y0 + padding;
    let px1 = x1 - padding;
    let py1 = y1 - padding;

    if px1 <= px0 || py1 <= py0 {
        return;
    }

    if node.is_leaf() {
        // Add leaf rectangle
        results.push(TreemapRect {
            x0: px0,
            y0: py0,
            x1: px1,
            y1: py1,
            name: node.name.clone(),
            value: node.value.unwrap_or(0),
            depth,
            category_index,
        });
    } else {
        // Layout children based on tiling method
        let children: Vec<_> = node
            .children
            .iter()
            .map(|c| (c, c.total_value() as f64))
            .collect();

        let rects = match method {
            TilingMethod::Squarify => tile_squarify(&children, px0, py0, px1, py1, total_value),
            TilingMethod::Binary => tile_binary(&children, px0, py0, px1, py1, total_value),
            TilingMethod::Slice => tile_slice(&children, px0, py0, px1, py1, total_value),
            TilingMethod::Dice => tile_dice(&children, px0, py0, px1, py1, total_value),
            TilingMethod::SliceDice => {
                tile_slice_dice(&children, px0, py0, px1, py1, total_value, depth)
            }
        };

        // Recursively process children
        for (i, ((child, _), (cx0, cy0, cx1, cy1))) in children.iter().zip(rects.iter()).enumerate()
        {
            let child_category = if depth == 0 { i } else { category_index };
            compute_treemap(
                child,
                *cx0,
                *cy0,
                *cx1,
                *cy1,
                method,
                padding,
                depth + 1,
                child_category,
                results,
            );
        }
    }
}

/// Slice tiling - horizontal strips
fn tile_slice(
    children: &[(&HierarchyNode, f64)],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    total: f64,
) -> Vec<(f64, f64, f64, f64)> {
    let height = y1 - y0;
    let mut rects = Vec::new();
    let mut y = y0;

    for (_, value) in children {
        let h = (value / total) * height;
        rects.push((x0, y, x1, y + h));
        y += h;
    }

    rects
}

/// Dice tiling - vertical strips
fn tile_dice(
    children: &[(&HierarchyNode, f64)],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    total: f64,
) -> Vec<(f64, f64, f64, f64)> {
    let width = x1 - x0;
    let mut rects = Vec::new();
    let mut x = x0;

    for (_, value) in children {
        let w = (value / total) * width;
        rects.push((x, y0, x + w, y1));
        x += w;
    }

    rects
}

/// Slice-Dice tiling - alternates between slice and dice based on depth
fn tile_slice_dice(
    children: &[(&HierarchyNode, f64)],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    total: f64,
    depth: usize,
) -> Vec<(f64, f64, f64, f64)> {
    if depth.is_multiple_of(2) {
        tile_slice(children, x0, y0, x1, y1, total)
    } else {
        tile_dice(children, x0, y0, x1, y1, total)
    }
}

/// Binary tiling - recursively subdivides into two halves
fn tile_binary(
    children: &[(&HierarchyNode, f64)],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    _total: f64,
) -> Vec<(f64, f64, f64, f64)> {
    if children.is_empty() {
        return Vec::new();
    }
    if children.len() == 1 {
        return vec![(x0, y0, x1, y1)];
    }

    // Find the partition point that balances the two halves
    let total: f64 = children.iter().map(|(_, v)| *v).sum();
    let mut cumsum = 0.0;
    let mut split_idx = 0;
    let half = total / 2.0;

    for (i, (_, value)) in children.iter().enumerate() {
        cumsum += *value;
        if cumsum >= half {
            split_idx = i + 1;
            break;
        }
    }

    // Ensure we don't get empty splits
    split_idx = split_idx.max(1).min(children.len() - 1);

    let left: f64 = children[..split_idx].iter().map(|(_, v)| *v).sum();
    let right: f64 = children[split_idx..].iter().map(|(_, v)| *v).sum();
    let left_ratio = left / (left + right);

    let width = x1 - x0;
    let height = y1 - y0;

    let mut rects = Vec::new();

    if width >= height {
        // Split horizontally
        let mid_x = x0 + width * left_ratio;
        let left_rects = tile_binary(&children[..split_idx], x0, y0, mid_x, y1, left);
        let right_rects = tile_binary(&children[split_idx..], mid_x, y0, x1, y1, right);
        rects.extend(left_rects);
        rects.extend(right_rects);
    } else {
        // Split vertically
        let mid_y = y0 + height * left_ratio;
        let top_rects = tile_binary(&children[..split_idx], x0, y0, x1, mid_y, left);
        let bottom_rects = tile_binary(&children[split_idx..], x0, mid_y, x1, y1, right);
        rects.extend(top_rects);
        rects.extend(bottom_rects);
    }

    rects
}

/// Squarify tiling - creates rectangles with aspect ratios close to 1
fn tile_squarify(
    children: &[(&HierarchyNode, f64)],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    _total: f64,
) -> Vec<(f64, f64, f64, f64)> {
    if children.is_empty() {
        return Vec::new();
    }

    let width = x1 - x0;
    let height = y1 - y0;
    let area = width * height;
    let total: f64 = children.iter().map(|(_, v)| *v).sum();

    if total == 0.0 || area == 0.0 {
        return children.iter().map(|_| (x0, y0, x0, y0)).collect();
    }

    // Scale factor to convert values to areas
    let scale = area / total;

    let mut rects = vec![(0.0, 0.0, 0.0, 0.0); children.len()];
    let mut remaining_x0 = x0;
    let mut remaining_y0 = y0;
    let remaining_x1 = x1;
    let remaining_y1 = y1;

    let mut i = 0;
    while i < children.len() {
        let remaining_width = remaining_x1 - remaining_x0;
        let remaining_height = remaining_y1 - remaining_y0;

        if remaining_width <= 0.0 || remaining_height <= 0.0 {
            break;
        }

        // Determine if we're laying out horizontally or vertically
        let horizontal = remaining_width >= remaining_height;
        let side = if horizontal {
            remaining_height
        } else {
            remaining_width
        };

        // Find the best row
        let (row_end, row_sum) =
            find_best_row(&children[i..], side, scale, children.len() - i, horizontal);

        // Layout this row
        let row_length = row_sum * scale / side;

        if horizontal {
            // Layout vertically within a horizontal strip
            let mut y = remaining_y0;
            for j in i..(i + row_end) {
                let h = (children[j].1 * scale) / row_length;
                rects[j] = (remaining_x0, y, remaining_x0 + row_length, y + h);
                y += h;
            }
            remaining_x0 += row_length;
        } else {
            // Layout horizontally within a vertical strip
            let mut x = remaining_x0;
            for j in i..(i + row_end) {
                let w = (children[j].1 * scale) / row_length;
                rects[j] = (x, remaining_y0, x + w, remaining_y0 + row_length);
                x += w;
            }
            remaining_y0 += row_length;
        }

        i += row_end;
    }

    rects
}

/// Find the best row for squarify algorithm
fn find_best_row(
    children: &[(&HierarchyNode, f64)],
    side: f64,
    scale: f64,
    max_count: usize,
    _horizontal: bool,
) -> (usize, f64) {
    if children.is_empty() {
        return (0, 0.0);
    }

    let mut best_count = 1;
    let mut best_ratio = f64::MAX;
    let mut sum = 0.0;

    for (count, (_, value)) in children.iter().enumerate().take(max_count) {
        sum += *value;
        let row_length = sum * scale / side;
        if row_length == 0.0 {
            continue;
        }

        // Calculate the worst aspect ratio in this row
        let mut worst_ratio = 0.0f64;
        for (_, v) in children.iter().take(count + 1) {
            let rect_size = *v * scale / row_length;
            let ratio = if rect_size > row_length {
                rect_size / row_length
            } else {
                row_length / rect_size
            };
            worst_ratio = worst_ratio.max(ratio);
        }

        if worst_ratio < best_ratio {
            best_ratio = worst_ratio;
            best_count = count + 1;
        } else if count > 0 {
            // Ratio got worse, stop here
            break;
        }
    }

    let final_sum: f64 = children.iter().take(best_count).map(|(_, v)| *v).sum();
    (best_count, final_sum)
}

/// Get color for a category index using Tableau10 scheme
fn category_color(index: usize) -> D3Color {
    let colors = [
        0x4e79a7, // blue
        0xf28e2c, // orange
        0xe15759, // red
        0x76b7b2, // teal
        0x59a14f, // green
        0xedc949, // yellow
        0xaf7aa1, // purple
        0xff9da7, // pink
        0x9c755f, // brown
        0xbab0ab, // gray
    ];
    D3Color::from_hex(colors[index % colors.len()])
}

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let entity = cx.entity().clone();

    // Get parameters from app state
    let tiling_method = app.treemap_tiling;
    let padding = app.treemap_padding;

    // Build the treemap layout
    let root = flare_hierarchy();
    let total_value = root.total_value();

    // Plot dimensions
    let plot_size = 500.0_f32;

    // Compute layout
    let mut rects = Vec::new();
    compute_treemap(
        &root,
        0.0,
        0.0,
        plot_size as f64,
        plot_size as f64,
        tiling_method,
        padding as f64,
        0,
        0,
        &mut rects,
    );

    let categories = top_level_categories();

    div()
        .flex()
        .flex_col()
        .gap_6()
        // Title
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::BOLD)
                        .child("Treemap"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x666666))
                        .child("Ported from Observable: d3/treemap"),
                ),
        )
        // Main content
        .child(
            div()
                .flex()
                .gap_8()
                // Left: Visualization
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("Flare Package Hierarchy"),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .bg(rgb(0x007acc))
                                        .rounded_md()
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child(format!("{} rects", rects.len())),
                                ),
                        )
                        .child(
                            div()
                                .w(px(plot_size))
                                .h(px(plot_size))
                                .bg(rgb(0xffffff))
                                .border_1()
                                .border_color(rgb(0xe0e0e0))
                                .rounded_md()
                                .relative()
                                .overflow_hidden()
                                // Treemap rectangles
                                .children(rects.iter().filter_map(|rect| {
                                    let w = rect.width() as f32;
                                    let h = rect.height() as f32;
                                    // Skip very small rectangles
                                    if w < 2.0 || h < 2.0 {
                                        return None;
                                    }

                                    let color = category_color(rect.category_index);
                                    let show_label = w > 30.0 && h > 14.0;

                                    Some(
                                        div()
                                            .absolute()
                                            .left(px(rect.x0 as f32))
                                            .top(px(rect.y0 as f32))
                                            .w(px(w))
                                            .h(px(h))
                                            .bg(color.to_rgba())
                                            .opacity(0.7)
                                            .border_1()
                                            .border_color(rgba(0xffffff66))
                                            .overflow_hidden()
                                            .when(show_label, |this| {
                                                this.child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(0xffffff))
                                                        .p_1()
                                                        .overflow_hidden()
                                                        .child(rect.name.clone()),
                                                )
                                            }),
                                    )
                                })),
                        ),
                )
                // Right: Controls and legend
                .child(
                    div()
                        .w(px(300.0))
                        .flex()
                        .flex_col()
                        .gap_4()
                        // Controls panel
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_3()
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
                                // Tiling method selector
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(0x555555))
                                                .child("Tiling Method"),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .flex_wrap()
                                                .gap_2()
                                                .children(TilingMethod::all().into_iter().map(
                                                    |method| {
                                                        let entity = entity.clone();
                                                        let is_selected = method == tiling_method;
                                                        let bg = if is_selected {
                                                            rgb(0x007acc)
                                                        } else {
                                                            rgb(0xe0e0e0)
                                                        };
                                                        let text_color = if is_selected {
                                                            rgb(0xffffff)
                                                        } else {
                                                            rgb(0x333333)
                                                        };

                                                        div()
                                                            .id(ElementId::Name(
                                                                format!("tile-{}", method.label())
                                                                    .into(),
                                                            ))
                                                            .px_2()
                                                            .py_1()
                                                            .bg(bg)
                                                            .hover(|s| s.opacity(0.8))
                                                            .rounded_md()
                                                            .cursor_pointer()
                                                            .text_xs()
                                                            .text_color(text_color)
                                                            .child(method.label())
                                                            .on_click(move |_, _window, cx| {
                                                                entity.update(cx, |this, _| {
                                                                    this.treemap_tiling = method;
                                                                });
                                                            })
                                                    },
                                                )),
                                        ),
                                )
                                // Padding slider
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(0x555555))
                                                .child("Padding"),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .gap_2()
                                                .children([0.0_f32, 1.0, 2.0, 3.0].iter().map(
                                                    |&p| {
                                                        let entity = entity.clone();
                                                        let is_selected =
                                                            (padding - p).abs() < 0.1;
                                                        let bg = if is_selected {
                                                            rgb(0x007acc)
                                                        } else {
                                                            rgb(0xe0e0e0)
                                                        };
                                                        let text_color = if is_selected {
                                                            rgb(0xffffff)
                                                        } else {
                                                            rgb(0x333333)
                                                        };

                                                        div()
                                                            .id(ElementId::Name(
                                                                format!("pad-{}", p as i32).into(),
                                                            ))
                                                            .px_2()
                                                            .py_1()
                                                            .bg(bg)
                                                            .hover(|s| s.opacity(0.8))
                                                            .rounded_md()
                                                            .cursor_pointer()
                                                            .text_xs()
                                                            .text_color(text_color)
                                                            .child(format!("{}", p as i32))
                                                            .on_click(move |_, _window, cx| {
                                                                entity.update(cx, |this, _| {
                                                                    this.treemap_padding = p;
                                                                });
                                                            })
                                                    },
                                                )),
                                        ),
                                ),
                        )
                        // Legend
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .p_4()
                                .bg(rgb(0xffffff))
                                .border_1()
                                .border_color(rgb(0xe0e0e0))
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(0x888888))
                                        .child("CATEGORIES"),
                                )
                                .children(categories.iter().enumerate().map(|(i, &name)| {
                                    let color = category_color(i);
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .w(px(12.0))
                                                .h(px(12.0))
                                                .rounded_sm()
                                                .bg(color.to_rgba()),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0x333333))
                                                .child(name),
                                        )
                                })),
                        )
                        // Statistics
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .p_4()
                                .bg(rgb(0xffffff))
                                .border_1()
                                .border_color(rgb(0xe0e0e0))
                                .rounded_lg()
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
                                        .child(format!("Total size: {} bytes", total_value)),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(0x333333))
                                        .child(format!("Leaf nodes: {}", rects.len())),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(0x333333))
                                        .child(format!("Categories: {}", categories.len())),
                                ),
                        )
                        // Algorithm info
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .p_4()
                                .bg(rgb(0x1e1e1e))
                                .border_1()
                                .border_color(rgb(0x333333))
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(0x888888))
                                        .child("TILING ALGORITHMS"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0xd4d4d4))
                                        .child(match tiling_method {
                                            TilingMethod::Squarify => {
                                                "Squarify: Creates rectangles with aspect ratios close to 1 (square-like). Best for comparing sizes."
                                            }
                                            TilingMethod::Binary => {
                                                "Binary: Recursively partitions into two halves. Good for balanced trees."
                                            }
                                            TilingMethod::Slice => {
                                                "Slice: Horizontal strips. Simple but can create very thin rectangles."
                                            }
                                            TilingMethod::Dice => {
                                                "Dice: Vertical strips. Similar to slice but oriented differently."
                                            }
                                            TilingMethod::SliceDice => {
                                                "Slice-Dice: Alternates between slice and dice at each level. Shows hierarchy clearly."
                                            }
                                        }),
                                ),
                        ),
                ),
        )
}
