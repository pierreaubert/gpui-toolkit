//! Keyboard Shortcut Label Debug Example
//!
//! Demonstrates the KeyboardShortcutLabel component:
//! - Default and large sizes
//! - Various key combinations

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct KeyboardShortcutLabelDebug;

impl Render for KeyboardShortcutLabelDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("keyboard-shortcut-label-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Keyboard Shortcut Label Debug"))
            // Default Size
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Default Size (Md)").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(KeyboardShortcutLabel::new("Cmd+K"))
                            .child(KeyboardShortcutLabel::new("Ctrl+Shift+P"))
                            .child(KeyboardShortcutLabel::new("Alt+F4"))
                            .child(KeyboardShortcutLabel::new("Cmd+S"))
                            .child(KeyboardShortcutLabel::new("Esc")),
                    ),
            )
            // Large Size
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Large Size").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(
                                KeyboardShortcutLabel::new("Cmd+K").size(KeyboardShortcutSize::Lg),
                            )
                            .child(
                                KeyboardShortcutLabel::new("Ctrl+Shift+P")
                                    .size(KeyboardShortcutSize::Lg),
                            )
                            .child(
                                KeyboardShortcutLabel::new("Space").size(KeyboardShortcutSize::Lg),
                            ),
                    ),
            )
            // In context
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("In Context").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Text::new("Save file"))
                            .child(KeyboardShortcutLabel::new("Cmd+S")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Text::new("Open command palette"))
                            .child(KeyboardShortcutLabel::new("Cmd+Shift+P")),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Keyboard Shortcut Label Debug")
            .size(600.0, 500.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| KeyboardShortcutLabelDebug),
    );
}
