//! TreeView component
//!
//! A hierarchical collapsible list for displaying tree-structured data.
//!
//! # Usage
//!
//! ```ignore
//! TreeView::new("file-tree", vec![
//!     TreeNode::new("src", "src/")
//!         .children(vec![
//!             TreeNode::new("main", "main.rs").leaf(true),
//!             TreeNode::new("lib", "lib.rs").leaf(true),
//!         ]),
//!     TreeNode::new("tests", "tests/")
//!         .children(vec![
//!             TreeNode::new("test1", "test_main.rs").leaf(true),
//!         ]),
//! ])
//! ```

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::data_navigation::{DataNavigationAction, DataVirtualWindow, move_key};
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, Div, ElementId, FocusHandle, KeyDownEvent, Pixels, Rgba, SharedString,
    Stateful, StyleRefinement, Window, div, px,
};
use std::collections::HashSet;

/// Theme colors for tree view
#[derive(Debug, Clone, ComponentTheme)]
pub struct TreeViewTheme {
    /// Item text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text: Rgba,
    /// Selected item background
    #[theme(default = 0x2a2a4aff, from = accent)]
    pub selected_bg: Rgba,
    /// Selected item text
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub selected_text: Rgba,
    /// Hover background
    #[theme(default = 0x2a2a2aff, from = surface_hover)]
    pub hover_bg: Rgba,
    /// Branch/indent guide color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub guide_color: Rgba,
    /// Expand/collapse icon color
    #[theme(default = 0x888888ff, from = text_muted)]
    pub toggle_color: Rgba,
}

/// A node in the tree
pub struct TreeNode {
    id: SharedString,
    label: SharedString,
    icon: Option<SharedString>,
    children: Vec<TreeNode>,
    leaf: bool,
}

impl TreeNode {
    /// Create a new tree node
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            children: Vec::new(),
            leaf: false,
        }
    }

    /// Set an icon
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set children
    pub fn children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }

    /// Mark as a leaf node (no expand/collapse toggle)
    pub fn leaf(mut self, leaf: bool) -> Self {
        self.leaf = leaf;
        self
    }
}

/// A tree view component
pub struct TreeView {
    id: ElementId,
    nodes: Vec<TreeNode>,
    expanded: HashSet<SharedString>,
    selected: Option<SharedString>,
    focused: Option<SharedString>,
    focus_handle: Option<FocusHandle>,
    virtual_window: Option<DataVirtualWindow>,
    virtual_row_height: Option<f32>,
    indent_size: Pixels,
    show_guides: bool,
    on_select: Option<Box<dyn Fn(SharedString, &mut Window, &mut App) + 'static>>,
    on_focus_change: Option<Box<dyn Fn(Option<SharedString>, &mut Window, &mut App) + 'static>>,
    on_toggle: Option<Box<dyn Fn(SharedString, bool, &mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl TreeView {
    /// Create a new tree view
    pub fn new(id: impl Into<ElementId>, nodes: Vec<TreeNode>) -> Self {
        Self {
            id: id.into(),
            nodes,
            expanded: HashSet::new(),
            selected: None,
            focused: None,
            focus_handle: None,
            virtual_window: None,
            virtual_row_height: None,
            indent_size: px(16.0),
            show_guides: true,
            on_select: None,
            on_focus_change: None,
            on_toggle: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set which nodes are expanded
    pub fn expanded(mut self, expanded: HashSet<SharedString>) -> Self {
        self.expanded = expanded;
        self
    }

    /// Set the selected node
    pub fn selected(mut self, selected: impl Into<SharedString>) -> Self {
        self.selected = Some(selected.into());
        self
    }

    /// Set the currently keyboard-focused node id.
    pub fn focused(mut self, focused: impl Into<SharedString>) -> Self {
        self.focused = Some(focused.into());
        self
    }

    /// Set the focus handle used for tree keyboard navigation.
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Set a virtual visible-node window for large trees.
    pub fn virtual_window(mut self, window: DataVirtualWindow) -> Self {
        self.virtual_window = Some(window);
        self
    }

    /// Set a virtual visible-node window and fixed row height for scroll extent spacers.
    pub fn virtual_window_with_row_height(
        mut self,
        window: DataVirtualWindow,
        row_height: f32,
    ) -> Self {
        self.virtual_window = Some(window);
        self.virtual_row_height = Some(row_height);
        self
    }

    /// Compute and set a virtual visible-node window from scroll geometry.
    pub fn virtual_viewport(
        mut self,
        scroll_offset: f32,
        row_height: f32,
        viewport_height: f32,
        overscan: usize,
    ) -> Self {
        let visible_count = visible_tree_node_count(&self.nodes, &self.expanded);
        self.virtual_window = Some(DataVirtualWindow::from_viewport(
            visible_count,
            scroll_offset,
            row_height,
            viewport_height,
            overscan,
        ));
        self.virtual_row_height = Some(row_height);
        self
    }

    /// Set indent size per level
    pub fn indent_size(mut self, size: Pixels) -> Self {
        self.indent_size = size;
        self
    }

    /// Show/hide indent guide lines
    pub fn show_guides(mut self, show: bool) -> Self {
        self.show_guides = show;
        self
    }

    /// Called when a node is selected
    pub fn on_select(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }

    /// Called when the keyboard-focused node changes.
    pub fn on_focus_change(
        mut self,
        handler: impl Fn(Option<SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_focus_change = Some(Box::new(handler));
        self
    }

    /// Called when a node is expanded/collapsed
    pub fn on_toggle(
        mut self,
        handler: impl Fn(SharedString, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Box::new(handler));
        self
    }

    /// Set an explicit ARIA label
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Tree)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    #[allow(clippy::too_many_arguments)]
    fn render_visible_nodes<F>(
        nodes: &[VisibleTreeNode],
        selected: &Option<SharedString>,
        focused: &Option<SharedString>,
        indent_size: Pixels,
        _show_guides: bool,
        theme: &TreeViewTheme,
        apply_hover: F,
        elements: &mut Vec<AnyElement>,
    ) where
        F: Fn(StyleRefinement) -> StyleRefinement + Copy,
    {
        for node in nodes {
            let is_selected = selected.as_ref() == Some(&node.id);
            let is_focused = focused.as_ref() == Some(&node.id);

            // Build node row
            let mut row = div()
                .w_full()
                .flex()
                .items_center()
                .gap_1()
                .pl(px(f32::from(indent_size) * node.depth as f32 + 8.0))
                .pr_2()
                .py(px(3.0))
                .text_sm()
                .rounded(px(4.0))
                .hover(apply_hover);

            if is_selected {
                row = row.bg(theme.selected_bg).text_color(theme.selected_text);
            } else if is_focused {
                row = row.bg(theme.hover_bg).text_color(theme.text);
            } else {
                row = row.text_color(theme.text);
            }

            // Toggle arrow
            if node.has_children {
                let arrow = if node.expanded {
                    "\u{25BE}" // ▾
                } else {
                    "\u{25B8}" // ▸
                };
                row = row.child(
                    div()
                        .w(px(14.0))
                        .text_xs()
                        .text_color(theme.toggle_color)
                        .child(arrow),
                );
            } else {
                row = row.child(div().w(px(14.0)));
            }

            // Icon
            if let Some(icon) = &node.icon {
                row = row.child(div().mr_1().child(icon.clone()));
            }

            // Label
            row = row.child(node.label.clone());

            elements.push(row.into_any_element());
        }
    }

    /// Build the tree view with theme
    pub fn build_with_theme(self, theme: &TreeViewTheme, cx: &mut App) -> Stateful<Div> {
        let focus_handle = self
            .focus_handle
            .clone()
            .unwrap_or_else(|| cx.focus_handle());
        // Count the expanded rows without materializing the whole flattened
        // tree. Rendering only needs the requested window; keyboard handling
        // below retains the full list only when navigation callbacks are in
        // use.
        let visible_count = visible_tree_node_count(&self.nodes, &self.expanded);
        let virtual_window = self
            .virtual_window
            .unwrap_or_else(|| DataVirtualWindow::full(visible_count))
            .with_total(visible_count);
        let visible_nodes = visible_tree_node_window(
            &self.nodes,
            &self.expanded,
            virtual_window.start,
            virtual_window.end,
        );
        let focused = self
            .focused
            .clone()
            .or_else(|| self.selected.clone())
            .filter(|id| visible_tree_node_exists(&self.nodes, &self.expanded, id));
        let virtual_row_height = self.virtual_row_height;
        let hover_bg = theme.hover_bg;
        let apply_hover = move |s: StyleRefinement| s.bg(hover_bg);
        let mut elements = Vec::new();
        Self::render_visible_nodes(
            &visible_nodes,
            &self.selected,
            &focused,
            self.indent_size,
            self.show_guides,
            theme,
            apply_hover,
            &mut elements,
        );

        let mut container = div()
            .id(self.id)
            .flex()
            .flex_col()
            .w_full()
            .track_focus(&focus_handle)
            .focusable();

        if self.on_focus_change.is_some() || self.on_select.is_some() || self.on_toggle.is_some() {
            let focus_handle_for_key = focus_handle.clone();
            let visible_nodes_for_key = visible_tree_nodes(&self.nodes, &self.expanded);
            let visible_ids_for_key: Vec<SharedString> = visible_nodes_for_key
                .iter()
                .map(|node| node.id.clone())
                .collect();
            let focused_for_key = focused.clone();
            let on_focus_change = self.on_focus_change.map(std::rc::Rc::new);
            let on_select = self.on_select.map(std::rc::Rc::new);
            let on_toggle = self.on_toggle.map(std::rc::Rc::new);
            container = container.on_key_down(
                move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
                    if !focus_handle_for_key.is_focused(window) {
                        return;
                    }

                    let Some(action) = DataNavigationAction::from_key(event.keystroke.key.as_str())
                    else {
                        return;
                    };

                    match action {
                        DataNavigationAction::Previous
                        | DataNavigationAction::Next
                        | DataNavigationAction::First
                        | DataNavigationAction::Last => {
                            let next = move_key(
                                &visible_ids_for_key,
                                focused_for_key.as_ref(),
                                action,
                                false,
                            );
                            if next != focused_for_key {
                                cx.stop_propagation();
                                if let Some(ref handler) = on_focus_change {
                                    handler(next, window, cx);
                                }
                            }
                        }
                        DataNavigationAction::Activate => {
                            if let Some(id) = focused_for_key.clone()
                                && let Some(ref handler) = on_select
                            {
                                cx.stop_propagation();
                                handler(id, window, cx);
                            }
                        }
                        DataNavigationAction::Expand | DataNavigationAction::Collapse => {
                            if let Some(id) = focused_for_key.clone()
                                && let Some(node) =
                                    visible_nodes_for_key.iter().find(|node| node.id == id)
                                && node.has_children
                                && node.expanded != (action == DataNavigationAction::Expand)
                                && let Some(ref handler) = on_toggle
                            {
                                cx.stop_propagation();
                                handler(id, action == DataNavigationAction::Expand, window, cx);
                            }
                        }
                        _ => {}
                    }
                },
            );
        }

        if let Some(height) =
            virtual_spacer_height(virtual_row_height, virtual_window.before_count())
        {
            container = container.child(div().h(height).flex_shrink_0());
        }

        for element in elements {
            container = container.child(element);
        }

        if let Some(height) =
            virtual_spacer_height(virtual_row_height, virtual_window.after_count())
        {
            container = container.child(div().h(height).flex_shrink_0());
        }

        container
    }
}

fn virtual_spacer_height(row_height: Option<f32>, row_count: usize) -> Option<Pixels> {
    if row_count == 0 {
        return None;
    }

    let row_height = row_height?;
    if !row_height.is_finite() || row_height <= 0.0 {
        return None;
    }

    let height = row_height * row_count as f32;
    if height.is_finite() && height > 0.0 {
        Some(px(height))
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleTreeNode {
    id: SharedString,
    label: SharedString,
    icon: Option<SharedString>,
    depth: usize,
    has_children: bool,
    expanded: bool,
}

fn visible_tree_nodes(
    nodes: &[TreeNode],
    expanded: &HashSet<SharedString>,
) -> Vec<VisibleTreeNode> {
    fn collect(
        nodes: &[TreeNode],
        expanded: &HashSet<SharedString>,
        depth: usize,
        out: &mut Vec<VisibleTreeNode>,
    ) {
        for node in nodes {
            let has_children = !node.children.is_empty() && !node.leaf;
            let is_expanded = expanded.contains(&node.id);
            out.push(VisibleTreeNode {
                id: node.id.clone(),
                label: node.label.clone(),
                icon: node.icon.clone(),
                depth,
                has_children,
                expanded: is_expanded,
            });
            if has_children && is_expanded {
                collect(&node.children, expanded, depth + 1, out);
            }
        }
    }

    let mut out = Vec::new();
    collect(nodes, expanded, 0, &mut out);
    out
}

/// Count rows in the expanded tree without allocating a flattened buffer.
fn visible_tree_node_count(nodes: &[TreeNode], expanded: &HashSet<SharedString>) -> usize {
    fn count(nodes: &[TreeNode], expanded: &HashSet<SharedString>) -> usize {
        nodes
            .iter()
            .map(|node| {
                let has_children = !node.children.is_empty() && !node.leaf;
                1 + if has_children && expanded.contains(&node.id) {
                    count(&node.children, expanded)
                } else {
                    0
                }
            })
            .sum()
    }

    count(nodes, expanded)
}

/// Collect only the requested visible row range.
///
/// The traversal still walks expanded branches until it reaches `end`, but it
/// does not allocate or clone rows outside the viewport. This keeps virtual
/// rendering proportional to the visible window instead of the full expanded
/// tree.
fn visible_tree_node_window(
    nodes: &[TreeNode],
    expanded: &HashSet<SharedString>,
    start: usize,
    end: usize,
) -> Vec<VisibleTreeNode> {
    fn collect(
        nodes: &[TreeNode],
        expanded: &HashSet<SharedString>,
        depth: usize,
        start: usize,
        end: usize,
        index: &mut usize,
        out: &mut Vec<VisibleTreeNode>,
    ) {
        for node in nodes {
            if *index >= end {
                return;
            }

            let row_index = *index;
            *index += 1;
            let has_children = !node.children.is_empty() && !node.leaf;
            let is_expanded = expanded.contains(&node.id);

            if row_index >= start {
                out.push(VisibleTreeNode {
                    id: node.id.clone(),
                    label: node.label.clone(),
                    icon: node.icon.clone(),
                    depth,
                    has_children,
                    expanded: is_expanded,
                });
            }

            if has_children && is_expanded {
                collect(&node.children, expanded, depth + 1, start, end, index, out);
            }
        }
    }

    let mut out = Vec::new();
    if start < end {
        let mut index = 0;
        collect(nodes, expanded, 0, start, end, &mut index, &mut out);
    }
    out
}

fn visible_tree_node_exists(
    nodes: &[TreeNode],
    expanded: &HashSet<SharedString>,
    id: &SharedString,
) -> bool {
    fn contains(nodes: &[TreeNode], expanded: &HashSet<SharedString>, id: &SharedString) -> bool {
        for node in nodes {
            if &node.id == id {
                return true;
            }
            let has_children = !node.children.is_empty() && !node.leaf;
            if has_children && expanded.contains(&node.id) && contains(&node.children, expanded, id)
            {
                return true;
            }
        }
        false
    }

    contains(nodes, expanded, id)
}

impl RenderOnce for TreeView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Register in accessibility tree
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_default(),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Tree)),
        });

        let global_theme = cx.theme();
        let theme = TreeViewTheme::from(global_theme);
        self.build_with_theme(&theme, cx)
    }
}

impl IntoElement for TreeView {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DataVirtualWindow, TreeNode, TreeView, virtual_spacer_height, visible_tree_node_count,
        visible_tree_node_window, visible_tree_nodes,
    };
    use gpui::{SharedString, px};
    use std::collections::HashSet;

    #[test]
    fn tree_view_visible_nodes_follow_expansion_state() {
        let nodes = vec![
            TreeNode::new("src", "src").children(vec![TreeNode::new("lib", "lib.rs").leaf(true)]),
            TreeNode::new("README", "README.md").leaf(true),
        ];
        let mut expanded = HashSet::new();

        let collapsed = visible_tree_nodes(&nodes, &expanded);
        assert_eq!(collapsed.len(), 2);
        assert_eq!(collapsed[0].id, SharedString::from("src"));
        assert_eq!(collapsed[0].label, SharedString::from("src"));
        assert_eq!(collapsed[0].depth, 0);
        assert!(collapsed[0].has_children);
        assert!(!collapsed[0].expanded);

        expanded.insert(SharedString::from("src"));
        let open = visible_tree_nodes(&nodes, &expanded);
        assert_eq!(
            open.iter().map(|node| node.id.as_ref()).collect::<Vec<_>>(),
            vec!["src", "lib", "README"]
        );
        assert!(open[0].expanded);
        assert_eq!(open[1].depth, 1);
    }

    #[test]
    fn tree_view_builder_records_keyboard_navigation_state() {
        let tree = TreeView::new("files", vec![TreeNode::new("src", "src")])
            .focused("src")
            .on_focus_change(|_, _, _| {})
            .on_select(|_, _, _| {})
            .on_toggle(|_, _, _, _| {});

        assert_eq!(tree.focused.as_deref(), Some("src"));
        assert!(tree.on_focus_change.is_some());
        assert!(tree.on_select.is_some());
        assert!(tree.on_toggle.is_some());
    }

    #[test]
    fn tree_view_builder_records_virtual_window() {
        let tree = TreeView::new("files", vec![TreeNode::new("src", "src")])
            .virtual_window(DataVirtualWindow::new(10, 2, 6));

        assert_eq!(tree.virtual_window, Some(DataVirtualWindow::new(10, 2, 6)));
        assert_eq!(tree.virtual_row_height, None);
    }

    #[test]
    fn tree_view_builder_records_virtual_window_with_row_height() {
        let tree = TreeView::new("files", vec![TreeNode::new("src", "src")])
            .virtual_window_with_row_height(DataVirtualWindow::new(10, 2, 6), 20.0);

        assert_eq!(tree.virtual_window, Some(DataVirtualWindow::new(10, 2, 6)));
        assert_eq!(tree.virtual_row_height, Some(20.0));
    }

    #[test]
    fn tree_view_builder_computes_virtual_viewport_from_visible_nodes() {
        let nodes = vec![
            TreeNode::new("src", "src").children(vec![TreeNode::new("lib", "lib.rs").leaf(true)]),
            TreeNode::new("README", "README.md").leaf(true),
        ];
        let mut expanded = HashSet::new();
        expanded.insert(SharedString::from("src"));

        let tree = TreeView::new("files", nodes)
            .expanded(expanded)
            .virtual_viewport(10.0, 10.0, 10.0, 1);

        assert_eq!(tree.virtual_window, Some(DataVirtualWindow::new(3, 0, 3)));
        assert_eq!(tree.virtual_row_height, Some(10.0));
    }

    #[test]
    fn tree_view_virtual_window_does_not_materialize_rows_outside_window() {
        let nodes = vec![
            TreeNode::new("root", "root").children(vec![
                TreeNode::new("a", "a").leaf(true),
                TreeNode::new("b", "b").leaf(true),
                TreeNode::new("c", "c").leaf(true),
            ]),
            TreeNode::new("tail", "tail").leaf(true),
        ];
        let expanded = HashSet::from([SharedString::from("root")]);

        assert_eq!(visible_tree_node_count(&nodes, &expanded), 5);
        let window = visible_tree_node_window(&nodes, &expanded, 2, 4);
        assert_eq!(
            window
                .iter()
                .map(|node| node.id.as_ref())
                .collect::<Vec<_>>(),
            vec!["b", "c"]
        );
    }

    #[test]
    fn virtual_spacer_height_rejects_invalid_geometry() {
        assert_eq!(virtual_spacer_height(None, 3), None);
        assert_eq!(virtual_spacer_height(Some(0.0), 3), None);
        assert_eq!(virtual_spacer_height(Some(f32::NAN), 3), None);
        assert_eq!(virtual_spacer_height(Some(12.0), 0), None);
        assert_eq!(virtual_spacer_height(Some(12.0), 3), Some(px(36.0)));
    }
}
