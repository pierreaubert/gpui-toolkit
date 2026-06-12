//! NumberInput Debug Example
//!
//! Demonstrates the NumberInput component:
//! - Basic with range and step
//! - With units (Hz, dB, ms)
//! - Different sizes

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct NumberInputDebug;

impl Render for NumberInputDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("number-input-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("NumberInput Debug"))
            // Basic
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Basic").weight(TextWeight::Bold))
                    .child(
                        NumberInput::new("num-basic")
                            .value(50.0)
                            .range(0.0, 100.0)
                            .step(1.0)
                            .label("Volume"),
                    ),
            )
            // With units
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Text::new("With Units").weight(TextWeight::Bold))
                    .child(
                        NumberInput::new("num-freq")
                            .value(1000.0)
                            .range(20.0, 20000.0)
                            .step(10.0)
                            .unit("Hz")
                            .label("Frequency"),
                    )
                    .child(
                        NumberInput::new("num-gain")
                            .value(0.0)
                            .range(-24.0, 24.0)
                            .step(0.5)
                            .decimals(1)
                            .unit("dB")
                            .label("Gain"),
                    )
                    .child(
                        NumberInput::new("num-attack")
                            .value(10.0)
                            .range(0.1, 100.0)
                            .step(0.1)
                            .decimals(1)
                            .unit("ms")
                            .label("Attack"),
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
                        NumberInput::new("num-xs")
                            .value(42.0)
                            .size(NumberInputSize::Xs)
                            .label("Extra Small"),
                    )
                    .child(
                        NumberInput::new("num-sm")
                            .value(42.0)
                            .size(NumberInputSize::Sm)
                            .label("Small"),
                    )
                    .child(
                        NumberInput::new("num-lg")
                            .value(42.0)
                            .size(NumberInputSize::Lg)
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
                        NumberInput::new("num-disabled")
                            .value(44100.0)
                            .unit("Hz")
                            .label("Sample Rate")
                            .disabled(true),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("NumberInput Debug")
            .size(500.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| NumberInputDebug),
    );
}
