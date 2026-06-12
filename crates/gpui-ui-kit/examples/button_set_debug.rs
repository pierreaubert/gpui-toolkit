//! ButtonSet Debug Example
//!
//! Demonstrates the ButtonSet component:
//! - Basic button set with selection
//! - Different sizes
//! - Disabled state

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct ButtonSetDebug {
    selected: SharedString,
}

impl Render for ButtonSetDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("button-set-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("ButtonSet Debug"))
            .child(Text::new(format!("Selected: {}", self.selected)).color(theme.accent))
            // Default
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Default").weight(TextWeight::Bold))
                    .child(
                        ButtonSet::new("bset-default")
                            .options(vec![
                                ButtonSetOption::new("stereo", "Stereo"),
                                ButtonSetOption::new("surround", "5.0 Surround"),
                                ButtonSetOption::new("atmos", "Atmos"),
                            ])
                            .selected(self.selected.clone()),
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
                        ButtonSet::new("bset-xs")
                            .options(vec![
                                ButtonSetOption::new("a", "A"),
                                ButtonSetOption::new("b", "B"),
                                ButtonSetOption::new("c", "C"),
                            ])
                            .selected(SharedString::from("a"))
                            .size(ButtonSetSize::Xs),
                    )
                    .child(
                        ButtonSet::new("bset-sm")
                            .options(vec![
                                ButtonSetOption::new("a", "A"),
                                ButtonSetOption::new("b", "B"),
                                ButtonSetOption::new("c", "C"),
                            ])
                            .selected(SharedString::from("b"))
                            .size(ButtonSetSize::Sm),
                    )
                    .child(
                        ButtonSet::new("bset-lg")
                            .options(vec![
                                ButtonSetOption::new("a", "A"),
                                ButtonSetOption::new("b", "B"),
                                ButtonSetOption::new("c", "C"),
                            ])
                            .selected(SharedString::from("c"))
                            .size(ButtonSetSize::Lg),
                    ),
            )
            // Disabled
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Disabled").weight(TextWeight::Bold))
                    .child(
                        ButtonSet::new("bset-disabled")
                            .options(vec![
                                ButtonSetOption::new("on", "On"),
                                ButtonSetOption::new("off", "Off"),
                            ])
                            .selected(SharedString::from("on"))
                            .disabled(true),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("ButtonSet Debug")
            .size(600.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| {
            cx.new(|_cx| ButtonSetDebug {
                selected: "stereo".into(),
            })
        },
    );
}
