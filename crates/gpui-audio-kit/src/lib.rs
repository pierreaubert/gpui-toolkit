//! Audio-focused component kit for GPUI.
//!
//! `gpui-audio-kit` owns controls and visualizations that are specific to
//! audio, plugins, playback, and meters. General-purpose controls stay in
//! `gpui-ui-kit`.

#![allow(clippy::type_complexity)]
#![allow(clippy::wrong_self_convention)]

pub mod audio;
pub mod audio_design_tokens;
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
pub use audio_design_tokens::AudioDesignTokens;
pub use meter::{
    HorizontalMeterTheme, LevelMeterElement, MeterColors, db_to_position,
    render_horizontal_meter_bar, render_horizontal_meter_bar_with,
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
        let style = match tokens.toggle_variant {
            AudioDesignTokens::TOGGLE_SEGMENTED => gpui_ui_kit::ToggleStyle::Segmented,
            _ => gpui_ui_kit::ToggleStyle::Sliding,
        };
        self.style(style)
    }
}
