//! AutoEQ form theme definition.

use gpui::*;
use gpui_ui_kit::ComponentTheme;
use gpui_ui_kit::number_input::NumberInputTheme;
use gpui_ui_kit::select::SelectTheme;
use gpui_ui_kit::theme::Theme;

/// Theme for the AutoEQ form
#[derive(Debug, Clone, ComponentTheme)]
pub struct AutoEqFormTheme {
    /// Card background
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub card_bg: Rgba,
    /// Section header color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub header_color: Rgba,
    /// Label color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub label_color: Rgba,
    /// Description color
    #[theme(default = 0x888888ff, from = text_muted)]
    pub description_color: Rgba,
    /// Accent color
    #[theme(default = 0x007accff, from = accent)]
    pub accent: Rgba,
    /// Toggle theme colors
    #[theme(default = 0x007accff, from = accent)]
    pub toggle_checked_bg: Rgba,
    #[theme(default = 0x4a4a4aff, from = muted)]
    pub toggle_unchecked_bg: Rgba,
    #[theme(default = 0xffffffff, from = text_primary)]
    pub toggle_knob: Rgba,
    /// Border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Text muted color
    #[theme(default = 0x888888ff, from = text_muted)]
    pub text_muted: Rgba,
    /// NumberInput theme
    #[theme(
        default_expr = "NumberInputTheme::default()",
        from_expr = "NumberInputTheme::from(theme)"
    )]
    pub number_input_theme: NumberInputTheme,
    /// Select theme
    #[theme(
        default_expr = "SelectTheme::default()",
        from_expr = "SelectTheme::from(theme)"
    )]
    pub select_theme: SelectTheme,
}
