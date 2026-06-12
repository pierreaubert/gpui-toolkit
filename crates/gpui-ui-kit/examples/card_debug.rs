//! Card Debug Example
//!
//! Demonstrates the Card component:
//! - Basic card with content
//! - Card with header, content, footer
//! - Custom background

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct CardDebug;

impl Render for CardDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("card-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("Card Debug"))
            // Basic
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Basic Card").weight(TextWeight::Bold))
                    .child(
                        Card::new().content(
                            div()
                                .p_4()
                                .child(Text::new("Simple card with content only.")),
                        ),
                    ),
            )
            // With header and footer
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Header + Content + Footer").weight(TextWeight::Bold))
                    .child(
                        Card::new()
                            .header(
                                div()
                                    .p_4()
                                    .child(Text::new("Track Info").weight(TextWeight::Bold)),
                            )
                            .content(
                                div()
                                    .p_4()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(Text::new("Moonlight Sonata"))
                                    .child(
                                        Text::new("Ludwig van Beethoven")
                                            .size(TextSize::Sm)
                                            .muted(true),
                                    ),
                            )
                            .footer(div().p_4().child(
                                Text::new("Duration: 05:30").size(TextSize::Xs).muted(true),
                            )),
                    ),
            )
            // Multiple cards in a row
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Card Grid").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(
                                div().flex_1().child(
                                    Card::new()
                                        .header(
                                            div()
                                                .p_3()
                                                .child(Text::new("EQ").weight(TextWeight::Bold)),
                                        )
                                        .content(
                                            div().p_3().child(
                                                Text::new("7 bands active").size(TextSize::Sm),
                                            ),
                                        ),
                                ),
                            )
                            .child(
                                div().flex_1().child(
                                    Card::new()
                                        .header(
                                            div().p_3().child(
                                                Text::new("Upmixer").weight(TextWeight::Bold),
                                            ),
                                        )
                                        .content(
                                            div().p_3().child(
                                                Text::new("5.0 surround").size(TextSize::Sm),
                                            ),
                                        ),
                                ),
                            )
                            .child(
                                div().flex_1().child(
                                    Card::new()
                                        .header(
                                            div().p_3().child(
                                                Text::new("Limiter").weight(TextWeight::Bold),
                                            ),
                                        )
                                        .content(div().p_3().child(
                                            Text::new("-1.0 dB ceiling").size(TextSize::Sm),
                                        )),
                                ),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Card Debug")
            .size(700.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| CardDebug),
    );
}
