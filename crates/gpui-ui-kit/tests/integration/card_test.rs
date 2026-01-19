//! Integration tests for Card component
//!
//! Tests the Card component including:
//! - Basic rendering
//! - Header, content, and footer sections
//! - Custom background colors
//! - Custom border colors
//! - Combined configurations

use gpui::{Context, TestAppContext, Window, div, prelude::*, rgb};
use gpui_ui_kit::card::Card;

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct CardTestView;

impl Render for CardTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Card::new().content(div().child("Card content")))
    }
}

#[gpui::test]
async fn test_card_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| CardTestView);
}

// ============================================================================
// Section Tests
// ============================================================================

#[gpui::test]
async fn test_card_with_header(cx: &mut TestAppContext) {
    struct HeaderCardView;

    impl Render for HeaderCardView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Card::new()
                    .header(div().child("Card Header"))
                    .content(div().child("Card content")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| HeaderCardView);
}

#[gpui::test]
async fn test_card_with_footer(cx: &mut TestAppContext) {
    struct FooterCardView;

    impl Render for FooterCardView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Card::new()
                    .content(div().child("Card content"))
                    .footer(div().child("Card Footer")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FooterCardView);
}

#[gpui::test]
async fn test_card_with_all_sections(cx: &mut TestAppContext) {
    struct AllSectionsView;

    impl Render for AllSectionsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Card::new()
                    .header(div().child("Header"))
                    .content(div().child("Content"))
                    .footer(div().child("Footer")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllSectionsView);
}

#[gpui::test]
async fn test_card_header_only(cx: &mut TestAppContext) {
    struct HeaderOnlyView;

    impl Render for HeaderOnlyView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Card::new().header(div().child("Just a header")))
        }
    }

    let _window = cx.add_window(|_window, _cx| HeaderOnlyView);
}

#[gpui::test]
async fn test_card_content_only(cx: &mut TestAppContext) {
    struct ContentOnlyView;

    impl Render for ContentOnlyView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Card::new().content(div().child("Just content")))
        }
    }

    let _window = cx.add_window(|_window, _cx| ContentOnlyView);
}

#[gpui::test]
async fn test_card_footer_only(cx: &mut TestAppContext) {
    struct FooterOnlyView;

    impl Render for FooterOnlyView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Card::new().footer(div().child("Just a footer")))
        }
    }

    let _window = cx.add_window(|_window, _cx| FooterOnlyView);
}

// ============================================================================
// Custom Color Tests
// ============================================================================

#[gpui::test]
async fn test_card_custom_background(cx: &mut TestAppContext) {
    struct CustomBgView;

    impl Render for CustomBgView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Card::new()
                    .background(rgb(0x1a1a2e))
                    .content(div().child("Custom background")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomBgView);
}

#[gpui::test]
async fn test_card_custom_header_background(cx: &mut TestAppContext) {
    struct CustomHeaderBgView;

    impl Render for CustomHeaderBgView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Card::new()
                    .header_background(rgb(0x16213e))
                    .header(div().child("Custom header bg"))
                    .content(div().child("Content")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomHeaderBgView);
}

#[gpui::test]
async fn test_card_custom_border(cx: &mut TestAppContext) {
    struct CustomBorderView;

    impl Render for CustomBorderView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Card::new()
                    .border(rgb(0xe94560))
                    .content(div().child("Custom border")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomBorderView);
}

#[gpui::test]
async fn test_card_all_custom_colors(cx: &mut TestAppContext) {
    struct AllCustomColorsView;

    impl Render for AllCustomColorsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Card::new()
                    .background(rgb(0x0f0f0f))
                    .header_background(rgb(0x1a1a1a))
                    .border(rgb(0x333333))
                    .header(div().child("Header"))
                    .content(div().child("Content"))
                    .footer(div().child("Footer")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllCustomColorsView);
}

// ============================================================================
// Style Tests
// ============================================================================

#[gpui::test]
async fn test_card_with_style(cx: &mut TestAppContext) {
    struct StyledCardView;

    impl Render for StyledCardView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Card::new()
                    .style(|div| div.w_64().h_48())
                    .content(div().child("Styled card")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| StyledCardView);
}

// ============================================================================
// Complex Content Tests
// ============================================================================

#[gpui::test]
async fn test_card_with_complex_content(cx: &mut TestAppContext) {
    struct ComplexContentView;

    impl Render for ComplexContentView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Card::new()
                    .header(
                        div()
                            .flex()
                            .justify_between()
                            .child("Title")
                            .child("Action"),
                    )
                    .content(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(div().child("Line 1"))
                            .child(div().child("Line 2"))
                            .child(div().child("Line 3")),
                    )
                    .footer(div().flex().justify_end().child("OK")),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ComplexContentView);
}

// ============================================================================
// Default Tests
// ============================================================================

#[gpui::test]
async fn test_card_default(cx: &mut TestAppContext) {
    struct DefaultView;

    impl Render for DefaultView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Card::default())
        }
    }

    let _window = cx.add_window(|_window, _cx| DefaultView);
}

// ============================================================================
// Multiple Cards Tests
// ============================================================================

#[gpui::test]
async fn test_multiple_cards(cx: &mut TestAppContext) {
    struct MultipleCardsView;

    impl Render for MultipleCardsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    Card::new()
                        .header(div().child("Card 1"))
                        .content(div().child("Content 1")),
                )
                .child(
                    Card::new()
                        .header(div().child("Card 2"))
                        .content(div().child("Content 2")),
                )
                .child(
                    Card::new()
                        .header(div().child("Card 3"))
                        .content(div().child("Content 3")),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| MultipleCardsView);
}
