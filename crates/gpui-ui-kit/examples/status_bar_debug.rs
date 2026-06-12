//! Status Bar Debug Example
//!
//! Demonstrates the StatusBar component:
//! - Bottom and top positions
//! - Left, center, and right slots

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct StatusBarDebug;

impl Render for StatusBarDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("status-bar-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Status Bar Debug"))
            // Bottom position
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Bottom Position").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .overflow_hidden()
                            .child(
                                StatusBar::new("status-bottom")
                                    .position(StatusBarPosition::Bottom)
                                    .left(
                                        Text::new("Track 1 - Moonlight Sonata").size(TextSize::Xs),
                                    )
                                    .center(Text::new("02:15 / 05:30").size(TextSize::Xs))
                                    .right(Text::new("Vol: 80%").size(TextSize::Xs)),
                            ),
                    ),
            )
            // Top position
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Top Position").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .overflow_hidden()
                            .child(
                                StatusBar::new("status-top")
                                    .position(StatusBarPosition::Top)
                                    .left(Text::new("SOTF Player").size(TextSize::Xs))
                                    .center(Text::new("44.1kHz / 16-bit").size(TextSize::Xs))
                                    .right(Text::new("5.0ch").size(TextSize::Xs)),
                            ),
                    ),
            )
            // Left only
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Left Slot Only").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .overflow_hidden()
                            .child(
                                StatusBar::new("status-left-only")
                                    .left(Text::new("Ready").size(TextSize::Xs)),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Status Bar Debug")
            .size(600.0, 500.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| StatusBarDebug),
    );
}
