//! Button Debug Example
//!
//! Demonstrates the Button component:
//! - All variants (Primary, Secondary, Destructive, Ghost, Outline)
//! - All sizes (Xs, Sm, Md, Lg)
//! - Disabled state, icons, full width

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct ButtonDebug;

impl Render for ButtonDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("button-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("Button Debug"))
            // Variants
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Variants").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .flex_wrap()
                            .child(
                                Button::new("btn-primary", "Primary")
                                    .variant(ButtonVariant::Primary),
                            )
                            .child(
                                Button::new("btn-secondary", "Secondary")
                                    .variant(ButtonVariant::Secondary),
                            )
                            .child(
                                Button::new("btn-destructive", "Destructive")
                                    .variant(ButtonVariant::Destructive),
                            )
                            .child(Button::new("btn-ghost", "Ghost").variant(ButtonVariant::Ghost))
                            .child(
                                Button::new("btn-outline", "Outline")
                                    .variant(ButtonVariant::Outline),
                            ),
                    ),
            )
            // Sizes
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Sizes").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(Button::new("btn-xs", "Extra Small").size(ButtonSize::Xs))
                            .child(Button::new("btn-sm", "Small").size(ButtonSize::Sm))
                            .child(Button::new("btn-md", "Medium").size(ButtonSize::Md))
                            .child(Button::new("btn-lg", "Large").size(ButtonSize::Lg)),
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
                        div()
                            .flex()
                            .gap_3()
                            .child(
                                Button::new("btn-dis-primary", "Disabled Primary").disabled(true),
                            )
                            .child(
                                Button::new("btn-dis-secondary", "Disabled Secondary")
                                    .variant(ButtonVariant::Secondary)
                                    .disabled(true),
                            ),
                    ),
            )
            // With icons
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("With Icons").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(Button::new("btn-icon-left", "Save").icon_left("+"))
                            .child(Button::new("btn-icon-right", "Next").icon_right(">"))
                            .child(
                                Button::new("btn-icons-both", "Upload")
                                    .icon_left("+")
                                    .icon_right(">"),
                            ),
                    ),
            )
            // Full width
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Full Width").weight(TextWeight::Bold))
                    .child(Button::new("btn-full", "Full Width Button").full_width(true)),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Button Debug")
            .size(700.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| ButtonDebug),
    );
}
