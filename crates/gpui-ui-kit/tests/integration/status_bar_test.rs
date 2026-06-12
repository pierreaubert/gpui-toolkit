//! Integration tests for StatusBar component
//!
//! Tests the StatusBar component including:
//! - Basic rendering
//! - Top and bottom positions
//! - With left, center, and right sections
//! - With custom height
//! - Full configuration

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px};
use gpui_ui_kit::status_bar::{StatusBar, StatusBarPosition};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct StatusBarTestView;

impl Render for StatusBarTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(StatusBar::new("test-status-bar"))
    }
}

#[gpui::test]
async fn test_status_bar_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| StatusBarTestView);
}

// ============================================================================
// Position Tests
// ============================================================================

#[gpui::test]
async fn test_status_bar_top_position(cx: &mut TestAppContext) {
    struct TopView;

    impl Render for TopView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                StatusBar::new("top-bar")
                    .position(StatusBarPosition::Top)
                    .center("Top Status Bar"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| TopView);
}

#[gpui::test]
async fn test_status_bar_bottom_position(cx: &mut TestAppContext) {
    struct BottomView;

    impl Render for BottomView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                StatusBar::new("bottom-bar")
                    .position(StatusBarPosition::Bottom)
                    .center("Bottom Status Bar"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| BottomView);
}

// ============================================================================
// Section Tests
// ============================================================================

#[gpui::test]
async fn test_status_bar_with_left(cx: &mut TestAppContext) {
    struct LeftView;

    impl Render for LeftView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(StatusBar::new("left-bar").left("Left content"))
        }
    }

    let _window = cx.add_window(|_window, _cx| LeftView);
}

#[gpui::test]
async fn test_status_bar_with_center(cx: &mut TestAppContext) {
    struct CenterView;

    impl Render for CenterView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(StatusBar::new("center-bar").center("Center content"))
        }
    }

    let _window = cx.add_window(|_window, _cx| CenterView);
}

#[gpui::test]
async fn test_status_bar_with_right(cx: &mut TestAppContext) {
    struct RightView;

    impl Render for RightView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(StatusBar::new("right-bar").right("Right content"))
        }
    }

    let _window = cx.add_window(|_window, _cx| RightView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_status_bar_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                StatusBar::new("full-bar")
                    .position(StatusBarPosition::Bottom)
                    .height(px(40.0))
                    .left(div().child("Ready"))
                    .center(div().child("main.rs - SOTF"))
                    .right(div().child("Ln 42, Col 15")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}

#[gpui::test]
async fn test_status_bar_custom_height(cx: &mut TestAppContext) {
    struct CustomHeightView;

    impl Render for CustomHeightView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                StatusBar::new("tall-bar")
                    .height(px(48.0))
                    .center("Tall status bar"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomHeightView);
}
