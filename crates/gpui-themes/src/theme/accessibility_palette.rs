use serde::{Deserialize, Serialize};

/// Accessibility classification for presets and community themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccessibilityPalette {
    #[default]
    Standard,
    HighContrast,
    Protanopia,
    Deuteranopia,
    Tritanopia,
}

impl AccessibilityPalette {
    pub fn all() -> &'static [AccessibilityPalette] {
        &[
            AccessibilityPalette::Standard,
            AccessibilityPalette::HighContrast,
            AccessibilityPalette::Protanopia,
            AccessibilityPalette::Deuteranopia,
            AccessibilityPalette::Tritanopia,
        ]
    }

    pub fn name(self) -> &'static str {
        match self {
            AccessibilityPalette::Standard => "Standard",
            AccessibilityPalette::HighContrast => "High Contrast",
            AccessibilityPalette::Protanopia => "Protanopia",
            AccessibilityPalette::Deuteranopia => "Deuteranopia",
            AccessibilityPalette::Tritanopia => "Tritanopia",
        }
    }

    pub fn is_color_blind_safe(self) -> bool {
        matches!(
            self,
            AccessibilityPalette::Protanopia
                | AccessibilityPalette::Deuteranopia
                | AccessibilityPalette::Tritanopia
        )
    }
}
