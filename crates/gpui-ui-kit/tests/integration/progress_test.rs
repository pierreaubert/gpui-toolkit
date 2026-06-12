//! Integration tests for Progress component
//!
//! Tests the Progress and CircularProgress components including:
//! - All variants rendering
//! - All sizes rendering
//! - With label display
//! - Zero and full values
//! - Circular progress rendering

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div};
use gpui_ui_kit::progress::{CircularProgress, Progress, ProgressSize, ProgressVariant};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct ProgressTestView;

impl Render for ProgressTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Progress::new(0.75))
    }
}

#[gpui::test]
async fn test_progress_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| ProgressTestView);
}

// ============================================================================
// All Variants Tests
// ============================================================================

#[gpui::test]
async fn test_progress_default_variant(cx: &mut TestAppContext) {
    struct DefaultVariantView;

    impl Render for DefaultVariantView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Progress::new(0.5).variant(ProgressVariant::Default))
        }
    }

    let _window = cx.add_window(|_window, _cx| DefaultVariantView);
}

#[gpui::test]
async fn test_progress_success_variant(cx: &mut TestAppContext) {
    struct SuccessVariantView;

    impl Render for SuccessVariantView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Progress::new(0.8).variant(ProgressVariant::Success))
        }
    }

    let _window = cx.add_window(|_window, _cx| SuccessVariantView);
}

#[gpui::test]
async fn test_progress_warning_variant(cx: &mut TestAppContext) {
    struct WarningVariantView;

    impl Render for WarningVariantView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Progress::new(0.5).variant(ProgressVariant::Warning))
        }
    }

    let _window = cx.add_window(|_window, _cx| WarningVariantView);
}

#[gpui::test]
async fn test_progress_error_variant(cx: &mut TestAppContext) {
    struct ErrorVariantView;

    impl Render for ErrorVariantView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Progress::new(0.3).variant(ProgressVariant::Error))
        }
    }

    let _window = cx.add_window(|_window, _cx| ErrorVariantView);
}

#[gpui::test]
async fn test_progress_all_variants_together(cx: &mut TestAppContext) {
    struct AllVariantsView;

    impl Render for AllVariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(Progress::new(0.25).variant(ProgressVariant::Default))
                .child(Progress::new(0.5).variant(ProgressVariant::Success))
                .child(Progress::new(0.75).variant(ProgressVariant::Warning))
                .child(Progress::new(1.0).variant(ProgressVariant::Error))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllVariantsView);
}

// ============================================================================
// All Sizes Tests
// ============================================================================

#[gpui::test]
async fn test_progress_all_sizes(cx: &mut TestAppContext) {
    struct AllSizesView;

    impl Render for AllSizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(Progress::new(0.5).size(ProgressSize::Xs))
                .child(Progress::new(0.5).size(ProgressSize::Sm))
                .child(Progress::new(0.5).size(ProgressSize::Md))
                .child(Progress::new(0.5).size(ProgressSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllSizesView);
}

// ============================================================================
// With Label Tests
// ============================================================================

#[gpui::test]
async fn test_progress_with_label(cx: &mut TestAppContext) {
    struct LabelView;

    impl Render for LabelView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Progress::new(0.65)
                    .max(1.0)
                    .show_label(true)
                    .size(ProgressSize::Lg),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| LabelView);
}

// ============================================================================
// Edge Value Tests
// ============================================================================

#[gpui::test]
async fn test_progress_zero_value(cx: &mut TestAppContext) {
    struct ZeroValueView;

    impl Render for ZeroValueView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Progress::new(0.0))
        }
    }

    let _window = cx.add_window(|_window, _cx| ZeroValueView);
}

#[gpui::test]
async fn test_progress_full_value(cx: &mut TestAppContext) {
    struct FullValueView;

    impl Render for FullValueView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Progress::new(1.0).max(1.0))
        }
    }

    let _window = cx.add_window(|_window, _cx| FullValueView);
}

// ============================================================================
// Striped and Animated Tests
// ============================================================================

#[gpui::test]
async fn test_progress_striped_animated(cx: &mut TestAppContext) {
    struct StripedView;

    impl Render for StripedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Progress::new(0.7)
                    .striped(true)
                    .animated(true)
                    .size(ProgressSize::Lg),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| StripedView);
}

// ============================================================================
// Circular Progress Tests
// ============================================================================

#[gpui::test]
async fn test_circular_progress_renders(cx: &mut TestAppContext) {
    struct CircularView;

    impl Render for CircularView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                CircularProgress::new(75.0)
                    .max(100.0)
                    .size(gpui::px(64.0))
                    .thickness(gpui::px(8.0))
                    .variant(ProgressVariant::Success)
                    .show_label(true),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CircularView);
}

#[gpui::test]
async fn test_circular_progress_all_variants(cx: &mut TestAppContext) {
    struct CircularVariantsView;

    impl Render for CircularVariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(CircularProgress::new(25.0).variant(ProgressVariant::Default))
                .child(CircularProgress::new(50.0).variant(ProgressVariant::Success))
                .child(CircularProgress::new(75.0).variant(ProgressVariant::Warning))
                .child(CircularProgress::new(100.0).variant(ProgressVariant::Error))
        }
    }

    let _window = cx.add_window(|_window, _cx| CircularVariantsView);
}
