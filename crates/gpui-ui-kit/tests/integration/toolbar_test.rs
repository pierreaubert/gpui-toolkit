//! Integration tests for Toolbar component

use gpui::{Context, IntoElement, ParentElement, Render, TestAppContext, Window, div};
use gpui_ui_kit::toolbar::{Toolbar, ToolbarItem};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct ToolbarTestView;

impl Render for ToolbarTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Toolbar::new("tb-1"))
    }
}

#[gpui::test]
async fn test_toolbar_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| ToolbarTestView);
}

// ============================================================================
// Item Tests
// ============================================================================

#[gpui::test]
async fn test_toolbar_with_buttons(cx: &mut TestAppContext) {
    struct ButtonsView;

    impl Render for ButtonsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Toolbar::new("tb-btns")
                    .item(ToolbarItem::button("bold", "B"))
                    .item(ToolbarItem::button("italic", "I"))
                    .separator()
                    .item(ToolbarItem::button("align-left", "<")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ButtonsView);
}

// ============================================================================
// Active / Disabled Tests
// ============================================================================

#[gpui::test]
async fn test_toolbar_active_and_disabled(cx: &mut TestAppContext) {
    struct ActiveDisabledView;

    impl Render for ActiveDisabledView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Toolbar::new("tb-states")
                    .item(ToolbarItem::button("bold", "B").active(true))
                    .item(ToolbarItem::button("redo", "Redo").disabled(true)),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ActiveDisabledView);
}

// ============================================================================
// Custom Item Tests
// ============================================================================

#[gpui::test]
async fn test_toolbar_custom_item(cx: &mut TestAppContext) {
    struct CustomItemView;

    impl Render for CustomItemView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Toolbar::new("tb-custom")
                    .item(ToolbarItem::button("bold", "B"))
                    .separator()
                    .item(ToolbarItem::custom(div().child("Zoom: 100%"))),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomItemView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_toolbar_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Toolbar::new("tb-full")
                    .bordered(true)
                    .item(ToolbarItem::button("bold", "B").active(true))
                    .item(ToolbarItem::button("italic", "I").on_click(|_window, _cx| {}))
                    .separator()
                    .item(ToolbarItem::button("undo", "Undo").disabled(true))
                    .item(ToolbarItem::custom(div().child("Zoom: 100%"))),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
