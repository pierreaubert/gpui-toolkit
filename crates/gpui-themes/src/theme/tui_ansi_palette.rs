use super::editor_theme::EditorTheme;
use super::misc::{contrast_ratio, readable_text_color, relative_luminance};
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
        // ANSI index 0 is not necessarily the terminal surface. Using the
        // terminal background keeps the editor's text/surface WCAG contract
        // coherent, while still mapping ANSI hues to semantic accents.
        theme.surface = self.background;
        theme.surface_hover = self.background;
        theme.text_primary = if contrast_ratio(self.foreground, self.background) >= 4.5 {
            self.foreground
        } else {
            readable_text_color(self.background)
        };
        theme.accent = self.ansi[4];
        theme.accent_hover = self.ansi[12];
        theme.accent_muted = self.ansi[4].with_alpha(0.32);
        theme.text_on_accent = readable_text_color(theme.accent);
        theme.error = self.ansi[1];
        theme.success = self.ansi[2];
        theme.warning = self.ansi[3];
        theme.info = self.ansi[6];
        theme
    }
}
