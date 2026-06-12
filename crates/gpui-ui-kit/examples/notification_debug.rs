//! Notification Debug Example
//!
//! Demonstrates the Notification component:
//! - All variants
//! - With description and actions

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct NotificationDebug;

impl Render for NotificationDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("notification-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("Notification Debug"))
            // Variants
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Variants").weight(TextWeight::Bold))
                    .child(
                        Notification::new("notif-info", "System Update Available")
                            .variant(NotificationVariant::Info)
                            .description("Version 0.5.4 is ready to install."),
                    )
                    .child(
                        Notification::new("notif-success", "Export Complete")
                            .variant(NotificationVariant::Success)
                            .description("Your EQ preset was exported to preset.json."),
                    )
                    .child(
                        Notification::new("notif-warning", "High CPU Usage")
                            .variant(NotificationVariant::Warning)
                            .description("Audio processing is using 85% CPU. Consider reducing plugin count."),
                    )
                    .child(
                        Notification::new("notif-error", "Audio Device Lost")
                            .variant(NotificationVariant::Error)
                            .description("The selected output device was disconnected."),
                    ),
            )
            // With action
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("With Action Button").weight(TextWeight::Bold))
                    .child(
                        Notification::new("notif-action", "New Measurement Available")
                            .variant(NotificationVariant::Info)
                            .description("KEF R3 ASR measurement has been updated.")
                            .action("View", |_window, _cx| {}),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Notification Debug")
            .size(600.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| NotificationDebug),
    );
}
