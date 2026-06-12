//! Avatar Debug Example
//!
//! Demonstrates the Avatar and AvatarGroup components:
//! - All sizes
//! - Circle and Square shapes
//! - Status indicators
//! - AvatarGroup

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct AvatarDebug;

impl Render for AvatarDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("avatar-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("Avatar Debug"))
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
                            .child(Avatar::new().name("XS").size(AvatarSize::Xs))
                            .child(Avatar::new().name("SM").size(AvatarSize::Sm))
                            .child(Avatar::new().name("MD").size(AvatarSize::Md))
                            .child(Avatar::new().name("LG").size(AvatarSize::Lg))
                            .child(Avatar::new().name("XL").size(AvatarSize::Xl))
                            .child(Avatar::new().name("XXL").size(AvatarSize::Xxl)),
                    ),
            )
            // Shapes
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Shapes").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                Avatar::new()
                                    .name("Circle")
                                    .size(AvatarSize::Lg)
                                    .shape(AvatarShape::Circle),
                            )
                            .child(
                                Avatar::new()
                                    .name("Square")
                                    .size(AvatarSize::Lg)
                                    .shape(AvatarShape::Square),
                            ),
                    ),
            )
            // Status
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Status Indicators").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                Avatar::new()
                                    .name("ON")
                                    .size(AvatarSize::Lg)
                                    .status(AvatarStatus::Online),
                            )
                            .child(
                                Avatar::new()
                                    .name("AW")
                                    .size(AvatarSize::Lg)
                                    .status(AvatarStatus::Away),
                            )
                            .child(
                                Avatar::new()
                                    .name("BU")
                                    .size(AvatarSize::Lg)
                                    .status(AvatarStatus::Busy),
                            )
                            .child(
                                Avatar::new()
                                    .name("OF")
                                    .size(AvatarSize::Lg)
                                    .status(AvatarStatus::Offline),
                            ),
                    ),
            )
            // AvatarGroup
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Avatar Group").weight(TextWeight::Bold))
                    .child(
                        AvatarGroup::new()
                            .avatars(vec![
                                Avatar::new().name("Alice"),
                                Avatar::new().name("Bob"),
                                Avatar::new().name("Carol"),
                                Avatar::new().name("Dave"),
                                Avatar::new().name("Eve"),
                                Avatar::new().name("Frank"),
                            ])
                            .max_display(4)
                            .size(AvatarSize::Lg),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Avatar Debug")
            .size(600.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| AvatarDebug),
    );
}
