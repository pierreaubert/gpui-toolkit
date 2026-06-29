use super::editor_theme::EditorTheme;
use super::misc::relative_luminance;
use super::tui_theme_preset::TuiThemePreset;
pub use gpui_ui_kit::Color;
use serde::{Deserialize, Serialize};

/// 16-color ANSI terminal palette.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TuiAnsiPalette {
    pub preset: TuiThemePreset,
    pub name: String,
    pub background: Color,
    pub foreground: Color,
    pub ansi: [Color; 16],
}

impl TuiAnsiPalette {
    pub(super) fn new(
        preset: TuiThemePreset,
        background: Color,
        foreground: Color,
        ansi_hex: [u32; 16],
    ) -> Self {
        Self {
            preset,
            name: preset.name().to_string(),
            background,
            foreground,
            ansi: ansi_hex.map(Color::from_hex),
        }
    }

    pub fn to_editor_theme(&self) -> EditorTheme {
        let mut theme = if relative_luminance(self.background) >= 0.5 {
            EditorTheme::light()
        } else {
            EditorTheme::dark()
        };
        theme.name = self.name.clone();
        theme.background = self.background;
        theme.surface = self.ansi[0];
        theme.text_primary = self.foreground;
        theme.accent = self.ansi[4];
        theme.accent_hover = self.ansi[12];
        theme.accent_muted = self.ansi[4].with_alpha(0.32);
        theme.error = self.ansi[1];
        theme.success = self.ansi[2];
        theme.warning = self.ansi[3];
        theme.info = self.ansi[6];
        theme
    }
}
