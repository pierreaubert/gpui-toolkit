use crate::ComponentTheme;
use gpui::{App, Rgba, SharedString, Window};

/// Theme colors for accordion styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct AccordionTheme {
    #[theme(default = 0x252525, from = muted)]
    pub header_bg: Rgba,
    #[theme(default = 0x2a2a2a, from = surface_hover)]
    pub header_hover_bg: Rgba,
    #[theme(default = 0x007acc33, from = accent_muted)]
    pub header_active_bg: Rgba,
    #[theme(default = 0x1e1e1e, from = background)]
    pub content_bg: Rgba,
    #[theme(default = 0x3a3a3a, from = border)]
    pub border: Rgba,
    #[theme(default = 0x007acc33, from = accent_muted)]
    pub accent_tint: Rgba,
    #[theme(default = 0x007acc, from = accent)]
    pub accent: Rgba,
    #[theme(default = 0xffffff, from = text_primary)]
    pub title_color: Rgba,
    #[theme(default = 0x888888, from = text_muted)]
    pub indicator_color: Rgba,
}

pub(super) type AccordionChangeHandler =
    std::rc::Rc<Box<dyn Fn(&SharedString, bool, &mut Window, &mut App) + 'static>>;

/// Accordion behavior mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccordionMode {
    /// Only one item can be open at a time
    #[default]
    Single,
    /// Multiple items can be open
    Multiple,
}

/// Accordion orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccordionOrientation {
    /// Vertical layout: headers stacked vertically, content expands downward (default)
    #[default]
    Vertical,
    /// Horizontal layout: headers arranged horizontally, content expands downward
    Horizontal,
    /// Side layout: headers stacked vertically on left, content expands to right
    Side,
}
