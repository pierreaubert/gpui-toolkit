//! Spinner Debug Example
//!
//! Demonstrates the Spinner and LoadingDots components:
//! - All sizes
//! - With label
//! - LoadingDots

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct SpinnerDebug;

impl Render for SpinnerDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("spinner-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Spinner Debug"))
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
                            .gap_4()
                            .child(Spinner::new().size(SpinnerSize::Xs))
                            .child(Spinner::new().size(SpinnerSize::Sm))
                            .child(Spinner::new().size(SpinnerSize::Md))
                            .child(Spinner::new().size(SpinnerSize::Lg))
                            .child(Spinner::new().size(SpinnerSize::Xl)),
                    ),
            )
            // With label
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("With Label").weight(TextWeight::Bold))
                    .child(Spinner::new().label("Loading audio...")),
            )
            // LoadingDots
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("LoadingDots").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(LoadingDots::new().size(SpinnerSize::Sm))
                            .child(LoadingDots::new().size(SpinnerSize::Md))
                            .child(LoadingDots::new().size(SpinnerSize::Lg)),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Spinner Debug")
            .size(500.0, 500.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| SpinnerDebug),
    );
}
