//! Text Debug Example
//!
//! Demonstrates Text, Heading, Link, and Code components:
//! - Text sizes and weights
//! - Heading levels
//! - Links and Code

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct TextDebug;

impl Render for TextDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("text-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("Text Debug"))
            // Sizes
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Text::new("Headings").weight(TextWeight::Bold))
                    .child(Heading::h1("Heading 1"))
                    .child(Heading::h2("Heading 2"))
                    .child(Heading::h3("Heading 3"))
                    .child(Heading::h4("Heading 4")),
            )
            // Text sizes
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Text::new("Text Sizes").weight(TextWeight::Bold))
                    .child(Text::new("Extra Small Text").size(TextSize::Xs))
                    .child(Text::new("Small Text").size(TextSize::Sm))
                    .child(Text::new("Medium Text (Default)").size(TextSize::Md))
                    .child(Text::new("Large Text").size(TextSize::Lg))
                    .child(Text::new("Extra Large Text").size(TextSize::Xl))
                    .child(Text::new("XXL Text").size(TextSize::Xxl)),
            )
            // Weights
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Text::new("Weights").weight(TextWeight::Bold))
                    .child(Text::new("Light weight").weight(TextWeight::Light))
                    .child(Text::new("Normal weight").weight(TextWeight::Normal))
                    .child(Text::new("Medium weight").weight(TextWeight::Medium))
                    .child(Text::new("Semibold weight").weight(TextWeight::Semibold))
                    .child(Text::new("Bold weight").weight(TextWeight::Bold)),
            )
            // Muted and colored
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Text::new("Styles").weight(TextWeight::Bold))
                    .child(Text::new("Normal text"))
                    .child(Text::new("Muted text").muted(true))
                    .child(Text::new("Accent colored").color(theme.accent))
                    .child(Text::new("Truncated text that is very long and should be cut off with an ellipsis at the end of the line if it exceeds the container width").truncate(true)),
            )
            // Code
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Code").weight(TextWeight::Bold))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Text::new("Inline:"))
                            .child(Code::new("cargo run --release")),
                    )
                    .child(Code::block("fn main() {\n    println!(\"Hello, SOTF!\");\n}")),
            )
            // Link
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Links").weight(TextWeight::Bold))
                    .child(Link::new("link-1", "Internal link"))
                    .child(Link::new("link-2", "External link").external(true)),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Text Debug")
            .size(600.0, 900.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| TextDebug),
    );
}
