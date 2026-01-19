//! Integration tests for Text component
//!
//! Tests the Text, Heading, Code, and Link components including:
//! - All text sizes and weights
//! - Muted and truncated text
//! - Heading levels
//! - Inline and block code
//! - Link with click callback

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, prelude::*,
    rgb,
};
use gpui_ui_kit::text::{Code, Heading, Link, Text, TextSize, TextWeight};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct TextTestView;

impl Render for TextTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Text::new("Hello, World!"))
    }
}

#[gpui::test]
async fn test_text_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| TextTestView);
}

// ============================================================================
// Size Tests
// ============================================================================

#[gpui::test]
async fn test_text_size_xs(cx: &mut TestAppContext) {
    struct XsSizeView;

    impl Render for XsSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Text::new("Extra small text").size(TextSize::Xs))
        }
    }

    let _window = cx.add_window(|_window, _cx| XsSizeView);
}

#[gpui::test]
async fn test_text_size_sm(cx: &mut TestAppContext) {
    struct SmSizeView;

    impl Render for SmSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Text::new("Small text").size(TextSize::Sm))
        }
    }

    let _window = cx.add_window(|_window, _cx| SmSizeView);
}

#[gpui::test]
async fn test_text_size_md(cx: &mut TestAppContext) {
    struct MdSizeView;

    impl Render for MdSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Text::new("Medium text").size(TextSize::Md))
        }
    }

    let _window = cx.add_window(|_window, _cx| MdSizeView);
}

#[gpui::test]
async fn test_text_size_lg(cx: &mut TestAppContext) {
    struct LgSizeView;

    impl Render for LgSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Text::new("Large text").size(TextSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| LgSizeView);
}

#[gpui::test]
async fn test_text_size_xl(cx: &mut TestAppContext) {
    struct XlSizeView;

    impl Render for XlSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Text::new("Extra large text").size(TextSize::Xl))
        }
    }

    let _window = cx.add_window(|_window, _cx| XlSizeView);
}

#[gpui::test]
async fn test_text_size_xxl(cx: &mut TestAppContext) {
    struct XxlSizeView;

    impl Render for XxlSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Text::new("2X Large text").size(TextSize::Xxl))
        }
    }

    let _window = cx.add_window(|_window, _cx| XxlSizeView);
}

#[gpui::test]
async fn test_text_all_sizes(cx: &mut TestAppContext) {
    struct AllSizesView;

    impl Render for AllSizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(Text::new("XS").size(TextSize::Xs))
                .child(Text::new("SM").size(TextSize::Sm))
                .child(Text::new("MD").size(TextSize::Md))
                .child(Text::new("LG").size(TextSize::Lg))
                .child(Text::new("XL").size(TextSize::Xl))
                .child(Text::new("XXL").size(TextSize::Xxl))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllSizesView);
}

// ============================================================================
// Weight Tests
// ============================================================================

#[gpui::test]
async fn test_text_weight_light(cx: &mut TestAppContext) {
    struct LightWeightView;

    impl Render for LightWeightView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Text::new("Light weight").weight(TextWeight::Light))
        }
    }

    let _window = cx.add_window(|_window, _cx| LightWeightView);
}

#[gpui::test]
async fn test_text_weight_normal(cx: &mut TestAppContext) {
    struct NormalWeightView;

    impl Render for NormalWeightView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Text::new("Normal weight").weight(TextWeight::Normal))
        }
    }

    let _window = cx.add_window(|_window, _cx| NormalWeightView);
}

#[gpui::test]
async fn test_text_weight_medium(cx: &mut TestAppContext) {
    struct MediumWeightView;

    impl Render for MediumWeightView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Text::new("Medium weight").weight(TextWeight::Medium))
        }
    }

    let _window = cx.add_window(|_window, _cx| MediumWeightView);
}

#[gpui::test]
async fn test_text_weight_semibold(cx: &mut TestAppContext) {
    struct SemiboldWeightView;

    impl Render for SemiboldWeightView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Text::new("Semibold weight").weight(TextWeight::Semibold))
        }
    }

    let _window = cx.add_window(|_window, _cx| SemiboldWeightView);
}

#[gpui::test]
async fn test_text_weight_bold(cx: &mut TestAppContext) {
    struct BoldWeightView;

    impl Render for BoldWeightView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Text::new("Bold weight").weight(TextWeight::Bold))
        }
    }

    let _window = cx.add_window(|_window, _cx| BoldWeightView);
}

#[gpui::test]
async fn test_text_all_weights(cx: &mut TestAppContext) {
    struct AllWeightsView;

    impl Render for AllWeightsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(Text::new("Light").weight(TextWeight::Light))
                .child(Text::new("Normal").weight(TextWeight::Normal))
                .child(Text::new("Medium").weight(TextWeight::Medium))
                .child(Text::new("Semibold").weight(TextWeight::Semibold))
                .child(Text::new("Bold").weight(TextWeight::Bold))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllWeightsView);
}

// ============================================================================
// Style Tests
// ============================================================================

#[gpui::test]
async fn test_text_muted(cx: &mut TestAppContext) {
    struct MutedView;

    impl Render for MutedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Text::new("Muted text").muted(true))
        }
    }

    let _window = cx.add_window(|_window, _cx| MutedView);
}

#[gpui::test]
async fn test_text_custom_color(cx: &mut TestAppContext) {
    struct CustomColorView;

    impl Render for CustomColorView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Text::new("Colored text").color(rgb(0xe94560)))
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomColorView);
}

#[gpui::test]
async fn test_text_truncate(cx: &mut TestAppContext) {
    struct TruncateView;

    impl Render for TruncateView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().w_32().child(
                Text::new("This is a very long text that should be truncated").truncate(true),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| TruncateView);
}

// ============================================================================
// Heading Tests
// ============================================================================

#[gpui::test]
async fn test_heading_renders(cx: &mut TestAppContext) {
    struct HeadingView;

    impl Render for HeadingView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Heading::new("Heading"))
        }
    }

    let _window = cx.add_window(|_window, _cx| HeadingView);
}

#[gpui::test]
async fn test_heading_h1(cx: &mut TestAppContext) {
    struct H1View;

    impl Render for H1View {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Heading::h1("Heading 1"))
        }
    }

    let _window = cx.add_window(|_window, _cx| H1View);
}

#[gpui::test]
async fn test_heading_h2(cx: &mut TestAppContext) {
    struct H2View;

    impl Render for H2View {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Heading::h2("Heading 2"))
        }
    }

    let _window = cx.add_window(|_window, _cx| H2View);
}

#[gpui::test]
async fn test_heading_h3(cx: &mut TestAppContext) {
    struct H3View;

    impl Render for H3View {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Heading::h3("Heading 3"))
        }
    }

    let _window = cx.add_window(|_window, _cx| H3View);
}

#[gpui::test]
async fn test_heading_h4(cx: &mut TestAppContext) {
    struct H4View;

    impl Render for H4View {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Heading::h4("Heading 4"))
        }
    }

    let _window = cx.add_window(|_window, _cx| H4View);
}

#[gpui::test]
async fn test_heading_all_levels(cx: &mut TestAppContext) {
    struct AllLevelsView;

    impl Render for AllLevelsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(Heading::h1("Heading 1"))
                .child(Heading::h2("Heading 2"))
                .child(Heading::h3("Heading 3"))
                .child(Heading::h4("Heading 4"))
                .child(Heading::new("Heading 5").level(5))
                .child(Heading::new("Heading 6").level(6))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllLevelsView);
}

// ============================================================================
// Code Tests
// ============================================================================

#[gpui::test]
async fn test_code_inline(cx: &mut TestAppContext) {
    struct InlineCodeView;

    impl Render for InlineCodeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Code::new("inline_code()"))
        }
    }

    let _window = cx.add_window(|_window, _cx| InlineCodeView);
}

#[gpui::test]
async fn test_code_block(cx: &mut TestAppContext) {
    struct BlockCodeView;

    impl Render for BlockCodeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Code::block("fn main() {\n    println!(\"Hello\");\n}"))
        }
    }

    let _window = cx.add_window(|_window, _cx| BlockCodeView);
}

#[gpui::test]
async fn test_code_in_text(cx: &mut TestAppContext) {
    struct CodeInTextView;

    impl Render for CodeInTextView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_1()
                .child(Text::new("Use the"))
                .child(Code::new("println!"))
                .child(Text::new("macro."))
        }
    }

    let _window = cx.add_window(|_window, _cx| CodeInTextView);
}

// ============================================================================
// Link Tests
// ============================================================================

#[gpui::test]
async fn test_link_renders(cx: &mut TestAppContext) {
    struct LinkView;

    impl Render for LinkView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Link::new("link1", "Click me"))
        }
    }

    let _window = cx.add_window(|_window, _cx| LinkView);
}

#[gpui::test]
async fn test_link_with_href(cx: &mut TestAppContext) {
    struct HrefLinkView;

    impl Render for HrefLinkView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Link::new("link1", "Visit site").href("https://example.com"))
        }
    }

    let _window = cx.add_window(|_window, _cx| HrefLinkView);
}

#[gpui::test]
async fn test_link_external(cx: &mut TestAppContext) {
    struct ExternalLinkView;

    impl Render for ExternalLinkView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Link::new("link1", "External")
                    .href("https://example.com")
                    .external(true),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ExternalLinkView);
}

struct ClickableLinkView {
    click_count: Arc<AtomicUsize>,
}

impl Render for ClickableLinkView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let click_count = self.click_count.clone();

        div()
            .size_full()
            .child(
                Link::new("clickable-link", "Click me").on_click(move |_window, _cx| {
                    click_count.fetch_add(1, Ordering::SeqCst);
                }),
            )
    }
}

#[gpui::test]
async fn test_link_click_callback(cx: &mut TestAppContext) {
    let click_count = Arc::new(AtomicUsize::new(0));
    let click_count_clone = click_count.clone();

    let window = cx.add_window(move |_window, _cx| ClickableLinkView {
        click_count: click_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("clickable-link") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            click_count.load(Ordering::SeqCst),
            1,
            "Link click callback should have been called"
        );
    }
}

// ============================================================================
// Combined Feature Tests
// ============================================================================

#[gpui::test]
async fn test_text_all_features(cx: &mut TestAppContext) {
    struct AllFeaturesView;

    impl Render for AllFeaturesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Text::new("Styled text")
                    .size(TextSize::Lg)
                    .weight(TextWeight::Bold)
                    .color(rgb(0x007acc)),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllFeaturesView);
}
