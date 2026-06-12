//! Slider Debug Example
//!
//! Demonstrates the Slider component:
//! - Default with value display
//! - All sizes
//! - With label, disabled

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct SliderDebug;

impl Render for SliderDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("slider-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Slider Debug"))
            // Default
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Default").weight(TextWeight::Bold))
                    .child(
                        Slider::new("slider-default")
                            .value(0.65)
                            .show_value(true)
                            .label("Volume"),
                    ),
            )
            // With range
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Custom Range").weight(TextWeight::Bold))
                    .child(
                        Slider::new("slider-range")
                            .value(1000.0)
                            .range(20.0, 20000.0)
                            .show_value(true)
                            .label("Frequency"),
                    ),
            )
            // Sizes
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("Sizes").weight(TextWeight::Bold))
                    .child(
                        Slider::new("slider-sm")
                            .value(0.3)
                            .size(SliderSize::Sm)
                            .label("Small"),
                    )
                    .child(
                        Slider::new("slider-md")
                            .value(0.5)
                            .size(SliderSize::Md)
                            .label("Medium"),
                    )
                    .child(
                        Slider::new("slider-lg")
                            .value(0.7)
                            .size(SliderSize::Lg)
                            .label("Large"),
                    ),
            )
            // Disabled
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Disabled").weight(TextWeight::Bold))
                    .child(
                        Slider::new("slider-disabled")
                            .value(0.5)
                            .disabled(true)
                            .label("Locked"),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Slider Debug")
            .size(600.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| SliderDebug),
    );
}
