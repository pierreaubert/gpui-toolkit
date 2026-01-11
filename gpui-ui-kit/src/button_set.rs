//! ButtonSet component - A group of mutually exclusive buttons
//!
//! Provides a segmented control / button group where only one button can be selected at a time.
//! Buttons are visually connected with rounded corners only on the first and last buttons.
//!
//! # Example
//!
//! ```ignore
//! ButtonSet::new("view-mode")
//!     .options(vec![
//!         ButtonSetOption::new("list", "List"),
//!         ButtonSetOption::new("grid", "Grid"),
//!         ButtonSetOption::new("table", "Table"),
//!     ])
//!     .selected("grid")
//!     .on_change(|value, window, cx| {
//!         println!("Selected: {}", value);
//!     })
//! ```

use crate::ComponentTheme;
use crate::theme::{ThemeExt, glow_shadow};
use gpui::prelude::*;
use gpui::*;

/// Theme colors for button set styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct ButtonSetTheme {
    /// Background color for unselected buttons
    #[theme(default = 0x3c3c3cff, from = surface)]
    pub bg: Rgba,
    /// Background color on hover
    #[theme(default = 0x4a4a4aff, from = surface_hover)]
    pub bg_hover: Rgba,
    /// Background color for selected button
    #[theme(default = 0x007accff, from = accent)]
    pub bg_selected: Rgba,
    /// Text color for unselected buttons
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text_color: Rgba,
    /// Text color for selected button
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_color_selected: Rgba,
    /// Border color
    #[theme(default = 0x555555ff, from = border)]
    pub border: Rgba,
    /// Border color for selected button
    #[theme(default = 0x007accff, from = accent)]
    pub border_selected: Rgba,
}

/// Button set size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSetSize {
    /// Extra small
    Xs,
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
}

impl From<crate::ComponentSize> for ButtonSetSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs => Self::Xs,
            crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// An option in the button set
#[derive(Clone)]
pub struct ButtonSetOption {
    /// Option value (used for selection)
    pub value: SharedString,
    /// Display label
    pub label: SharedString,
    /// Optional icon (displayed before label)
    pub icon: Option<SharedString>,
    /// Whether this option is disabled
    pub disabled: bool,
}

impl ButtonSetOption {
    /// Create a new button set option
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            icon: None,
            disabled: false,
        }
    }

    /// Add an icon to the option
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A group of mutually exclusive buttons (segmented control)
pub struct ButtonSet {
    id: ElementId,
    options: Vec<ButtonSetOption>,
    selected: Option<SharedString>,
    size: ButtonSetSize,
    disabled: bool,
    theme: Option<ButtonSetTheme>,
    on_change: Option<Box<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
}

impl ButtonSet {
    /// Create a new button set
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
            selected: None,
            size: ButtonSetSize::default(),
            disabled: false,
            theme: None,
            on_change: None,
        }
    }

    /// Set the options
    pub fn options(mut self, options: Vec<ButtonSetOption>) -> Self {
        self.options = options;
        self
    }

    /// Set the selected value
    pub fn selected(mut self, value: impl Into<SharedString>) -> Self {
        self.selected = Some(value.into());
        self
    }

    /// Set the size
    pub fn size(mut self, size: ButtonSetSize) -> Self {
        self.size = size;
        self
    }

    /// Disable the entire button set
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set custom theme
    pub fn theme(mut self, theme: ButtonSetTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set change handler
    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Build into element
    fn build(self, theme: &ButtonSetTheme) -> Stateful<Div> {
        let (px_val, py_val, text_size) = match self.size {
            ButtonSetSize::Xs => (px(6.0), px(2.0), "xs"),
            ButtonSetSize::Sm => (px(8.0), px(4.0), "sm"),
            ButtonSetSize::Md => (px(12.0), px(6.0), "md"),
            ButtonSetSize::Lg => (px(16.0), px(8.0), "lg"),
        };

        let border_radius = match self.size {
            ButtonSetSize::Xs => px(4.0),
            ButtonSetSize::Sm => px(4.0),
            ButtonSetSize::Md => px(6.0),
            ButtonSetSize::Lg => px(8.0),
        };

        let on_change_rc = self.on_change.map(std::rc::Rc::new);
        let num_options = self.options.len();

        let mut container = div()
            .id(self.id)
            .flex()
            .flex_row()
            .border_1()
            .border_color(theme.border)
            .rounded(border_radius);

        for (idx, option) in self.options.into_iter().enumerate() {
            let is_first = idx == 0;
            let is_last = idx == num_options - 1;
            let is_selected = self.selected.as_ref() == Some(&option.value);
            let is_disabled = self.disabled || option.disabled;
            let option_value = option.value.clone();

            // Determine colors based on state
            let (bg, text_color) = if is_selected {
                (theme.bg_selected, theme.text_color_selected)
            } else {
                (theme.bg, theme.text_color)
            };

            let mut button = div()
                .id(("buttonset", idx))
                .flex_1() // Equal width for all buttons
                .flex()
                .items_center()
                .justify_center()
                .gap_1()
                .px(px_val)
                .py(py_val)
                .bg(bg)
                .text_color(text_color)
                .cursor_pointer();

            // Apply text size
            button = match text_size {
                "xs" => button.text_xs(),
                "sm" => button.text_sm(),
                "lg" => button.text_lg(),
                _ => button.text_sm(),
            };

            // Apply border radius only to first and last buttons
            if is_first && is_last {
                // Single button - round all corners (but slightly less due to container)
                button = button.rounded(border_radius - px(1.0));
            } else if is_first {
                // First button - round left corners only
                button = button.rounded_l(border_radius - px(1.0)).rounded_r_none();
            } else if is_last {
                // Last button - round right corners only
                button = button.rounded_r(border_radius - px(1.0)).rounded_l_none();
            } else {
                // Middle buttons - no rounding
                button = button.rounded_none();
            }

            // Add border between buttons (not on last)
            if !is_last {
                button = button.border_r_1().border_color(theme.border);
            }

            // Handle disabled state
            if is_disabled {
                button = button.opacity(0.5).cursor_not_allowed();
            } else {
                // Hover effect (only for non-selected buttons)
                if !is_selected {
                    let hover_bg = theme.bg_hover;
                    button =
                        button.hover(move |style| style.bg(hover_bg).shadow(glow_shadow(hover_bg)));
                }

                // Click handler
                if let Some(ref handler) = on_change_rc {
                    let handler = handler.clone();
                    button = button.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        handler(&option_value, window, cx);
                    });
                }
            }

            // Add icon if present
            if let Some(icon) = option.icon {
                button = button.child(icon);
            }

            // Add label
            button = button.child(option.label);

            container = container.child(button);
        }

        container
    }
}

impl RenderOnce for ButtonSet {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| ButtonSetTheme::from(&global_theme));

        self.build(&theme)
    }
}

impl IntoElement for ButtonSet {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let theme = self.theme.clone().unwrap_or_default();
        self.build(&theme)
    }
}
