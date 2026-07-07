//! ImageView Debug Example
//!
//! Demonstrates the ImageView component:
//! - Different fit modes
//! - With border and rounded corners
//! - Placeholder state

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct ImageViewDebug;

impl Render for ImageViewDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("image-view-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("ImageView Debug"))
            // Placeholder (no src)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Placeholder (no src)").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(ImageView::new("img-placeholder-1").size(px(80.0)))
                            .child(
                                ImageView::new("img-placeholder-2")
                                    .size(px(80.0))
                                    .rounded(px(8.0))
                                    .show_border(true),
                            ),
                    ),
            )
            // Fit modes
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Fit Modes (with placeholder)").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(Text::new("Cover").size(TextSize::Xs).muted(true))
                                    .child(
                                        ImageView::new("img-cover")
                                            .width(px(120.0))
                                            .height(px(80.0))
                                            .fit(ImageFit::Cover)
                                            .show_border(true),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(Text::new("Contain").size(TextSize::Xs).muted(true))
                                    .child(
                                        ImageView::new("img-contain")
                                            .width(px(120.0))
                                            .height(px(80.0))
                                            .fit(ImageFit::Contain)
                                            .show_border(true),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(Text::new("Fill").size(TextSize::Xs).muted(true))
                                    .child(
                                        ImageView::new("img-fill")
                                            .width(px(120.0))
                                            .height(px(80.0))
                                            .fit(ImageFit::Fill)
                                            .show_border(true),
                                    ),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("ImageView Debug")
            .size(600.0, 500.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| ImageViewDebug),
    );
}
