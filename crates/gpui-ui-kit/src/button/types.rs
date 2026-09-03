use crate::{ComponentTheme, ComponentVariant};
use gpui::Rgba;

/// Button visual variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ComponentVariant)]
pub enum ButtonVariant {
    /// Primary action button (accent color)
    #[default]
    Primary,
    /// Secondary action button (muted)
    Secondary,
    /// Destructive action (red)
    Destructive,
    /// Ghost button (transparent until hover)
    Ghost,
    /// Outline button (border only)
    Outline,
}

/// Theme colors for button styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct ButtonTheme {
    #[theme(default = 0x007acc, from = accent)]
    pub accent: Rgba,
    #[theme(default = 0x0098ff, from = accent_hover)]
    pub accent_hover: Rgba,
    #[theme(default = 0x3c3c3c, from = surface)]
    pub surface: Rgba,
    #[theme(default = 0x4a4a4a, from = surface_hover)]
    pub surface_hover: Rgba,
    #[theme(default = 0xffffff, from = text_primary)]
    pub text_primary: Rgba,
    #[theme(default = 0xcccccc, from = text_secondary)]
    pub text_secondary: Rgba,
    /// Text color for Primary variant buttons (on accent background)
    #[theme(default = 0xffffff, from = text_on_accent)]
    pub text_on_accent: Rgba,
    #[theme(default = 0xcc3333, from = error)]
    pub error: Rgba,
    /// Error hover color (for destructive button hover)
    #[theme(default = 0xe64545, from = error)]
    pub error_hover: Rgba,
    #[theme(default = 0x555555, from = border)]
    pub border: Rgba,
    /// Transparent color (for ghost/outline backgrounds)
    #[theme(default = 0x00000000, from = transparent)]
    pub transparent: Rgba,
}
