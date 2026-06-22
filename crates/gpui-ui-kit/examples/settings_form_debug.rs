//! SettingsForm Debug Example
//!
//! Demonstrates the SettingsForm and SettingsRow components:
//! - Rows with various controls
//! - Section headers

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct SettingsFormDebug {
    sample_rate: SharedString,
    buffer_size: f32,
    eq_enabled: bool,
    upmix_enabled: bool,
    entity: Entity<Self>,
}

impl Render for SettingsFormDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();

        div()
            .id("settings-form-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("SettingsForm Debug"))
            .child(
                SettingsForm::new("settings-demo")
                    .section("Audio Output")
                    .row(
                        SettingsRow::new("Sample Rate")
                            .description("Output sample rate for audio playback")
                            .control(
                                Select::new("sr-select")
                                    .options(vec![
                                        SelectOption::new("44100", "44.1 kHz"),
                                        SelectOption::new("48000", "48 kHz"),
                                        SelectOption::new("96000", "96 kHz"),
                                    ])
                                    .selected(self.sample_rate.clone())
                                    .on_change({
                                        let entity = entity.clone();
                                        move |value, _window, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.sample_rate = value.clone();
                                            });
                                        }
                                    }),
                            ),
                    )
                    .row(
                        SettingsRow::new("Buffer Size")
                            .description("Lower values reduce latency but increase CPU usage")
                            .control(
                                Slider::new("buffer-slider")
                                    .value(self.buffer_size)
                                    .range(64.0, 2048.0)
                                    .show_value(true)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |value, _window, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.buffer_size = value;
                                            });
                                        }
                                    }),
                            ),
                    )
                    .section("Processing")
                    .row(
                        SettingsRow::new("Enable EQ")
                            .description("Apply parametric equalization")
                            .control(Toggle::new("eq-toggle").checked(self.eq_enabled).on_change(
                                {
                                    let entity = entity.clone();
                                    move |checked, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            this.eq_enabled = checked;
                                        });
                                    }
                                },
                            )),
                    )
                    .row(
                        SettingsRow::new("Enable Upmixer")
                            .description("Upmix stereo to 5.0 surround")
                            .control(
                                Toggle::new("upmix-toggle")
                                    .checked(self.upmix_enabled)
                                    .on_change({
                                        let entity = entity.clone();
                                        move |checked, _window, cx| {
                                            entity.update(cx, |this, _cx| {
                                                this.upmix_enabled = checked;
                                            });
                                        }
                                    }),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("SettingsForm Debug")
            .size(700.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| {
            cx.new(|cx| SettingsFormDebug {
                sample_rate: "48000".into(),
                buffer_size: 256.0,
                eq_enabled: true,
                upmix_enabled: false,
                entity: cx.entity().clone(),
            })
        },
    );
}
