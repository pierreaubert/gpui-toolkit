//! Integration tests for LoadingOverlay component

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div};
use gpui_ui_kit::loading_overlay::LoadingOverlay;
use gpui_ui_kit::spinner::SpinnerSize;

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct LoadingOverlayTestView;

impl Render for LoadingOverlayTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .child(LoadingOverlay::new("test-overlay"))
    }
}

#[gpui::test]
async fn test_loading_overlay_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| LoadingOverlayTestView);
}

// ============================================================================
// Message Tests
// ============================================================================

#[gpui::test]
async fn test_loading_overlay_with_message(cx: &mut TestAppContext) {
    struct MessageView;

    impl Render for MessageView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .relative()
                .child(LoadingOverlay::new("msg-overlay").message("Loading library..."))
        }
    }

    let _window = cx.add_window(|_window, _cx| MessageView);
}

#[gpui::test]
async fn test_loading_overlay_with_subtitle(cx: &mut TestAppContext) {
    struct SubtitleView;

    impl Render for SubtitleView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().relative().child(
                LoadingOverlay::new("sub-overlay")
                    .message("Loading")
                    .subtitle("This may take a moment"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SubtitleView);
}

// ============================================================================
// Spinner Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_loading_overlay_spinner_sizes(cx: &mut TestAppContext) {
    struct SpinnerView;

    impl Render for SpinnerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .relative()
                .child(LoadingOverlay::new("spinner-overlay").spinner_size(SpinnerSize::Sm))
        }
    }

    let _window = cx.add_window(|_window, _cx| SpinnerView);
}

#[gpui::test]
async fn test_loading_overlay_spinner_color(cx: &mut TestAppContext) {
    struct ColorView;

    impl Render for ColorView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .relative()
                .child(LoadingOverlay::new("color-overlay").spinner_color(gpui::rgba(0x22c55eff)))
        }
    }

    let _window = cx.add_window(|_window, _cx| ColorView);
}

// ============================================================================
// Dismissible Tests
// ============================================================================

#[gpui::test]
async fn test_loading_overlay_dismissible(cx: &mut TestAppContext) {
    struct DismissView;

    impl Render for DismissView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().relative().child(
                LoadingOverlay::new("dismiss-overlay")
                    .dismissible(true)
                    .message("Click to dismiss"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| DismissView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_loading_overlay_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().relative().child(
                LoadingOverlay::new("full-overlay")
                    .message("Loading library...")
                    .subtitle("Scanning audio files")
                    .spinner_size(SpinnerSize::Lg)
                    .spinner_color(gpui::rgba(0x007accff))
                    .dismissible(true),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
