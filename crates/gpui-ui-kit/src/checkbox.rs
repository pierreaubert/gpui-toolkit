//! Checkbox component
//!
//! A checkbox input with optional label.
//!
//! Features:
//! - Keyboard support: Space or Enter to toggle
//! - Mouse support: click to toggle
//! - Indeterminate state support

use crate::ComponentTheme;
use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState, apply_native_accessibility,
};
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    App, Div, ElementId, FontWeight, MouseButton, Pixels, Rgba, SharedString, Stateful, Window,
    div, px,
};
use gpui_design::DesignSystem;
use std::sync::Arc;

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
    /// Check mark color (on accent background)
    #[theme(default = 0xffffff, from = text_on_accent)]
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
    fn size_with_design(&self, design: &DesignSystem) -> Pixels {
        match self {
            CheckboxSize::Sm => px(design.interaction.min_touch_target * 0.4375),
            CheckboxSize::Md => px(design.interaction.min_touch_target * 0.5625),
            CheckboxSize::Lg => px(design.interaction.min_touch_target * 0.6875),
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
    design: Option<Arc<DesignSystem>>,
    on_change: Option<Box<dyn Fn(bool, &mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
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
            design: None,
            on_change: None,
            aria_label: None,
            aria_role: None,
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

    /// Override the design system used for checkbox sizing and spacing.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set change handler
    pub fn on_change(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Set an explicit ARIA label
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Checkbox)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &CheckboxTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build into element with theme and design-system sizing tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &CheckboxTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let effective_label = self
            .aria_label
            .clone()
            .or_else(|| self.label.clone())
            .unwrap_or_default();
        let mut native_props = AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Checkbox))
            .maybe_state(self.disabled, AriaState::Disabled);
        native_props = if self.indeterminate {
            native_props.state(AriaState::Mixed)
        } else {
            native_props.state(AriaState::Checked(self.checked))
        };
        let size = self.size.size_with_design(design);
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
            .gap(px(design.spacing.control_gap))
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
            .rounded(px(design.corners.sm))
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
                    .rounded(px(design.corners.sm * 0.5)),
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

            // AccessKit actions do not synthesize mouse events for handlers
            // registered with on_mouse_up, so wire the screen-reader action
            // explicitly alongside the existing mouse and keyboard paths.
            let a11y_handler = handler_rc.clone();
            container = container.on_a11y_action(
                gpui::AccessibleAction::Click,
                move |_data, window, cx| {
                    a11y_handler(new_checked, window, cx);
                },
            );

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

        apply_native_accessibility(container, effective_label, &native_props)
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Register in accessibility tree
        let effective_label = self
            .aria_label
            .clone()
            .or_else(|| self.label.clone())
            .unwrap_or_default();
        let role = self.aria_role.unwrap_or(AriaRole::Checkbox);
        let mut props = AriaProps::with_role(role).maybe_state(self.disabled, AriaState::Disabled);
        if self.indeterminate {
            props = props.state(AriaState::Mixed);
        } else {
            props = props.state(AriaState::Checked(self.checked));
        }
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: effective_label,
            props,
        });

        let global_theme = cx.theme();
        let checkbox_theme = CheckboxTheme::from(global_theme);
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&checkbox_theme, &design)
    }
}

impl IntoElement for Checkbox {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
