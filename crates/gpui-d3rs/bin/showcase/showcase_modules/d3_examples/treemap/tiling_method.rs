use super::super::flare_data::{HierarchyNode, flare_hierarchy, top_level_categories};
use super::misc::category_color;
use super::tile::tile_binary;
use super::tile::tile_dice;
use super::tile::tile_slice;
use super::tile::tile_slice_dice;
use super::tile::tile_squarify;
use super::treemap_rect::TreemapRect;
use crate::ShowcaseApp;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

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

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let entity = cx.entity().clone();
    let theme = cx.theme();

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
                                        .bg(theme.accent)
                                        .rounded_md()
                                        .text_xs()
                                        .child(format!("{} rects", rects.len())),
                                ),
                        )
                        .child(
                            div()
                                .w(px(plot_size))
                                .h(px(plot_size))
                                .bg(theme.surface)
                                .border_1()
                                .border_color(theme.border)
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
                                .bg(theme.surface)
                                .border_1()
                                .border_color(theme.border)
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_weight(FontWeight::SEMIBOLD)
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
                                                            theme.accent
                                                        } else {
                                                            theme.surface_hover
                                                        };
                                                        let text_color = if is_selected {
                                                            theme.text_on_accent
                                                        } else {
                                                            theme.text_primary
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
                                                            theme.accent
                                                        } else {
                                                            theme.surface_hover
                                                        };
                                                        let text_color = if is_selected {
                                                            theme.text_on_accent
                                                        } else {
                                                            theme.text_primary
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
                                .bg(theme.surface)
                                .border_1()
                                .border_color(theme.border)
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
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
                                .bg(theme.surface)
                                .border_1()
                                .border_color(theme.border)
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child("STATISTICS"),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .child(format!("Total size: {} bytes", total_value)),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .child(format!("Leaf nodes: {}", rects.len())),
                                )
                                .child(
                                    div()
                                        .text_sm()
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
                                        .child("TILING ALGORITHMS"),
                                )
                                .child(
                                    div()
                                        .text_xs()
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
