//! Integration tests for Dialog component
//!
//! Tests the dialog/modal component including:
//! - Basic rendering
//! - Size variants
//! - Title and content
//! - Footer section
//! - Close button
//! - Backdrop click behavior
//! - Theme customization

use gpui::{Context, TestAppContext, Window, div, prelude::*};
use gpui_ui_kit::dialog::{Dialog, DialogSize, DialogTheme};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct DialogTestView;

impl Render for DialogTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            Dialog::new("test-dialog")
                .title("Test Dialog")
                .content(div().child("Dialog content goes here")),
        )
    }
}

#[gpui::test]
async fn test_dialog_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| DialogTestView);
}

// ============================================================================
// Size Variant Tests
// ============================================================================

#[gpui::test]
async fn test_dialog_size_sm(cx: &mut TestAppContext) {
    struct SmallDialogView;

    impl Render for SmallDialogView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Dialog::new("sm-dialog")
                    .size(DialogSize::Sm)
                    .title("Small Dialog")
                    .content("Small content"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SmallDialogView);
}

#[gpui::test]
async fn test_dialog_size_md(cx: &mut TestAppContext) {
    struct MediumDialogView;

    impl Render for MediumDialogView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Dialog::new("md-dialog")
                    .size(DialogSize::Md)
                    .title("Medium Dialog")
                    .content("Medium content"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| MediumDialogView);
}

#[gpui::test]
async fn test_dialog_size_lg(cx: &mut TestAppContext) {
    struct LargeDialogView;

    impl Render for LargeDialogView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Dialog::new("lg-dialog")
                    .size(DialogSize::Lg)
                    .title("Large Dialog")
                    .content("Large content"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| LargeDialogView);
}

#[gpui::test]
async fn test_dialog_size_xl(cx: &mut TestAppContext) {
    struct XLDialogView;

    impl Render for XLDialogView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Dialog::new("xl-dialog")
                    .size(DialogSize::Xl)
                    .title("Extra Large Dialog")
                    .content("Extra large content"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| XLDialogView);
}

#[gpui::test]
async fn test_dialog_size_full(cx: &mut TestAppContext) {
    struct FullDialogView;

    impl Render for FullDialogView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Dialog::new("full-dialog")
                    .size(DialogSize::Full)
                    .title("Full Width Dialog")
                    .content("Full width content"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullDialogView);
}

// ============================================================================
// Content Tests
// ============================================================================

#[gpui::test]
async fn test_dialog_with_rich_content(cx: &mut TestAppContext) {
    struct RichContentView;

    impl Render for RichContentView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Dialog::new("rich-dialog").title("Rich Content").content(
                    div()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child(div().child("Paragraph 1"))
                        .child(div().child("Paragraph 2"))
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(div().bg(gpui::rgb(0xff0000)).size(gpui::px(20.0)))
                                .child(div().bg(gpui::rgb(0x00ff00)).size(gpui::px(20.0))),
                        ),
                ),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| RichContentView);
}

#[gpui::test]
async fn test_dialog_with_child_alias(cx: &mut TestAppContext) {
    struct ChildAliasView;

    impl Render for ChildAliasView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Dialog::new("child-alias-dialog")
                    .title("Using child()")
                    .child(div().child("Content via child() method")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ChildAliasView);
}

// ============================================================================
// Footer Tests
// ============================================================================

#[gpui::test]
async fn test_dialog_with_footer(cx: &mut TestAppContext) {
    struct FooterView;

    impl Render for FooterView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Dialog::new("footer-dialog")
                    .title("Dialog with Footer")
                    .content("Main content area")
                    .footer(
                        div()
                            .flex()
                            .gap_2()
                            .justify_end()
                            .child(div().px_4().py_2().bg(gpui::rgb(0x444444)).child("Cancel"))
                            .child(div().px_4().py_2().bg(gpui::rgb(0x007acc)).child("OK")),
                    ),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FooterView);
}

// ============================================================================
// Close Button Tests
// ============================================================================

#[gpui::test]
async fn test_dialog_with_close_button(cx: &mut TestAppContext) {
    struct CloseButtonView {
        close_count: Arc<AtomicUsize>,
    }

    impl Render for CloseButtonView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let close_count = self.close_count.clone();

            div().size_full().child(
                Dialog::new("close-btn-dialog")
                    .title("With Close Button")
                    .content("Click X to close")
                    .show_close_button(true)
                    .on_close(move |_window, _cx| {
                        close_count.fetch_add(1, Ordering::SeqCst);
                    }),
            )
        }
    }

    let close_count = Arc::new(AtomicUsize::new(0));
    let close_count_clone = close_count.clone();

    let _window = cx.add_window(move |_window, _cx| CloseButtonView {
        close_count: close_count_clone,
    });
}

#[gpui::test]
async fn test_dialog_without_close_button(cx: &mut TestAppContext) {
    struct NoCloseButtonView;

    impl Render for NoCloseButtonView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Dialog::new("no-close-btn-dialog")
                    .title("No Close Button")
                    .content("Close button hidden")
                    .show_close_button(false),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| NoCloseButtonView);
}

// ============================================================================
// Backdrop Tests
// ============================================================================

#[gpui::test]
async fn test_dialog_close_on_backdrop(cx: &mut TestAppContext) {
    struct BackdropCloseView {
        close_triggered: Arc<AtomicBool>,
    }

    impl Render for BackdropCloseView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let close_triggered = self.close_triggered.clone();

            div().size_full().child(
                Dialog::new("backdrop-close-dialog")
                    .title("Click Backdrop to Close")
                    .content("Clicking backdrop will close")
                    .close_on_backdrop(true)
                    .on_close(move |_window, _cx| {
                        close_triggered.store(true, Ordering::SeqCst);
                    }),
            )
        }
    }

    let close_triggered = Arc::new(AtomicBool::new(false));
    let close_triggered_clone = close_triggered.clone();

    let _window = cx.add_window(move |_window, _cx| BackdropCloseView {
        close_triggered: close_triggered_clone,
    });
}

#[gpui::test]
async fn test_dialog_no_close_on_backdrop(cx: &mut TestAppContext) {
    struct NoBackdropCloseView;

    impl Render for NoBackdropCloseView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Dialog::new("no-backdrop-close-dialog")
                    .title("Backdrop Disabled")
                    .content("Clicking backdrop won't close")
                    .close_on_backdrop(false),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| NoBackdropCloseView);
}

// ============================================================================
// Theme Tests
// ============================================================================

#[gpui::test]
async fn test_dialog_with_custom_theme(cx: &mut TestAppContext) {
    struct ThemedDialogView;

    impl Render for ThemedDialogView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let custom_theme = DialogTheme {
                backdrop: gpui::rgba(0x000000cc),
                background: gpui::rgb(0x2a2a2a),
                border: gpui::rgb(0xff6600),
                header_border: gpui::rgb(0x444444),
                title: gpui::rgb(0xffffff),
                close: gpui::rgb(0xaaaaaa),
                close_hover: gpui::rgb(0xffffff),
                close_hover_bg: gpui::rgb(0x444444),
            };

            div()
                .size_full()
                .child(Dialog::new("themed-dialog").build_with_theme(&custom_theme))
        }
    }

    let _window = cx.add_window(|_window, _cx| ThemedDialogView);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[gpui::test]
async fn test_dialog_no_title(cx: &mut TestAppContext) {
    struct NoTitleView;

    impl Render for NoTitleView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(Dialog::new("no-title-dialog").content("Content without title"))
        }
    }

    let _window = cx.add_window(|_window, _cx| NoTitleView);
}

#[gpui::test]
async fn test_dialog_long_title(cx: &mut TestAppContext) {
    struct LongTitleView;

    impl Render for LongTitleView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Dialog::new("long-title-dialog")
                    .title("This is a very long dialog title that might need to wrap or truncate")
                    .content("Content"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| LongTitleView);
}

#[gpui::test]
async fn test_dialog_empty_content(cx: &mut TestAppContext) {
    struct EmptyContentView;

    impl Render for EmptyContentView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(Dialog::new("empty-content-dialog").title("Empty Content Dialog"))
        }
    }

    let _window = cx.add_window(|_window, _cx| EmptyContentView);
}
