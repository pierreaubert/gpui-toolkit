//! Vertical Slider Debug Example
//!
//! Interactive showcase for the VerticalSlider component:
//! - Different sizes (Sm, Md, Lg)
//! - Different scales (Linear, Logarithmic)
//! - Selected state
//! - Different value ranges and units
//! - Scroll wheel adjustment (Shift for fine control)
//! - Double-click to reset

use gpui::*;
use gpui_ui_kit::audio::vertical_slider::{VerticalSlider, VerticalSliderSize};
use gpui_ui_kit::i18n::{I18nExt, TranslationKey};
use gpui_ui_kit::scale::Scale;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

/// Number of sliders in the demo
const NUM_SLIDERS: usize = 11;

/// Demo state
pub struct VerticalSliderDebug {
    /// Threshold value (dB) - linear
    threshold_db: f64,
    /// Frequency value (Hz) - logarithmic
    frequency_hz: f64,
    /// Attack time (ms) - logarithmic
    attack_ms: f64,
    /// Ratio (compressor style) - linear
    ratio: f64,
    /// Gain value (dB) - linear, for Scale Types section
    gain_db: f64,
    /// Mix percentage - linear
    mix_pct: f64,
    /// Currently selected slider index
    selected_slider: Option<usize>,
    /// Entity reference
    entity: Entity<Self>,
    /// Focus handles for each slider (needed for keyboard navigation)
    focus_handles: Vec<gpui::FocusHandle>,
}

impl VerticalSliderDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        // Create focus handles for all sliders
        let focus_handles: Vec<_> = (0..NUM_SLIDERS).map(|_| cx.focus_handle()).collect();

        Self {
            threshold_db: -20.0,
            frequency_hz: 1000.0,
            attack_ms: 10.0,
            ratio: 4.0,
            gain_db: -12.0,
            mix_pct: 100.0,
            selected_slider: None,
            entity: cx.entity().clone(),
            focus_handles,
        }
    }

    /// Select a slider (focus is managed via the focus_handle passed to each slider)
    fn select_slider(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected_slider = Some(index);
        cx.notify(); // Trigger re-render
    }

    fn reset_threshold(&mut self, cx: &mut Context<Self>) {
        self.threshold_db = -20.0;
        cx.notify();
    }

    fn reset_frequency(&mut self, cx: &mut Context<Self>) {
        self.frequency_hz = 1000.0;
        cx.notify();
    }

    fn reset_attack(&mut self, cx: &mut Context<Self>) {
        self.attack_ms = 10.0;
        cx.notify();
    }

    fn reset_ratio(&mut self, cx: &mut Context<Self>) {
        self.ratio = 4.0;
        cx.notify();
    }

    fn reset_gain(&mut self, cx: &mut Context<Self>) {
        self.gain_db = -12.0;
        cx.notify();
    }

    fn reset_mix(&mut self, cx: &mut Context<Self>) {
        self.mix_pct = 100.0;
        cx.notify();
    }
}

impl Render for VerticalSliderDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = self.entity.clone();
        let theme = cx.theme();

        div()
            .id("vertical-slider-debug-root")
            .w_full()
            .h_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_6()
            .flex()
            .flex_col()
            .gap_6()
            // Header
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Heading::h1("Vertical Slider Component Debug"))
                    .child(
                        Text::new("Scroll to adjust values (Shift for fine control). Double-click to reset. Click to select.")
                            .muted(true),
                    ),
            )
            // i18n Status Bar
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_3()
                    .bg(theme.surface)
                    .rounded_lg()
                    .child(Text::new(format!("🌐 {}: ", cx.t(TranslationKey::MenuLanguage))).weight(TextWeight::Medium))
                    .child(Text::new(cx.language().native_name()).color(theme.accent))
                    .child(Text::new(" | "))
                    .child(Text::new("Vertical Sliders").color(theme.text_secondary)),
            )
            // Current values
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_3()
                    .bg(theme.surface)
                    .rounded_lg()
                    .child(Text::new(format!("Threshold: {:.1} dB", self.threshold_db)).size(TextSize::Sm))
                    .child(Text::new(format!("Freq: {:.0} Hz", self.frequency_hz)).size(TextSize::Sm))
                    .child(Text::new(format!("Attack: {:.1} ms", self.attack_ms)).size(TextSize::Sm))
                    .child(Text::new(format!("Ratio: {:.1}:1", self.ratio)).size(TextSize::Sm))
                    .child(Text::new(format!("Gain: {:.1} dB", self.gain_db)).size(TextSize::Sm))
                    .child(Text::new(format!("Mix: {:.0}%", self.mix_pct)).size(TextSize::Sm)),
            )
            // Size Comparison
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("Size Comparison (Sm, Md, Lg)")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Same parameter at different sizes").size(TextSize::Sm).muted(true))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            .align(StackAlign::End)
                            .child(
                                VerticalSlider::new("threshold-sm")
                                    .value(self.threshold_db)
                                    .min(-60.0)
                                    .max(0.0)
                                    .unit("dB")
                                    .label("Threshold")
                                    .shortcut_key('t')
                                    .size(VerticalSliderSize::Sm)
                                    .selected(self.selected_slider == Some(0))
                                    .focus_handle(self.focus_handles[0].clone())
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.threshold_db = val;
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.select_slider(0, cx);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.reset_threshold(cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                VerticalSlider::new("threshold-md")
                                    .value(self.threshold_db)
                                    .min(-60.0)
                                    .max(0.0)
                                    .unit("dB")
                                    .label("Threshold")
                                    .shortcut_key('t')
                                    .size(VerticalSliderSize::Md)
                                    .selected(self.selected_slider == Some(1))
                                    .focus_handle(self.focus_handles[1].clone())
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.threshold_db = val;
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.select_slider(1, cx);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.reset_threshold(cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                VerticalSlider::new("threshold-lg")
                                    .value(self.threshold_db)
                                    .min(-60.0)
                                    .max(0.0)
                                    .unit("dB")
                                    .label("Threshold")
                                    .shortcut_key('t')
                                    .size(VerticalSliderSize::Lg)
                                    .selected(self.selected_slider == Some(2))
                                    .focus_handle(self.focus_handles[2].clone())
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.threshold_db = val;
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.select_slider(2, cx);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.reset_threshold(cx);
                                            });
                                        }
                                    }),
                            ),
                    ),
            )
            // Scale Types
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("Scale Types (Linear vs Logarithmic)")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Frequency and Attack use log scale - equal visual distance = equal ratio").size(TextSize::Sm).muted(true))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            .align(StackAlign::End)
                            .child(
                                VerticalSlider::new("frequency")
                                    .value(self.frequency_hz)
                                    .min(20.0)
                                    .max(20000.0)
                                    .unit("Hz")
                                    .label("Frequency")
                                    .shortcut_key('f')
                                    .size(VerticalSliderSize::Lg)
                                    .scale(Scale::Logarithmic)
                                    .with_ticks()
                                    .selected(self.selected_slider == Some(3))
                                    .focus_handle(self.focus_handles[3].clone())
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.frequency_hz = val;
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.select_slider(3, cx);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.reset_frequency(cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                VerticalSlider::new("attack")
                                    .value(self.attack_ms)
                                    .min(0.1)
                                    .max(200.0)
                                    .unit("ms")
                                    .label("Attack")
                                    .shortcut_key('a')
                                    .size(VerticalSliderSize::Lg)
                                    .scale(Scale::Logarithmic)
                                    .with_ticks()
                                    .selected(self.selected_slider == Some(4))
                                    .focus_handle(self.focus_handles[4].clone())
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.attack_ms = val;
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.select_slider(4, cx);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.reset_attack(cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                VerticalSlider::new("ratio")
                                    .value(self.ratio)
                                    .min(1.0)
                                    .max(20.0)
                                    .unit(":1")
                                    .label("Ratio")
                                    .shortcut_key('r')
                                    .size(VerticalSliderSize::Lg)
                                    .scale(Scale::Linear)
                                    .with_ticks()
                                    .selected(self.selected_slider == Some(5))
                                    .focus_handle(self.focus_handles[5].clone())
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.ratio = val;
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.select_slider(5, cx);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.reset_ratio(cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                VerticalSlider::new("gain")
                                    .value(self.gain_db)
                                    .min(-60.0)
                                    .max(0.0)
                                    .unit("dB")
                                    .label("Gain")
                                    .shortcut_key('g')
                                    .size(VerticalSliderSize::Lg)
                                    .scale(Scale::Linear)
                                    .with_ticks()
                                    .selected(self.selected_slider == Some(6))
                                    .focus_handle(self.focus_handles[6].clone())
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.gain_db = val;
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.select_slider(6, cx);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.reset_gain(cx);
                                            });
                                        }
                                    }),
                            ),
                    ),
            )
            // Different Units
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("Different Units (%, :1, dB, ms)")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Units affect value display formatting").size(TextSize::Sm).muted(true))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            .align(StackAlign::End)
                            .child(
                                VerticalSlider::new("mix")
                                    .value(self.mix_pct)
                                    .min(0.0)
                                    .max(100.0)
                                    .unit("%")
                                    .label("Mix")
                                    .shortcut_key('m')
                                    .size(VerticalSliderSize::Md)
                                    .selected(self.selected_slider == Some(7))
                                    .focus_handle(self.focus_handles[7].clone())
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.mix_pct = val;
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.select_slider(7, cx);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.reset_mix(cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                VerticalSlider::new("ratio-display")
                                    .value(self.ratio)
                                    .min(1.0)
                                    .max(20.0)
                                    .unit(":1")
                                    .label("Ratio")
                                    .shortcut_key('r')
                                    .size(VerticalSliderSize::Md)
                                    .selected(self.selected_slider == Some(8))
                                    .focus_handle(self.focus_handles[8].clone())
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.ratio = val;
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.select_slider(8, cx);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.reset_ratio(cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                VerticalSlider::new("threshold-display")
                                    .value(self.threshold_db)
                                    .min(-60.0)
                                    .max(0.0)
                                    .unit("dB")
                                    .label("Threshold")
                                    .shortcut_key('t')
                                    .size(VerticalSliderSize::Md)
                                    .selected(self.selected_slider == Some(9))
                                    .focus_handle(self.focus_handles[9].clone())
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.threshold_db = val;
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.select_slider(9, cx);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.reset_threshold(cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                VerticalSlider::new("attack-display")
                                    .value(self.attack_ms)
                                    .min(0.1)
                                    .max(200.0)
                                    .unit("ms")
                                    .label("Attack")
                                    .shortcut_key('a')
                                    .size(VerticalSliderSize::Md)
                                    .scale(Scale::Logarithmic)
                                    .selected(self.selected_slider == Some(10))
                                    .focus_handle(self.focus_handles[10].clone())
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.attack_ms = val;
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.select_slider(10, cx);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.reset_attack(cx);
                                            });
                                        }
                                    }),
                            ),
                    ),
            )
            // Disabled state
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        Text::new("Disabled State")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Disabled sliders are non-interactive").size(TextSize::Sm).muted(true))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            .child(
                                VerticalSlider::new("disabled-slider")
                                    .value(-10.0)
                                    .min(-60.0)
                                    .max(0.0)
                                    .unit("dB")
                                    .label("Disabled")
                                    .size(VerticalSliderSize::Md)
                                    .disabled(true),
                            ),
                    ),
            )
            // Instructions
            .child(
                div()
                    .p_4()
                    .bg(theme.surface_hover)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Interactions:").weight(TextWeight::Bold))
                            .child(Text::new("- Click + Drag: Drag vertically to adjust value").size(TextSize::Sm))
                            .child(Text::new("- Scroll wheel: Adjust value (5% steps)").size(TextSize::Sm))
                            .child(Text::new("- Shift + Scroll: Fine adjustment (0.5% steps)").size(TextSize::Sm))
                            .child(Text::new("- Double-click: Reset to default").size(TextSize::Sm))
                            .child(Text::new("- Arrow Up/Right: Increase by 5%").size(TextSize::Sm))
                            .child(Text::new("- Arrow Down/Left: Decrease by 5%").size(TextSize::Sm))
                            .child(Text::new("- Page Up/Down: Adjust by 10%").size(TextSize::Sm))
                            .child(Text::new("- Home/End: Jump to min/max").size(TextSize::Sm))
                            .child(Text::new("- Escape: Reset to default").size(TextSize::Sm)),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Vertical Slider Debug")
            .size(900.0, 1100.0)
            .scrollable(true)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(VerticalSliderDebug::new),
    );
}
