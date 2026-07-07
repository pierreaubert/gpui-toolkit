//! CommandPalette Debug Example
//!
//! Demonstrates the CommandPalette component:
//! - With items, shortcuts, categories

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct CommandPaletteDebug;

impl Render for CommandPaletteDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let items = vec![
            CommandItem::new("play", "Play / Pause")
                .shortcut("Space")
                .category("Playback"),
            CommandItem::new("next", "Next Track")
                .shortcut("Cmd+Right")
                .category("Playback"),
            CommandItem::new("prev", "Previous Track")
                .shortcut("Cmd+Left")
                .category("Playback"),
            CommandItem::new("eq", "Open EQ Settings")
                .shortcut("Cmd+E")
                .category("Settings"),
            CommandItem::new("theme", "Toggle Theme")
                .shortcut("Cmd+T")
                .category("Settings"),
            CommandItem::new("disabled-cmd", "Premium Feature")
                .category("Premium")
                .disabled(true),
        ];

        div()
            .id("command-palette-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("CommandPalette Debug"))
            .child(Text::new("Command palette rendered inline for demo").muted(true))
            .child(
                div()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .p_4()
                    .child(
                        CommandPalette::new("cmd-palette-demo", items)
                            .placeholder("Type a command..."),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("CommandPalette Debug")
            .size(600.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| CommandPaletteDebug),
    );
}
