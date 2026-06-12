//! SettingsForm Debug Example
//!
//! Demonstrates the SettingsForm and SettingsRow components:
//! - Rows with various controls
//! - Section headers

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct SettingsFormDebug;

impl Render for SettingsFormDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

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
                                    .selected(SharedString::from("48000")),
                            ),
                    )
                    .row(
                        SettingsRow::new("Buffer Size")
                            .description("Lower values reduce latency but increase CPU usage")
                            .control(
                                Slider::new("buffer-slider")
                                    .value(256.0)
                                    .range(64.0, 2048.0)
                                    .show_value(true),
                            ),
                    )
                    .section("Processing")
                    .row(
                        SettingsRow::new("Enable EQ")
                            .description("Apply parametric equalization")
                            .control(Toggle::new("eq-toggle").checked(true)),
                    )
                    .row(
                        SettingsRow::new("Enable Upmixer")
                            .description("Upmix stereo to 5.0 surround")
                            .control(Toggle::new("upmix-toggle")),
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
        |cx| cx.new(|_cx| SettingsFormDebug),
    );
}
