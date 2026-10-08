//! `DateRangeInput` component
//!
//! A date-time field showing a start and end date. Bounds are
//! parent-owned props; the field itself offers no calendar popup or
//! interactive range selection, only display, an optional clear
//! affordance, and change callbacks.
//!
//! # Usage
//!
//! ```ignore
//! DateRangeInput::new("stay")
//!     .start(CalendarDate::new(2026, 10, 1).unwrap())
//!     .end(CalendarDate::new(2026, 10, 7).unwrap())
//!     .on_change(|start, end, _window, _cx| { /* store bounds */ })
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState, apply_native_accessibility,
};
use crate::date_picker::CalendarDate;
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Div, ElementId, KeyDownEvent, MouseButton, Rgba, SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::rc::Rc;
use std::sync::Arc;

/// Change handler receiving the new bounds.
type DateRangeChange =
    Rc<dyn Fn(Option<CalendarDate>, Option<CalendarDate>, &mut Window, &mut App) + 'static>;

/// Theme colors for the date range field.
#[derive(Debug, Clone, ComponentTheme)]
pub struct DateRangeInputTheme {
    /// Field background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Field border.
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Entered date color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text: Rgba,
    /// Placeholder color for missing bounds.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub placeholder: Rgba,
}

/// Date range field size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DateRangeInputSize {
    /// Compact field.
    Sm,
    /// Standard field (default).
    #[default]
    Md,
    /// Large field.
    Lg,
}

impl From<crate::ComponentSize> for DateRangeInputSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Date range field without range selection.
#[derive(IntoElement)]
pub struct DateRangeInput {
    id: ElementId,
    start: Option<CalendarDate>,
    end: Option<CalendarDate>,
    placeholder: SharedString,
    size: DateRangeInputSize,
    disabled: bool,
    clearable: bool,
    on_change: Option<DateRangeChange>,
    theme: Option<DateRangeInputTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl DateRangeInput {
    /// Create a new empty range field.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            start: None,
            end: None,
            placeholder: SharedString::from("Select dates"),
            size: DateRangeInputSize::default(),
            disabled: false,
            clearable: false,
            on_change: None,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the range start bound.
    pub fn start(mut self, start: CalendarDate) -> Self {
        self.start = Some(start);
        self
    }

    /// Set the range end bound.
    pub fn end(mut self, end: CalendarDate) -> Self {
        self.end = Some(end);
        self
    }

    /// Set the placeholder for missing bounds.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the field size.
    pub fn size(mut self, size: DateRangeInputSize) -> Self {
        self.size = size;
        self
    }

    /// Disable the field.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Show a clear affordance when bounds exist.
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }

    /// Set the bounds change handler.
    pub fn on_change(
        mut self,
        handler: impl Fn(Option<CalendarDate>, Option<CalendarDate>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: DateRangeInputTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit ARIA label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Group).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Whether the bounds form a valid range.
    ///
    /// Missing bounds are valid; two present bounds are valid unless
    /// the start falls after the end.
    pub fn is_valid_range(start: Option<CalendarDate>, end: Option<CalendarDate>) -> bool {
        match (start, end) {
            (Some(from), Some(to)) => from <= to,
            _ => true,
        }
    }

    /// Rendered text for one bound or its placeholder.
    fn bound_text(bound: Option<CalendarDate>, placeholder: &str) -> String {
        bound.map_or_else(|| placeholder.to_string(), CalendarDate::to_ymd_string)
    }

    /// Build the field with theme.
    pub fn build_with_theme(self, theme: &DateRangeInputTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the field with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &DateRangeInputTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let clear_element_id = (self.id.clone(), "date-range-clear");
        let start_text = Self::bound_text(self.start, &self.placeholder);
        let end_text = Self::bound_text(self.end, &self.placeholder);
        let native_label = self.aria_label.clone().unwrap_or_else(|| {
            SharedString::from(format!("Date range {start_text} to {end_text}"))
        });
        let native_props = AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group))
            .maybe_state(self.disabled, AriaState::Disabled);

        let mut field = div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(design.spacing.control_gap))
            .px(px(design.spacing.control_padding_x))
            .py(px(design.spacing.control_padding_y * 0.5))
            .rounded(px(design.corners.md))
            .border_1()
            .border_color(theme.border)
            .bg(theme.background);
        field = match self.size {
            DateRangeInputSize::Sm => field.text_xs(),
            DateRangeInputSize::Md => field.text_sm(),
            DateRangeInputSize::Lg => field.text_lg(),
        };

        let mut start_el = if self.start.is_some() {
            div().text_color(theme.text)
        } else {
            div().text_color(theme.placeholder)
        };
        start_el = start_el.child(start_text);
        let mut end_el = if self.end.is_some() {
            div().text_color(theme.text)
        } else {
            div().text_color(theme.placeholder)
        };
        end_el = end_el.child(end_text);
        field = field
            .child(start_el)
            .child(div().text_color(theme.placeholder).child("-"))
            .child(end_el);

        if self.disabled {
            field = field.opacity(0.5).cursor_not_allowed();
        } else if self.clearable
            && (self.start.is_some() || self.end.is_some())
            && let Some(handler) = self.on_change
        {
            let mouse_handler = handler.clone();
            let mut clear = div()
                .id(clear_element_id)
                .px(px(design.spacing.grid_unit * 0.5))
                .rounded(px(design.corners.sm))
                .text_color(theme.placeholder)
                .cursor_pointer()
                .child("x")
                .on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                    mouse_handler(None, None, window, cx);
                });
            clear = clear.on_key_down(move |event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
                    handler(None, None, window, cx);
                    cx.stop_propagation();
                }
            });
            field = field.child(clear);
        }

        apply_native_accessibility(field, native_label, &native_props)
    }
}

impl RenderOnce for DateRangeInput {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let start_text = Self::bound_text(self.start, &self.placeholder);
        let end_text = Self::bound_text(self.end, &self.placeholder);
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_else(|| {
                SharedString::from(format!("Date range {start_text} to {end_text}"))
            }),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group))
                .maybe_state(self.disabled, AriaState::Disabled),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| DateRangeInputTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{DateRangeInput, DateRangeInputSize, DateRangeInputTheme};
    use crate::date_picker::CalendarDate;
    use gpui::rgba;

    #[test]
    fn range_defaults_to_empty_enabled_field() {
        let field = DateRangeInput::new("stay");
        assert!(field.start.is_none());
        assert!(field.end.is_none());
        assert_eq!(field.size, DateRangeInputSize::Md);
        assert!(!field.disabled);
        assert!(!field.clearable);
        assert_eq!(field.placeholder.as_ref(), "Select dates");
    }

    #[test]
    fn range_builders_store_bounds_and_options() {
        let from = CalendarDate::new(2026, 10, 1).unwrap();
        let to = CalendarDate::new(2026, 10, 7).unwrap();
        let field = DateRangeInput::new("stay")
            .start(from)
            .end(to)
            .placeholder("Pick dates")
            .size(DateRangeInputSize::Lg)
            .disabled(true)
            .clearable(true)
            .on_change(|_, _, _, _| {});
        assert_eq!(field.start, Some(from));
        assert_eq!(field.end, Some(to));
        assert_eq!(field.size, DateRangeInputSize::Lg);
        assert!(field.disabled);
        assert!(field.clearable);
    }

    #[test]
    fn range_validity_accepts_open_and_ordered_bounds() {
        let from = CalendarDate::new(2026, 10, 1).unwrap();
        let to = CalendarDate::new(2026, 10, 7).unwrap();
        assert!(DateRangeInput::is_valid_range(None, None));
        assert!(DateRangeInput::is_valid_range(Some(from), None));
        assert!(DateRangeInput::is_valid_range(None, Some(to)));
        assert!(DateRangeInput::is_valid_range(Some(from), Some(to)));
        assert!(DateRangeInput::is_valid_range(Some(from), Some(from)));
        assert!(!DateRangeInput::is_valid_range(Some(to), Some(from)));
    }

    #[test]
    fn range_bound_text_falls_back_to_placeholder() {
        let from = CalendarDate::new(2026, 10, 1).unwrap();
        assert_eq!(DateRangeInput::bound_text(Some(from), "Pick"), "2026-10-01");
        assert_eq!(DateRangeInput::bound_text(None, "Pick"), "Pick");
    }

    #[test]
    fn range_theme_defaults_match_compiled_tokens() {
        let theme = DateRangeInputTheme::default();
        assert_eq!(theme.background, rgba(0x1e1e1eff));
        assert_eq!(theme.border, rgba(0x3a3a3aff));
        assert_eq!(theme.text, rgba(0xccccccff));
        assert_eq!(theme.placeholder, rgba(0x777777ff));
    }
}
