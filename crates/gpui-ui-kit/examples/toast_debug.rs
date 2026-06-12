//! Toast Debug Example
//!
//! Demonstrates the Toast component:
//! - All variants (Info, Success, Warning, Error)
//! - With title
//! - ToastContainer with positioning

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct ToastDebug;

impl Render for ToastDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("toast-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("Toast Debug"))
            // Variants
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Variants").weight(TextWeight::Bold))
                    .child(
                        Toast::new("toast-info", "Configuration saved successfully.")
                            .variant(ToastVariant::Info)
                            .persistent(),
                    )
                    .child(
                        Toast::new("toast-success", "Audio engine started.")
                            .variant(ToastVariant::Success)
                            .persistent(),
                    )
                    .child(
                        Toast::new("toast-warning", "Sample rate mismatch detected.")
                            .variant(ToastVariant::Warning)
                            .persistent(),
                    )
                    .child(
                        Toast::new("toast-error", "Failed to load audio file.")
                            .variant(ToastVariant::Error)
                            .persistent(),
                    ),
            )
            // With title
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("With Title").weight(TextWeight::Bold))
                    .child(
                        Toast::new(
                            "toast-titled",
                            "The EQ preset has been applied to all active tracks.",
                        )
                        .title("Preset Applied")
                        .variant(ToastVariant::Success)
                        .persistent(),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Toast Debug")
            .size(500.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| ToastDebug),
    );
}
