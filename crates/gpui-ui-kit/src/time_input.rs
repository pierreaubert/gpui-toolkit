//! `TimeInput` component
//!
//! A clock-time field showing an optional `HH:MM` value. The value is a
//! parent-owned prop; the field itself offers no clock popup or keyboard
//! editing, only display, an optional clear affordance, and change
//! callbacks. This mirrors [`crate::DateRangeInput`], which is display-only
//! for the same reason.
//!
//! # Usage
//!
//! ```ignore
//! TimeInput::new("standup")
//!     .value(ClockTime::new(9, 30).unwrap())
//!     .on_change(|value, _window, _cx| { /* store value */ })
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState, apply_native_accessibility,
};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Div, ElementId, KeyDownEvent, MouseButton, Rgba, SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::rc::Rc;
use std::sync::Arc;

/// Change handler receiving the new time value.
type TimeChange = Rc<dyn Fn(Option<ClockTime>, &mut Window, &mut App) + 'static>;

/// A 24-hour clock time without date or timezone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClockTime {
    /// Hour of day, 0-23.
    pub hour: u8,
    /// Minute of hour, 0-59.
    pub minute: u8,
}

impl ClockTime {
    /// Build a time, returning `None` for out-of-range components.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// assert!(ClockTime::new(9, 30).is_some());
    /// assert!(ClockTime::new(24, 0).is_none());
    /// ```
    pub const fn new(hour: u8, minute: u8) -> Option<Self> {
        if hour > 23 || minute > 59 {
            return None;
        }
        Some(Self { hour, minute })
    }

    /// Parse `"HH:MM"`, returning `None` for malformed values.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// assert_eq!(ClockTime::parse_hm("09:30"), ClockTime::new(9, 30));
    /// assert_eq!(ClockTime::parse_hm("9:30"), None);
    /// ```
    pub fn parse_hm(value: &str) -> Option<Self> {
        let (hour, minute) = value.split_once(':')?;
        if hour.len() != 2 || minute.len() != 2 {
            return None;
        }
        Self::new(hour.parse().ok()?, minute.parse().ok()?)
    }

    /// Format as `"HH:MM"`.
    pub fn to_hm_string(self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }
}

/// Theme colors for the time field.
#[derive(Debug, Clone, ComponentTheme)]
pub struct TimeInputTheme {
    /// Field background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Field border.
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Entered time color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text: Rgba,
    /// Placeholder color for a missing value.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub placeholder: Rgba,
}

/// Time field size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeInputSize {
    /// Compact field.
    Sm,
    /// Standard field (default).
    #[default]
    Md,
    /// Large field.
    Lg,
}

impl From<crate::ComponentSize> for TimeInputSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Time field without clock selection.
#[derive(IntoElement)]
pub struct TimeInput {
    id: ElementId,
    value: Option<ClockTime>,
    placeholder: SharedString,
    size: TimeInputSize,
    disabled: bool,
    clearable: bool,
    on_change: Option<TimeChange>,
    theme: Option<TimeInputTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl TimeInput {
    /// Create a new empty time field.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: None,
            placeholder: SharedString::from("Select time"),
            size: TimeInputSize::default(),
            disabled: false,
            clearable: false,
            on_change: None,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the time value.
    pub fn value(mut self, value: ClockTime) -> Self {
        self.value = Some(value);
        self
    }

    /// Set the placeholder for a missing value.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the field size.
    pub fn size(mut self, size: TimeInputSize) -> Self {
        self.size = size;
        self
    }

    /// Disable the field.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Show a clear affordance when a value exists.
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }

    /// Set the value change handler.
    pub fn on_change(
        mut self,
        handler: impl Fn(Option<ClockTime>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: TimeInputTheme) -> Self {
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

    /// Override the default ARIA role (Textbox).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Rendered text for the value or its placeholder.
    fn value_text(value: Option<ClockTime>, placeholder: &str) -> String {
        value.map_or_else(|| placeholder.to_string(), ClockTime::to_hm_string)
    }

    /// Build the field with theme.
    pub fn build_with_theme(self, theme: &TimeInputTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the field with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &TimeInputTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let clear_element_id = (self.id.clone(), "time-clear");
        let value_text = Self::value_text(self.value, &self.placeholder);
        let native_label = self
            .aria_label
            .clone()
            .unwrap_or_else(|| SharedString::from(format!("Time {value_text}")));
        let native_props = AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Textbox))
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
            TimeInputSize::Sm => field.text_xs(),
            TimeInputSize::Md => field.text_sm(),
            TimeInputSize::Lg => field.text_lg(),
        };

        let mut value_el = if self.value.is_some() {
            div().text_color(theme.text)
        } else {
            div().text_color(theme.placeholder)
        };
        value_el = value_el.child(value_text);
        field = field.child(value_el);

        if self.disabled {
            field = field.opacity(0.5).cursor_not_allowed();
        } else if self.clearable
            && self.value.is_some()
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
                    mouse_handler(None, window, cx);
                });
            clear = clear.on_key_down(move |event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
                    handler(None, window, cx);
                    cx.stop_propagation();
                }
            });
            field = field.child(clear);
        }

        apply_native_accessibility(field, native_label, &native_props)
    }
}

impl RenderOnce for TimeInput {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let value_text = Self::value_text(self.value, &self.placeholder);
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| SharedString::from(format!("Time {value_text}"))),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Textbox))
                .maybe_state(self.disabled, AriaState::Disabled),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| TimeInputTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{ClockTime, TimeInput, TimeInputSize, TimeInputTheme};
    use gpui::rgba;

    #[test]
    fn clock_rejects_out_of_range_values() {
        assert!(ClockTime::new(0, 0).is_some());
        assert!(ClockTime::new(23, 59).is_some());
        assert!(ClockTime::new(24, 0).is_none());
        assert!(ClockTime::new(9, 60).is_none());
    }

    #[test]
    fn clock_hm_round_trip() {
        let time = ClockTime::new(9, 30).unwrap();
        assert_eq!(time.to_hm_string(), "09:30");
        assert_eq!(ClockTime::parse_hm("09:30"), Some(time));
        assert_eq!(ClockTime::parse_hm("9:30"), None);
        assert_eq!(ClockTime::parse_hm("24:00"), None);
        assert_eq!(ClockTime::parse_hm("not-a-time"), None);
    }

    #[test]
    fn time_defaults_to_empty_enabled_field() {
        let field = TimeInput::new("standup");
        assert!(field.value.is_none());
        assert_eq!(field.size, TimeInputSize::Md);
        assert!(!field.disabled);
        assert!(!field.clearable);
        assert_eq!(field.placeholder.as_ref(), "Select time");
    }

    #[test]
    fn time_builders_store_value_and_options() {
        let value = ClockTime::new(9, 30).unwrap();
        let field = TimeInput::new("standup")
            .value(value)
            .placeholder("Pick time")
            .size(TimeInputSize::Lg)
            .disabled(true)
            .clearable(true)
            .on_change(|_, _, _| {});
        assert_eq!(field.value, Some(value));
        assert_eq!(field.size, TimeInputSize::Lg);
        assert!(field.disabled);
        assert!(field.clearable);
    }

    #[test]
    fn time_value_text_falls_back_to_placeholder() {
        let value = ClockTime::new(9, 30).unwrap();
        assert_eq!(TimeInput::value_text(Some(value), "Pick"), "09:30");
        assert_eq!(TimeInput::value_text(None, "Pick"), "Pick");
    }

    #[test]
    fn time_theme_defaults_match_compiled_tokens() {
        let theme = TimeInputTheme::default();
        assert_eq!(theme.background, rgba(0x1e1e1eff));
        assert_eq!(theme.border, rgba(0x3a3a3aff));
        assert_eq!(theme.text, rgba(0xccccccff));
        assert_eq!(theme.placeholder, rgba(0x777777ff));
    }
}
