//! Badge Debug Example
//!
//! Demonstrates the Badge and BadgeDot components:
//! - All variants
//! - Sizes
//! - BadgeDot

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct BadgeDebug;

impl Render for BadgeDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("badge-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Badge Debug"))
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
                            .child(Badge::new("Default"))
                            .child(Badge::new("Primary").variant(BadgeVariant::Primary))
                            .child(Badge::new("Success").variant(BadgeVariant::Success))
                            .child(Badge::new("Warning").variant(BadgeVariant::Warning))
                            .child(Badge::new("Error").variant(BadgeVariant::Error))
                            .child(Badge::new("Info").variant(BadgeVariant::Info)),
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
                            .child(Badge::new("Sm").size(BadgeSize::Sm))
                            .child(Badge::new("Md").size(BadgeSize::Md))
                            .child(Badge::new("Lg").size(BadgeSize::Lg)),
                    ),
            )
            // Rounded
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Rounded (Pill)").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(
                                Badge::new("42")
                                    .rounded(true)
                                    .variant(BadgeVariant::Primary),
                            )
                            .child(
                                Badge::new("New")
                                    .rounded(true)
                                    .variant(BadgeVariant::Success),
                            ),
                    ),
            )
            // BadgeDot
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("BadgeDot").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(BadgeDot::new().variant(BadgeVariant::Default))
                            .child(BadgeDot::new().variant(BadgeVariant::Primary))
                            .child(BadgeDot::new().variant(BadgeVariant::Success))
                            .child(BadgeDot::new().variant(BadgeVariant::Warning))
                            .child(BadgeDot::new().variant(BadgeVariant::Error)),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Badge Debug")
            .size(500.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| BadgeDebug),
    );
}
