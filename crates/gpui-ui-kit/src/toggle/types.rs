use crate::ComponentTheme;
use gpui::Rgba;

/// Toggle visual style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToggleStyle {
    /// iOS-style sliding toggle (default)
    #[default]
    Sliding,
    /// Segmented [OFF | ON] style for audio plugins
    Segmented,
}

/// Theme colors for toggle styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct ToggleTheme {
    /// Background when checked
    #[theme(default = 0x007accff, from = accent)]
    pub checked_bg: Rgba,
    /// Background when unchecked
    #[theme(default = 0x3a3a3aff, from = muted)]
    pub unchecked_bg: Rgba,
    /// Knob/thumb color when unchecked
    #[theme(default = 0xffffffff, from = text_primary)]
    pub knob: Rgba,
    /// Knob/thumb color when checked (for contrast on light backgrounds)
    #[theme(default = 0xffffffff, from = background)]
    pub knob_on_checked: Rgba,
    /// Track border color (for visibility on dark backgrounds)
    #[theme(default = 0x3a3a3aff, from = border)]
    pub track_border: Rgba,
    /// Label text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub label: Rgba,
    /// Selected state accent color
    #[theme(default = 0x007accff, from = accent)]
    pub accent: Rgba,
    /// Selected state background
    #[theme(default = 0x007acc33, from = accent_muted)]
    pub accent_muted: Rgba,
    /// Success color for ON state (segmented style)
    #[theme(default = 0x22c55eff, from = success)]
    pub success: Rgba,
    /// Border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Text on accent background
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub text_on_accent: Rgba,
    /// Muted text color
    #[theme(default = 0x888888ff, from = text_muted)]
    pub text_muted: Rgba,
    /// Primary text color (for selected labels)
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_primary: Rgba,
    /// Surface hover color (for active segmented button)
    #[theme(default = 0x4a4a4aff, from = surface_hover)]
    pub surface_hover: Rgba,
    /// Background color (for inactive segmented button)
    #[theme(default = 0x2a2a2aff, from = background)]
    pub background: Rgba,
}
