//! Audio-focused component kit for GPUI.
//!
//! `gpui-audio-kit` owns controls and visualizations that are specific to
//! audio, plugins, playback, and meters. General-purpose controls stay in
//! `gpui-ui-kit`.

#![allow(clippy::type_complexity)]
#![allow(clippy::wrong_self_convention)]

pub mod audio;
mod audio_accessibility;
mod audio_automation_patterns;
pub mod audio_design_tokens;
mod audio_visual_regression;
pub mod meter;
pub mod scale;
pub mod spectrum;
pub mod ticks;

pub use audio::potentiometer::{
    Potentiometer, PotentiometerScale, PotentiometerSize, PotentiometerTheme,
};
pub use audio::vertical_slider::{
    VerticalSlider, VerticalSliderScale, VerticalSliderSize, VerticalSliderTheme,
};
pub use audio::volume_knob::{VolumeKnob, VolumeKnobTheme};
pub use audio::{
    DragState, InteractionConfig, ValueTracker, clear_drag_state, get_drag_state, handle_drag,
    handle_keyboard, handle_scroll, store_drag_state, value_tracker,
};
pub use audio_accessibility::AudioAccessibilitySummary;
pub use audio_automation_patterns::{
    AUDIO_AUTOMATION_PATTERN_REPORT_TYPE, AUDIO_AUTOMATION_PATTERN_SCHEMA_VERSION,
    AUDIO_AUTOMATION_PATTERNS, AudioAutomationPattern, AudioAutomationPatternReport,
    AudioAutomationPatternStatus, audio_automation_pattern_report,
};
pub use audio_design_tokens::AudioDesignTokens;
pub use audio_visual_regression::{
    AUDIO_VISUAL_COLOR_SCHEMES, AUDIO_VISUAL_REGRESSION_REPORT_TYPE,
    AUDIO_VISUAL_REGRESSION_SCHEMA_VERSION, AUDIO_VISUAL_STORIES, AUDIO_VISUAL_VIEWPORTS,
    AudioVisualCapture, AudioVisualColorScheme, AudioVisualRegressionManifest, AudioVisualStory,
    AudioVisualViewport, audio_visual_regression_manifest,
};
pub use meter::{
    HorizontalMeterTheme, LevelMeterElement, MeterColors, db_to_position,
    horizontal_meter_accessibility_summary, render_horizontal_meter_bar,
    render_horizontal_meter_bar_with,
};
pub use scale::Scale as AudioScale;
pub use spectrum::{
    MeterData, SpectrumAxisLabel, SpectrumAxisTheme, SpectrumColors, SpectrumDbAxisLabel,
    SpectrumElement, format_spectrum_frequency_label, logarithmic_frequency_position,
    render_spectrum_db_axis, render_spectrum_frequency_axis, spectrum_db_axis_labels,
    spectrum_frequency_axis_labels,
};
pub use ticks::{ScaleType, TickConfig, TickMark, render_tick_row};

pub use gpui_ui_kit::{ComponentBuilder, ComponentSize, ComponentTheme};

pub mod accessibility {
    pub use gpui_ui_kit::accessibility::*;
}

pub mod theme {
    pub use gpui_ui_kit::theme::*;
}

/// Extension methods for applying audio design tokens to general UI-kit controls.
pub trait AudioToggleExt {
    /// Set a `gpui-ui-kit` toggle's visual style from audio design tokens.
    fn design_tokens(self, tokens: &AudioDesignTokens) -> Self;
}

impl AudioToggleExt for gpui_ui_kit::Toggle {
    fn design_tokens(self, tokens: &AudioDesignTokens) -> Self {
        self.style(audio_toggle_style(tokens))
    }
}

fn audio_toggle_style(tokens: &AudioDesignTokens) -> gpui_ui_kit::ToggleStyle {
    match tokens.toggle_variant {
        AudioDesignTokens::TOGGLE_SLIDING => gpui_ui_kit::ToggleStyle::Sliding,
        AudioDesignTokens::TOGGLE_SEGMENTED => gpui_ui_kit::ToggleStyle::Segmented,
        AudioDesignTokens::TOGGLE_THUMB_ON_TRACK => gpui_ui_kit::ToggleStyle::ThumbOnTrack,
        AudioDesignTokens::TOGGLE_PILL => gpui_ui_kit::ToggleStyle::Pill,
        _ => gpui_ui_kit::ToggleStyle::Sliding,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_toggle_tokens_map_every_supported_platform_style() {
        for (variant, expected) in [
            (
                AudioDesignTokens::TOGGLE_SLIDING,
                gpui_ui_kit::ToggleStyle::Sliding,
            ),
            (
                AudioDesignTokens::TOGGLE_SEGMENTED,
                gpui_ui_kit::ToggleStyle::Segmented,
            ),
            (
                AudioDesignTokens::TOGGLE_THUMB_ON_TRACK,
                gpui_ui_kit::ToggleStyle::ThumbOnTrack,
            ),
            (
                AudioDesignTokens::TOGGLE_PILL,
                gpui_ui_kit::ToggleStyle::Pill,
            ),
        ] {
            assert_eq!(
                audio_toggle_style(&AudioDesignTokens {
                    toggle_variant: variant,
                    ..Default::default()
                }),
                expected
            );
        }
    }
}
