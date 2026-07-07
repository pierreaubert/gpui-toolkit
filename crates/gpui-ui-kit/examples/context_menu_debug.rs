//! ContextMenu Debug Example
//!
//! Demonstrates the ContextMenu component:
//! - Menu items with labels
//! - Items with shortcuts and icons
//! - Separators and disabled items

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::context_menu::ContextMenu;
use gpui_ui_kit::menu::MenuItem;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct ContextMenuDebug;

impl Render for ContextMenuDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let items = vec![
            MenuItem::new("cut", "Cut").with_shortcut("Cmd+X"),
            MenuItem::new("copy", "Copy").with_shortcut("Cmd+C"),
            MenuItem::new("paste", "Paste").with_shortcut("Cmd+V"),
            MenuItem::separator(),
            MenuItem::new("select-all", "Select All").with_shortcut("Cmd+A"),
            MenuItem::separator(),
            MenuItem::new("delete", "Delete").danger(),
        ];

        div()
            .id("context-menu-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("ContextMenu Debug"))
            .child(Text::new("ContextMenu rendered inline for demo purposes").muted(true))
            .child(
                div()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .p_4()
                    .child(ContextMenu::new("ctx-menu-demo", items).min_width(px(200.0))),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("ContextMenu Debug")
            .size(500.0, 500.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| ContextMenuDebug),
    );
}
