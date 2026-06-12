//! Integration tests for Tag component

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div};
use gpui_ui_kit::tag::{Tag, TagSize, TagVariant};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct TagTestView;

impl Render for TagTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Tag::new("tag-1", "FLAC"))
    }
}

#[gpui::test]
async fn test_tag_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| TagTestView);
}

// ============================================================================
// Variant Tests
// ============================================================================

#[gpui::test]
async fn test_tag_all_variants(cx: &mut TestAppContext) {
    struct AllVariantsView;

    impl Render for AllVariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_2()
                .child(Tag::new("t-def", "Default").variant(TagVariant::Default))
                .child(Tag::new("t-pri", "Primary").variant(TagVariant::Primary))
                .child(Tag::new("t-suc", "Success").variant(TagVariant::Success))
                .child(Tag::new("t-wrn", "Warning").variant(TagVariant::Warning))
                .child(Tag::new("t-err", "Error").variant(TagVariant::Error))
                .child(Tag::new("t-out", "Outlined").variant(TagVariant::Outlined))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllVariantsView);
}

// ============================================================================
// Size Tests
// ============================================================================

#[gpui::test]
async fn test_tag_all_sizes(cx: &mut TestAppContext) {
    struct AllSizesView;

    impl Render for AllSizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_2()
                .child(Tag::new("t-sm", "Small").size(TagSize::Sm))
                .child(Tag::new("t-md", "Medium").size(TagSize::Md))
                .child(Tag::new("t-lg", "Large").size(TagSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllSizesView);
}

// ============================================================================
// Removable Tests
// ============================================================================

#[gpui::test]
async fn test_tag_removable(cx: &mut TestAppContext) {
    struct RemovableView;

    impl Render for RemovableView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Tag::new("t-rm", "Removable")
                    .removable(true)
                    .on_remove(|_window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| RemovableView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_tag_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Tag::new("t-full", "FLAC")
                    .variant(TagVariant::Success)
                    .size(TagSize::Lg)
                    .icon("🎵")
                    .removable(true)
                    .on_click(|_window, _cx| {})
                    .on_remove(|_window, _cx| {}),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
