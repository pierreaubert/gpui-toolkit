//! `DateTimeInput` component
//!
//! A date-time field showing an optional date and clock time. Both parts
//! are parent-owned props; the field itself offers no calendar or clock
//! popup, only display, an optional clear affordance, and change
//! callbacks. This mirrors [`crate::DateRangeInput`], which is
//! display-only for the same reason.
//!
//! # Usage
//!
//! ```ignore
//! DateTimeInput::new("launch")
//!     .date(CalendarDate::new(2026, 10, 7).unwrap())
//!     .time(ClockTime::new(9, 30).unwrap())
//!     .on_change(|date, time, _window, _cx| { /* store parts */ })
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState, apply_native_accessibility,
};
use crate::date_picker::CalendarDate;
use crate::theme::ThemeExt;
use crate::time_input::ClockTime;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Div, ElementId, KeyDownEvent, MouseButton, Rgba, SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::rc::Rc;
use std::sync::Arc;

/// Change handler receiving the new date and time parts.
type DateTimeChange =
    Rc<dyn Fn(Option<CalendarDate>, Option<ClockTime>, &mut Window, &mut App) + 'static>;

/// Theme colors for the date-time field.
#[derive(Debug, Clone, ComponentTheme)]
pub struct DateTimeInputTheme {
    /// Field background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Field border.
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Entered date-time color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text: Rgba,
    /// Placeholder color for missing parts.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub placeholder: Rgba,
}

/// Date-time field size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DateTimeInputSize {
    /// Compact field.
    Sm,
    /// Standard field (default).
    #[default]
    Md,
    /// Large field.
    Lg,
}

impl From<crate::ComponentSize> for DateTimeInputSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Date-time field without calendar or clock selection.
#[derive(IntoElement)]
pub struct DateTimeInput {
    id: ElementId,
    date: Option<CalendarDate>,
    time: Option<ClockTime>,
    placeholder: SharedString,
    size: DateTimeInputSize,
    disabled: bool,
    clearable: bool,
    on_change: Option<DateTimeChange>,
    theme: Option<DateTimeInputTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl DateTimeInput {
    /// Create a new empty date-time field.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            date: None,
            time: None,
            placeholder: SharedString::from("Select date and time"),
            size: DateTimeInputSize::default(),
            disabled: false,
            clearable: false,
            on_change: None,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the date part.
    pub fn date(mut self, date: CalendarDate) -> Self {
        self.date = Some(date);
        self
    }

    /// Set the time part.
    pub fn time(mut self, time: ClockTime) -> Self {
        self.time = Some(time);
        self
    }

    /// Set the placeholder for missing parts.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the field size.
    pub fn size(mut self, size: DateTimeInputSize) -> Self {
        self.size = size;
        self
    }

    /// Disable the field.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Show a clear affordance when parts exist.
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }

    /// Set the parts change handler.
    pub fn on_change(
        mut self,
        handler: impl Fn(Option<CalendarDate>, Option<ClockTime>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: DateTimeInputTheme) -> Self {
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

    /// Rendered text for the date part or its placeholder.
    fn date_text(date: Option<CalendarDate>, placeholder: &str) -> String {
        date.map_or_else(|| placeholder.to_string(), CalendarDate::to_ymd_string)
    }

    /// Rendered text for the time part or its placeholder.
    fn time_text(time: Option<ClockTime>, placeholder: &str) -> String {
        time.map_or_else(|| placeholder.to_string(), ClockTime::to_hm_string)
    }

    /// Build the field with theme.
    pub fn build_with_theme(self, theme: &DateTimeInputTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the field with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &DateTimeInputTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let clear_element_id = (self.id.clone(), "date-time-clear");
        let date_text = Self::date_text(self.date, &self.placeholder);
        let time_text = Self::time_text(self.time, &self.placeholder);
        let native_label = self.aria_label.clone().unwrap_or_else(|| {
            SharedString::from(format!("Date and time {date_text} {time_text}"))
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
            DateTimeInputSize::Sm => field.text_xs(),
            DateTimeInputSize::Md => field.text_sm(),
            DateTimeInputSize::Lg => field.text_lg(),
        };

        let mut date_el = if self.date.is_some() {
            div().text_color(theme.text)
        } else {
            div().text_color(theme.placeholder)
        };
        date_el = date_el.child(date_text);
        let mut time_el = if self.time.is_some() {
            div().text_color(theme.text)
        } else {
            div().text_color(theme.placeholder)
        };
        time_el = time_el.child(time_text);
        field = field.child(date_el).child(time_el);

        if self.disabled {
            field = field.opacity(0.5).cursor_not_allowed();
        } else if self.clearable
            && (self.date.is_some() || self.time.is_some())
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

impl RenderOnce for DateTimeInput {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let date_text = Self::date_text(self.date, &self.placeholder);
        let time_text = Self::time_text(self.time, &self.placeholder);
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_else(|| {
                SharedString::from(format!("Date and time {date_text} {time_text}"))
            }),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group))
                .maybe_state(self.disabled, AriaState::Disabled),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| DateTimeInputTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{DateTimeInput, DateTimeInputSize, DateTimeInputTheme};
    use crate::date_picker::CalendarDate;
    use crate::time_input::ClockTime;
    use gpui::rgba;

    #[test]
    fn datetime_defaults_to_empty_enabled_field() {
        let field = DateTimeInput::new("launch");
        assert!(field.date.is_none());
        assert!(field.time.is_none());
        assert_eq!(field.size, DateTimeInputSize::Md);
        assert!(!field.disabled);
        assert!(!field.clearable);
        assert_eq!(field.placeholder.as_ref(), "Select date and time");
    }

    #[test]
    fn datetime_builders_store_parts_and_options() {
        let date = CalendarDate::new(2026, 10, 7).unwrap();
        let time = ClockTime::new(9, 30).unwrap();
        let field = DateTimeInput::new("launch")
            .date(date)
            .time(time)
            .placeholder("Pick")
            .size(DateTimeInputSize::Lg)
            .disabled(true)
            .clearable(true)
            .on_change(|_, _, _, _| {});
        assert_eq!(field.date, Some(date));
        assert_eq!(field.time, Some(time));
        assert_eq!(field.size, DateTimeInputSize::Lg);
        assert!(field.disabled);
        assert!(field.clearable);
    }

    #[test]
    fn datetime_part_text_falls_back_to_placeholder() {
        let date = CalendarDate::new(2026, 10, 7).unwrap();
        let time = ClockTime::new(9, 30).unwrap();
        assert_eq!(DateTimeInput::date_text(Some(date), "Pick"), "2026-10-07");
        assert_eq!(DateTimeInput::date_text(None, "Pick"), "Pick");
        assert_eq!(DateTimeInput::time_text(Some(time), "Pick"), "09:30");
        assert_eq!(DateTimeInput::time_text(None, "Pick"), "Pick");
    }

    #[test]
    fn datetime_theme_defaults_match_compiled_tokens() {
        let theme = DateTimeInputTheme::default();
        assert_eq!(theme.background, rgba(0x1e1e1eff));
        assert_eq!(theme.border, rgba(0x3a3a3aff));
        assert_eq!(theme.text, rgba(0xccccccff));
        assert_eq!(theme.placeholder, rgba(0x777777ff));
    }
}
