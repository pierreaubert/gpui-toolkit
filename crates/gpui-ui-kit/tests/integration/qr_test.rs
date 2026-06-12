//! Integration tests for QrCode component
//!
//! Tests the QrCode component including:
//! - Basic rendering
//! - Custom sizes
//! - Custom colors
//! - Edge cases (empty string)

use gpui::{Context, IntoElement, ParentElement, Render, TestAppContext, Window, div, px, rgba};
use gpui_ui_kit::qr::QrCode;

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct QrCodeTestView;

impl Render for QrCodeTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(QrCode::new("https://example.com"))
    }
}

#[gpui::test]
async fn test_qr_code_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| QrCodeTestView);
}

// ============================================================================
// Size Tests
// ============================================================================

#[gpui::test]
async fn test_qr_code_small(cx: &mut TestAppContext) {
    struct SmallView;
    impl Render for SmallView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(QrCode::new("test").size(px(100.0)))
        }
    }
    let _window = cx.add_window(|_window, _cx| SmallView);
}

#[gpui::test]
async fn test_qr_code_large(cx: &mut TestAppContext) {
    struct LargeView;
    impl Render for LargeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(QrCode::new("test").size(px(400.0)))
        }
    }
    let _window = cx.add_window(|_window, _cx| LargeView);
}

// ============================================================================
// Color Tests
// ============================================================================

#[gpui::test]
async fn test_qr_code_custom_colors(cx: &mut TestAppContext) {
    struct CustomColorView;
    impl Render for CustomColorView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                QrCode::new("colored")
                    .fg(rgba(0x000000ff))
                    .bg(rgba(0xffffffff)),
            )
        }
    }
    let _window = cx.add_window(|_window, _cx| CustomColorView);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[gpui::test]
async fn test_qr_code_empty_string(cx: &mut TestAppContext) {
    struct EmptyView;
    impl Render for EmptyView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(QrCode::new(""))
        }
    }
    let _window = cx.add_window(|_window, _cx| EmptyView);
}

#[gpui::test]
async fn test_qr_code_long_content(cx: &mut TestAppContext) {
    struct LongContentView;
    impl Render for LongContentView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(QrCode::new(
                "https://example.com/very/long/url/path/that/contains/a/lot/of/data",
            ))
        }
    }
    let _window = cx.add_window(|_window, _cx| LongContentView);
}

// ============================================================================
// Combined Feature Tests
// ============================================================================

#[gpui::test]
async fn test_qr_code_all_features(cx: &mut TestAppContext) {
    struct AllFeaturesView;
    impl Render for AllFeaturesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                QrCode::new("https://example.com")
                    .size(px(250.0))
                    .fg(rgba(0x1a1a1aff))
                    .bg(rgba(0xf0f0f0ff)),
            )
        }
    }
    let _window = cx.add_window(|_window, _cx| AllFeaturesView);
}
