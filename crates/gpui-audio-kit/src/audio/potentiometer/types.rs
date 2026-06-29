use crate::ComponentTheme;
use crate::scale::Scale;
use gpui::*;

/// Theme colors for potentiometer styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct PotentiometerTheme {
    /// Background color of the container
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub surface: Rgba,
    /// Surface hover color
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub surface_hover: Rgba,
    /// Knob background color
    #[theme(default = 0x1a1a1aff, from = muted)]
    pub knob_bg: Rgba,
    /// Accent color
    #[theme(default = 0x007accff, from = accent)]
    pub accent: Rgba,
    /// Accent muted (for selection background)
    #[theme(
        default = 0x007acc33,
        from_expr = "Rgba { r: theme.accent.r, g: theme.accent.g, b: theme.accent.b, a: 0.2 }"
    )]
    pub accent_muted: Rgba,
    /// Border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Label text color
    #[theme(default = 0xaaaaaaff, from = text_secondary)]
    pub text_secondary: Rgba,
    /// Value text color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_primary: Rgba,
    /// Muted text color (for indicator when not selected)
    #[theme(default = 0x888888ff, from = text_muted)]
    pub text_muted: Rgba,
    /// Text on accent background
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub text_on_accent: Rgba,
    /// Background secondary (for value badge)
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub background_secondary: Rgba,
}

/// Scale type for potentiometer value mapping
/// Re-exported from scale module for API consistency
pub type PotentiometerScale = Scale;
