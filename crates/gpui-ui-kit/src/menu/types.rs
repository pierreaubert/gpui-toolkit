use crate::ComponentTheme;
use crate::theme::glow_shadow;
use gpui::prelude::{InteractiveElement, ParentElement, Styled};
use gpui::{Div, FontWeight, Rgba, SharedString, Stateful, div, px};

/// Theme colors for menu styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct MenuTheme {
    /// Menu background color
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub background: Rgba,
    /// Menu border color
    #[theme(default = 0x444444ff, from = border)]
    pub border: Rgba,
    /// Separator color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub separator: Rgba,
    /// Normal item text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text: Rgba,
    /// Hovered item text color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_hover: Rgba,
    /// Disabled item text color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub text_disabled: Rgba,
    /// Shortcut text color
    #[theme(default = 0x777777ff, from = text_muted)]
    pub text_shortcut: Rgba,
    /// Item hover background color
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub hover_bg: Rgba,
    /// Danger item hover background (for destructive actions like Quit)
    #[theme(default = 0xdc2626ff, from = error)]
    pub danger_hover_bg: Rgba,
}

/// Helper to build a single menu bar button without handlers
/// Use this when you need to add cx.listener() handlers
pub fn menu_bar_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    is_open: bool,
    theme: &MenuTheme,
) -> Stateful<Div> {
    let id = id.into();
    let label = label.into();

    let mut button = div()
        .id(SharedString::from(format!("menubar-{}", id)))
        .px_3()
        .py_1()
        .rounded(px(3.0))
        .text_sm()
        .cursor_pointer();

    if is_open {
        button = button
            .bg(theme.hover_bg)
            .font_weight(FontWeight::BOLD)
            .text_color(theme.text_hover);
    } else {
        let hover_bg = theme.hover_bg;
        button = button
            .text_color(theme.text)
            .hover(move |style| style.bg(hover_bg).shadow(glow_shadow(hover_bg)));
    }

    button.child(label)
}
