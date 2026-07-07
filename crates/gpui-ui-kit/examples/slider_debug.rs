//! Slider Debug Example
//!
//! Demonstrates the Slider component:
//! - Default with value display
//! - All sizes
//! - With label, disabled

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct SliderDebug {
    volume: f32,
    frequency: f32,
    small: f32,
    medium: f32,
    large: f32,
    entity: Entity<Self>,
}

impl SliderDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            volume: 65.0,
            frequency: 1000.0,
            small: 30.0,
            medium: 50.0,
            large: 70.0,
            entity: cx.entity().clone(),
        }
    }
}

impl Render for SliderDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();

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
                            .range(0.0, 100.0)
                            .value(self.volume)
                            .step(1.0)
                            .show_value(true)
                            .label("Volume")
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.volume = value;
                                        cx.notify();
                                    });
                                }
                            }),
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
                            .range(20.0, 20000.0)
                            .value(self.frequency)
                            .step(10.0)
                            .show_value(true)
                            .label("Frequency")
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.frequency = value;
                                        cx.notify();
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
                        Slider::new("slider-sm")
                            .range(0.0, 100.0)
                            .value(self.small)
                            .step(1.0)
                            .size(SliderSize::Sm)
                            .label("Small")
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.small = value;
                                        cx.notify();
                                    });
                                }
                            }),
                    )
                    .child(
                        Slider::new("slider-md")
                            .range(0.0, 100.0)
                            .value(self.medium)
                            .step(1.0)
                            .size(SliderSize::Md)
                            .label("Medium")
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.medium = value;
                                        cx.notify();
                                    });
                                }
                            }),
                    )
                    .child(
                        Slider::new("slider-lg")
                            .range(0.0, 100.0)
                            .value(self.large)
                            .step(1.0)
                            .size(SliderSize::Lg)
                            .label("Large")
                            .on_change({
                                let entity = entity.clone();
                                move |value, _window, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.large = value;
                                        cx.notify();
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
                        Slider::new("slider-disabled")
                            .range(0.0, 100.0)
                            .value(50.0)
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
        |cx| cx.new(SliderDebug::new),
    );
}
