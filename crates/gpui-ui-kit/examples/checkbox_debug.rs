//! Checkbox Debug Example
//!
//! Demonstrates the Checkbox component:
//! - Checked, unchecked, indeterminate states
//! - All sizes
//! - With label, disabled

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct CheckboxDebug;

impl Render for CheckboxDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("checkbox-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Checkbox Debug"))
            // States
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("States").weight(TextWeight::Bold))
                    .child(Checkbox::new("cb-unchecked").label("Unchecked"))
                    .child(Checkbox::new("cb-checked").checked(true).label("Checked"))
                    .child(
                        Checkbox::new("cb-indeterminate")
                            .indeterminate(true)
                            .label("Indeterminate"),
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
                        Checkbox::new("cb-sm")
                            .checked(true)
                            .size(CheckboxSize::Sm)
                            .label("Small"),
                    )
                    .child(
                        Checkbox::new("cb-md")
                            .checked(true)
                            .size(CheckboxSize::Md)
                            .label("Medium"),
                    )
                    .child(
                        Checkbox::new("cb-lg")
                            .checked(true)
                            .size(CheckboxSize::Lg)
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
                        Checkbox::new("cb-dis-off")
                            .disabled(true)
                            .label("Disabled unchecked"),
                    )
                    .child(
                        Checkbox::new("cb-dis-on")
                            .checked(true)
                            .disabled(true)
                            .label("Disabled checked"),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Checkbox Debug")
            .size(500.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| CheckboxDebug),
    );
}
