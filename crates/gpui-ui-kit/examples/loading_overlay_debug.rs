//! LoadingOverlay Debug Example
//!
//! Demonstrates the LoadingOverlay component:
//! - With message and subtitle
//! - Different spinner sizes

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct LoadingOverlayDebug;

impl Render for LoadingOverlayDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("loading-overlay-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("LoadingOverlay Debug"))
            .child(Text::new("Overlays shown inline in bordered containers").muted(true))
            // Basic
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("With Message").weight(TextWeight::Bold))
                    .child(
                        div()
                            .relative()
                            .h(px(150.0))
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .overflow_hidden()
                            .child(LoadingOverlay::new("overlay-msg").message("Loading audio...")),
                    ),
            )
            // With subtitle
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("With Subtitle").weight(TextWeight::Bold))
                    .child(
                        div()
                            .relative()
                            .h(px(150.0))
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .overflow_hidden()
                            .child(
                                LoadingOverlay::new("overlay-sub")
                                    .message("Optimizing EQ")
                                    .subtitle("Running differential evolution..."),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("LoadingOverlay Debug")
            .size(600.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| LoadingOverlayDebug),
    );
}
