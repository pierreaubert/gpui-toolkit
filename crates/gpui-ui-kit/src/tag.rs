//! Tag / Chip component
//!
//! Small labeled elements for categories, statuses, or metadata.
//! Similar to Badge but designed for interactive use with optional remove button.
//!
//! # Usage
//!
//! ```ignore
//! Tag::new("tag-flac", "FLAC")
//!     .variant(TagVariant::Primary)
//!     .removable(true)
//!     .on_remove(|window, cx| { /* handle remove */ })
//! ```

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Div, ElementId, FontWeight, MouseButton, Rgba, SharedString, Stateful, Window, div, px,
};

/// Tag variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TagVariant {
    /// Default subtle style
    #[default]
    Default,
    /// Primary accent
    Primary,
    /// Success green
    Success,
    /// Warning yellow
    Warning,
    /// Error red
    Error,
    /// Outlined (border only, no fill)
    Outlined,
}

/// Tag size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TagSize {
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
}

/// Theme colors for tag
#[derive(Debug, Clone, ComponentTheme)]
pub struct TagTheme {
    /// Default background
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub default_bg: Rgba,
    /// Default text
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub default_text: Rgba,
    /// Primary background
    #[theme(default = 0x1a3a5cff, from = badge_primary_bg)]
    pub primary_bg: Rgba,
    /// Primary text
    #[theme(default = 0x60a5faff, from = badge_primary_text)]
    pub primary_text: Rgba,
    /// Success background
    #[theme(default = 0x1a3a2aff, from = badge_success_bg)]
    pub success_bg: Rgba,
    /// Success text
    #[theme(default = 0x4ade80ff, from = badge_success_text)]
    pub success_text: Rgba,
    /// Warning background
    #[theme(default = 0x3a3a1aff, from = badge_warning_bg)]
    pub warning_bg: Rgba,
    /// Warning text
    #[theme(default = 0xfbbf24ff, from = badge_warning_text)]
    pub warning_text: Rgba,
    /// Error background
    #[theme(default = 0x3a1a1aff, from = badge_error_bg)]
    pub error_bg: Rgba,
    /// Error text
    #[theme(default = 0xf87171ff, from = badge_error_text)]
    pub error_text: Rgba,
    /// Outlined border
    #[theme(default = 0x3a3a3aff, from = border)]
    pub outlined_border: Rgba,
    /// Remove button hover color
    #[theme(default = 0x55555588, from = surface_hover)]
    pub remove_hover: Rgba,
}

/// A tag/chip component
pub struct Tag {
    id: ElementId,
    label: SharedString,
    variant: TagVariant,
    size: TagSize,
    icon: Option<SharedString>,
    removable: bool,
    on_click: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    on_remove: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl Tag {
    /// Create a new tag
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: TagVariant::default(),
            size: TagSize::default(),
            icon: None,
            removable: false,
            on_click: None,
            on_remove: None,
        }
    }

    /// Set variant
    pub fn variant(mut self, variant: TagVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set size
    pub fn size(mut self, size: TagSize) -> Self {
        self.size = size;
        self
    }

    /// Set leading icon
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Make the tag removable (shows X button)
    pub fn removable(mut self, removable: bool) -> Self {
        self.removable = removable;
        self
    }

    /// Called when the tag is clicked
    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Called when the remove button is clicked
    pub fn on_remove(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Box::new(handler));
        self
    }

    /// Build the tag with theme
    pub fn build_with_theme(self, theme: &TagTheme) -> Stateful<Div> {
        let (bg, text_color, border) = match self.variant {
            TagVariant::Default => (Some(theme.default_bg), theme.default_text, None),
            TagVariant::Primary => (Some(theme.primary_bg), theme.primary_text, None),
            TagVariant::Success => (Some(theme.success_bg), theme.success_text, None),
            TagVariant::Warning => (Some(theme.warning_bg), theme.warning_text, None),
            TagVariant::Error => (Some(theme.error_bg), theme.error_text, None),
            TagVariant::Outlined => (None, theme.default_text, Some(theme.outlined_border)),
        };

        let remove_id = (self.id.clone(), "remove");

        let mut tag = div()
            .id(self.id)
            .flex()
            .items_center()
            .gap_1()
            .rounded(px(4.0))
            .text_color(text_color);

        // Size-specific padding and font
        tag = match self.size {
            TagSize::Sm => tag.px_1p5().py(px(1.0)).text_xs(),
            TagSize::Md => tag.px_2().py(px(2.0)).text_xs(),
            TagSize::Lg => tag.px_3().py_1().text_sm(),
        };

        if let Some(bg) = bg {
            tag = tag.bg(bg);
        }

        if let Some(border_color) = border {
            tag = tag.border_1().border_color(border_color);
        }

        if let Some(handler) = self.on_click {
            tag = tag
                .cursor_pointer()
                .on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                    handler(window, cx);
                });
        }

        // Leading icon
        if let Some(icon) = self.icon {
            tag = tag.child(div().child(icon));
        }

        // Label
        tag = tag.child(div().font_weight(FontWeight::MEDIUM).child(self.label));

        // Remove button
        if self.removable {
            let remove_hover = theme.remove_hover;
            let mut remove_btn = div()
                .id(remove_id)
                .ml_1()
                .cursor_pointer()
                .rounded(px(2.0))
                .hover(move |s| s.bg(remove_hover))
                .child("\u{2715}"); // ✕

            if let Some(handler) = self.on_remove {
                remove_btn =
                    remove_btn.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                        handler(window, cx);
                    });
            }
            tag = tag.child(remove_btn);
        }

        tag
    }
}

impl RenderOnce for Tag {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = TagTheme::from(&global_theme);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for Tag {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
