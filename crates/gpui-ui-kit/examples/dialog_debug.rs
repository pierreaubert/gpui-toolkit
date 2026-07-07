//! Dialog Debug Example
//!
//! Demonstrates the Dialog component:
//! - Different sizes (Sm, Md, Lg)
//! - With title, content, and footer

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct DialogDebug;

impl Render for DialogDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("dialog-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("Dialog Debug"))
            .child(Text::new("Dialogs rendered inline for demo purposes").muted(true))
            // Small
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Small Dialog").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_4()
                            .child(
                                Dialog::new("dialog-sm")
                                    .title("Quick Action")
                                    .size(DialogSize::Sm)
                                    .content(Text::new("A small dialog for simple confirmations.")),
                            ),
                    ),
            )
            // Medium
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Medium Dialog").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_4()
                            .child(
                                Dialog::new("dialog-md")
                                    .title("Plugin Settings")
                                    .size(DialogSize::Md)
                                    .content(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_2()
                                            .child(Text::new("Configure your audio plugin settings here."))
                                            .child(Text::new("Adjust parameters as needed.").muted(true)),
                                    )
                                    .footer(
                                        div()
                                            .flex()
                                            .justify_end()
                                            .gap_2()
                                            .child(Button::new("cancel-md", "Cancel").variant(ButtonVariant::Ghost))
                                            .child(Button::new("save-md", "Save")),
                                    ),
                            ),
                    ),
            )
            // Large
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Large Dialog").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_4()
                            .child(
                                Dialog::new("dialog-lg")
                                    .title("EQ Configuration")
                                    .size(DialogSize::Lg)
                                    .content(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_2()
                                            .child(Text::new("Configure parametric EQ filters for your speaker setup."))
                                            .child(Text::new("Add up to 20 filters with customizable frequency, Q, and gain.").muted(true))
                                            .child(Text::new("Supported filter types: Peak, Lowshelf, Highshelf, Lowpass, Highpass").size(TextSize::Sm).muted(true)),
                                    )
                                    .footer(
                                        div()
                                            .flex()
                                            .justify_end()
                                            .gap_2()
                                            .child(Button::new("cancel-lg", "Cancel").variant(ButtonVariant::Ghost))
                                            .child(Button::new("apply-lg", "Apply")),
                                    ),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Dialog Debug")
            .size(700.0, 800.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| DialogDebug),
    );
}
