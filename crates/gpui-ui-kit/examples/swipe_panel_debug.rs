//! SwipePanel Debug Example
//!
//! Demonstrates the SwipePanel bottom-sheet component:
//! - Peek state (half-hidden by default)
//! - Drag up to expand
//! - Drag down to collapse or return to peek
//! - Tap the handle to toggle

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::swipe_panel::{SwipePanel, SwipePanelState};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::{Heading, Text};

pub struct SwipePanelDebug;

impl Render for SwipePanelDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("swipe-panel-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .child(Heading::h1("SwipePanel Debug"))
            .child(Text::new("Drag the handle up/down or tap it to toggle.").muted(true))
            .child(
                SwipePanel::new("demo-panel")
                    .state(SwipePanelState::Peek)
                    .peek_height(px(80.0))
                    .expanded_height(px(300.0))
                    .content(
                        div()
                            .p_4()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(Heading::h3("Panel Content"))
                            .child(Text::new("Line 1"))
                            .child(Text::new("Line 2"))
                            .child(Text::new("Line 3"))
                            .child(Text::new("Line 4"))
                            .child(Text::new("Line 5")),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("SwipePanel Debug")
            .size(400.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| SwipePanelDebug),
    );
}
