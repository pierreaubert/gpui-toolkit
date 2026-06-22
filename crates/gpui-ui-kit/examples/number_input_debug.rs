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

pub struct NumberInputDebug {
    volume: f64,
    frequency: f64,
    gain: f64,
    attack: f64,
    xs_value: f64,
    sm_value: f64,
    lg_value: f64,
    entity: Entity<Self>,
}

impl Render for NumberInputDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();

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
                            .value(self.volume)
                            .range(0.0, 100.0)
                            .step(1.0)
                            .label("Volume")
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.volume = value;
                                    });
                                }
                            }),
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
                            .value(self.frequency)
                            .range(20.0, 20000.0)
                            .step(10.0)
                            .unit("Hz")
                            .label("Frequency")
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.frequency = value;
                                    });
                                }
                            }),
                    )
                    .child(
                        NumberInput::new("num-gain")
                            .value(self.gain)
                            .range(-24.0, 24.0)
                            .step(0.5)
                            .decimals(1)
                            .unit("dB")
                            .label("Gain")
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.gain = value;
                                    });
                                }
                            }),
                    )
                    .child(
                        NumberInput::new("num-attack")
                            .value(self.attack)
                            .range(0.1, 100.0)
                            .step(0.1)
                            .decimals(1)
                            .unit("ms")
                            .label("Attack")
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.attack = value;
                                    });
                                }
                            }),
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
                            .value(self.xs_value)
                            .size(NumberInputSize::Xs)
                            .label("Extra Small")
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.xs_value = value;
                                    });
                                }
                            }),
                    )
                    .child(
                        NumberInput::new("num-sm")
                            .value(self.sm_value)
                            .size(NumberInputSize::Sm)
                            .label("Small")
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.sm_value = value;
                                    });
                                }
                            }),
                    )
                    .child(
                        NumberInput::new("num-lg")
                            .value(self.lg_value)
                            .size(NumberInputSize::Lg)
                            .label("Large")
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, _cx| {
                                        this.lg_value = value;
                                    });
                                }
                            }),
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
        |cx| {
            cx.new(|cx| NumberInputDebug {
                volume: 50.0,
                frequency: 1000.0,
                gain: 0.0,
                attack: 10.0,
                xs_value: 42.0,
                sm_value: 42.0,
                lg_value: 42.0,
                entity: cx.entity().clone(),
            })
        },
    );
}
