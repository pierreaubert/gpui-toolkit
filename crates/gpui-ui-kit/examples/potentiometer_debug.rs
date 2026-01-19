//! Potentiometer Debug Example
//!
//! Interactive showcase for the Potentiometer component:
//! - Different sizes (Sm, Md, Lg)
//! - Different scales (Linear, Logarithmic)
//! - Selected state
//! - Different value ranges and units
//! - Scroll wheel adjustment
//! - Double-click to reset

use gpui::*;
use gpui_ui_kit::audio::potentiometer::{Potentiometer, PotentiometerScale, PotentiometerSize};
use gpui_ui_kit::i18n::{I18nExt, TranslationKey};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

/// Demo state
pub struct PotentiometerDebug {
    /// Gain value (dB)
    gain_db: f64,
    /// Frequency value (Hz) - logarithmic
    frequency_hz: f64,
    /// Q factor (linear)
    q_factor: f64,
    /// Mix percentage
    mix_pct: f64,
    /// Ratio (compressor style)
    ratio: f64,
    /// Currently selected knob index
    selected_knob: Option<usize>,
    /// Entity reference
    entity: Entity<Self>,
}

impl PotentiometerDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            gain_db: 0.0,
            frequency_hz: 1000.0,
            q_factor: 1.0,
            mix_pct: 100.0,
            ratio: 4.0,
            selected_knob: None,
            entity: cx.entity().clone(),
        }
    }

    fn reset_gain(&mut self) {
        self.gain_db = 0.0;
    }

    fn reset_frequency(&mut self) {
        self.frequency_hz = 1000.0;
    }

    fn reset_q(&mut self) {
        self.q_factor = 1.0;
    }

    fn reset_mix(&mut self) {
        self.mix_pct = 100.0;
    }

    fn reset_ratio(&mut self) {
        self.ratio = 4.0;
    }
}

impl Render for PotentiometerDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = self.entity.clone();
        let theme = cx.theme();

        div()
            .id("potentiometer-debug-root")
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
                    .child(Heading::h1("Potentiometer Component Debug"))
                    .child(
                        Text::new("Scroll to adjust values. Double-click to reset. Click to select. Arrow keys adjust when selected.")
                            .muted(true),
                    ),
            )
            // i18n Status Bar - demonstrates language switching works
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
                    .child(Text::new(cx.t(TranslationKey::SectionPotentiometers)).color(theme.text_secondary)),
            )
            // Current values
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_3()
                    .bg(theme.surface)
                    .rounded_lg()
                    .child(Text::new(format!("Gain: {:.1} dB", self.gain_db)).size(TextSize::Sm))
                    .child(Text::new(format!("Freq: {:.0} Hz", self.frequency_hz)).size(TextSize::Sm))
                    .child(Text::new(format!("Q: {:.2}", self.q_factor)).size(TextSize::Sm))
                    .child(Text::new(format!("Mix: {:.0}%", self.mix_pct)).size(TextSize::Sm))
                    .child(Text::new(format!("Ratio: {:.1}:1", self.ratio)).size(TextSize::Sm)),
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
                                Potentiometer::new("gain-sm")
                                    .value(self.gain_db)
                                    .min(-12.0)
                                    .max(12.0)
                                    .unit("dB")
                                    .label("Gain")
                                    .shortcut_key('g')
                                    .size(PotentiometerSize::Sm)
                                    .selected(self.selected_knob == Some(0))
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.gain_db = val;
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.selected_knob = Some(0);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.reset_gain();
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Potentiometer::new("gain-md")
                                    .value(self.gain_db)
                                    .min(-12.0)
                                    .max(12.0)
                                    .unit("dB")
                                    .label("Gain")
                                    .shortcut_key('g')
                                    .size(PotentiometerSize::Md)
                                    .selected(self.selected_knob == Some(1))
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.gain_db = val;
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.selected_knob = Some(1);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.reset_gain();
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Potentiometer::new("gain-lg")
                                    .value(self.gain_db)
                                    .min(-12.0)
                                    .max(12.0)
                                    .unit("dB")
                                    .label("Gain")
                                    .shortcut_key('g')
                                    .size(PotentiometerSize::Lg)
                                    .selected(self.selected_knob == Some(2))
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.gain_db = val;
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.selected_knob = Some(2);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.reset_gain();
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
                    .child(Text::new("Frequency uses log scale - equal visual distance = equal ratio").size(TextSize::Sm).muted(true))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            .child(
                                Potentiometer::new("frequency")
                                    .value(self.frequency_hz)
                                    .min(20.0)
                                    .max(20000.0)
                                    .unit("Hz")
                                    .label("Frequency")
                                    .shortcut_key('f')
                                    .size(PotentiometerSize::Lg)
                                    .scale(PotentiometerScale::Logarithmic)
                                    .selected(self.selected_knob == Some(3))
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.frequency_hz = val;
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.selected_knob = Some(3);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.reset_frequency();
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Potentiometer::new("q-factor")
                                    .value(self.q_factor)
                                    .min(0.1)
                                    .max(10.0)
                                    .label("Q Factor")
                                    .shortcut_key('q')
                                    .size(PotentiometerSize::Lg)
                                    .scale(PotentiometerScale::Linear)
                                    .selected(self.selected_knob == Some(4))
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.q_factor = val;
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.selected_knob = Some(4);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.reset_q();
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
                        Text::new("Different Units (%, :1, dB)")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Units affect value display formatting").size(TextSize::Sm).muted(true))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            .child(
                                Potentiometer::new("mix")
                                    .value(self.mix_pct)
                                    .min(0.0)
                                    .max(100.0)
                                    .unit("%")
                                    .label("Mix")
                                    .shortcut_key('m')
                                    .size(PotentiometerSize::Md)
                                    .selected(self.selected_knob == Some(5))
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.mix_pct = val;
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.selected_knob = Some(5);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.reset_mix();
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Potentiometer::new("ratio")
                                    .value(self.ratio)
                                    .min(1.0)
                                    .max(20.0)
                                    .unit(":1")
                                    .label("Ratio")
                                    .shortcut_key('r')
                                    .size(PotentiometerSize::Md)
                                    .selected(self.selected_knob == Some(6))
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.ratio = val;
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.selected_knob = Some(6);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.reset_ratio();
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Potentiometer::new("gain-display")
                                    .value(self.gain_db)
                                    .min(-12.0)
                                    .max(12.0)
                                    .unit("dB")
                                    .label("Gain")
                                    .shortcut_key('g')
                                    .size(PotentiometerSize::Md)
                                    .selected(self.selected_knob == Some(7))
                                    .on_change({
                                        let entity = entity.clone();
                                        move |val, _w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.gain_db = val;
                                            });
                                        }
                                    })
                                    .on_select({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.selected_knob = Some(7);
                                            });
                                        }
                                    })
                                    .on_reset({
                                        let entity = entity.clone();
                                        move |_w, cx| {
                                            entity.update(cx, |this, _| {
                                                this.reset_gain();
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
                    .child(Text::new("Disabled knobs are non-interactive").size(TextSize::Sm).muted(true))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            .child(
                                Potentiometer::new("disabled-knob")
                                    .value(5.0)
                                    .min(0.0)
                                    .max(10.0)
                                    .unit("dB")
                                    .label("Disabled")
                                    .size(PotentiometerSize::Md)
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
                            .child(Text::new("- Scroll wheel: Adjust value (Shift for fine control)").size(TextSize::Sm))
                            .child(Text::new("- Click: Select and increment by 10%").size(TextSize::Sm))
                            .child(Text::new("- Double-click: Reset to default").size(TextSize::Sm))
                            .child(Text::new("- Arrow keys (when selected): Adjust by 5%").size(TextSize::Sm))
                            .child(Text::new("- Escape (when selected): Reset to default").size(TextSize::Sm)),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Potentiometer Debug")
            .size(900.0, 1000.0)
            .scrollable(true)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(PotentiometerDebug::new),
    );
}
