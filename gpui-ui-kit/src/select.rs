//! Select/Dropdown component
//!
//! A dropdown select component for choosing from options with theming support.
//!
//! Features:
//! - Keyboard navigation:
//!   - Arrow Up/Down: navigate options
//!   - Enter: select highlighted option
//!   - Escape: close dropdown
//!   - Space: toggle dropdown open/closed
//! - Mouse support: click to toggle, hover to highlight
//!
//! Note: Uses `deferred()` to ensure dropdown renders on top of other content.

use gpui::prelude::*;
use gpui::{deferred, *};

use crate::ComponentTheme;
use crate::theme::ThemeExt;

/// Theme colors for select styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct SelectTheme {
    /// Trigger background color
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub trigger_bg: Rgba,
    /// Trigger border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub trigger_border: Rgba,
    /// Trigger border color on hover
    #[theme(default = 0x007accff, from = accent)]
    pub trigger_border_hover: Rgba,
    /// Trigger border color when focused/open
    #[theme(default = 0x007accff, from = accent)]
    pub trigger_border_focused: Rgba,
    /// Dropdown background color
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub dropdown_bg: Rgba,
    /// Dropdown border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub dropdown_border: Rgba,
    /// Selected option background
    #[theme(default = 0x007accff, from = accent)]
    pub selected_bg: Rgba,
    /// Option hover background
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub option_hover_bg: Rgba,
    /// Label text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub label_color: Rgba,
    /// Text color for selected value
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_color: Rgba,
    /// Placeholder text color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub placeholder_color: Rgba,
    /// Option text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub option_text_color: Rgba,
    /// Selected option text color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub selected_text_color: Rgba,
    /// Disabled text color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub disabled_color: Rgba,
    /// Arrow/chevron color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub arrow_color: Rgba,
}

/// Select size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectSize {
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
}

impl From<crate::ComponentSize> for SelectSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// A select option
#[derive(Clone)]
pub struct SelectOption {
    /// Option value
    pub value: SharedString,
    /// Display label
    pub label: SharedString,
    /// Whether option is disabled
    pub disabled: bool,
}

impl SelectOption {
    /// Create a new select option
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A select dropdown component with theming support
pub struct Select {
    id: ElementId,
    options: Vec<SelectOption>,
    selected: Option<SharedString>,
    placeholder: Option<SharedString>,
    label: Option<SharedString>,
    size: SelectSize,
    disabled: bool,
    is_open: bool,
    highlighted_index: Option<usize>,
    theme: Option<SelectTheme>,
    on_change: Option<Box<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
    on_toggle: Option<Box<dyn Fn(bool, &mut Window, &mut App) + 'static>>,
    on_highlight: Option<Box<dyn Fn(Option<usize>, &mut Window, &mut App) + 'static>>,
}

impl Select {
    /// Create a new select
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
            selected: None,
            placeholder: None,
            label: None,
            size: SelectSize::default(),
            disabled: false,
            is_open: false,
            highlighted_index: None,
            theme: None,
            on_change: None,
            on_toggle: None,
            on_highlight: None,
        }
    }

    /// Set options
    pub fn options(mut self, options: Vec<SelectOption>) -> Self {
        self.options = options;
        self
    }

    /// Set selected value
    pub fn selected(mut self, value: impl Into<SharedString>) -> Self {
        self.selected = Some(value.into());
        self
    }

    /// Set placeholder
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set label
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set size
    pub fn size(mut self, size: SelectSize) -> Self {
        self.size = size;
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set open state (for controlled component)
    pub fn is_open(mut self, is_open: bool) -> Self {
        self.is_open = is_open;
        self
    }

    /// Set highlighted index (for keyboard navigation)
    pub fn highlighted_index(mut self, index: Option<usize>) -> Self {
        self.highlighted_index = index;
        self
    }

    /// Set theme
    pub fn theme(mut self, theme: SelectTheme) -> Self {
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

    /// Set toggle handler (called when trigger is clicked)
    pub fn on_toggle(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Box::new(handler));
        self
    }

    /// Set highlight handler (called when highlighted option changes during keyboard navigation)
    pub fn on_highlight(
        mut self,
        handler: impl Fn(Option<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_highlight = Some(Box::new(handler));
        self
    }

    /// Build into element
    fn build(self, theme: &SelectTheme) -> Div {
        let (py, _text_size_class) = match self.size {
            SelectSize::Sm => (px(4.0), "sm"),
            SelectSize::Md => (px(8.0), "md"),
            SelectSize::Lg => (px(12.0), "lg"),
        };

        let mut container = div().relative().flex().flex_col().gap_1();

        // Label
        if let Some(label) = self.label {
            container = container.child(
                div()
                    .text_sm()
                    .text_color(theme.label_color)
                    .font_weight(FontWeight::MEDIUM)
                    .child(label),
            );
        }

        // Find selected option label
        let selected_label = self.selected.as_ref().and_then(|val| {
            self.options
                .iter()
                .find(|o| &o.value == val)
                .map(|o| o.label.clone())
        });

        // Select trigger
        let border_color = if self.is_open {
            theme.trigger_border_focused
        } else {
            theme.trigger_border
        };

        // Clone ID for use in dropdown (self.id is moved to trigger)
        let dropdown_id = self.id.clone();

        let mut trigger = div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .py(py)
            .min_w(px(120.0))
            .bg(theme.trigger_bg)
            .border_1()
            .border_color(border_color)
            .rounded_md()
            .cursor_pointer()
            .focusable();

        // Apply text size
        trigger = match self.size {
            SelectSize::Sm => trigger.text_xs(),
            SelectSize::Md => trigger.text_sm(),
            SelectSize::Lg => trigger,
        };

        // Convert handlers to Rc upfront so we can use them in closures
        let on_toggle_rc = self.on_toggle.map(std::rc::Rc::new);
        let on_change_rc = self.on_change.map(std::rc::Rc::new);
        let on_highlight_rc = self.on_highlight.map(std::rc::Rc::new);

        let currently_open = self.is_open;
        let num_options = self.options.len();
        let current_highlight = self.highlighted_index;

        if self.disabled {
            trigger = trigger.opacity(0.5).cursor_not_allowed();
        } else {
            let hover_border = theme.trigger_border_hover;
            trigger = trigger.hover(move |s| s.border_color(hover_border));

            // Mouse click handler - use on_mouse_down for more reliable response
            if let Some(ref handler) = on_toggle_rc {
                let handler = handler.clone();
                trigger = trigger.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    (handler)(!currently_open, window, cx);
                });
            }

            // Keyboard handler
            if let Some(ref toggle_handler) = on_toggle_rc {
                let toggle_rc = toggle_handler.clone();
                let change_rc = on_change_rc.clone();
                let highlight_rc = on_highlight_rc.clone();
                let options_clone = self.options.clone();

                trigger = trigger.on_key_down(move |event, window, cx| {
                    match event.keystroke.key.as_str() {
                        "space" | " " => {
                            // Toggle open/closed
                            toggle_rc(!currently_open, window, cx);
                        }
                        "escape" if currently_open => {
                            // Close dropdown
                            toggle_rc(false, window, cx);
                        }
                        "enter" if currently_open => {
                            // Select highlighted option
                            if let Some(idx) = current_highlight
                                && idx < options_clone.len()
                                && !options_clone[idx].disabled
                            {
                                if let Some(ref change_handler) = change_rc {
                                    change_handler(&options_clone[idx].value, window, cx);
                                }
                                toggle_rc(false, window, cx);
                            }
                        }
                        "down" | "up" if currently_open => {
                            // Navigate options
                            let delta = if event.keystroke.key == "down" {
                                1
                            } else {
                                -1_i32
                            };
                            let new_idx = if let Some(idx) = current_highlight {
                                let new = idx as i32 + delta;
                                if new < 0 {
                                    Some(num_options.saturating_sub(1))
                                } else if new >= num_options as i32 {
                                    Some(0)
                                } else {
                                    Some(new as usize)
                                }
                            } else {
                                // No highlight yet, start at first/last
                                if delta > 0 {
                                    Some(0)
                                } else {
                                    Some(num_options.saturating_sub(1))
                                }
                            };

                            if let Some(ref highlight_handler) = highlight_rc {
                                highlight_handler(new_idx, window, cx);
                            }
                        }
                        _ => {}
                    }
                });
            }
        }

        // Display value or placeholder
        let display_text = if let Some(label) = selected_label {
            div().text_color(theme.text_color).child(label)
        } else if let Some(placeholder) = self.placeholder {
            div().text_color(theme.placeholder_color).child(placeholder)
        } else {
            div().text_color(theme.placeholder_color).child("Select...")
        };

        trigger = trigger.child(display_text);

        // Dropdown arrow
        trigger = trigger.child(div().text_xs().text_color(theme.arrow_color).child("▼"));

        container = container.child(trigger);

        // Dropdown menu (only shown when open)
        // Use deferred() to ensure the dropdown renders on top of other content
        if self.is_open {
            let mut dropdown = div()
                .id((dropdown_id, "dropdown"))
                .absolute()
                .top_full()
                .left_0()
                .min_w_full() // Ensure dropdown is at least as wide as trigger
                .mt_1()
                .bg(theme.dropdown_bg)
                .border_1()
                .border_color(theme.dropdown_border)
                .rounded_md()
                .shadow_lg()
                .max_h(px(200.0))
                .overflow_y_scroll()
                .py_1()
                .occlude(); // Block mouse events from passing through

            for (idx, option) in self.options.iter().enumerate() {
                let is_selected = self.selected.as_ref() == Some(&option.value);
                let is_highlighted = self.highlighted_index == Some(idx);
                let option_value = option.value.clone();

                let mut option_el = div()
                    .id(("select-option", idx))
                    .px_3()
                    .py(px(6.0))
                    .cursor_pointer();

                // Apply text size
                option_el = match self.size {
                    SelectSize::Sm => option_el.text_xs(),
                    SelectSize::Md => option_el.text_sm(),
                    SelectSize::Lg => option_el,
                };

                if option.disabled {
                    option_el = option_el
                        .bg(theme.dropdown_bg)
                        .text_color(theme.disabled_color)
                        .cursor_not_allowed();
                } else {
                    // Apply styling based on state
                    if is_selected {
                        option_el = option_el
                            .bg(theme.selected_bg)
                            .text_color(theme.selected_text_color);
                    } else if is_highlighted {
                        // Highlight option for keyboard navigation
                        option_el = option_el
                            .bg(theme.option_hover_bg)
                            .text_color(theme.option_text_color);
                    } else {
                        let hover_bg = theme.option_hover_bg;
                        option_el = option_el
                            .bg(theme.dropdown_bg)
                            .text_color(theme.option_text_color)
                            .hover(move |s| s.bg(hover_bg));
                    }

                    // Add click handler for ALL non-disabled options
                    let change_handler = on_change_rc.clone();
                    let toggle_handler = on_toggle_rc.clone();
                    option_el =
                        option_el.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                            // Call change handler if provided
                            if let Some(ref handler) = change_handler {
                                handler(&option_value, window, cx);
                            }
                            // Close the dropdown
                            if let Some(ref handler) = toggle_handler {
                                handler(false, window, cx);
                            }
                        });
                }

                option_el = option_el.child(option.label.clone());
                dropdown = dropdown.child(option_el);
            }

            // Wrap dropdown in deferred() with priority to render on top of other elements
            container = container.child(deferred(dropdown).with_priority(1));
        }

        container
    }
}

impl RenderOnce for Select {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| SelectTheme::from(&global_theme));

        self.build(&theme)
    }
}

impl IntoElement for Select {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        // When used without context, fall back to default theme
        let theme = self.theme.clone().unwrap_or_default();
        self.build(&theme)
    }
}
