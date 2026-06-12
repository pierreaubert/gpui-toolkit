//! Alert Debug Example
//!
//! Demonstrates the Alert and InlineAlert components:
//! - All variants (Info, Success, Warning, Error)
//! - With and without title
//! - InlineAlert

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct AlertDebug;

impl Render for AlertDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("alert-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("Alert Debug"))
            // Variants
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Alert Variants").weight(TextWeight::Bold))
                    .child(
                        Alert::new("alert-info", "Audio engine initialized at 44.1kHz, 16-bit.")
                            .variant(AlertVariant::Info)
                            .title("Info"),
                    )
                    .child(
                        Alert::new("alert-success", "EQ preset applied successfully.")
                            .variant(AlertVariant::Success)
                            .title("Success"),
                    )
                    .child(
                        Alert::new(
                            "alert-warning",
                            "Sample rate mismatch between source and output device.",
                        )
                        .variant(AlertVariant::Warning)
                        .title("Warning"),
                    )
                    .child(
                        Alert::new(
                            "alert-error",
                            "Failed to open audio device. Check your settings.",
                        )
                        .variant(AlertVariant::Error)
                        .title("Error"),
                    ),
            )
            // Without title
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Without Title").weight(TextWeight::Bold))
                    .child(
                        Alert::new(
                            "alert-no-title",
                            "Plugin chain reloaded with 5 active plugins.",
                        )
                        .variant(AlertVariant::Info),
                    ),
            )
            // Closeable
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Closeable").weight(TextWeight::Bold))
                    .child(
                        Alert::new("alert-closeable", "This alert can be dismissed.")
                            .variant(AlertVariant::Success)
                            .closeable(true),
                    ),
            )
            // InlineAlert
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("InlineAlert").weight(TextWeight::Bold))
                    .child(InlineAlert::new("Inline info message").variant(AlertVariant::Info))
                    .child(
                        InlineAlert::new("Inline warning message").variant(AlertVariant::Warning),
                    )
                    .child(InlineAlert::new("Inline error message").variant(AlertVariant::Error)),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Alert Debug")
            .size(600.0, 800.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| AlertDebug),
    );
}
