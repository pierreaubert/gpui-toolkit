use gpui::prelude::{InteractiveElement, IntoElement, RenderOnce, Styled};
use gpui::{App, Div, Pixels, Rgba, SharedString, Stateful, Window, div, px, rgb};

/// A divider line
pub struct Divider {
    pub(super) id: Option<SharedString>,
    pub(super) vertical: bool,
    pub(super) color: Option<Rgba>,
    pub(super) hover_color: Option<Rgba>,
    pub(super) thickness: Option<Pixels>,
    pub(super) interactive: bool,
}

impl Divider {
    /// Create a new horizontal divider
    pub fn new() -> Self {
        Self {
            id: None,
            vertical: false,
            color: None,
            hover_color: None,
            thickness: None,
            interactive: false,
        }
    }

    /// Create a vertical divider
    pub fn vertical() -> Self {
        Self {
            id: None,
            vertical: true,
            color: None,
            hover_color: None,
            thickness: None,
            interactive: false,
        }
    }

    /// Set an ID for the divider (required for interactive dividers)
    pub fn id(mut self, id: impl Into<SharedString>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set custom color
    pub fn color(mut self, color: Rgba) -> Self {
        self.color = Some(color);
        self
    }

    /// Set hover color (for interactive dividers)
    pub fn hover_color(mut self, color: Rgba) -> Self {
        self.hover_color = Some(color);
        self
    }

    /// Set custom thickness
    pub fn thickness(mut self, thickness: Pixels) -> Self {
        self.thickness = Some(thickness);
        self
    }

    /// Make this an interactive resize divider
    pub fn interactive(mut self) -> Self {
        self.interactive = true;
        self
    }

    /// Build into a stateful element that can have handlers attached
    /// Use this when you need to add event handlers (e.g., for resize dividers)
    pub fn build(self) -> Stateful<Div> {
        let color = self.color.unwrap_or(rgb(0x3a3a3a));
        let id = self.id.unwrap_or_else(|| SharedString::from("divider"));

        let base = if self.vertical {
            let thickness = self.thickness.unwrap_or(px(1.0));
            div().id(id).w(thickness).h_full().bg(color)
        } else {
            let thickness = self.thickness.unwrap_or(px(1.0));
            div().id(id).h(thickness).w_full().bg(color)
        };

        if self.interactive {
            let hover_color = self.hover_color.unwrap_or(rgb(0x007acc));
            let cursor = if self.vertical {
                gpui::CursorStyle::ResizeLeftRight
            } else {
                gpui::CursorStyle::ResizeUpDown
            };
            base.cursor(cursor)
                .hover(move |style| style.bg(hover_color))
        } else {
            base
        }
    }

    /// Get the resolved color for this divider given a theme.
    /// Returns the explicit color if set, otherwise theme.border.
    pub fn resolve_color(&self, theme: &crate::theme::Theme) -> Rgba {
        self.color.unwrap_or(theme.border)
    }

    /// Build into a stateful element using theme defaults
    pub fn build_with_theme(self, theme: &crate::theme::Theme) -> Stateful<Div> {
        let color = self.color.unwrap_or(theme.border);
        let id = self.id.unwrap_or_else(|| SharedString::from("divider"));

        let base = if self.vertical {
            let thickness = self.thickness.unwrap_or(px(1.0));
            div().id(id).w(thickness).h_full().bg(color)
        } else {
            let thickness = self.thickness.unwrap_or(px(1.0));
            div().id(id).h(thickness).w_full().bg(color)
        };

        if self.interactive {
            let hover_color = self.hover_color.unwrap_or(theme.accent);
            let cursor = if self.vertical {
                gpui::CursorStyle::ResizeLeftRight
            } else {
                gpui::CursorStyle::ResizeUpDown
            };
            base.cursor(cursor)
                .hover(move |style| style.bg(hover_color))
        } else {
            base
        }
    }

    /// Build into a non-stateful element (for simple visual dividers)
    pub fn build_simple(self) -> Div {
        let color = self.color.unwrap_or(rgb(0x3a3a3a));

        if self.vertical {
            let thickness = self.thickness.unwrap_or(px(1.0));
            div().w(thickness).h_full().bg(color)
        } else {
            let thickness = self.thickness.unwrap_or(px(1.0));
            div().h(thickness).w_full().bg(color)
        }
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoElement for Divider {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

impl RenderOnce for Divider {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = crate::theme::ThemeExt::theme(cx);
        self.build_with_theme(&theme)
    }
}
