//! Integration tests for ContextMenu component
//!
//! Tests the ContextMenu component including:
//! - Basic rendering
//! - With position
//! - With handlers (on_select, on_close)
//! - With min width
//! - Full configuration

use gpui::{
    Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, point, px,
};
use gpui_ui_kit::context_menu::ContextMenu;
use gpui_ui_kit::menu::MenuItem;

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct ContextMenuTestView;

impl Render for ContextMenuTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let items = vec![
            MenuItem::new("cut", "Cut"),
            MenuItem::separator(),
            MenuItem::new("paste", "Paste"),
        ];
        div().size_full().child(ContextMenu::new("ctx-menu", items))
    }
}

#[gpui::test]
async fn test_context_menu_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| ContextMenuTestView);
}

// ============================================================================
// Position Tests
// ============================================================================

#[gpui::test]
async fn test_context_menu_with_position(cx: &mut TestAppContext) {
    struct PositionedView;

    impl Render for PositionedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let items = vec![
                MenuItem::new("copy", "Copy"),
                MenuItem::new("paste", "Paste"),
            ];
            div()
                .size_full()
                .child(ContextMenu::new("pos-menu", items).position(point(px(100.0), px(200.0))))
        }
    }

    let _window = cx.add_window(|_window, _cx| PositionedView);
}

// ============================================================================
// Handler Tests
// ============================================================================

#[gpui::test]
async fn test_context_menu_with_on_select(cx: &mut TestAppContext) {
    struct OnSelectView;

    impl Render for OnSelectView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let items = vec![
                MenuItem::new("action1", "Action 1"),
                MenuItem::new("action2", "Action 2"),
            ];
            div()
                .size_full()
                .child(ContextMenu::new("select-menu", items).on_select(|_id, _window, _cx| {}))
        }
    }

    let _window = cx.add_window(|_window, _cx| OnSelectView);
}

#[gpui::test]
async fn test_context_menu_with_on_close(cx: &mut TestAppContext) {
    struct OnCloseView;

    impl Render for OnCloseView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let items = vec![MenuItem::new("item", "Item")];
            div()
                .size_full()
                .child(ContextMenu::new("close-menu", items).on_close(|_window, _cx| {}))
        }
    }

    let _window = cx.add_window(|_window, _cx| OnCloseView);
}

#[gpui::test]
async fn test_context_menu_with_all_handlers(cx: &mut TestAppContext) {
    struct AllHandlersView;

    impl Render for AllHandlersView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let items = vec![
                MenuItem::new("cut", "Cut").with_shortcut("⌘X"),
                MenuItem::new("copy", "Copy").with_shortcut("⌘C"),
                MenuItem::separator(),
                MenuItem::new("paste", "Paste").with_shortcut("⌘V"),
            ];
            div().size_full().child(
                ContextMenu::new("handlers-menu", items)
                    .on_select(|_id, _window, _cx| {})
                    .on_close(|_window, _cx| {})
                    .on_focus_change(|_index, _window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllHandlersView);
}

// ============================================================================
// Min Width Tests
// ============================================================================

#[gpui::test]
async fn test_context_menu_with_min_width(cx: &mut TestAppContext) {
    struct MinWidthView;

    impl Render for MinWidthView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let items = vec![MenuItem::new("item", "Short")];
            div()
                .size_full()
                .child(ContextMenu::new("width-menu", items).min_width(px(250.0)))
        }
    }

    let _window = cx.add_window(|_window, _cx| MinWidthView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_context_menu_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let items = vec![
                MenuItem::new("new", "New File").with_shortcut("⌘N"),
                MenuItem::new("open", "Open File").with_shortcut("⌘O"),
                MenuItem::separator(),
                MenuItem::new("delete", "Delete").danger(),
            ];
            div().size_full().child(
                ContextMenu::new("full-menu", items)
                    .position(point(px(50.0), px(75.0)))
                    .min_width(px(220.0))
                    .focused_index(0)
                    .on_select(|_id, _window, _cx| {})
                    .on_close(|_window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
