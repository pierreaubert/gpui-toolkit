//! `ButtonSet` component - A group of mutually exclusive buttons
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
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState};
use crate::theme::{ThemeExt, glow_shadow};
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    AnyElement, App, Div, ElementId, MouseButton, Rgba, SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::sync::Arc;

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
    /// Text color for selected button (on accent background)
    #[theme(default = 0xffffffff, from = text_on_accent)]
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
#[derive(IntoElement)]
pub struct ButtonSet {
    id: ElementId,
    options: Vec<ButtonSetOption>,
    selected: Option<SharedString>,
    size: ButtonSetSize,
    disabled: bool,
    theme: Option<ButtonSetTheme>,
    on_change: Option<Box<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
    option_wrapper: Option<Box<dyn Fn(usize, Stateful<Div>) -> AnyElement>>,
    design: Option<Arc<DesignSystem>>,
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
            option_wrapper: None,
            design: None,
        }
    }

    /// Set the design system (falls back to the app-global design when unset)
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
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

    /// Wrap rendered options without changing their interaction or focus behavior.
    ///
    /// The index corresponds to the option's position in `options`. Hosts can
    /// attach observation or test instrumentation to individual segments.
    pub fn option_wrapper(
        mut self,
        wrapper: impl Fn(usize, Stateful<Div>) -> AnyElement + 'static,
    ) -> Self {
        self.option_wrapper = Some(Box::new(wrapper));
        self
    }

    /// Build into element
    fn build(self, theme: &ButtonSetTheme, design: &DesignSystem, cx: &mut App) -> Stateful<Div> {
        let (px_val, py_val, text_size) = match self.size {
            ButtonSetSize::Xs => (
                px(design.spacing.grid_unit * 1.5),
                px(design.spacing.grid_unit * 0.5),
                px(design.typography.small_size),
            ),
            ButtonSetSize::Sm => (
                px(design.spacing.control_gap),
                px(design.spacing.grid_unit),
                px(design.typography.base_size),
            ),
            ButtonSetSize::Md => (
                px(design.spacing.control_padding_x),
                px(design.spacing.grid_unit * 1.5),
                px(design.typography.base_size),
            ),
            ButtonSetSize::Lg => (
                px(design.spacing.section_gap),
                px(design.spacing.control_padding_y),
                px(design.typography.large_size),
            ),
        };

        let border_radius = match self.size {
            ButtonSetSize::Xs => px(design.corners.sm),
            ButtonSetSize::Sm => px(design.corners.sm),
            ButtonSetSize::Md => px(design.corners.sm * 1.5),
            ButtonSetSize::Lg => px(design.corners.md),
        };

        let on_change_rc = self.on_change.map(std::rc::Rc::new);
        let num_options = self.options.len();

        let group_id = self.id.clone();
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

            let option_id: ElementId =
                SharedString::from(format!("{group_id:?}-option-{idx}")).into();
            let focus =
                crate::Button::focus_handle_for(option_id.clone(), cx).tab_stop(!is_disabled);
            cx.register_accessible(AccessibilityNode {
                element_id: option_id.clone(),
                label: option.label.clone(),
                props: AriaProps::with_role(AriaRole::Radio)
                    .state(AriaState::Checked(is_selected))
                    .maybe_state(is_disabled, AriaState::Disabled),
            });
            let focus_color = theme.border_selected;
            let mut button = div()
                .id(option_id)
                .key_context("ToolkitButton")
                .track_focus_element(&focus)
                .on_key_down(|event, window, cx| {
                    if event.keystroke.key == "tab" {
                        if event.keystroke.modifiers.shift {
                            window.focus_prev(cx);
                        } else {
                            window.focus_next(cx);
                        }
                        cx.stop_propagation();
                    }
                })
                .focus_visible(move |style| style.border_2().border_color(focus_color))
                .flex_1() // Equal width for all buttons
                .flex()
                .items_center()
                .justify_center()
                .gap(px(design.spacing.grid_unit))
                .px(px_val)
                .py(py_val)
                .bg(bg)
                .text_color(text_color)
                .text_size(text_size)
                .cursor_pointer();

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
                    let mouse_handler = handler.clone();
                    let key_handler = handler.clone();
                    let key_value = option_value.clone();
                    button = button
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            focus.focus(window, cx);
                            mouse_handler(&option_value, window, cx);
                            cx.stop_propagation();
                        })
                        .on_key_down(move |event, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                key_handler(&key_value, window, cx);
                                cx.stop_propagation();
                            }
                        });
                }
            }

            // Add icon if present
            if let Some(icon) = option.icon {
                button = button.child(icon);
            }

            // Add label
            button = button.child(option.label);

            container = container.child(if let Some(wrapper) = &self.option_wrapper {
                wrapper(idx, button)
            } else {
                button.into_any_element()
            });
        }

        container
    }
}

impl RenderOnce for ButtonSet {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let mut this = self;
        let theme = this
            .theme
            .take()
            .unwrap_or_else(|| ButtonSetTheme::from(global_theme.as_ref()));
        let design = crate::design::resolve_design(this.design.clone(), cx);

        this.build(&theme, &design, cx)
    }
}
