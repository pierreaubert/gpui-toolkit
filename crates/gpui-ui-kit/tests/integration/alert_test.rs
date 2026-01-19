//! Integration tests for Alert component
//!
//! Tests the Alert and InlineAlert components including:
//! - All variants (Info, Success, Warning, Error)
//! - Closeable alerts with callback
//! - Alerts with title
//! - Custom icons
//! - InlineAlert variants

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, prelude::*,
};
use gpui_ui_kit::alert::{Alert, AlertVariant, InlineAlert};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct AlertTestView;

impl Render for AlertTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Alert::new("test-alert", "This is a test message").variant(AlertVariant::Info))
    }
}

#[gpui::test]
async fn test_alert_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| AlertTestView);
}

// ============================================================================
// Variant Tests
// ============================================================================

#[gpui::test]
async fn test_alert_info_variant(cx: &mut TestAppContext) {
    struct InfoAlertView;

    impl Render for InfoAlertView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Alert::new("info-alert", "Informational message").variant(AlertVariant::Info),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| InfoAlertView);
}

#[gpui::test]
async fn test_alert_success_variant(cx: &mut TestAppContext) {
    struct SuccessAlertView;

    impl Render for SuccessAlertView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Alert::new("success-alert", "Operation completed successfully")
                    .variant(AlertVariant::Success),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SuccessAlertView);
}

#[gpui::test]
async fn test_alert_warning_variant(cx: &mut TestAppContext) {
    struct WarningAlertView;

    impl Render for WarningAlertView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Alert::new("warning-alert", "Please review before continuing")
                    .variant(AlertVariant::Warning),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| WarningAlertView);
}

#[gpui::test]
async fn test_alert_error_variant(cx: &mut TestAppContext) {
    struct ErrorAlertView;

    impl Render for ErrorAlertView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Alert::new("error-alert", "An error occurred").variant(AlertVariant::Error))
        }
    }

    let _window = cx.add_window(|_window, _cx| ErrorAlertView);
}

#[gpui::test]
async fn test_alert_all_variants(cx: &mut TestAppContext) {
    struct AllVariantsView;

    impl Render for AllVariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(Alert::new("info", "Info").variant(AlertVariant::Info))
                .child(Alert::new("success", "Success").variant(AlertVariant::Success))
                .child(Alert::new("warning", "Warning").variant(AlertVariant::Warning))
                .child(Alert::new("error", "Error").variant(AlertVariant::Error))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllVariantsView);
}

// ============================================================================
// Title Tests
// ============================================================================

#[gpui::test]
async fn test_alert_with_title(cx: &mut TestAppContext) {
    struct TitleAlertView;

    impl Render for TitleAlertView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Alert::new("titled-alert", "This is the alert message body")
                    .title("Important Notice")
                    .variant(AlertVariant::Info),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| TitleAlertView);
}

#[gpui::test]
async fn test_alert_with_title_all_variants(cx: &mut TestAppContext) {
    struct TitledVariantsView;

    impl Render for TitledVariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    Alert::new("info-titled", "Info message")
                        .title("Information")
                        .variant(AlertVariant::Info),
                )
                .child(
                    Alert::new("success-titled", "Success message")
                        .title("Success!")
                        .variant(AlertVariant::Success),
                )
                .child(
                    Alert::new("warning-titled", "Warning message")
                        .title("Warning")
                        .variant(AlertVariant::Warning),
                )
                .child(
                    Alert::new("error-titled", "Error message")
                        .title("Error")
                        .variant(AlertVariant::Error),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| TitledVariantsView);
}

// ============================================================================
// Closeable Alert Tests
// ============================================================================

struct CloseableAlertView {
    close_count: Arc<AtomicUsize>,
}

impl Render for CloseableAlertView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let close_count = self.close_count.clone();

        div().size_full().child(
            Alert::new("closeable-alert", "This alert can be closed")
                .variant(AlertVariant::Info)
                .closeable(true)
                .on_close(move |_window, _cx| {
                    close_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_alert_closeable_renders(cx: &mut TestAppContext) {
    let close_count = Arc::new(AtomicUsize::new(0));
    let close_count_clone = close_count.clone();

    let _window = cx.add_window(move |_window, _cx| CloseableAlertView {
        close_count: close_count_clone,
    });
}

#[gpui::test]
async fn test_alert_close_callback(cx: &mut TestAppContext) {
    let close_count = Arc::new(AtomicUsize::new(0));
    let close_count_clone = close_count.clone();

    let window = cx.add_window(move |_window, _cx| CloseableAlertView {
        close_count: close_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Click on the close button
    if let Some(bounds) = cx.debug_bounds("alert-close") {
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
// Custom Icon Tests
// ============================================================================

#[gpui::test]
async fn test_alert_custom_icon(cx: &mut TestAppContext) {
    struct CustomIconView;

    impl Render for CustomIconView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Alert::new("custom-icon-alert", "Alert with custom icon")
                    .variant(AlertVariant::Info)
                    .icon("⚡"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomIconView);
}

// ============================================================================
// InlineAlert Tests
// ============================================================================

#[gpui::test]
async fn test_inline_alert_renders(cx: &mut TestAppContext) {
    struct InlineAlertView;

    impl Render for InlineAlertView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(InlineAlert::new("Inline alert message"))
        }
    }

    let _window = cx.add_window(|_window, _cx| InlineAlertView);
}

#[gpui::test]
async fn test_inline_alert_all_variants(cx: &mut TestAppContext) {
    struct InlineVariantsView;

    impl Render for InlineVariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(InlineAlert::new("Info inline").variant(AlertVariant::Info))
                .child(InlineAlert::new("Success inline").variant(AlertVariant::Success))
                .child(InlineAlert::new("Warning inline").variant(AlertVariant::Warning))
                .child(InlineAlert::new("Error inline").variant(AlertVariant::Error))
        }
    }

    let _window = cx.add_window(|_window, _cx| InlineVariantsView);
}

// ============================================================================
// Complex Alert Tests
// ============================================================================

#[gpui::test]
async fn test_alert_full_featured(cx: &mut TestAppContext) {
    struct FullFeaturedView;

    impl Render for FullFeaturedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Alert::new(
                    "full-alert",
                    "This is a full-featured alert with all options",
                )
                .title("Complete Alert")
                .variant(AlertVariant::Warning)
                .icon("⚠️")
                .closeable(true),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullFeaturedView);
}
