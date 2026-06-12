//! Toggle Debug Example
//!
//! Demonstrates the Toggle component:
//! - Sliding and Segmented styles
//! - All sizes
//! - With label, disabled

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct ToggleDebug;

impl Render for ToggleDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("toggle-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Toggle Debug"))
            // Sliding style
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Sliding Style (Default)").weight(TextWeight::Bold))
                    .child(Toggle::new("toggle-off").label("Off"))
                    .child(Toggle::new("toggle-on").checked(true).label("On")),
            )
            // Segmented style
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Segmented Style").weight(TextWeight::Bold))
                    .child(
                        Toggle::new("toggle-seg-off")
                            .style(ToggleStyle::Segmented)
                            .label("Bypass"),
                    )
                    .child(
                        Toggle::new("toggle-seg-on")
                            .checked(true)
                            .style(ToggleStyle::Segmented)
                            .label("Active"),
                    ),
            )
            // Sizes
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Sizes").weight(TextWeight::Bold))
                    .child(
                        Toggle::new("toggle-sm")
                            .checked(true)
                            .size(ToggleSize::Sm)
                            .label("Small"),
                    )
                    .child(
                        Toggle::new("toggle-md")
                            .checked(true)
                            .size(ToggleSize::Md)
                            .label("Medium"),
                    )
                    .child(
                        Toggle::new("toggle-lg")
                            .checked(true)
                            .size(ToggleSize::Lg)
                            .label("Large"),
                    ),
            )
            // Disabled
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Disabled").weight(TextWeight::Bold))
                    .child(
                        Toggle::new("toggle-dis-off")
                            .disabled(true)
                            .label("Disabled off"),
                    )
                    .child(
                        Toggle::new("toggle-dis-on")
                            .checked(true)
                            .disabled(true)
                            .label("Disabled on"),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Toggle Debug")
            .size(500.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| ToggleDebug),
    );
}
