//! Integration tests for ImageView component

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px};
use gpui_ui_kit::image_view::{ImageFit, ImageView};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct ImageViewTestView;

impl Render for ImageViewTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(ImageView::new("test-image"))
    }
}

#[gpui::test]
async fn test_image_view_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| ImageViewTestView);
}

// ============================================================================
// Source and Alt Tests
// ============================================================================

#[gpui::test]
async fn test_image_view_with_src(cx: &mut TestAppContext) {
    struct SrcView;

    impl Render for SrcView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                ImageView::new("src-image")
                    .src("path/to/image.png")
                    .alt("Test image"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SrcView);
}

// ============================================================================
// Sizing Tests
// ============================================================================

#[gpui::test]
async fn test_image_view_with_size(cx: &mut TestAppContext) {
    struct SizedView;

    impl Render for SizedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                ImageView::new("sized-image")
                    .width(px(200.0))
                    .height(px(150.0)),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SizedView);
}

#[gpui::test]
async fn test_image_view_square_size(cx: &mut TestAppContext) {
    struct SquareView;

    impl Render for SquareView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(ImageView::new("square-image").size(px(100.0)))
        }
    }

    let _window = cx.add_window(|_window, _cx| SquareView);
}

// ============================================================================
// Fit Mode Tests
// ============================================================================

#[gpui::test]
async fn test_image_view_fit_modes(cx: &mut TestAppContext) {
    struct FitView;

    impl Render for FitView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .flex()
                .gap_2()
                .child(ImageView::new("cover").fit(ImageFit::Cover).size(px(80.0)))
                .child(
                    ImageView::new("contain")
                        .fit(ImageFit::Contain)
                        .size(px(80.0)),
                )
                .child(ImageView::new("fill").fit(ImageFit::Fill).size(px(80.0)))
        }
    }

    let _window = cx.add_window(|_window, _cx| FitView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_image_view_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                ImageView::new("album-art")
                    .src("cover.jpg")
                    .alt("Album cover")
                    .width(px(300.0))
                    .height(px(300.0))
                    .fit(ImageFit::Cover)
                    .rounded(px(8.0))
                    .show_border(true)
                    .placeholder_icon("🎵"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
