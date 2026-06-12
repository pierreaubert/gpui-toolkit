//! Sidebar Debug Example
//!
//! Demonstrates the Sidebar component:
//! - Left and right positions
//! - Header and content slots
//! - Custom width

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct SidebarDebug;

impl Render for SidebarDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("sidebar-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("Sidebar Debug"))
            // Left sidebar
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Left Sidebar").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .h(px(200.0))
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .overflow_hidden()
                            .child(
                                Sidebar::new("sidebar-left")
                                    .side(SidebarSide::Left)
                                    .width(px(180.0))
                                    .header(
                                        div().p_2().child(
                                            Text::new("Navigation").weight(TextWeight::Bold),
                                        ),
                                    )
                                    .content(
                                        div()
                                            .p_2()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(Text::new("Library"))
                                            .child(Text::new("Playlists"))
                                            .child(Text::new("Settings")),
                                    ),
                            )
                            .child(div().flex_1().p_4().child(Text::new("Main content area"))),
                    ),
            )
            // Right sidebar
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Right Sidebar").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .h(px(200.0))
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .overflow_hidden()
                            .child(div().flex_1().p_4().child(Text::new("Main content area")))
                            .child(
                                Sidebar::new("sidebar-right")
                                    .side(SidebarSide::Right)
                                    .width(px(200.0))
                                    .header(
                                        div()
                                            .p_2()
                                            .child(Text::new("Details").weight(TextWeight::Bold)),
                                    )
                                    .content(
                                        div()
                                            .p_2()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(Text::new("Track: Moonlight Sonata"))
                                            .child(Text::new("Artist: Beethoven"))
                                            .child(Text::new("Duration: 05:30")),
                                    ),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Sidebar Debug")
            .size(700.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| SidebarDebug),
    );
}
