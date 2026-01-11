//! Treemap - Plotly Express style API for hierarchical data visualization.
//!
//! Treemaps display hierarchical data as nested rectangles. Each rectangle's area
//! is proportional to the value it represents. Multiple tiling algorithms are supported
//! for different visual layouts.
//!
//! # Example
//! ```ignore
//! use gpui_px::{treemap, TilingMethod};
//!
//! let root = TreemapNode::new("root", 100.0)
//!     .add_child(TreemapNode::new("A", 30.0))
//!     .add_child(TreemapNode::new("B", 70.0));
//!
//! let chart = treemap(&root)
//!     .title("Sales by Region")
//!     .tiling_method(TilingMethod::Squarify)
//!     .padding(2.0)
//!     .build()
//!     .unwrap();
//! ```

use crate::error::ChartError;
use crate::{DEFAULT_HEIGHT, DEFAULT_WIDTH, TITLE_AREA_HEIGHT, validate_dimensions};
use d3rs::color::ColorScheme;
use d3rs::text::{VectorFontConfig, render_vector_text};
use gpui::prelude::*;
use gpui::{IntoElement, MouseButton, Rgba, div, hsla, px, rgb};
use std::rc::Rc;

/// Tiling algorithm for treemap layout.
///
/// Different algorithms create different visual patterns:
/// - **Squarify**: Optimizes for square-like rectangles (best for size comparison)
/// - **Binary**: Recursive binary subdivision (balanced tree structure)
/// - **Slice**: Horizontal strips (simple, can create thin rectangles)
/// - **Dice**: Vertical strips (simple, can create thin rectangles)
/// - **SliceDice**: Alternates slice/dice by depth (clear hierarchy)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TilingMethod {
    /// Create rectangles with aspect ratios close to 1 (most readable)
    #[default]
    Squarify,
    /// Recursively subdivide into two balanced halves
    Binary,
    /// Horizontal strips (can create thin rectangles)
    Slice,
    /// Vertical strips (can create thin rectangles)
    Dice,
    /// Alternates between slice and dice at each depth level
    SliceDice,
}

/// A node in the treemap hierarchy.
///
/// Nodes can be leaf nodes (with a value) or internal nodes (with children).
/// The total value of a node is the sum of its value plus all descendant values.
#[derive(Debug, Clone)]
pub struct TreemapNode {
    /// Display name for this node
    pub name: String,
    /// Direct value (for leaf nodes)
    pub value: f64,
    /// Child nodes
    pub children: Vec<TreemapNode>,
}

impl TreemapNode {
    /// Create a new leaf node with a value.
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
            children: Vec::new(),
        }
    }

    /// Create a new internal node with children.
    pub fn with_children(name: impl Into<String>, children: Vec<TreemapNode>) -> Self {
        Self {
            name: name.into(),
            value: 0.0,
            children,
        }
    }

    /// Add a child node (builder pattern).
    pub fn add_child(mut self, child: TreemapNode) -> Self {
        self.children.push(child);
        self
    }

    /// Get the total value including all descendants.
    pub fn total_value(&self) -> f64 {
        if self.children.is_empty() {
            self.value
        } else {
            self.value + self.children.iter().map(|c| c.total_value()).sum::<f64>()
        }
    }

    /// Check if this is a leaf node (has no children).
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

/// A rectangle in the computed treemap layout.
#[derive(Debug, Clone)]
struct TreemapRect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    name: String,
    value: f64,
    _depth: usize,
    category_index: usize,
}

impl TreemapRect {
    fn width(&self) -> f64 {
        self.x1 - self.x0
    }

    fn height(&self) -> f64 {
        self.y1 - self.y0
    }
}

/// Treemap chart builder.
pub struct Treemap {
    root: TreemapNode,
    title: Option<String>,
    tiling_method: TilingMethod,
    padding: f64,
    width: f32,
    height: f32,
    color_scheme: Option<ColorScheme>,
    on_click: Option<Rc<dyn Fn(&str, f64) + 'static>>,
    hover_enabled: bool,
}

impl Treemap {
    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the tiling algorithm.
    ///
    /// Default: `TilingMethod::Squarify`
    pub fn tiling_method(mut self, method: TilingMethod) -> Self {
        self.tiling_method = method;
        self
    }

    /// Set the padding between rectangles in pixels.
    ///
    /// Default: 1.0
    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self
    }

    /// Set the chart size in pixels.
    ///
    /// Default: 600 x 400
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set a custom color scheme.
    ///
    /// Default: ColorScheme::tableau10()
    pub fn color_scheme(mut self, scheme: ColorScheme) -> Self {
        self.color_scheme = Some(scheme);
        self
    }

    /// Set a click handler for treemap nodes.
    ///
    /// The handler receives the node name and value when clicked.
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&str, f64) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Enable hover highlighting (default: true).
    pub fn hover(mut self, enabled: bool) -> Self {
        self.hover_enabled = enabled;
        self
    }

    /// Build the treemap chart.
    pub fn build(self) -> Result<impl IntoElement, ChartError> {
        // Validate
        validate_dimensions(self.width, self.height)?;

        let total_value = self.root.total_value();
        if total_value <= 0.0 {
            return Err(ChartError::InvalidData {
                field: "root",
                reason: "Total value must be positive",
            });
        }

        // Calculate layout
        let title_height = if self.title.is_some() {
            TITLE_AREA_HEIGHT
        } else {
            0.0
        };

        let margin = 10.0;
        let plot_width = (self.width as f64 - 2.0 * margin).max(0.0);
        let plot_height = (self.height as f64 - title_height as f64 - 2.0 * margin).max(0.0);

        // Compute treemap layout
        let mut rects = Vec::new();
        compute_treemap(
            &self.root,
            0.0,
            0.0,
            plot_width,
            plot_height,
            self.tiling_method,
            self.padding,
            0,
            0,
            &mut rects,
        );

        // Render rectangles
        let color_scheme = self
            .color_scheme
            .unwrap_or_else(|| ColorScheme::tableau10());
        let mut plot_content = div()
            .w(px(plot_width as f32))
            .h(px(plot_height as f32))
            .relative()
            .bg(rgb(0xffffff));

        let on_click = self.on_click;
        let hover_enabled = self.hover_enabled;

        for rect in &rects {
            let color = color_scheme.color(rect.category_index);
            let rgba = Rgba {
                r: (color.r as f32) / 255.0,
                g: (color.g as f32) / 255.0,
                b: (color.b as f32) / 255.0,
                a: 0.8,
            };

            let border_color = Rgba {
                r: rgba.r * 0.7,
                g: rgba.g * 0.7,
                b: rgba.b * 0.7,
                a: 1.0,
            };

            let rect_name = rect.name.clone();
            let rect_value = rect.value;

            // Render rectangle
            let mut rect_div = div()
                .absolute()
                .left(px(rect.x0 as f32))
                .top(px(rect.y0 as f32))
                .w(px(rect.width() as f32))
                .h(px(rect.height() as f32))
                .bg(rgba)
                .border_1()
                .border_color(border_color);

            // Add hover effect
            if hover_enabled {
                let hover_color = Rgba {
                    r: (rgba.r * 1.1).min(1.0),
                    g: (rgba.g * 1.1).min(1.0),
                    b: (rgba.b * 1.1).min(1.0),
                    a: 0.9,
                };
                rect_div = rect_div.hover(|style| style.bg(hover_color).cursor_pointer());
            }

            // Add click handler
            if let Some(handler) = on_click.as_ref() {
                let handler = Rc::clone(handler);
                let name = rect_name.clone();
                let value = rect_value;
                rect_div =
                    rect_div.on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                        handler(&name, value);
                    });
            }

            let rect_div = rect_div;

            // Add label if rectangle is large enough
            let rect_div = if rect.width() > 30.0 && rect.height() > 15.0 {
                let font_size = (rect.height() * 0.2).min(12.0).max(8.0);

                // Calculate text color based on background luminance
                // Using relative luminance formula: 0.2126*R + 0.7152*G + 0.0722*B
                let luminance = 0.2126 * rgba.r + 0.7152 * rgba.g + 0.0722 * rgba.b;
                let text_color = if luminance > 0.5 {
                    hsla(0.0, 0.0, 0.1, 1.0) // Dark text for light backgrounds
                } else {
                    hsla(0.0, 0.0, 0.95, 1.0) // White text for dark backgrounds
                };

                let font_config = VectorFontConfig::horizontal(font_size as f32, text_color);

                rect_div
                    .flex()
                    .flex_col()
                    .justify_center()
                    .items_center()
                    .child(
                        div()
                            .overflow_hidden()
                            .text_ellipsis()
                            .px_1()
                            .child(render_vector_text(&rect.name, &font_config)),
                    )
            } else {
                rect_div
            };

            plot_content = plot_content.child(rect_div);
        }

        // Build container
        let mut container = div()
            .w(px(self.width))
            .h(px(self.height))
            .flex()
            .flex_col()
            .bg(rgb(0xffffff));

        // Add title if present
        if let Some(title) = &self.title {
            let font_config = VectorFontConfig::horizontal(16.0, hsla(0.0, 0.0, 0.2, 1.0));
            container = container.child(
                div()
                    .w_full()
                    .h(px(title_height))
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(render_vector_text(title, &font_config)),
            );
        }

        // Add plot
        container = container.child(
            div()
                .flex()
                .justify_center()
                .items_center()
                .flex_1()
                .child(plot_content),
        );

        Ok(container)
    }
}

/// Create a treemap chart from hierarchical data.
///
/// # Arguments
/// * `root` - Root node of the hierarchy
///
/// # Example
/// ```ignore
/// let root = TreemapNode::new("Sales", 0.0)
///     .add_child(TreemapNode::new("East", 45.0))
///     .add_child(TreemapNode::new("West", 55.0));
///
/// let chart = treemap(&root)
///     .title("Regional Sales")
///     .build()
///     .unwrap();
/// ```
pub fn treemap(root: &TreemapNode) -> Treemap {
    Treemap {
        root: root.clone(),
        title: None,
        tiling_method: TilingMethod::default(),
        padding: 1.0,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        color_scheme: None,
        on_click: None,
        hover_enabled: true,
    }
}

// ============================================================================
// Layout Algorithms
// ============================================================================

/// Compute treemap layout recursively.
fn compute_treemap(
    node: &TreemapNode,
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
    let total_value = node.total_value();
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
            value: node.value,
            _depth: depth,
            category_index,
        });
    } else {
        // Layout children based on tiling method
        let children: Vec<_> = node.children.iter().map(|c| (c, c.total_value())).collect();

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

/// Slice tiling - horizontal strips.
fn tile_slice(
    children: &[(&TreemapNode, f64)],
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

/// Dice tiling - vertical strips.
fn tile_dice(
    children: &[(&TreemapNode, f64)],
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

/// Slice-Dice tiling - alternates between slice and dice based on depth.
fn tile_slice_dice(
    children: &[(&TreemapNode, f64)],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    total: f64,
    depth: usize,
) -> Vec<(f64, f64, f64, f64)> {
    if depth % 2 == 0 {
        tile_slice(children, x0, y0, x1, y1, total)
    } else {
        tile_dice(children, x0, y0, x1, y1, total)
    }
}

/// Binary tiling - recursively subdivides into two halves.
fn tile_binary(
    children: &[(&TreemapNode, f64)],
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

    // Find partition point that balances the two halves
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

/// Squarify tiling - creates rectangles with aspect ratios close to 1.
fn tile_squarify(
    children: &[(&TreemapNode, f64)],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    total: f64,
) -> Vec<(f64, f64, f64, f64)> {
    if children.is_empty() {
        return Vec::new();
    }

    let width = x1 - x0;
    let height = y1 - y0;

    // Sort by value descending for better packing
    let mut sorted: Vec<_> = children.iter().map(|(n, v)| (*n, *v)).collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let mut rects = Vec::new();
    let mut remaining: Vec<_> = sorted.iter().collect();
    let mut x = x0;
    let mut y = y0;
    let mut w = width;
    let mut h = height;

    while !remaining.is_empty() {
        // Try to find best row
        let mut best_row_len = 1;
        let mut best_worst_ratio = f64::INFINITY;

        for row_len in 1..=remaining.len() {
            let row = &remaining[..row_len];
            let row_sum: f64 = row.iter().map(|(_n, v)| *v).sum();

            let short_side = w.min(h);
            let long_side = w.max(h);

            // Calculate worst aspect ratio in this row
            let mut worst_ratio: f64 = 0.0;
            for (_n, value) in row {
                let rect_area =
                    (*value / row_sum) * (short_side * long_side / (w.max(h) / short_side));
                let rect_short = rect_area / long_side;
                let ratio = rect_short.max(long_side / rect_short);
                worst_ratio = worst_ratio.max(ratio);
            }

            if worst_ratio < best_worst_ratio {
                best_worst_ratio = worst_ratio;
                best_row_len = row_len;
            } else {
                break; // Aspect ratios getting worse
            }
        }

        // Layout the best row
        let row = &remaining[..best_row_len];
        let row_sum: f64 = row.iter().map(|(_n, v)| *v).sum();

        let use_width = w <= h;
        let area = w * h;
        if use_width {
            // Layout horizontally
            let row_height = (row_sum / total) * area / w;
            let mut rx = x;
            for (_n, value) in row {
                let rw = (*value / row_sum) * w;
                rects.push((rx, y, rx + rw, y + row_height));
                rx += rw;
            }
            y += row_height;
            h -= row_height;
        } else {
            // Layout vertically
            let row_width = (row_sum / total) * area / h;
            let mut ry = y;
            for (_n, value) in row {
                let rh = (*value / row_sum) * h;
                rects.push((x, ry, x + row_width, ry + rh));
                ry += rh;
            }
            x += row_width;
            w -= row_width;
        }

        remaining = remaining[best_row_len..].to_vec();
    }

    rects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_treemap_node_creation() {
        let node = TreemapNode::new("Test", 100.0);
        assert_eq!(node.name, "Test");
        assert_eq!(node.value, 100.0);
        assert_eq!(node.total_value(), 100.0);
        assert!(node.is_leaf());
    }

    #[test]
    fn test_treemap_node_with_children() {
        let child1 = TreemapNode::new("Child1", 30.0);
        let child2 = TreemapNode::new("Child2", 70.0);
        let parent = TreemapNode::with_children("Parent", vec![child1, child2]);

        assert_eq!(parent.total_value(), 100.0);
        assert!(!parent.is_leaf());
    }

    #[test]
    fn test_treemap_node_builder() {
        let root = TreemapNode::new("Root", 0.0)
            .add_child(TreemapNode::new("A", 20.0))
            .add_child(TreemapNode::new("B", 30.0))
            .add_child(TreemapNode::new("C", 50.0));

        assert_eq!(root.total_value(), 100.0);
        assert_eq!(root.children.len(), 3);
    }

    #[test]
    fn test_treemap_zero_value() {
        let root = TreemapNode::new("Empty", 0.0);
        let result = treemap(&root).build();
        assert!(matches!(result, Err(ChartError::InvalidData { .. })));
    }

    #[test]
    fn test_treemap_negative_dimensions() {
        let root = TreemapNode::new("Test", 100.0);
        let result = treemap(&root).size(-100.0, 400.0).build();
        assert!(matches!(result, Err(ChartError::InvalidDimension { .. })));
    }

    #[test]
    fn test_treemap_successful_build() {
        let root = TreemapNode::new("Root", 0.0)
            .add_child(TreemapNode::new("A", 30.0))
            .add_child(TreemapNode::new("B", 70.0));

        let result = treemap(&root)
            .title("Test Treemap")
            .tiling_method(TilingMethod::Squarify)
            .padding(2.0)
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_treemap_all_tiling_methods() {
        let root = TreemapNode::new("Root", 0.0)
            .add_child(TreemapNode::new("A", 25.0))
            .add_child(TreemapNode::new("B", 35.0))
            .add_child(TreemapNode::new("C", 40.0));

        for method in [
            TilingMethod::Squarify,
            TilingMethod::Binary,
            TilingMethod::Slice,
            TilingMethod::Dice,
            TilingMethod::SliceDice,
        ] {
            let result = treemap(&root).tiling_method(method).build();
            assert!(result.is_ok(), "Failed for method {:?}", method);
        }
    }

    #[test]
    fn test_treemap_nested_hierarchy() {
        let level2_a = TreemapNode::new("L2-A", 0.0)
            .add_child(TreemapNode::new("L3-A1", 10.0))
            .add_child(TreemapNode::new("L3-A2", 15.0));

        let level2_b = TreemapNode::new("L2-B", 0.0)
            .add_child(TreemapNode::new("L3-B1", 25.0))
            .add_child(TreemapNode::new("L3-B2", 50.0));

        let root = TreemapNode::new("Root", 0.0)
            .add_child(level2_a)
            .add_child(level2_b);

        assert_eq!(root.total_value(), 100.0);

        let result = treemap(&root).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_tile_slice() {
        let node_a = TreemapNode::new("A", 30.0);
        let node_b = TreemapNode::new("B", 70.0);
        let nodes = vec![(&node_a, 30.0), (&node_b, 70.0)];

        let rects = tile_slice(&nodes, 0.0, 0.0, 100.0, 100.0, 100.0);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0], (0.0, 0.0, 100.0, 30.0));
        assert_eq!(rects[1], (0.0, 30.0, 100.0, 100.0));
    }

    #[test]
    fn test_tile_dice() {
        let node_a = TreemapNode::new("A", 40.0);
        let node_b = TreemapNode::new("B", 60.0);
        let nodes = vec![(&node_a, 40.0), (&node_b, 60.0)];

        let rects = tile_dice(&nodes, 0.0, 0.0, 100.0, 100.0, 100.0);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0], (0.0, 0.0, 40.0, 100.0));
        assert_eq!(rects[1], (40.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn test_builder_chaining() {
        let root = TreemapNode::new("Root", 100.0);
        let result = treemap(&root)
            .title("Chained")
            .tiling_method(TilingMethod::Binary)
            .padding(3.0)
            .size(800.0, 600.0)
            .build();

        assert!(result.is_ok());
    }
}
