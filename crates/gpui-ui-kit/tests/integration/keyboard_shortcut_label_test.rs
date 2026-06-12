//! Integration tests for KeyboardShortcutLabel component
//!
//! Tests the KeyboardShortcutLabel component including:
//! - Basic rendering
//! - Different shortcuts
//! - All sizes
//! - Custom separator

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div};
use gpui_ui_kit::keyboard_shortcut_label::{KeyboardShortcutLabel, KeyboardShortcutSize};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct KeyboardShortcutLabelTestView;

impl Render for KeyboardShortcutLabelTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(KeyboardShortcutLabel::new("⌘+K"))
    }
}

#[gpui::test]
async fn test_keyboard_shortcut_label_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| KeyboardShortcutLabelTestView);
}

// ============================================================================
// Different Shortcuts Tests
// ============================================================================

#[gpui::test]
async fn test_keyboard_shortcut_single_key(cx: &mut TestAppContext) {
    struct SingleKeyView;

    impl Render for SingleKeyView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(KeyboardShortcutLabel::new("Esc"))
        }
    }

    let _window = cx.add_window(|_window, _cx| SingleKeyView);
}

#[gpui::test]
async fn test_keyboard_shortcut_multi_key(cx: &mut TestAppContext) {
    struct MultiKeyView;

    impl Render for MultiKeyView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(KeyboardShortcutLabel::new("Ctrl+Shift+P"))
        }
    }

    let _window = cx.add_window(|_window, _cx| MultiKeyView);
}

#[gpui::test]
async fn test_keyboard_shortcut_various(cx: &mut TestAppContext) {
    struct VariousView;

    impl Render for VariousView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(KeyboardShortcutLabel::new("⌘+C"))
                .child(KeyboardShortcutLabel::new("⌘+Shift+S"))
                .child(KeyboardShortcutLabel::new("Alt+F4"))
                .child(KeyboardShortcutLabel::new("Ctrl+Alt+Delete"))
        }
    }

    let _window = cx.add_window(|_window, _cx| VariousView);
}

// ============================================================================
// Size Tests
// ============================================================================

#[gpui::test]
async fn test_keyboard_shortcut_size_sm(cx: &mut TestAppContext) {
    struct SmView;

    impl Render for SmView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(KeyboardShortcutLabel::new("⌘+K").size(KeyboardShortcutSize::Sm))
        }
    }

    let _window = cx.add_window(|_window, _cx| SmView);
}

#[gpui::test]
async fn test_keyboard_shortcut_size_md(cx: &mut TestAppContext) {
    struct MdView;

    impl Render for MdView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(KeyboardShortcutLabel::new("⌘+K").size(KeyboardShortcutSize::Md))
        }
    }

    let _window = cx.add_window(|_window, _cx| MdView);
}

#[gpui::test]
async fn test_keyboard_shortcut_size_lg(cx: &mut TestAppContext) {
    struct LgView;

    impl Render for LgView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(KeyboardShortcutLabel::new("⌘+K").size(KeyboardShortcutSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| LgView);
}

#[gpui::test]
async fn test_keyboard_shortcut_all_sizes(cx: &mut TestAppContext) {
    struct AllSizesView;

    impl Render for AllSizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(KeyboardShortcutLabel::new("⌘+K").size(KeyboardShortcutSize::Sm))
                .child(KeyboardShortcutLabel::new("⌘+K").size(KeyboardShortcutSize::Md))
                .child(KeyboardShortcutLabel::new("⌘+K").size(KeyboardShortcutSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllSizesView);
}

// ============================================================================
// Custom Separator Tests
// ============================================================================

#[gpui::test]
async fn test_keyboard_shortcut_custom_separator(cx: &mut TestAppContext) {
    struct CustomSepView;

    impl Render for CustomSepView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(KeyboardShortcutLabel::new("Ctrl-Shift-P").separator("-"))
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomSepView);
}

#[gpui::test]
async fn test_keyboard_shortcut_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                KeyboardShortcutLabel::new("Ctrl-Shift-P")
                    .size(KeyboardShortcutSize::Lg)
                    .separator("-"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
