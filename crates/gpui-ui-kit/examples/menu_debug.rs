//! Menu Debug Example
//!
//! Demonstrates the Menu component:
//! - Menu with items, shortcuts, separators
//! - Checkbox items
//! - Disabled and danger items

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::menu::{Menu, MenuItem};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct MenuDebug;

impl Render for MenuDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let items = vec![
            MenuItem::new("new", "New File").with_shortcut("Cmd+N"),
            MenuItem::new("open", "Open File").with_shortcut("Cmd+O"),
            MenuItem::new("save", "Save").with_shortcut("Cmd+S"),
            MenuItem::separator(),
            MenuItem::checkbox("auto-save", "Auto Save", true),
            MenuItem::checkbox("show-hidden", "Show Hidden Files", false),
            MenuItem::separator(),
            MenuItem::new("disabled-item", "Unavailable Feature").disabled(true),
            MenuItem::separator(),
            MenuItem::new("close", "Close Project").danger(),
        ];

        div()
            .id("menu-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Menu Debug"))
            .child(Text::new("Menu rendered inline for demo purposes").muted(true))
            .child(
                div()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .p_4()
                    .child(Menu::new("menu-demo", items).min_width(px(250.0))),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Menu Debug")
            .size(500.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| MenuDebug),
    );
}
