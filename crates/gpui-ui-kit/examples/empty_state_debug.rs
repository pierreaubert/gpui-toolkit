//! Empty State Debug Example
//!
//! Demonstrates the EmptyState component:
//! - Basic empty state with title
//! - With description
//! - With icon

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct EmptyStateDebug;

impl Render for EmptyStateDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("empty-state-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Empty State Debug"))
            // Basic
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Basic").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_4()
                            .child(EmptyState::new("No items found")),
                    ),
            )
            // With description
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("With Description").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_4()
                            .child(
                                EmptyState::new("No results")
                                    .description("Try adjusting your search or filters"),
                            ),
                    ),
            )
            // With icon
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("With Icon").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_4()
                            .child(
                                EmptyState::new("No tracks in library")
                                    .description("Add some music to get started")
                                    .icon("?"),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Empty State Debug")
            .size(600.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| EmptyStateDebug),
    );
}
