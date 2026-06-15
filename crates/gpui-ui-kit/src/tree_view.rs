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
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    AnyElement, App, Div, ElementId, Pixels, Rgba, SharedString, Stateful, StyleRefinement, Window,
    div, px,
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
    indent_size: Pixels,
    show_guides: bool,
    on_select: Option<Box<dyn Fn(SharedString, &mut Window, &mut App) + 'static>>,
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
            indent_size: px(16.0),
            show_guides: true,
            on_select: None,
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

    fn render_nodes<F>(
        nodes: &[TreeNode],
        depth: usize,
        expanded: &HashSet<SharedString>,
        selected: &Option<SharedString>,
        indent_size: Pixels,
        _show_guides: bool,
        theme: &TreeViewTheme,
        apply_hover: F,
        elements: &mut Vec<AnyElement>,
    ) where
        F: Fn(StyleRefinement) -> StyleRefinement + Copy,
    {
        for node in nodes {
            let is_expanded = expanded.contains(&node.id);
            let is_selected = selected.as_ref() == Some(&node.id);
            let has_children = !node.children.is_empty() && !node.leaf;

            // Build node row
            let mut row = div()
                .w_full()
                .flex()
                .items_center()
                .gap_1()
                .pl(px(f32::from(indent_size) * depth as f32 + 8.0))
                .pr_2()
                .py(px(3.0))
                .text_sm()
                .rounded(px(4.0))
                .hover(apply_hover);

            if is_selected {
                row = row.bg(theme.selected_bg).text_color(theme.selected_text);
            } else {
                row = row.text_color(theme.text);
            }

            // Toggle arrow
            if has_children {
                let arrow = if is_expanded {
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

            // Children (if expanded)
            if has_children && is_expanded {
                Self::render_nodes(
                    &node.children,
                    depth + 1,
                    expanded,
                    selected,
                    indent_size,
                    _show_guides,
                    theme,
                    apply_hover,
                    elements,
                );
            }
        }
    }

    /// Build the tree view with theme
    pub fn build_with_theme(self, theme: &TreeViewTheme) -> Stateful<Div> {
        let hover_bg = theme.hover_bg;
        let apply_hover = move |s: StyleRefinement| s.bg(hover_bg);
        let mut elements = Vec::new();
        Self::render_nodes(
            &self.nodes,
            0,
            &self.expanded,
            &self.selected,
            self.indent_size,
            self.show_guides,
            theme,
            apply_hover,
            &mut elements,
        );

        let mut container = div().id(self.id).flex().flex_col().w_full();

        for element in elements {
            container = container.child(element);
        }

        container
    }
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
        self.build_with_theme(&theme)
    }
}

impl IntoElement for TreeView {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
