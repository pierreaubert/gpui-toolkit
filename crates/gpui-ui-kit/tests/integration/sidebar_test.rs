//! Integration tests for Sidebar component
//!
//! Tests the Sidebar component including:
//! - Basic rendering
//! - Left and right sides
//! - Collapsed state
//! - With header and footer
//! - With content
//! - Full configuration

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px};
use gpui_ui_kit::sidebar::{Sidebar, SidebarSide};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct SidebarTestView;

impl Render for SidebarTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(Sidebar::new("test-sidebar"))
    }
}

#[gpui::test]
async fn test_sidebar_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| SidebarTestView);
}

// ============================================================================
// Side Tests
// ============================================================================

#[gpui::test]
async fn test_sidebar_left_side(cx: &mut TestAppContext) {
    struct LeftSideView;

    impl Render for LeftSideView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Sidebar::new("left-sidebar")
                    .side(SidebarSide::Left)
                    .content("Left sidebar content"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| LeftSideView);
}

#[gpui::test]
async fn test_sidebar_right_side(cx: &mut TestAppContext) {
    struct RightSideView;

    impl Render for RightSideView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Sidebar::new("right-sidebar")
                    .side(SidebarSide::Right)
                    .content("Right sidebar content"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| RightSideView);
}

// ============================================================================
// Collapsed Tests
// ============================================================================

#[gpui::test]
async fn test_sidebar_collapsed(cx: &mut TestAppContext) {
    struct CollapsedView;

    impl Render for CollapsedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Sidebar::new("collapsed-sidebar")
                    .collapsed(true)
                    .content("This content is hidden"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CollapsedView);
}

#[gpui::test]
async fn test_sidebar_expanded(cx: &mut TestAppContext) {
    struct ExpandedView;

    impl Render for ExpandedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Sidebar::new("expanded-sidebar")
                    .collapsed(false)
                    .content("This content is visible"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ExpandedView);
}

// ============================================================================
// Header and Footer Tests
// ============================================================================

#[gpui::test]
async fn test_sidebar_with_header(cx: &mut TestAppContext) {
    struct HeaderView;

    impl Render for HeaderView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Sidebar::new("header-sidebar")
                    .header(div().px_3().py_2().child("Navigation"))
                    .content("Main content"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| HeaderView);
}

#[gpui::test]
async fn test_sidebar_with_footer(cx: &mut TestAppContext) {
    struct FooterView;

    impl Render for FooterView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Sidebar::new("footer-sidebar")
                    .content("Main content")
                    .footer(div().px_3().py_2().child("Footer")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FooterView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_sidebar_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Sidebar::new("full-sidebar")
                    .side(SidebarSide::Left)
                    .width(px(280.0))
                    .collapsed(false)
                    .show_border(true)
                    .header(div().px_3().py_2().child("App Title"))
                    .content(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().child("Item 1"))
                            .child(div().child("Item 2"))
                            .child(div().child("Item 3")),
                    )
                    .footer(div().px_3().py_2().child("v1.0.0")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}

#[gpui::test]
async fn test_sidebar_no_border(cx: &mut TestAppContext) {
    struct NoBorderView;

    impl Render for NoBorderView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Sidebar::new("no-border-sidebar")
                    .show_border(false)
                    .content("Borderless sidebar"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| NoBorderView);
}
