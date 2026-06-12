//! Toolbar Debug Example
//!
//! Demonstrates the Toolbar component:
//! - Button items
//! - Active and disabled states
//! - Separators

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct ToolbarDebug;

impl Render for ToolbarDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("toolbar-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Toolbar Debug"))
            // Basic
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Basic Toolbar").weight(TextWeight::Bold))
                    .child(
                        Toolbar::new("toolbar-basic")
                            .item(ToolbarItem::button("tb-play", "Play"))
                            .item(ToolbarItem::button("tb-pause", "Pause"))
                            .item(ToolbarItem::button("tb-stop", "Stop"))
                            .separator()
                            .item(ToolbarItem::button("tb-prev", "Prev"))
                            .item(ToolbarItem::button("tb-next", "Next")),
                    ),
            )
            // With active and disabled
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Active & Disabled States").weight(TextWeight::Bold))
                    .child(
                        Toolbar::new("toolbar-states")
                            .item(ToolbarItem::button("tb-eq", "EQ").active(true))
                            .item(ToolbarItem::button("tb-comp", "Compressor"))
                            .item(ToolbarItem::button("tb-upmix", "Upmixer"))
                            .separator()
                            .item(ToolbarItem::button("tb-locked", "Premium").disabled(true)),
                    ),
            )
            // No border
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("No Border").weight(TextWeight::Bold))
                    .child(
                        Toolbar::new("toolbar-noborder")
                            .bordered(false)
                            .item(ToolbarItem::button("tb-a", "Action A"))
                            .item(ToolbarItem::button("tb-b", "Action B")),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Toolbar Debug")
            .size(700.0, 500.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| ToolbarDebug),
    );
}
