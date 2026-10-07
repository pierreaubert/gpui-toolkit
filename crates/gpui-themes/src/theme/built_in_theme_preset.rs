use super::accessibility_palette::AccessibilityPalette;
use super::editor_theme::EditorTheme;
use super::misc::normalize_theme_id;
use super::types::ThemeAppearance;
use serde::{Deserialize, Serialize};

/// Built-in editor theme presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BuiltInThemePreset {
    #[default]
    Dark,
    Light,
    HighContrast,
    Nord,
    Dracula,
    Protanopia,
    Deuteranopia,
    Tritanopia,
    MacosDark,
    MacosLight,
    FluentDark,
    FluentLight,
}

impl BuiltInThemePreset {
    pub fn all() -> &'static [BuiltInThemePreset] {
        &[
            BuiltInThemePreset::Dark,
            BuiltInThemePreset::Light,
            BuiltInThemePreset::HighContrast,
            BuiltInThemePreset::Nord,
            BuiltInThemePreset::Dracula,
            BuiltInThemePreset::Protanopia,
            BuiltInThemePreset::Deuteranopia,
            BuiltInThemePreset::Tritanopia,
            BuiltInThemePreset::MacosDark,
            BuiltInThemePreset::MacosLight,
            BuiltInThemePreset::FluentDark,
            BuiltInThemePreset::FluentLight,
        ]
    }

    pub fn id(self) -> &'static str {
        match self {
            BuiltInThemePreset::Dark => "dark",
            BuiltInThemePreset::Light => "light",
            BuiltInThemePreset::HighContrast => "high_contrast",
            BuiltInThemePreset::Nord => "nord",
            BuiltInThemePreset::Dracula => "dracula",
            BuiltInThemePreset::Protanopia => "protanopia",
            BuiltInThemePreset::Deuteranopia => "deuteranopia",
            BuiltInThemePreset::Tritanopia => "tritanopia",
            BuiltInThemePreset::MacosDark => "macos_dark",
            BuiltInThemePreset::MacosLight => "macos_light",
            BuiltInThemePreset::FluentDark => "fluent_dark",
            BuiltInThemePreset::FluentLight => "fluent_light",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            BuiltInThemePreset::Dark => "Dark",
            BuiltInThemePreset::Light => "Light",
            BuiltInThemePreset::HighContrast => "High Contrast",
            BuiltInThemePreset::Nord => "Nord",
            BuiltInThemePreset::Dracula => "Dracula",
            BuiltInThemePreset::Protanopia => "Protanopia",
            BuiltInThemePreset::Deuteranopia => "Deuteranopia",
            BuiltInThemePreset::Tritanopia => "Tritanopia",
            BuiltInThemePreset::MacosDark => "macOS Dark",
            BuiltInThemePreset::MacosLight => "macOS Light",
            BuiltInThemePreset::FluentDark => "Fluent Dark",
            BuiltInThemePreset::FluentLight => "Fluent Light",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match normalize_theme_id(id).as_str() {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            "high_contrast" | "highcontrast" => Some(Self::HighContrast),
            "nord" => Some(Self::Nord),
            "dracula" => Some(Self::Dracula),
            "protanopia" => Some(Self::Protanopia),
            "deuteranopia" => Some(Self::Deuteranopia),
            "tritanopia" => Some(Self::Tritanopia),
            "macos_dark" | "macosdark" => Some(Self::MacosDark),
            "macos_light" | "macoslight" => Some(Self::MacosLight),
            "fluent_dark" | "fluentdark" => Some(Self::FluentDark),
            "fluent_light" | "fluentlight" => Some(Self::FluentLight),
            _ => None,
        }
    }

    pub fn accessibility(self) -> AccessibilityPalette {
        match self {
            BuiltInThemePreset::HighContrast => AccessibilityPalette::HighContrast,
            BuiltInThemePreset::Protanopia => AccessibilityPalette::Protanopia,
            BuiltInThemePreset::Deuteranopia => AccessibilityPalette::Deuteranopia,
            BuiltInThemePreset::Tritanopia => AccessibilityPalette::Tritanopia,
            _ => AccessibilityPalette::Standard,
        }
    }

    /// Appearance metadata without constructing the full editor theme.
    pub fn appearance(self) -> ThemeAppearance {
        match self {
            BuiltInThemePreset::Light
            | BuiltInThemePreset::MacosLight
            | BuiltInThemePreset::FluentLight => ThemeAppearance::Light,
            BuiltInThemePreset::Dark
            | BuiltInThemePreset::HighContrast
            | BuiltInThemePreset::Nord
            | BuiltInThemePreset::Dracula
            | BuiltInThemePreset::Protanopia
            | BuiltInThemePreset::Deuteranopia
            | BuiltInThemePreset::Tritanopia
            | BuiltInThemePreset::MacosDark
            | BuiltInThemePreset::FluentDark => ThemeAppearance::Dark,
        }
    }

    pub fn to_theme(self) -> EditorTheme {
        match self {
            BuiltInThemePreset::Dark => EditorTheme::dark(),
            BuiltInThemePreset::Light => EditorTheme::light(),
            BuiltInThemePreset::HighContrast => EditorTheme::high_contrast(),
            BuiltInThemePreset::Nord => EditorTheme::nord(),
            BuiltInThemePreset::Dracula => EditorTheme::dracula(),
            BuiltInThemePreset::Protanopia => EditorTheme::protanopia(),
            BuiltInThemePreset::Deuteranopia => EditorTheme::deuteranopia(),
            BuiltInThemePreset::Tritanopia => EditorTheme::tritanopia(),
            BuiltInThemePreset::MacosDark => EditorTheme::macos_dark(),
            BuiltInThemePreset::MacosLight => EditorTheme::macos_light(),
            BuiltInThemePreset::FluentDark => EditorTheme::fluent_dark(),
            BuiltInThemePreset::FluentLight => EditorTheme::fluent_light(),
        }
    }
}
