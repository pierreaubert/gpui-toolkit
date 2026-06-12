//! Progress Debug Example
//!
//! Demonstrates the Progress and CircularProgress components:
//! - All variants
//! - All sizes
//! - With label
//! - CircularProgress

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct ProgressDebug;

impl Render for ProgressDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("progress-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("Progress Debug"))
            // Variants
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Variants").weight(TextWeight::Bold))
                    .child(
                        Progress::new(0.6)
                            .variant(ProgressVariant::Default)
                            .show_label(true),
                    )
                    .child(
                        Progress::new(0.8)
                            .variant(ProgressVariant::Success)
                            .show_label(true),
                    )
                    .child(
                        Progress::new(0.4)
                            .variant(ProgressVariant::Warning)
                            .show_label(true),
                    )
                    .child(
                        Progress::new(0.9)
                            .variant(ProgressVariant::Error)
                            .show_label(true),
                    ),
            )
            // Sizes
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Sizes").weight(TextWeight::Bold))
                    .child(Progress::new(0.5).size(ProgressSize::Xs))
                    .child(Progress::new(0.5).size(ProgressSize::Sm))
                    .child(Progress::new(0.5).size(ProgressSize::Md))
                    .child(Progress::new(0.5).size(ProgressSize::Lg)),
            )
            // Striped
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Striped").weight(TextWeight::Bold))
                    .child(Progress::new(0.7).striped(true).size(ProgressSize::Lg)),
            )
            // Circular
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Circular Progress").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(CircularProgress::new(0.25).show_label(true))
                            .child(
                                CircularProgress::new(0.5)
                                    .variant(ProgressVariant::Success)
                                    .show_label(true),
                            )
                            .child(
                                CircularProgress::new(0.75)
                                    .variant(ProgressVariant::Warning)
                                    .show_label(true),
                            )
                            .child(
                                CircularProgress::new(1.0)
                                    .variant(ProgressVariant::Error)
                                    .show_label(true),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Progress Debug")
            .size(600.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| ProgressDebug),
    );
}
