//! ConfirmDialog Debug Example
//!
//! Demonstrates the ConfirmDialog component:
//! - Default, Destructive, Warning variants
//! - Custom labels

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct ConfirmDialogDebug;

impl Render for ConfirmDialogDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("confirm-dialog-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("ConfirmDialog Debug"))
            // Default
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Default Variant").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_4()
                            .child(
                                ConfirmDialog::new("confirm-default")
                                    .title("Save Changes?")
                                    .message("You have unsaved changes. Would you like to save them?"),
                            ),
                    ),
            )
            // Destructive
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Destructive Variant").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_4()
                            .child(
                                ConfirmDialog::new("confirm-destructive")
                                    .title("Delete Preset?")
                                    .message("This action cannot be undone. The preset will be permanently deleted.")
                                    .variant(ConfirmDialogVariant::Destructive)
                                    .confirm_label("Delete"),
                            ),
                    ),
            )
            // Warning
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Warning Variant").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_4()
                            .child(
                                ConfirmDialog::new("confirm-warning")
                                    .title("Reset All Settings?")
                                    .message("All plugin settings will be reset to factory defaults.")
                                    .variant(ConfirmDialogVariant::Warning)
                                    .confirm_label("Reset")
                                    .cancel_label("Keep Current"),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("ConfirmDialog Debug")
            .size(600.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| ConfirmDialogDebug),
    );
}
