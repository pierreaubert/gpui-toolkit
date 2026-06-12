//! Integration tests for Notification component

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div};
use gpui_ui_kit::notification::{Notification, NotificationVariant};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct NotificationTestView;

impl Render for NotificationTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Notification::new("notif-1", "File saved"))
    }
}

#[gpui::test]
async fn test_notification_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| NotificationTestView);
}

// ============================================================================
// Variant Tests
// ============================================================================

#[gpui::test]
async fn test_notification_all_variants(cx: &mut TestAppContext) {
    struct AllVariantsView;

    impl Render for AllVariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(Notification::new("n-info", "Info").variant(NotificationVariant::Info))
                .child(
                    Notification::new("n-success", "Success").variant(NotificationVariant::Success),
                )
                .child(
                    Notification::new("n-warning", "Warning").variant(NotificationVariant::Warning),
                )
                .child(Notification::new("n-error", "Error").variant(NotificationVariant::Error))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllVariantsView);
}

// ============================================================================
// Description and Action Tests
// ============================================================================

#[gpui::test]
async fn test_notification_with_description(cx: &mut TestAppContext) {
    struct DescriptionView;

    impl Render for DescriptionView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Notification::new("n-desc", "Connected")
                    .variant(NotificationVariant::Success)
                    .description("Your wallet is ready to use"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| DescriptionView);
}

#[gpui::test]
async fn test_notification_with_action(cx: &mut TestAppContext) {
    struct ActionView;

    impl Render for ActionView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Notification::new("n-action", "Wallet connected")
                    .variant(NotificationVariant::Success)
                    .action("Disconnect", |_window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ActionView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_notification_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Notification::new("n-full", "Wallet connected")
                    .variant(NotificationVariant::Success)
                    .description("Your wallet is ready to use")
                    .icon("✓")
                    .dismissible(true)
                    .on_dismiss(|_window, _cx| {})
                    .action("Disconnect", |_window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
