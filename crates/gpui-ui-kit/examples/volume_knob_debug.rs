//! VolumeKnob Debug Example
//!
//! Interactive showcase for the VolumeKnob component:
//! - Different sizes
//! - Different values and fill levels
//! - Muted state
//! - Custom colors
//! - Scroll wheel adjustment
//! - Double-click to mute

use gpui::*;
use gpui_ui_kit::audio::volume_knob::VolumeKnob;
use gpui_ui_kit::i18n::{I18nExt, TranslationKey};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

/// Demo state
pub struct VolumeKnobDebug {
    /// Main volume value (0.0 - 1.0)
    volume: f32,
    /// Muted state
    muted: bool,
    /// Second volume (for comparison)
    volume2: f32,
    /// Second muted state
    muted2: bool,
    /// Third volume
    volume3: f32,
    /// Third muted state
    muted3: bool,

    // Focus handles
    focus_sm: FocusHandle,
    focus_md: FocusHandle,
    focus_lg: FocusHandle,
    focus_active: FocusHandle,
    focus_muted: FocusHandle,

    /// Entity reference
    entity: Entity<Self>,
}

impl VolumeKnobDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            volume: 0.75,
            muted: false,
            volume2: 0.5,
            muted2: false,
            volume3: 1.0,
            muted3: true,
            focus_sm: cx.focus_handle(),
            focus_md: cx.focus_handle(),
            focus_lg: cx.focus_handle(),
            focus_active: cx.focus_handle(),
            focus_muted: cx.focus_handle(),
            entity: cx.entity().clone(),
        }
    }
}

impl Render for VolumeKnobDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = self.entity.clone();
        let theme = cx.theme();

        div()
            .id("volume-knob-debug-root")
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
                    .child(Heading::h1("VolumeKnob Component Debug"))
                    .child(
                        Text::new("Scroll to adjust volume. Double-click to toggle mute. Arrow keys when focused.")
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
                    .child(Text::new(cx.t(TranslationKey::SectionFormControls)).color(theme.text_secondary)),
            )
            // Current values
            .child(
                div()
                    .flex()
                    .gap_4()
                    .p_3()
                    .bg(theme.surface)
                    .rounded_lg()
                    .child(Text::new(format!("Vol 1: {:.0}% {}", self.volume * 100.0, if self.muted { "(Muted)" } else { "" })).size(TextSize::Sm))
                    .child(Text::new(format!("Vol 2: {:.0}% {}", self.volume2 * 100.0, if self.muted2 { "(Muted)" } else { "" })).size(TextSize::Sm))
                    .child(Text::new(format!("Vol 3: {:.0}% {}", self.volume3 * 100.0, if self.muted3 { "(Muted)" } else { "" })).size(TextSize::Sm)),
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
                        Text::new("Size Comparison")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Different sizes for different UI contexts").size(TextSize::Sm).muted(true))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            .align(StackAlign::End)
                            // Small
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .align(StackAlign::Center)
                                    .child(Text::new("Small (30px)").size(TextSize::Xs).muted(true))
                                    .child(
                                        VolumeKnob::new()
                                            .id("vol-sm")
                                            .value(self.volume)
                                            .muted(self.muted)
                                            .size(px(30.0))
                                            .label(format!("{:.0}", self.volume * 100.0))
                                            .accent_color(theme.accent)
                                            .bg_color(theme.muted)
                                            .text_color(theme.text_primary)
                                            .muted_color(theme.text_muted)
                                            .focus_handle(self.focus_sm.clone())
                                            .on_change({
                                                let entity = entity.clone();
                                                move |val, _w, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.volume = val;
                                                    });
                                                }
                                            })
                                            .on_mute_toggle({
                                                let entity = entity.clone();
                                                move |muted, _w, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.muted = muted;
                                                    });
                                                }
                                            }),
                                    ),
                            )
                            // Medium
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .align(StackAlign::Center)
                                    .child(Text::new("Medium (50px)").size(TextSize::Xs).muted(true))
                                    .child(
                                        VolumeKnob::new()
                                            .id("vol-md")
                                            .value(self.volume)
                                            .muted(self.muted)
                                            .size(px(50.0))
                                            .label(format!("{:.0}", self.volume * 100.0))
                                            .accent_color(theme.accent)
                                            .bg_color(theme.muted)
                                            .text_color(theme.text_primary)
                                            .muted_color(theme.text_muted)
                                            .focus_handle(self.focus_md.clone())
                                            .on_change({
                                                let entity = entity.clone();
                                                move |val, _w, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.volume = val;
                                                    });
                                                }
                                            })
                                            .on_mute_toggle({
                                                let entity = entity.clone();
                                                move |muted, _w, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.muted = muted;
                                                    });
                                                }
                                            }),
                                    ),
                            )
                            // Large
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .align(StackAlign::Center)
                                    .child(Text::new("Large (80px)").size(TextSize::Xs).muted(true))
                                    .child(
                                        VolumeKnob::new()
                                            .id("vol-lg")
                                            .value(self.volume)
                                            .muted(self.muted)
                                            .size(px(80.0))
                                            .label(format!("{:.0}%", self.volume * 100.0))
                                            .accent_color(theme.accent)
                                            .bg_color(theme.muted)
                                            .text_color(theme.text_primary)
                                            .muted_color(theme.text_muted)
                                            .focus_handle(self.focus_lg.clone())
                                            .on_change({
                                                let entity = entity.clone();
                                                move |val, _w, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.volume = val;
                                                    });
                                                }
                                            })
                                            .on_mute_toggle({
                                                let entity = entity.clone();
                                                move |muted, _w, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.muted = muted;
                                                    });
                                                }
                                            }),
                                    ),
                            ),
                    ),
            )
            // Fill Level Examples
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
                        Text::new("Fill Level Examples")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Different volume levels showing fill animation").size(TextSize::Sm).muted(true))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            // 0%
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .align(StackAlign::Center)
                                    .child(Text::new("0%").size(TextSize::Xs).muted(true))
                                    .child(
                                        VolumeKnob::new()
                                            .id("vol-0")
                                            .value(0.0)
                                            .size(px(50.0))
                                            .label("0")
                                            .accent_color(theme.accent)
                                            .bg_color(theme.muted)
                                            .text_color(theme.text_primary),
                                    ),
                            )
                            // 25%
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .align(StackAlign::Center)
                                    .child(Text::new("25%").size(TextSize::Xs).muted(true))
                                    .child(
                                        VolumeKnob::new()
                                            .id("vol-25")
                                            .value(0.25)
                                            .size(px(50.0))
                                            .label("25")
                                            .accent_color(theme.accent)
                                            .bg_color(theme.muted)
                                            .text_color(theme.text_primary),
                                    ),
                            )
                            // 50%
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .align(StackAlign::Center)
                                    .child(Text::new("50%").size(TextSize::Xs).muted(true))
                                    .child(
                                        VolumeKnob::new()
                                            .id("vol-50")
                                            .value(0.5)
                                            .size(px(50.0))
                                            .label("50")
                                            .accent_color(theme.accent)
                                            .bg_color(theme.muted)
                                            .text_color(theme.text_primary),
                                    ),
                            )
                            // 75%
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .align(StackAlign::Center)
                                    .child(Text::new("75%").size(TextSize::Xs).muted(true))
                                    .child(
                                        VolumeKnob::new()
                                            .id("vol-75")
                                            .value(0.75)
                                            .size(px(50.0))
                                            .label("75")
                                            .accent_color(theme.accent)
                                            .bg_color(theme.muted)
                                            .text_color(theme.text_primary),
                                    ),
                            )
                            // 100%
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .align(StackAlign::Center)
                                    .child(Text::new("100%").size(TextSize::Xs).muted(true))
                                    .child(
                                        VolumeKnob::new()
                                            .id("vol-100")
                                            .value(1.0)
                                            .size(px(50.0))
                                            .label("100")
                                            .accent_color(theme.accent)
                                            .bg_color(theme.muted)
                                            .text_color(theme.text_primary),
                                    ),
                            ),
                    ),
            )
            // Muted vs Active
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
                        Text::new("Muted vs Active State")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("Double-click to toggle mute. Muted shows empty fill.").size(TextSize::Sm).muted(true))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            // Active
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .align(StackAlign::Center)
                                    .child(Text::new("Active").size(TextSize::Sm))
                                    .child(
                                        VolumeKnob::new()
                                            .id("vol-active")
                                            .value(self.volume2)
                                            .muted(self.muted2)
                                            .size(px(60.0))
                                            .label(if self.muted2 { "M".into() } else { format!("{:.0}", self.volume2 * 100.0) })
                                            .accent_color(theme.success)
                                            .bg_color(theme.muted)
                                            .text_color(theme.text_primary)
                                            .muted_color(theme.text_muted)
                                            .focus_handle(self.focus_active.clone())
                                            .on_change({
                                                let entity = entity.clone();
                                                move |val, _w, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.volume2 = val;
                                                    });
                                                }
                                            })
                                            .on_mute_toggle({
                                                let entity = entity.clone();
                                                move |muted, _w, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.muted2 = muted;
                                                    });
                                                }
                                            }),
                                    ),
                            )
                            // Muted
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .align(StackAlign::Center)
                                    .child(Text::new("Muted").size(TextSize::Sm))
                                    .child(
                                        VolumeKnob::new()
                                            .id("vol-muted-demo")
                                            .value(self.volume3)
                                            .muted(self.muted3)
                                            .size(px(60.0))
                                            .label(if self.muted3 { "M".into() } else { format!("{:.0}", self.volume3 * 100.0) })
                                            .accent_color(theme.success)
                                            .bg_color(theme.muted)
                                            .text_color(theme.text_primary)
                                            .muted_color(theme.error)
                                            .focus_handle(self.focus_muted.clone())
                                            .on_change({
                                                let entity = entity.clone();
                                                move |val, _w, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.volume3 = val;
                                                    });
                                                }
                                            })
                                            .on_mute_toggle({
                                                let entity = entity.clone();
                                                move |muted, _w, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.muted3 = muted;
                                                    });
                                                }
                                            }),
                                    ),
                            ),
                    ),
            )
            // Custom Colors
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
                        Text::new("Custom Colors")
                            .weight(TextWeight::Bold)
                            .size(TextSize::Md),
                    )
                    .child(Text::new("VolumeKnob supports custom accent, background, and text colors").size(TextSize::Sm).muted(true))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Xl)
                            // Blue
                            .child(
                                VolumeKnob::new()
                                    .id("vol-blue")
                                    .value(0.6)
                                    .size(px(50.0))
                                    .label("60")
                                    .accent_color(hsla(0.6, 0.8, 0.5, 1.0)) // Blue
                                    .bg_color(hsla(0.6, 0.3, 0.15, 1.0))
                                    .text_color(hsla(0.0, 0.0, 1.0, 1.0)),
                            )
                            // Green
                            .child(
                                VolumeKnob::new()
                                    .id("vol-green")
                                    .value(0.8)
                                    .size(px(50.0))
                                    .label("80")
                                    .accent_color(hsla(0.35, 0.8, 0.5, 1.0)) // Green
                                    .bg_color(hsla(0.35, 0.3, 0.15, 1.0))
                                    .text_color(hsla(0.0, 0.0, 1.0, 1.0)),
                            )
                            // Orange
                            .child(
                                VolumeKnob::new()
                                    .id("vol-orange")
                                    .value(0.4)
                                    .size(px(50.0))
                                    .label("40")
                                    .accent_color(hsla(0.08, 0.9, 0.55, 1.0)) // Orange
                                    .bg_color(hsla(0.08, 0.3, 0.15, 1.0))
                                    .text_color(hsla(0.0, 0.0, 1.0, 1.0)),
                            )
                            // Purple
                            .child(
                                VolumeKnob::new()
                                    .id("vol-purple")
                                    .value(0.9)
                                    .size(px(50.0))
                                    .label("90")
                                    .accent_color(hsla(0.75, 0.7, 0.6, 1.0)) // Purple
                                    .bg_color(hsla(0.75, 0.3, 0.15, 1.0))
                                    .text_color(hsla(0.0, 0.0, 1.0, 1.0)),
                            )
                            // Red
                            .child(
                                VolumeKnob::new()
                                    .id("vol-red")
                                    .value(0.2)
                                    .size(px(50.0))
                                    .label("20")
                                    .accent_color(hsla(0.0, 0.8, 0.55, 1.0)) // Red
                                    .bg_color(hsla(0.0, 0.3, 0.15, 1.0))
                                    .text_color(hsla(0.0, 0.0, 1.0, 1.0)),
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
                            .child(Text::new("- Scroll wheel: Adjust volume by 5%").size(TextSize::Sm))
                            .child(Text::new("- Double-click: Toggle mute").size(TextSize::Sm))
                            .child(Text::new("- Click to focus, then Arrow Up/Down: Adjust volume").size(TextSize::Sm))
                            .child(Text::new("- M key (when focused): Toggle mute").size(TextSize::Sm)),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("VolumeKnob Debug")
            .size(900.0, 950.0)
            .scrollable(true)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(VolumeKnobDebug::new),
    );
}
