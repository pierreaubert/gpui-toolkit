use super::misc::readable_text_color;
use super::misc::shift_lightness;
use super::types::AccentSource;
use super::types::ThemeAppearance;
pub use gpui_ui_kit::Color;
use serde::{Deserialize, Serialize};

/// Harmonized accent colors generated from a system, wallpaper, user, or preset seed.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AccentPalette {
    pub source: AccentSource,
    pub seed: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub accent_muted: Color,
    pub text_on_accent: Color,
}

impl AccentPalette {
    pub fn from_seed(seed: Color, source: AccentSource, appearance: ThemeAppearance) -> Self {
        let hover_delta = match appearance {
            ThemeAppearance::Light => -0.08,
            ThemeAppearance::Dark => 0.12,
        };
        let muted_lightness = match appearance {
            ThemeAppearance::Light => 0.88,
            ThemeAppearance::Dark => 0.24,
        };
        let (h, s, _) = seed.to_hsl();
        let accent_muted =
            Color::from_hsl(h, (s * 0.55).clamp(0.25, 0.8), muted_lightness).with_alpha(0.85);

        Self {
            source,
            seed,
            accent: seed,
            accent_hover: shift_lightness(seed, hover_delta),
            accent_muted,
            text_on_accent: readable_text_color(seed),
        }
    }
}
