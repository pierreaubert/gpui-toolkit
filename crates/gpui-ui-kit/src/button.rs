//! Button component with variants and sizes
//!
//! Provides a flexible button component with different visual styles.

use crate::ComponentTheme;
use crate::theme::{ThemeExt, glow_shadow};
use gpui::prelude::*;
use gpui::*;

/// Button visual variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

/// Button size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    /// Extra small button
    Xs,
    /// Small button
    Sm,
    /// Medium button (default)
    #[default]
    Md,
    /// Large button
    Lg,
}

impl From<crate::ComponentSize> for ButtonSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs => Self::Xs,
            crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
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
    #[theme(default = 0xffffff, from = text_primary)]
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

/// A styled button component
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    selected: bool,
    full_width: bool,
    icon_left: Option<SharedString>,
    icon_right: Option<SharedString>,
    theme: Option<ButtonTheme>,
    on_click: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl Button {
    /// Create a new button with a label
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::default(),
            size: ButtonSize::default(),
            disabled: false,
            selected: false,
            full_width: false,
            icon_left: None,
            icon_right: None,
            theme: None,
            on_click: None,
        }
    }

    /// Set the button variant
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the button size
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Disable the button
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set button selected state (for toggle buttons)
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Make button full width
    pub fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    /// Add an icon to the left of the label
    pub fn icon_left(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon_left = Some(icon.into());
        self
    }

    /// Add an icon to the right of the label
    pub fn icon_right(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon_right = Some(icon.into());
        self
    }

    /// Set custom theme colors
    pub fn theme(mut self, theme: ButtonTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set the click handler (for standalone use without cx.listener)
    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Compute colors based on variant and selected state
    /// Returns (bg, bg_hover, text_color, border_color)
    fn compute_colors(
        variant: ButtonVariant,
        selected: bool,
        theme: &ButtonTheme,
    ) -> (Rgba, Rgba, Rgba, Rgba) {
        if selected {
            (
                theme.accent,
                theme.accent_hover,
                theme.text_on_accent,
                theme.accent,
            )
        } else {
            match variant {
                ButtonVariant::Primary => (
                    theme.accent,
                    theme.accent_hover,
                    theme.text_on_accent,
                    theme.accent,
                ),
                ButtonVariant::Secondary => (
                    theme.surface,
                    theme.surface_hover,
                    theme.text_secondary,
                    theme.surface,
                ),
                ButtonVariant::Destructive => (
                    theme.error,
                    theme.error_hover,
                    theme.text_on_accent,
                    theme.error,
                ),
                ButtonVariant::Ghost => (
                    theme.transparent,
                    theme.surface_hover,
                    theme.text_secondary,
                    theme.transparent,
                ),
                ButtonVariant::Outline => (
                    theme.transparent,
                    theme.surface,
                    theme.text_secondary,
                    theme.border,
                ),
            }
        }
    }

    /// Build the button into a Stateful<Div> that can have additional handlers added
    /// Use this when you need to add a cx.listener() handler
    pub fn build(self) -> Stateful<Div> {
        let theme = self.theme.unwrap_or_default();
        let (bg, bg_hover, text_color, border_color) =
            Self::compute_colors(self.variant, self.selected, &theme);

        let (px_val, py_val) = match self.size {
            ButtonSize::Xs => (px(6.0), px(2.0)),
            ButtonSize::Sm => (px(8.0), px(4.0)),
            ButtonSize::Md => (px(12.0), px(6.0)),
            ButtonSize::Lg => (px(24.0), px(12.0)),
        };

        let mut el = div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .px(px_val)
            .py(py_val)
            .rounded_md()
            .bg(bg)
            .text_color(text_color)
            .border_1()
            .border_color(border_color);

        // Apply text size based on button size
        el = match self.size {
            ButtonSize::Xs => el.text_xs(),
            ButtonSize::Sm => el.text_xs(),
            ButtonSize::Md => el.text_sm(),
            ButtonSize::Lg => el.text_lg(),
        };

        // Apply full width
        if self.full_width {
            el = el.w_full();
        }

        if self.disabled {
            el = el.opacity(0.5).cursor_not_allowed();
        } else {
            el = el.cursor_pointer().hover(|style| style.bg(bg_hover));
        }

        // Add icon left
        if let Some(icon) = self.icon_left {
            el = el.child(icon);
        }

        // Add label
        el = el.child(self.label);

        // Add icon right
        if let Some(icon) = self.icon_right {
            el = el.child(icon);
        }

        el
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.unwrap_or_else(|| ButtonTheme::from(&cx.theme()));
        let (bg, bg_hover, text_color, border_color) =
            Self::compute_colors(self.variant, self.selected, &theme);

        let (px_val, py_val) = match self.size {
            ButtonSize::Xs => (px(6.0), px(2.0)),
            ButtonSize::Sm => (px(8.0), px(4.0)),
            ButtonSize::Md => (px(12.0), px(6.0)),
            ButtonSize::Lg => (px(24.0), px(12.0)),
        };

        let mut el = div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .px(px_val)
            .py(py_val)
            .rounded_md()
            .bg(bg)
            .text_color(text_color)
            .border_1()
            .border_color(border_color)
            .cursor_pointer();

        // Apply text size based on button size
        el = match self.size {
            ButtonSize::Xs => el.text_xs(),
            ButtonSize::Sm => el.text_xs(),
            ButtonSize::Md => el.text_sm(),
            ButtonSize::Lg => el.text_lg(),
        };

        // Apply full width
        if self.full_width {
            el = el.w_full();
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

        // Add icon left
        if let Some(icon) = self.icon_left {
            el = el.child(icon);
        }

        // Add label
        el = el.child(self.label);

        // Add icon right
        if let Some(icon) = self.icon_right {
            el = el.child(icon);
        }

        el
    }
}
