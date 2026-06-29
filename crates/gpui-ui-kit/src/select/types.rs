use crate::ComponentTheme;
use gpui::Rgba;

/// Theme colors for select styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct SelectTheme {
    /// Trigger background color
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub trigger_bg: Rgba,
    /// Trigger border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub trigger_border: Rgba,
    /// Trigger border color on hover
    #[theme(default = 0x007accff, from = accent)]
    pub trigger_border_hover: Rgba,
    /// Trigger border color when focused/open
    #[theme(default = 0x007accff, from = accent)]
    pub trigger_border_focused: Rgba,
    /// Dropdown background color
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub dropdown_bg: Rgba,
    /// Dropdown border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub dropdown_border: Rgba,
    /// Selected option background
    #[theme(default = 0x007accff, from = accent)]
    pub selected_bg: Rgba,
    /// Option hover background
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub option_hover_bg: Rgba,
    /// Label text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub label_color: Rgba,
    /// Text color for selected value
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_color: Rgba,
    /// Placeholder text color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub placeholder_color: Rgba,
    /// Option text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub option_text_color: Rgba,
    /// Selected option text color (on accent background)
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub selected_text_color: Rgba,
    /// Disabled text color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub disabled_color: Rgba,
    /// Arrow/chevron color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub arrow_color: Rgba,
}
