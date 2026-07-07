//! Tag Debug Example
//!
//! Demonstrates the Tag component:
//! - All variants
//! - Sizes
//! - Removable tags

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct TagDebug;

impl Render for TagDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("tag-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Tag Debug"))
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
                            .child(Tag::new("tag-default", "Default"))
                            .child(Tag::new("tag-primary", "Primary").variant(TagVariant::Primary))
                            .child(Tag::new("tag-success", "Success").variant(TagVariant::Success))
                            .child(Tag::new("tag-warning", "Warning").variant(TagVariant::Warning))
                            .child(Tag::new("tag-error", "Error").variant(TagVariant::Error))
                            .child(
                                Tag::new("tag-outlined", "Outlined").variant(TagVariant::Outlined),
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
                            .child(Tag::new("tag-sm", "Small").size(TagSize::Sm))
                            .child(Tag::new("tag-md", "Medium").size(TagSize::Md))
                            .child(Tag::new("tag-lg", "Large").size(TagSize::Lg)),
                    ),
            )
            // Removable
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Removable").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(
                                Tag::new("tag-rm-1", "FLAC")
                                    .variant(TagVariant::Primary)
                                    .removable(true),
                            )
                            .child(
                                Tag::new("tag-rm-2", "Lossless")
                                    .variant(TagVariant::Success)
                                    .removable(true),
                            )
                            .child(
                                Tag::new("tag-rm-3", "Hi-Res")
                                    .variant(TagVariant::Warning)
                                    .removable(true),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Tag Debug")
            .size(600.0, 500.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| TagDebug),
    );
}
