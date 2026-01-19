//! Integration tests for Spinner component
//!
//! Tests the Spinner and LoadingDots components including:
//! - All sizes (Xs to Xl)
//! - Custom colors
//! - With label
//! - LoadingDots variants

use gpui::{Context, TestAppContext, Window, div, prelude::*, rgb};
use gpui_ui_kit::spinner::{LoadingDots, Spinner, SpinnerSize};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct SpinnerTestView;

impl Render for SpinnerTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Spinner::new())
    }
}

#[gpui::test]
async fn test_spinner_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| SpinnerTestView);
}

// ============================================================================
// Size Tests
// ============================================================================

#[gpui::test]
async fn test_spinner_size_xs(cx: &mut TestAppContext) {
    struct XsSpinnerView;

    impl Render for XsSpinnerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Spinner::new().size(SpinnerSize::Xs))
        }
    }

    let _window = cx.add_window(|_window, _cx| XsSpinnerView);
}

#[gpui::test]
async fn test_spinner_size_sm(cx: &mut TestAppContext) {
    struct SmSpinnerView;

    impl Render for SmSpinnerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Spinner::new().size(SpinnerSize::Sm))
        }
    }

    let _window = cx.add_window(|_window, _cx| SmSpinnerView);
}

#[gpui::test]
async fn test_spinner_size_md(cx: &mut TestAppContext) {
    struct MdSpinnerView;

    impl Render for MdSpinnerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Spinner::new().size(SpinnerSize::Md))
        }
    }

    let _window = cx.add_window(|_window, _cx| MdSpinnerView);
}

#[gpui::test]
async fn test_spinner_size_lg(cx: &mut TestAppContext) {
    struct LgSpinnerView;

    impl Render for LgSpinnerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Spinner::new().size(SpinnerSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| LgSpinnerView);
}

#[gpui::test]
async fn test_spinner_size_xl(cx: &mut TestAppContext) {
    struct XlSpinnerView;

    impl Render for XlSpinnerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Spinner::new().size(SpinnerSize::Xl))
        }
    }

    let _window = cx.add_window(|_window, _cx| XlSpinnerView);
}

#[gpui::test]
async fn test_spinner_all_sizes(cx: &mut TestAppContext) {
    struct AllSizesView;

    impl Render for AllSizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .items_center()
                .gap_4()
                .child(Spinner::new().size(SpinnerSize::Xs))
                .child(Spinner::new().size(SpinnerSize::Sm))
                .child(Spinner::new().size(SpinnerSize::Md))
                .child(Spinner::new().size(SpinnerSize::Lg))
                .child(Spinner::new().size(SpinnerSize::Xl))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllSizesView);
}

// ============================================================================
// Custom Color Tests
// ============================================================================

#[gpui::test]
async fn test_spinner_custom_color(cx: &mut TestAppContext) {
    struct CustomColorView;

    impl Render for CustomColorView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Spinner::new().color(rgb(0xe94560)))
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomColorView);
}

#[gpui::test]
async fn test_spinner_multiple_colors(cx: &mut TestAppContext) {
    struct MultipleColorsView;

    impl Render for MultipleColorsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(Spinner::new().color(rgb(0xff0000))) // Red
                .child(Spinner::new().color(rgb(0x00ff00))) // Green
                .child(Spinner::new().color(rgb(0x0000ff))) // Blue
        }
    }

    let _window = cx.add_window(|_window, _cx| MultipleColorsView);
}

// ============================================================================
// Label Tests
// ============================================================================

#[gpui::test]
async fn test_spinner_with_label(cx: &mut TestAppContext) {
    struct LabelView;

    impl Render for LabelView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Spinner::new().label("Loading..."))
        }
    }

    let _window = cx.add_window(|_window, _cx| LabelView);
}

#[gpui::test]
async fn test_spinner_label_with_size(cx: &mut TestAppContext) {
    struct LabelSizeView;

    impl Render for LabelSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(Spinner::new().size(SpinnerSize::Sm).label("Small"))
                .child(Spinner::new().size(SpinnerSize::Md).label("Medium"))
                .child(Spinner::new().size(SpinnerSize::Lg).label("Large"))
        }
    }

    let _window = cx.add_window(|_window, _cx| LabelSizeView);
}

// ============================================================================
// LoadingDots Tests
// ============================================================================

#[gpui::test]
async fn test_loading_dots_renders(cx: &mut TestAppContext) {
    struct LoadingDotsView;

    impl Render for LoadingDotsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(LoadingDots::new())
        }
    }

    let _window = cx.add_window(|_window, _cx| LoadingDotsView);
}

#[gpui::test]
async fn test_loading_dots_all_sizes(cx: &mut TestAppContext) {
    struct LoadingDotsSizesView;

    impl Render for LoadingDotsSizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(LoadingDots::new().size(SpinnerSize::Xs))
                .child(LoadingDots::new().size(SpinnerSize::Sm))
                .child(LoadingDots::new().size(SpinnerSize::Md))
                .child(LoadingDots::new().size(SpinnerSize::Lg))
                .child(LoadingDots::new().size(SpinnerSize::Xl))
        }
    }

    let _window = cx.add_window(|_window, _cx| LoadingDotsSizesView);
}

#[gpui::test]
async fn test_loading_dots_custom_color(cx: &mut TestAppContext) {
    struct LoadingDotsColorView;

    impl Render for LoadingDotsColorView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(LoadingDots::new().color(rgb(0x22c55e)))
        }
    }

    let _window = cx.add_window(|_window, _cx| LoadingDotsColorView);
}

#[gpui::test]
async fn test_loading_dots_default(cx: &mut TestAppContext) {
    struct LoadingDotsDefaultView;

    impl Render for LoadingDotsDefaultView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(LoadingDots::default())
        }
    }

    let _window = cx.add_window(|_window, _cx| LoadingDotsDefaultView);
}

// ============================================================================
// Default Tests
// ============================================================================

#[gpui::test]
async fn test_spinner_default(cx: &mut TestAppContext) {
    struct DefaultView;

    impl Render for DefaultView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Spinner::default())
        }
    }

    let _window = cx.add_window(|_window, _cx| DefaultView);
}

// ============================================================================
// Combined Feature Tests
// ============================================================================

#[gpui::test]
async fn test_spinner_all_features(cx: &mut TestAppContext) {
    struct AllFeaturesView;

    impl Render for AllFeaturesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Spinner::new()
                    .size(SpinnerSize::Lg)
                    .color(rgb(0x007acc))
                    .label("Processing..."),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllFeaturesView);
}
