//! TreeView Debug Example
//!
//! Demonstrates the TreeView component:
//! - Nested nodes
//! - Leaf nodes
//! - Selected and expanded states

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct TreeViewDebug;

impl Render for TreeViewDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let nodes = vec![
            TreeNode::new("library", "Library").children(vec![
                TreeNode::new("albums", "Albums").children(vec![
                    TreeNode::new("beethoven", "Beethoven - Complete Sonatas").leaf(true),
                    TreeNode::new("bach", "Bach - Well-Tempered Clavier").leaf(true),
                    TreeNode::new("mozart", "Mozart - Piano Concertos").leaf(true),
                ]),
                TreeNode::new("playlists", "Playlists").children(vec![
                    TreeNode::new("favorites", "Favorites").leaf(true),
                    TreeNode::new("recent", "Recently Played").leaf(true),
                ]),
            ]),
            TreeNode::new("plugins", "Plugins").children(vec![
                TreeNode::new("eq", "Parametric EQ").leaf(true),
                TreeNode::new("comp", "Compressor").leaf(true),
                TreeNode::new("upmixer", "Upmixer").leaf(true),
                TreeNode::new("limiter", "Limiter").leaf(true),
            ]),
            TreeNode::new("settings", "Settings").leaf(true),
        ];

        let mut expanded = std::collections::HashSet::new();
        expanded.insert(SharedString::from("library"));
        expanded.insert(SharedString::from("albums"));

        div()
            .id("tree-view-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("TreeView Debug"))
            .child(
                div()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .p_4()
                    .child(
                        TreeView::new("tree-demo", nodes)
                            .expanded(expanded)
                            .selected("beethoven")
                            .show_guides(true),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("TreeView Debug")
            .size(500.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| TreeViewDebug),
    );
}
