//! Color picker component for theme editing
//!
//! Provides an interactive color picker with:
//! - RGB/HSL sliders (clickable bars)
//! - Color preview
//! - RGBA/HSL display

use crate::color::Color;
use crate::{
    Button, ButtonSize, ButtonVariant, HStack, StackSpacing, Text, TextSize, TextWeight, VStack,
};
use gpui::prelude::*;
use gpui::*;

/// Color picker mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::upper_case_acronyms)]
pub enum ColorPickerMode {
    #[default]
    RGB,
    HSL,
}

/// Standalone color picker view for use in dialogs
pub struct ColorPickerView {
    color: Color,
    original_color: Color,
    mode: ColorPickerMode,
    label: SharedString,
}

impl ColorPickerView {
    pub fn new(label: impl Into<SharedString>, color: Color) -> Self {
        Self {
            color,
            original_color: color,
            mode: ColorPickerMode::RGB,
            label: label.into(),
        }
    }

    /// Get current color
    pub fn color(&self) -> Color {
        self.color
    }

    /// Set color
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
        self.original_color = color;
    }

    fn update_red(&mut self, value: u8, cx: &mut Context<Self>) {
        self.color.r = value;
        cx.notify();
    }

    fn update_green(&mut self, value: u8, cx: &mut Context<Self>) {
        self.color.g = value;
        cx.notify();
    }

    fn update_blue(&mut self, value: u8, cx: &mut Context<Self>) {
        self.color.b = value;
        cx.notify();
    }

    fn update_alpha(&mut self, value: u8, cx: &mut Context<Self>) {
        self.color.a = value;
        cx.notify();
    }

    fn update_hue(&mut self, value: f32, cx: &mut Context<Self>) {
        let (_, s, l) = self.color.to_hsl();
        self.color = Color::from_hsl(value, s, l).with_alpha(self.color.a as f32 / 255.0);
        cx.notify();
    }

    fn update_saturation(&mut self, value: f32, cx: &mut Context<Self>) {
        let (h, _, l) = self.color.to_hsl();
        self.color = Color::from_hsl(h, value, l).with_alpha(self.color.a as f32 / 255.0);
        cx.notify();
    }

    fn update_lightness(&mut self, value: f32, cx: &mut Context<Self>) {
        let (h, s, _) = self.color.to_hsl();
        self.color = Color::from_hsl(h, s, value).with_alpha(self.color.a as f32 / 255.0);
        cx.notify();
    }

    fn toggle_mode(&mut self, cx: &mut Context<Self>) {
        self.mode = match self.mode {
            ColorPickerMode::RGB => ColorPickerMode::HSL,
            ColorPickerMode::HSL => ColorPickerMode::RGB,
        };
        cx.notify();
    }

    fn reset_color(&mut self, cx: &mut Context<Self>) {
        self.color = self.original_color;
        cx.notify();
    }

    /// Render a slider with full mouse interaction support:
    /// - Click and drag to set value
    /// - Scroll wheel to adjust value (shift for fine control)
    /// - Double-click to reset to default
    fn render_slider(
        &self,
        label: &'static str,
        value: f32,
        max: f32,
        default_value: f32,
        fill_color: Option<Rgba>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mode = self.mode;
        let ratio = value / max;
        let bar_width = 200.0;

        // Track colors
        let track_bg = Rgba {
            r: 0.2,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        };
        let fill_bg = fill_color.unwrap_or(Rgba {
            r: 0.0,
            g: 0.48,
            b: 0.8,
            a: 1.0,
        });

        // Helper to calculate new value from x position
        let calc_value = move |x: f32| -> f32 { (x / bar_width).clamp(0.0, 1.0) * max };

        HStack::new()
            .spacing(StackSpacing::Sm)
            .child(
                div()
                    .w(px(24.0))
                    .child(Text::new(label).size(TextSize::Sm).weight(TextWeight::Bold)),
            )
            .child(
                div()
                    .id(SharedString::from(format!("slider-{}", label)))
                    .w(px(bar_width))
                    .h(px(20.0))
                    .bg(track_bg)
                    .rounded(px(4.0))
                    .relative()
                    .cursor_ew_resize()
                    // Fill indicator
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .h_full()
                            .w(px(bar_width * ratio))
                            .bg(fill_bg)
                            .rounded(px(4.0)),
                    )
                    // Thumb indicator
                    .child(
                        div()
                            .absolute()
                            .top(px(2.0))
                            .left(px((bar_width * ratio - 8.0).max(0.0).min(bar_width - 16.0)))
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded_full()
                            .bg(Rgba {
                                r: 1.0,
                                g: 1.0,
                                b: 1.0,
                                a: 1.0,
                            })
                            .border_2()
                            .border_color(fill_bg)
                            .shadow_sm(),
                    )
                    // Mouse down - set value on click
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                            let new_val = calc_value(event.position.x.into());
                            Self::apply_slider_value(this, mode, label, new_val, cx);
                        }),
                    )
                    // Mouse move while pressed - drag to change value
                    .on_mouse_move(
                        cx.listener(move |this, event: &MouseMoveEvent, _window, cx| {
                            if event.pressed_button == Some(MouseButton::Left) {
                                let new_val = calc_value(event.position.x.into());
                                Self::apply_slider_value(this, mode, label, new_val, cx);
                            }
                        }),
                    )
                    // Double-click to reset
                    .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
                        if event.click_count() == 2 {
                            Self::apply_slider_value(this, mode, label, default_value, cx);
                        }
                    }))
                    // Scroll wheel - adjust value (shift for fine control)
                    .on_scroll_wheel(cx.listener(
                        move |this, event: &ScrollWheelEvent, _window, cx| {
                            // Get scroll delta
                            let delta_y: f32 = match event.delta {
                                gpui::ScrollDelta::Pixels(point) => point.y.into(),
                                gpui::ScrollDelta::Lines(point) => point.y * 20.0,
                            };

                            if delta_y.abs() < 0.0001 {
                                return;
                            }

                            // Scroll up (negative delta) = increase, scroll down = decrease
                            let direction = if delta_y < 0.0 { 1.0 } else { -1.0 };

                            // Step size: 5% normally, 0.5% with shift
                            let step_percent = if event.modifiers.shift { 0.005 } else { 0.05 };
                            let step = max * step_percent;

                            let new_val = (value + direction * step).clamp(0.0, max);
                            Self::apply_slider_value(this, mode, label, new_val, cx);
                        },
                    )),
            )
            .child(
                div().w(px(50.0)).child(
                    Text::new(SharedString::from(format!("{:.0}", value))).size(TextSize::Sm),
                ),
            )
            .build()
    }

    /// Apply a slider value to the appropriate color component
    fn apply_slider_value(
        this: &mut Self,
        mode: ColorPickerMode,
        label: &'static str,
        new_val: f32,
        cx: &mut Context<Self>,
    ) {
        match (mode, label) {
            (ColorPickerMode::RGB, "R") => this.update_red(new_val as u8, cx),
            (ColorPickerMode::RGB, "G") => this.update_green(new_val as u8, cx),
            (ColorPickerMode::RGB, "B") => this.update_blue(new_val as u8, cx),
            (ColorPickerMode::HSL, "H") => this.update_hue(new_val / 360.0, cx),
            (ColorPickerMode::HSL, "S") => this.update_saturation(new_val / 100.0, cx),
            (ColorPickerMode::HSL, "L") => this.update_lightness(new_val / 100.0, cx),
            (_, "A") => this.update_alpha(new_val as u8, cx),
            _ => {}
        }
    }
}

impl Render for ColorPickerView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let color = self.color;
        let original = self.original_color;
        let hex_string = color.to_hex_string();
        let mode = self.mode;
        let (h, s, l) = color.to_hsl();
        let (orig_h, orig_s, orig_l) = original.to_hsl();

        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .min_w(px(400.0))
            // Header
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        Text::new(self.label.clone())
                            .size(TextSize::Lg)
                            .weight(TextWeight::Bold),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new(
                            "mode-toggle",
                            if mode == ColorPickerMode::RGB {
                                "Switch to HSL"
                            } else {
                                "Switch to RGB"
                            },
                        )
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Sm)
                        .build()
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _window, cx| {
                                this.toggle_mode(cx);
                            },
                        )),
                    )
                    .build(),
            )
            // Color comparison
            .child(
                HStack::new()
                    .spacing(StackSpacing::Lg)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Xs)
                            .child(Text::new("Original").size(TextSize::Xs))
                            .child(
                                div()
                                    .w(px(80.0))
                                    .h(px(60.0))
                                    .rounded_lg()
                                    .bg(original.to_rgba())
                                    .border_1()
                                    .border_color(Rgba {
                                        r: 0.4,
                                        g: 0.4,
                                        b: 0.4,
                                        a: 1.0,
                                    }),
                            )
                            .build(),
                    )
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Xs)
                            .child(Text::new("New").size(TextSize::Xs))
                            .child(
                                div()
                                    .w(px(80.0))
                                    .h(px(60.0))
                                    .rounded_lg()
                                    .bg(color.to_rgba())
                                    .border_1()
                                    .border_color(Rgba {
                                        r: 0.4,
                                        g: 0.4,
                                        b: 0.4,
                                        a: 1.0,
                                    }),
                            )
                            .build(),
                    )
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Xs)
                            .child(
                                Text::new(SharedString::from(format!("Hex: {}", hex_string)))
                                    .size(TextSize::Sm)
                                    .weight(TextWeight::Bold),
                            )
                            .child(
                                Text::new(SharedString::from(format!(
                                    "RGBA: {}, {}, {}, {}",
                                    color.r, color.g, color.b, color.a
                                )))
                                .size(TextSize::Sm),
                            )
                            .child(
                                Text::new(SharedString::from(format!(
                                    "HSL: {:.0}°, {:.0}%, {:.0}%",
                                    h * 360.0,
                                    s * 100.0,
                                    l * 100.0
                                )))
                                .size(TextSize::Sm),
                            )
                            .build(),
                    )
                    .build(),
            )
            // Sliders
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .when(mode == ColorPickerMode::RGB, |el| {
                        el.child(self.render_slider(
                            "R",
                            color.r as f32,
                            255.0,
                            original.r as f32,
                            Some(Rgba {
                                r: 1.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            cx,
                        ))
                        .child(self.render_slider(
                            "G",
                            color.g as f32,
                            255.0,
                            original.g as f32,
                            Some(Rgba {
                                r: 0.0,
                                g: 1.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            cx,
                        ))
                        .child(self.render_slider(
                            "B",
                            color.b as f32,
                            255.0,
                            original.b as f32,
                            Some(Rgba {
                                r: 0.0,
                                g: 0.0,
                                b: 1.0,
                                a: 1.0,
                            }),
                            cx,
                        ))
                    })
                    .when(mode == ColorPickerMode::HSL, |el| {
                        el.child(self.render_slider(
                            "H",
                            h * 360.0,
                            360.0,
                            orig_h * 360.0,
                            Some(Rgba {
                                r: 1.0,
                                g: 0.0,
                                b: 0.5,
                                a: 1.0,
                            }),
                            cx,
                        ))
                        .child(self.render_slider(
                            "S",
                            s * 100.0,
                            100.0,
                            orig_s * 100.0,
                            Some(Rgba {
                                r: 0.0,
                                g: 0.7,
                                b: 1.0,
                                a: 1.0,
                            }),
                            cx,
                        ))
                        .child(self.render_slider(
                            "L",
                            l * 100.0,
                            100.0,
                            orig_l * 100.0,
                            Some(Rgba {
                                r: 0.5,
                                g: 0.5,
                                b: 0.5,
                                a: 1.0,
                            }),
                            cx,
                        ))
                    })
                    .child(self.render_slider(
                        "A",
                        color.a as f32,
                        255.0,
                        original.a as f32,
                        Some(Rgba {
                            r: 0.8,
                            g: 0.8,
                            b: 0.8,
                            a: 1.0,
                        }),
                        cx,
                    ))
                    .build(),
            )
            // Reset button
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        Button::new("reset-color", "Reset to Original")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Sm)
                            .build()
                            .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                this.reset_color(cx);
                            })),
                    )
                    .build(),
            )
    }
}
