//! Integration tests for ConfirmDialog component
//!
//! Tests the ConfirmDialog component including:
//! - Basic rendering
//! - All variants (Default, Destructive, Warning)
//! - With title
//! - With custom labels
//! - With handlers

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div};
use gpui_ui_kit::confirm_dialog::{ConfirmDialog, ConfirmDialogVariant};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct ConfirmDialogTestView;

impl Render for ConfirmDialogTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(ConfirmDialog::new("test-confirm"))
    }
}

#[gpui::test]
async fn test_confirm_dialog_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| ConfirmDialogTestView);
}

// ============================================================================
// Variant Tests
// ============================================================================

#[gpui::test]
async fn test_confirm_dialog_default_variant(cx: &mut TestAppContext) {
    struct DefaultView;

    impl Render for DefaultView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                ConfirmDialog::new("default-confirm")
                    .variant(ConfirmDialogVariant::Default)
                    .title("Confirm Action")
                    .message("Do you want to proceed?"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| DefaultView);
}

#[gpui::test]
async fn test_confirm_dialog_destructive_variant(cx: &mut TestAppContext) {
    struct DestructiveView;

    impl Render for DestructiveView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                ConfirmDialog::new("destructive-confirm")
                    .variant(ConfirmDialogVariant::Destructive)
                    .title("Delete Album")
                    .message("This action cannot be undone."),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| DestructiveView);
}

#[gpui::test]
async fn test_confirm_dialog_warning_variant(cx: &mut TestAppContext) {
    struct WarningView;

    impl Render for WarningView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                ConfirmDialog::new("warning-confirm")
                    .variant(ConfirmDialogVariant::Warning)
                    .title("Unsaved Changes")
                    .message("You have unsaved changes. Continue anyway?"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| WarningView);
}

#[gpui::test]
async fn test_confirm_dialog_all_variants(cx: &mut TestAppContext) {
    struct AllVariantsView;

    impl Render for AllVariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(
                    ConfirmDialog::new("v-default")
                        .variant(ConfirmDialogVariant::Default)
                        .message("Default variant"),
                )
                .child(
                    ConfirmDialog::new("v-destructive")
                        .variant(ConfirmDialogVariant::Destructive)
                        .message("Destructive variant"),
                )
                .child(
                    ConfirmDialog::new("v-warning")
                        .variant(ConfirmDialogVariant::Warning)
                        .message("Warning variant"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllVariantsView);
}

// ============================================================================
// Title Tests
// ============================================================================

#[gpui::test]
async fn test_confirm_dialog_with_title(cx: &mut TestAppContext) {
    struct TitleView;

    impl Render for TitleView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                ConfirmDialog::new("title-confirm")
                    .title("Important Decision")
                    .message("Are you absolutely sure?"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| TitleView);
}

// ============================================================================
// Custom Label Tests
// ============================================================================

#[gpui::test]
async fn test_confirm_dialog_custom_labels(cx: &mut TestAppContext) {
    struct CustomLabelsView;

    impl Render for CustomLabelsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                ConfirmDialog::new("labels-confirm")
                    .title("Delete Track")
                    .message("Remove this track from the library?")
                    .confirm_label("Yes, Delete")
                    .cancel_label("Keep It"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomLabelsView);
}

// ============================================================================
// Handler Tests
// ============================================================================

#[gpui::test]
async fn test_confirm_dialog_with_handlers(cx: &mut TestAppContext) {
    struct HandlersView;

    impl Render for HandlersView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                ConfirmDialog::new("handlers-confirm")
                    .title("Confirm")
                    .message("Proceed?")
                    .on_confirm(|_window, _cx| {})
                    .on_cancel(|_window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| HandlersView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_confirm_dialog_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                ConfirmDialog::new("full-confirm")
                    .variant(ConfirmDialogVariant::Destructive)
                    .title("Delete Album")
                    .message(
                        "Are you sure you want to delete this album? This action cannot be undone.",
                    )
                    .confirm_label("Delete")
                    .cancel_label("Cancel")
                    .on_confirm(|_window, _cx| {})
                    .on_cancel(|_window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
