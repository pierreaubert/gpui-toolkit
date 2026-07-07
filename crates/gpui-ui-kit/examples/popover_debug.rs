//! Popover Debug Example
//!
//! Demonstrates the Popover component:
//! - Different placements

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct PopoverDebug;

impl Render for PopoverDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("popover-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Popover Debug"))
            .child(Text::new("Popovers are shown via trigger interaction. See showcase example for full demo.").muted(true))
            // Static popover content preview
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Popover Content Preview").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_4()
                            .bg(theme.surface)
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(Text::new("Popover Title").weight(TextWeight::Bold))
                            .child(Text::new("This is what popover content looks like.").size(TextSize::Sm))
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(Button::new("pop-cancel", "Cancel").variant(ButtonVariant::Ghost).size(ButtonSize::Sm))
                                    .child(Button::new("pop-confirm", "Confirm").size(ButtonSize::Sm)),
                            ),
                    ),
            )
            // Placements list
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Available Placements").weight(TextWeight::Bold))
                    .child(Text::new("Top, Bottom, Left, Right, TopStart, TopEnd, BottomStart, BottomEnd").size(TextSize::Sm).muted(true)),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Popover Debug")
            .size(600.0, 500.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| PopoverDebug),
    );
}
