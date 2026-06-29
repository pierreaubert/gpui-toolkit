use super::accessibility_palette::AccessibilityPalette;
pub use gpui_ui_kit::Color;
use serde::{Deserialize, Serialize};

/// Plugin type color mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginColors {
    pub eq: Color,
    pub gain: Color,
    pub upmixer: Color,
    pub compressor: Color,
    pub limiter: Color,
    pub gate: Color,
    pub loudness: Color,
    pub binaural: Color,
    pub convolution: Color,
    pub monitor: Color,
    pub spectrum: Color,
    pub mute_solo: Color,
}

/// Graph visualization line colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphColors {
    pub input: Color,
    pub target: Color,
    pub filter_response: Color,
    pub corrected: Color,
    pub error: Color,
    pub deviation: Color,
    pub grid: Color,
    pub secondary_line: Color,
    pub directivity_er: Color,
    pub directivity_sp: Color,
}

/// EQ curve visualization colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EQCurveColors {
    pub background: Color,
    pub grid: Color,
    pub curve_boost: Color,
    pub curve_cut: Color,
    pub fill_boost: Color,
    pub fill_cut: Color,
    pub zero_line: Color,
}

/// Spectrum analyzer colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrumColors {
    pub background: Color,
    pub bass: Color,
    pub mids: Color,
    pub treble: Color,
}

/// Level meter colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterColors {
    pub background: Color,
    pub normal: Color,
    pub warning: Color,
    pub clip: Color,
    pub peak: Color,
    pub text: Color,
}

/// Resolved light/dark appearance after system preference and scheduling are applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeAppearance {
    Light,
    #[default]
    Dark,
}

/// Origin for an accent palette seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccentSource {
    #[default]
    Theme,
    System,
    Wallpaper,
    User,
}

/// Easing curve for theme transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeTransitionEasing {
    Linear,
    #[default]
    EaseOut,
    EaseInOut,
}

/// Theme gallery entry for a local marketplace or import screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeGalleryEntry {
    pub id: String,
    pub display_name: String,
    pub tags: Vec<String>,
    pub accessibility: AccessibilityPalette,
    pub appearance: ThemeAppearance,
}
