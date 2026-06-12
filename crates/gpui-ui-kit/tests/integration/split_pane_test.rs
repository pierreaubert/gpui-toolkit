//! Integration tests for SplitPane component

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px};
use gpui_ui_kit::split_pane::{SplitDirection, SplitPane};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct SplitPaneTestView;

impl Render for SplitPaneTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(SplitPane::new("test-split"))
    }
}

#[gpui::test]
async fn test_split_pane_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| SplitPaneTestView);
}

// ============================================================================
// Direction Tests
// ============================================================================

#[gpui::test]
async fn test_split_pane_horizontal(cx: &mut TestAppContext) {
    struct HorizontalView;

    impl Render for HorizontalView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                SplitPane::new("h-split")
                    .direction(SplitDirection::Horizontal)
                    .first(div().child("Left"))
                    .second(div().child("Right")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| HorizontalView);
}

#[gpui::test]
async fn test_split_pane_vertical(cx: &mut TestAppContext) {
    struct VerticalView;

    impl Render for VerticalView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                SplitPane::new("v-split")
                    .direction(SplitDirection::Vertical)
                    .first(div().child("Top"))
                    .second(div().child("Bottom")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| VerticalView);
}

// ============================================================================
// Content Tests
// ============================================================================

#[gpui::test]
async fn test_split_pane_with_content(cx: &mut TestAppContext) {
    struct ContentView;

    impl Render for ContentView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                SplitPane::new("content-split")
                    .first(
                        div()
                            .flex()
                            .flex_col()
                            .child(div().child("Item 1"))
                            .child(div().child("Item 2")),
                    )
                    .second(div().child("Main content area")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ContentView);
}

// ============================================================================
// Ratio Tests
// ============================================================================

#[gpui::test]
async fn test_split_pane_with_ratio(cx: &mut TestAppContext) {
    struct RatioView;

    impl Render for RatioView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                SplitPane::new("ratio-split")
                    .ratio(0.25)
                    .first(div().child("Narrow"))
                    .second(div().child("Wide")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| RatioView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_split_pane_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                SplitPane::new("full-split")
                    .direction(SplitDirection::Horizontal)
                    .ratio(0.3)
                    .min_first(px(150.0))
                    .min_second(px(200.0))
                    .divider_width(px(6.0))
                    .first(
                        div()
                            .flex()
                            .flex_col()
                            .child(div().child("Sidebar item 1"))
                            .child(div().child("Sidebar item 2")),
                    )
                    .second(div().child("Main content")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
