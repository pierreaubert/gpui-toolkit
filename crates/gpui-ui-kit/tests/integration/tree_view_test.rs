//! Integration tests for TreeView component

use gpui::{Context, IntoElement, ParentElement, Render, TestAppContext, Window, div};
use gpui_ui_kit::tree_view::{TreeNode, TreeView};
use std::collections::HashSet;

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct TreeViewTestView;

impl Render for TreeViewTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(TreeView::new("tree-1", vec![TreeNode::new("root", "Root")]))
    }
}

#[gpui::test]
async fn test_tree_view_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| TreeViewTestView);
}

// ============================================================================
// Nested Node Tests
// ============================================================================

#[gpui::test]
async fn test_tree_view_nested(cx: &mut TestAppContext) {
    struct NestedView;

    impl Render for NestedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut expanded = HashSet::new();
            expanded.insert("src".into());

            div().child(
                TreeView::new(
                    "tree-nested",
                    vec![TreeNode::new("src", "src/").icon("📁").children(vec![
                        TreeNode::new("main", "main.rs").icon("📄").leaf(true),
                        TreeNode::new("lib", "lib.rs").icon("📄").leaf(true),
                    ])],
                )
                .expanded(expanded),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| NestedView);
}

// ============================================================================
// Selection Tests
// ============================================================================

#[gpui::test]
async fn test_tree_view_selected(cx: &mut TestAppContext) {
    struct SelectedView;

    impl Render for SelectedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut expanded = HashSet::new();
            expanded.insert("src".into());

            div().child(
                TreeView::new(
                    "tree-sel",
                    vec![
                        TreeNode::new("src", "src/")
                            .children(vec![TreeNode::new("main", "main.rs").leaf(true)]),
                    ],
                )
                .expanded(expanded)
                .selected("main"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SelectedView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_tree_view_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let mut expanded = HashSet::new();
            expanded.insert("src".into());

            div().child(
                TreeView::new(
                    "tree-full",
                    vec![
                        TreeNode::new("src", "src/").icon("📁").children(vec![
                            TreeNode::new("main", "main.rs").icon("📄").leaf(true),
                            TreeNode::new("lib", "lib.rs").icon("📄").leaf(true),
                        ]),
                        TreeNode::new("readme", "README.md").icon("📝").leaf(true),
                    ],
                )
                .expanded(expanded)
                .selected("main")
                .indent_size(gpui::px(20.0))
                .show_guides(true)
                .on_select(|_id, _window, _cx| {})
                .on_toggle(|_id, _expanded, _window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
