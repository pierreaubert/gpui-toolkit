//! Stack Debug Example
//!
//! Demonstrates VStack, HStack, Spacer, and Divider components:
//! - Spacing options
//! - Alignment
//! - Dividers

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct StackDebug;

impl Render for StackDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("stack-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("Stack Debug"))
            // VStack
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("VStack with Md spacing").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_3()
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Md)
                                    .child(Text::new("Item 1"))
                                    .child(Text::new("Item 2"))
                                    .child(Text::new("Item 3")),
                            ),
                    ),
            )
            // HStack
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("HStack with Lg spacing").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_3()
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Lg)
                                    .child(Badge::new("Tag 1").variant(BadgeVariant::Primary))
                                    .child(Badge::new("Tag 2").variant(BadgeVariant::Success))
                                    .child(Badge::new("Tag 3").variant(BadgeVariant::Warning)),
                            ),
                    ),
            )
            // HStack with spacer
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("HStack with Spacer").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_3()
                            .child(
                                HStack::new()
                                    .width(StackSize::Full)
                                    .child(Text::new("Left"))
                                    .child(Spacer::new())
                                    .child(Text::new("Right")),
                            ),
                    ),
            )
            // Dividers
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Dividers").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_3()
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Md)
                                    .child(Text::new("Above divider"))
                                    .child(Divider::new())
                                    .child(Text::new("Below divider")),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Stack Debug")
            .size(600.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| StackDebug),
    );
}
