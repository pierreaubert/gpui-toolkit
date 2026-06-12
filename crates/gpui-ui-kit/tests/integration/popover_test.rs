//! Integration tests for Popover component
//!
//! Tests the Popover component including:
//! - Basic rendering
//! - All placements
//! - With content
//! - With width
//! - With on_close handler

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px};
use gpui_ui_kit::popover::{Popover, PopoverPlacement};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct PopoverTestView;

impl Render for PopoverTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(Popover::new("test-popover"))
    }
}

#[gpui::test]
async fn test_popover_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| PopoverTestView);
}

// ============================================================================
// Placement Tests
// ============================================================================

#[gpui::test]
async fn test_popover_all_placements(cx: &mut TestAppContext) {
    struct AllPlacementsView;

    impl Render for AllPlacementsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(
                    Popover::new("top")
                        .placement(PopoverPlacement::Top)
                        .content("Top"),
                )
                .child(
                    Popover::new("bottom")
                        .placement(PopoverPlacement::Bottom)
                        .content("Bottom"),
                )
                .child(
                    Popover::new("left")
                        .placement(PopoverPlacement::Left)
                        .content("Left"),
                )
                .child(
                    Popover::new("right")
                        .placement(PopoverPlacement::Right)
                        .content("Right"),
                )
                .child(
                    Popover::new("top-start")
                        .placement(PopoverPlacement::TopStart)
                        .content("TopStart"),
                )
                .child(
                    Popover::new("top-end")
                        .placement(PopoverPlacement::TopEnd)
                        .content("TopEnd"),
                )
                .child(
                    Popover::new("bottom-start")
                        .placement(PopoverPlacement::BottomStart)
                        .content("BottomStart"),
                )
                .child(
                    Popover::new("bottom-end")
                        .placement(PopoverPlacement::BottomEnd)
                        .content("BottomEnd"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllPlacementsView);
}

// ============================================================================
// Content Tests
// ============================================================================

#[gpui::test]
async fn test_popover_with_content(cx: &mut TestAppContext) {
    struct ContentView;

    impl Render for ContentView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Popover::new("content-popover").content(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().child("Item 1"))
                        .child(div().child("Item 2"))
                        .child(div().child("Item 3")),
                ),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ContentView);
}

// ============================================================================
// Width Tests
// ============================================================================

#[gpui::test]
async fn test_popover_with_width(cx: &mut TestAppContext) {
    struct WidthView;

    impl Render for WidthView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Popover::new("width-popover")
                    .width(px(300.0))
                    .content("Fixed width popover"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| WidthView);
}

// ============================================================================
// Handler Tests
// ============================================================================

#[gpui::test]
async fn test_popover_with_on_close(cx: &mut TestAppContext) {
    struct OnCloseView;

    impl Render for OnCloseView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Popover::new("close-popover")
                    .content("Dismissable popover")
                    .on_close(|_window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| OnCloseView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_popover_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Popover::new("full-popover")
                    .placement(PopoverPlacement::BottomStart)
                    .width(px(250.0))
                    .show_backdrop(true)
                    .content(div().p_3().child("Fully configured popover"))
                    .on_close(|_window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}

#[gpui::test]
async fn test_popover_no_backdrop(cx: &mut TestAppContext) {
    struct NoBackdropView;

    impl Render for NoBackdropView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Popover::new("no-backdrop-popover")
                    .show_backdrop(false)
                    .content("No backdrop"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| NoBackdropView);
}
