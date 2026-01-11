//! IconButton component
//!
//! A button that displays only an icon, with optional tooltip.
//! Supports both text/emoji icons and custom child elements (like SVG icons).

use crate::ComponentTheme;
use crate::theme::{ThemeExt, glow_shadow};
use gpui::prelude::*;
use gpui::*;

/// Theme colors for icon button styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct IconButtonTheme {
    /// Background color for ghost variant
    #[theme(default = 0x00000000, from = transparent)]
    pub ghost_bg: Rgba,
    /// Background color on hover for ghost variant
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub ghost_hover_bg: Rgba,
    /// Background color when selected
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub selected_bg: Rgba,
    /// Background color on hover when selected
    #[theme(default = 0x4a4a4aff, from = muted)]
    pub selected_hover_bg: Rgba,
    /// Filled variant background
    #[theme(default = 0x3a3a3aff, from = surface)]
    pub filled_bg: Rgba,
    /// Filled variant hover background
    #[theme(default = 0x4a4a4aff, from = surface_hover)]
    pub filled_hover_bg: Rgba,
    /// Accent color (for filled selected, outline border)
    #[theme(default = 0x007accff, from = accent)]
    pub accent: Rgba,
    /// Accent hover color
    #[theme(default = 0x0098ffff, from = accent)]
    pub accent_hover: Rgba,
    /// Default text/icon color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text: Rgba,
    /// Text color when selected or on accent background
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_on_accent: Rgba,
    /// Border color for outline variant
    #[theme(default = 0x555555ff, from = border)]
    pub border: Rgba,
}

/// IconButton size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconButtonSize {
    /// Extra small (16px)
    Xs,
    /// Small (20px)
    Sm,
    /// Medium (24px, default)
    #[default]
    Md,
    /// Large (32px)
    Lg,
    /// Extra large (48px)
    Xl,
    /// Custom size in pixels
    Custom(u32),
}

impl IconButtonSize {
    /// Get the size in pixels
    pub fn size(&self) -> Pixels {
        match self {
            IconButtonSize::Xs => px(16.0),
            IconButtonSize::Sm => px(20.0),
            IconButtonSize::Md => px(24.0),
            IconButtonSize::Lg => px(32.0),
            IconButtonSize::Xl => px(48.0),
            IconButtonSize::Custom(size) => px(*size as f32),
        }
    }
}

impl From<crate::ComponentSize> for IconButtonSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs => Self::Xs,
            crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg => Self::Lg,
            crate::ComponentSize::Xl => Self::Xl,
        }
    }
}

/// IconButton variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconButtonVariant {
    /// Ghost button (transparent, default)
    #[default]
    Ghost,
    /// Filled background
    Filled,
    /// Outline border
    Outline,
}

/// Icon content - either text/emoji or a custom element
enum IconContent {
    Text(SharedString),
    Element(AnyElement),
}

/// An icon-only button component
///
/// # Examples
///
/// ```ignore
/// // With text/emoji icon
/// IconButton::new("btn", "🔊")
///     .variant(IconButtonVariant::Ghost)
///     .on_click(|window, cx| { /* handle click */ })
///
/// // With custom element (e.g., SVG icon)
/// IconButton::with_child("btn", my_svg_icon)
///     .size(IconButtonSize::Lg)
///     .rounded_full()
///     .theme(my_theme)
/// ```
pub struct IconButton {
    id: ElementId,
    content: IconContent,
    size: IconButtonSize,
    variant: IconButtonVariant,
    disabled: bool,
    selected: bool,
    rounded_full: bool,
    padding: Option<Pixels>,
    theme: Option<IconButtonTheme>,
    on_click: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl IconButton {
    /// Create a new icon button with a text/emoji icon
    pub fn new(id: impl Into<ElementId>, icon: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            content: IconContent::Text(icon.into()),
            size: IconButtonSize::default(),
            variant: IconButtonVariant::default(),
            disabled: false,
            selected: false,
            rounded_full: false,
            padding: None,
            theme: None,
            on_click: None,
        }
    }

    /// Create a new icon button with a custom child element (e.g., SVG icon)
    pub fn with_child(id: impl Into<ElementId>, child: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            content: IconContent::Element(child.into_any_element()),
            size: IconButtonSize::default(),
            variant: IconButtonVariant::default(),
            disabled: false,
            selected: false,
            rounded_full: false,
            padding: None,
            theme: None,
            on_click: None,
        }
    }

    /// Set the button size
    pub fn size(mut self, size: IconButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Set the button variant
    pub fn variant(mut self, variant: IconButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set selected state
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set click handler
    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Use fully rounded corners (circular button)
    pub fn rounded_full(mut self) -> Self {
        self.rounded_full = true;
        self
    }

    /// Set custom padding (overrides default size-based padding)
    pub fn padding(mut self, padding: Pixels) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Set the button theme
    pub fn theme(mut self, theme: IconButtonTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Get the computed colors based on variant and state
    pub fn compute_colors(
        &self,
        fallback_theme: &IconButtonTheme,
    ) -> (Rgba, Rgba, Rgba, Option<Rgba>) {
        let theme = self.theme.as_ref().unwrap_or(fallback_theme);

        match self.variant {
            IconButtonVariant::Ghost => {
                if self.selected {
                    (
                        theme.selected_bg,
                        theme.selected_hover_bg,
                        theme.text_on_accent,
                        None,
                    )
                } else {
                    (theme.ghost_bg, theme.ghost_hover_bg, theme.text, None)
                }
            }
            IconButtonVariant::Filled => {
                if self.selected {
                    (theme.accent, theme.accent_hover, theme.text_on_accent, None)
                } else {
                    (theme.filled_bg, theme.filled_hover_bg, theme.text, None)
                }
            }
            IconButtonVariant::Outline => {
                if self.selected {
                    (
                        theme.selected_bg,
                        theme.selected_hover_bg,
                        theme.text_on_accent,
                        Some(theme.accent),
                    )
                } else {
                    (
                        theme.ghost_bg,
                        theme.ghost_hover_bg,
                        theme.text,
                        Some(theme.border),
                    )
                }
            }
        }
    }

    /// Build into element with theme
    pub fn build_with_theme(self, icon_theme: &IconButtonTheme) -> Stateful<Div> {
        let size = self.size.size();
        let (bg, bg_hover, text_color, border) = self.compute_colors(icon_theme);

        let mut el = div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_center()
            .w(size)
            .h(size)
            .bg(bg)
            .text_color(text_color)
            .cursor_pointer();

        // Apply padding if specified
        if let Some(padding) = self.padding {
            el = el.p(padding);
        }

        // Apply rounding
        if self.rounded_full {
            el = el.rounded_full();
        } else {
            el = el.rounded_md();
        }

        if let Some(border_color) = border {
            el = el.border_1().border_color(border_color);
        }

        if self.disabled {
            el = el.opacity(0.5).cursor_not_allowed();
        } else {
            el = el.hover(move |style| style.bg(bg_hover).shadow(glow_shadow(bg_hover)));

            if let Some(handler) = self.on_click {
                el = el.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                    handler(window, cx);
                });
            }
        }

        // Add content
        match self.content {
            IconContent::Text(text) => el.child(text),
            IconContent::Element(element) => el.child(element),
        }
    }
}

impl RenderOnce for IconButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let icon_theme = IconButtonTheme::from(&global_theme);
        self.build_with_theme(&icon_theme)
    }
}

impl IntoElement for IconButton {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
