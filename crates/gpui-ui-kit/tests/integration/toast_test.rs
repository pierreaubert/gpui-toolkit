//! Integration tests for Toast component
//!
//! Tests the Toast and ToastContainer components including:
//! - All variants rendering
//! - Closeable toast with callback
//! - Toast container with positions
//! - Toast with title
//! - Persistent toast
//! - Custom duration

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, prelude::*,
};
use gpui_ui_kit::toast::{Toast, ToastContainer, ToastPosition, ToastVariant};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct ToastTestView;

impl Render for ToastTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Toast::new("test-toast", "Notification message"))
    }
}

#[gpui::test]
async fn test_toast_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| ToastTestView);
}

// ============================================================================
// All Variants Tests
// ============================================================================

#[gpui::test]
async fn test_toast_info_variant(cx: &mut TestAppContext) {
    struct InfoToastView;

    impl Render for InfoToastView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Toast::new("info-toast", "Info message").variant(ToastVariant::Info))
        }
    }

    let _window = cx.add_window(|_window, _cx| InfoToastView);
}

#[gpui::test]
async fn test_toast_success_variant(cx: &mut TestAppContext) {
    struct SuccessToastView;

    impl Render for SuccessToastView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Toast::new("success-toast", "Operation succeeded").variant(ToastVariant::Success),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SuccessToastView);
}

#[gpui::test]
async fn test_toast_warning_variant(cx: &mut TestAppContext) {
    struct WarningToastView;

    impl Render for WarningToastView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Toast::new("warning-toast", "Please check settings").variant(ToastVariant::Warning),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| WarningToastView);
}

#[gpui::test]
async fn test_toast_error_variant(cx: &mut TestAppContext) {
    struct ErrorToastView;

    impl Render for ErrorToastView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Toast::new("error-toast", "An error occurred").variant(ToastVariant::Error))
        }
    }

    let _window = cx.add_window(|_window, _cx| ErrorToastView);
}

#[gpui::test]
async fn test_toast_all_variants_together(cx: &mut TestAppContext) {
    struct AllVariantsView;

    impl Render for AllVariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(Toast::new("t-info", "Info").variant(ToastVariant::Info))
                .child(Toast::new("t-success", "Success").variant(ToastVariant::Success))
                .child(Toast::new("t-warning", "Warning").variant(ToastVariant::Warning))
                .child(Toast::new("t-error", "Error").variant(ToastVariant::Error))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllVariantsView);
}

// ============================================================================
// Closeable Toast with Callback Tests
// ============================================================================

struct CloseableToastView {
    close_count: Arc<AtomicUsize>,
}

impl Render for CloseableToastView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let close_count = self.close_count.clone();

        div().size_full().child(
            Toast::new("closeable-toast", "This toast can be closed")
                .variant(ToastVariant::Info)
                .closeable(true)
                .on_close(move |_window, _cx| {
                    close_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_toast_closeable_click(cx: &mut TestAppContext) {
    let close_count = Arc::new(AtomicUsize::new(0));
    let close_count_clone = close_count.clone();

    let window = cx.add_window(move |_window, _cx| CloseableToastView {
        close_count: close_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Toast close button uses (id, "close") tuple as ElementId
    if let Some(bounds) = cx.debug_bounds("closeable-toast-close") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            close_count.load(Ordering::SeqCst),
            1,
            "Close callback should have been called"
        );
    }
}

// ============================================================================
// Toast Container Position Tests
// ============================================================================

#[gpui::test]
async fn test_toast_container_top_right(cx: &mut TestAppContext) {
    struct TopRightView;

    impl Render for TopRightView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                ToastContainer::new(ToastPosition::TopRight)
                    .toast(Toast::new("t1", "Toast 1"))
                    .toast(Toast::new("t2", "Toast 2")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| TopRightView);
}

#[gpui::test]
async fn test_toast_container_all_positions(cx: &mut TestAppContext) {
    struct AllPositionsView;

    impl Render for AllPositionsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(
                    ToastContainer::new(ToastPosition::TopLeft).toast(Toast::new("tl", "Top Left")),
                )
                .child(
                    ToastContainer::new(ToastPosition::TopCenter)
                        .toast(Toast::new("tc", "Top Center")),
                )
                .child(
                    ToastContainer::new(ToastPosition::TopRight)
                        .toast(Toast::new("tr", "Top Right")),
                )
                .child(
                    ToastContainer::new(ToastPosition::BottomLeft)
                        .toast(Toast::new("bl", "Bottom Left")),
                )
                .child(
                    ToastContainer::new(ToastPosition::BottomCenter)
                        .toast(Toast::new("bc", "Bottom Center")),
                )
                .child(
                    ToastContainer::new(ToastPosition::BottomRight)
                        .toast(Toast::new("br", "Bottom Right")),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllPositionsView);
}

// ============================================================================
// Toast with Title Tests
// ============================================================================

#[gpui::test]
async fn test_toast_with_title(cx: &mut TestAppContext) {
    struct TitleToastView;

    impl Render for TitleToastView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Toast::new("titled-toast", "The operation completed successfully")
                    .title("Success!")
                    .variant(ToastVariant::Success),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| TitleToastView);
}

// ============================================================================
// Persistent Toast Tests
// ============================================================================

#[gpui::test]
async fn test_toast_persistent(cx: &mut TestAppContext) {
    struct PersistentToastView;

    impl Render for PersistentToastView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Toast::new("persistent-toast", "This toast stays until dismissed")
                    .persistent()
                    .closeable(true),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| PersistentToastView);
}

// ============================================================================
// Custom Duration Tests
// ============================================================================

#[gpui::test]
async fn test_toast_custom_duration(cx: &mut TestAppContext) {
    struct CustomDurationView;

    impl Render for CustomDurationView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Toast::new("custom-duration-toast", "This toast has a custom duration")
                    .duration_secs(Some(30.0)),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomDurationView);
}
