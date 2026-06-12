//! SplitPane Debug Example
//!
//! Demonstrates the SplitPane component:
//! - Horizontal and vertical splits
//! - Custom ratio

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct SplitPaneDebug;

impl Render for SplitPaneDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("split-pane-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("SplitPane Debug"))
            // Horizontal
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Horizontal Split").weight(TextWeight::Bold))
                    .child(
                        div()
                            .h(px(200.0))
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .overflow_hidden()
                            .child(
                                SplitPane::new("split-horiz")
                                    .direction(SplitDirection::Horizontal)
                                    .ratio(0.3)
                                    .first(
                                        div()
                                            .size_full()
                                            .p_3()
                                            .bg(theme.surface)
                                            .child(Text::new("Left pane (30%)")),
                                    )
                                    .second(
                                        div()
                                            .size_full()
                                            .p_3()
                                            .child(Text::new("Right pane (70%)")),
                                    ),
                            ),
                    ),
            )
            // Vertical
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Vertical Split").weight(TextWeight::Bold))
                    .child(
                        div()
                            .h(px(250.0))
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .overflow_hidden()
                            .child(
                                SplitPane::new("split-vert")
                                    .direction(SplitDirection::Vertical)
                                    .ratio(0.5)
                                    .first(
                                        div()
                                            .size_full()
                                            .p_3()
                                            .bg(theme.surface)
                                            .child(Text::new("Top pane")),
                                    )
                                    .second(
                                        div().size_full().p_3().child(Text::new("Bottom pane")),
                                    ),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("SplitPane Debug")
            .size(700.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| SplitPaneDebug),
    );
}
