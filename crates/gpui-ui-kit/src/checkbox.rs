//! Checkbox component
//!
//! A checkbox input with optional label.
//!
//! Features:
//! - Keyboard support: Space or Enter to toggle
//! - Mouse support: click to toggle
//! - Indeterminate state support

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::*;
use gpui::*;

/// Theme colors for checkbox styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct CheckboxTheme {
    /// Background when checked
    #[theme(default = 0x007acc, from = accent)]
    pub checked_bg: Rgba,
    /// Background when unchecked (transparent)
    #[theme(default = 0x00000000, from = transparent)]
    pub unchecked_bg: Rgba,
    /// Border when unchecked
    #[theme(default = 0x555555, from = border)]
    pub unchecked_border: Rgba,
    /// Check mark color
    #[theme(default = 0xffffff, from = text_primary)]
    pub check_color: Rgba,
    /// Label color
    #[theme(default = 0xcccccc, from = text_secondary)]
    pub label: Rgba,
    /// Hover border color
    #[theme(default = 0x007acc, from = accent)]
    pub hover_border: Rgba,
}

/// Checkbox size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckboxSize {
    /// Small (14px)
    Sm,
    /// Medium (18px, default)
    #[default]
    Md,
    /// Large (22px)
    Lg,
}

impl CheckboxSize {
    fn size(&self) -> Pixels {
        match self {
            CheckboxSize::Sm => px(14.0),
            CheckboxSize::Md => px(18.0),
            CheckboxSize::Lg => px(22.0),
        }
    }
}

impl From<crate::ComponentSize> for CheckboxSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// A checkbox component
pub struct Checkbox {
    id: ElementId,
    checked: bool,
    indeterminate: bool,
    label: Option<SharedString>,
    size: CheckboxSize,
    disabled: bool,
    on_change: Option<Box<dyn Fn(bool, &mut Window, &mut App) + 'static>>,
}

impl Checkbox {
    /// Create a new checkbox
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            indeterminate: false,
            label: None,
            size: CheckboxSize::default(),
            disabled: false,
            on_change: None,
        }
    }

    /// Set checked state
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set indeterminate state
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    /// Set label
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set size
    pub fn size(mut self, size: CheckboxSize) -> Self {
        self.size = size;
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set change handler
    pub fn on_change(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &CheckboxTheme) -> Stateful<Div> {
        let size = self.size.size();
        let checked = self.checked;
        let indeterminate = self.indeterminate;

        let (bg, border_color) = if checked || indeterminate {
            (theme.checked_bg, theme.checked_bg)
        } else {
            (theme.unchecked_bg, theme.unchecked_border)
        };

        let mut container = div()
            .id(self.id)
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer();

        if self.disabled {
            container = container.opacity(0.5).cursor_not_allowed();
        }

        // Checkbox box
        let mut checkbox = div()
            .flex()
            .items_center()
            .justify_center()
            .w(size)
            .h(size)
            .rounded(px(3.0))
            .border_1()
            .border_color(border_color)
            .bg(bg);

        // Check mark or indeterminate line
        if indeterminate {
            checkbox = checkbox.child(
                div()
                    .w(size - px(6.0))
                    .h(px(2.0))
                    .bg(theme.check_color)
                    .rounded(px(1.0)),
            );
        } else if checked {
            checkbox = checkbox.child(
                div()
                    .text_color(theme.check_color)
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .child("✓"),
            );
        }

        if !self.disabled {
            let hover_border = theme.hover_border;
            checkbox = checkbox.hover(move |s| s.border_color(hover_border));
        }

        container = container.child(checkbox);

        // Label
        if let Some(label) = self.label {
            let label_el = match self.size {
                CheckboxSize::Sm => div().text_xs(),
                CheckboxSize::Md => div().text_sm(),
                CheckboxSize::Lg => div(),
            };
            container = container.child(label_el.text_color(theme.label).child(label));
        }

        // Event handlers
        if !self.disabled
            && let Some(handler) = self.on_change
        {
            let handler_rc = std::rc::Rc::new(handler);
            let new_checked = !checked;

            // Mouse click handler
            let click_handler = handler_rc.clone();
            container = container.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                click_handler(new_checked, window, cx);
            });

            // Keyboard handler (Space or Enter)
            let key_handler = handler_rc.clone();
            container = container.on_key_down(move |event, window, cx| {
                match event.keystroke.key.as_str() {
                    "space" | " " | "enter" => {
                        key_handler(new_checked, window, cx);
                    }
                    _ => {}
                }
            });
        }

        container
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let checkbox_theme = CheckboxTheme::from(&global_theme);
        self.build_with_theme(&checkbox_theme)
    }
}

impl IntoElement for Checkbox {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
