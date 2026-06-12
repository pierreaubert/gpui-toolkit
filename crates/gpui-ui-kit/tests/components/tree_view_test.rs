//! TreeView component tests

use gpui_ui_kit::tree_view::{TreeNode, TreeView};
use std::collections::HashSet;

#[test]
fn test_tree_view_creation() {
    let tree = TreeView::new("tree-1", vec![TreeNode::new("root", "Root")]);
    drop(tree);
}

#[test]
fn test_tree_node_with_icon() {
    let node = TreeNode::new("src", "src/").icon("📁");
    let tree = TreeView::new("tree-icon", vec![node]);
    drop(tree);
}

#[test]
fn test_tree_node_with_children() {
    let tree = TreeView::new(
        "tree-children",
        vec![TreeNode::new("src", "src/").children(vec![
            TreeNode::new("main", "main.rs").leaf(true),
            TreeNode::new("lib", "lib.rs").leaf(true),
        ])],
    );
    drop(tree);
}

#[test]
fn test_tree_node_leaf() {
    let node = TreeNode::new("file", "readme.md").leaf(true);
    let tree = TreeView::new("tree-leaf", vec![node]);
    drop(tree);
}

#[test]
fn test_tree_view_expanded_set() {
    let mut expanded = HashSet::new();
    expanded.insert("src".into());
    expanded.insert("tests".into());

    let tree = TreeView::new(
        "tree-expanded",
        vec![
            TreeNode::new("src", "src/")
                .children(vec![TreeNode::new("main", "main.rs").leaf(true)]),
            TreeNode::new("tests", "tests/")
                .children(vec![TreeNode::new("test1", "test_main.rs").leaf(true)]),
        ],
    )
    .expanded(expanded);
    drop(tree);
}

#[test]
fn test_tree_view_selected() {
    let tree = TreeView::new("tree-sel", vec![TreeNode::new("item", "Item")]).selected("item");
    drop(tree);
}

#[test]
fn test_tree_view_indent_size() {
    let tree =
        TreeView::new("tree-indent", vec![TreeNode::new("a", "A")]).indent_size(gpui::px(24.0));
    drop(tree);
}

#[test]
fn test_tree_view_show_guides() {
    let tree = TreeView::new("tree-guides", vec![TreeNode::new("a", "A")]).show_guides(false);
    drop(tree);
}

#[test]
fn test_tree_view_on_select() {
    let tree = TreeView::new("tree-on-sel", vec![TreeNode::new("a", "A")])
        .on_select(|_id, _window, _cx| {});
    drop(tree);
}

#[test]
fn test_tree_view_on_toggle() {
    let tree = TreeView::new("tree-on-tog", vec![TreeNode::new("a", "A")])
        .on_toggle(|_id, _expanded, _window, _cx| {});
    drop(tree);
}

#[test]
fn test_tree_view_full_configuration() {
    let mut expanded = HashSet::new();
    expanded.insert("src".into());

    let tree = TreeView::new(
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
    .on_toggle(|_id, _expanded, _window, _cx| {});
    drop(tree);
}
