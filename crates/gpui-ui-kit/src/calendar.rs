//! `Calendar` component
//!
//! A month-grid date view with parent-owned state: the caller holds the
//! visible month and the selected date, while the calendar renders the
//! title row with previous/next navigation, the weekday header, and the
//! day cells. Day clicks report through `on_select`, month moves through
//! `on_navigate`, and arrow keys move the selection day by day. This is
//! the full-month view; [`crate::DatePicker`] wraps the same
//! [`crate::CalendarDate`] math in a compact popup control.
//!
//! # Usage
//!
//! ```ignore
//! Calendar::new("departure")
//!     .visible(2026, 10)
//!     .selected(CalendarDate::new(2026, 10, 7).unwrap())
//!     .on_select(|date, _window, _cx| {
//!         // Store `date` as the new selection.
//!     })
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::date_picker::CalendarDate;
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, ClickEvent, Div, ElementId, KeyDownEvent, Rgba, SharedString, Stateful,
    Window, div,
};
use gpui_design::DesignSystem;
use std::rc::Rc;
use std::sync::Arc;

/// English month names for the calendar title.
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Weekday header labels, Sunday first.
const WEEKDAY_HEADERS: [&str; 7] = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

/// Theme colors for the calendar.
#[derive(Debug, Clone, ComponentTheme)]
pub struct CalendarTheme {
    /// Month panel background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub panel: Rgba,
    /// Month title color.
    #[theme(default = 0xffffffff, from = text_primary)]
    pub title: Rgba,
    /// Weekday header color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub weekday: Rgba,
    /// Day number color.
    #[theme(default = 0xffffffff, from = text_primary)]
    pub day: Rgba,
    /// Selected day background.
    #[theme(default = 0x007accff, from = accent)]
    pub day_selected: Rgba,
    /// Selected day number color.
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub day_selected_text: Rgba,
    /// Out-of-range day number color.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub day_disabled: Rgba,
    /// Previous/next control color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub nav: Rgba,
}

/// Calendar size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalendarSize {
    /// Compact day cells.
    Sm,
    /// Standard day cells (default).
    #[default]
    Md,
    /// Large day cells.
    Lg,
}

impl From<crate::ComponentSize> for CalendarSize {
    /// Maps the shared size scale onto calendar cells.
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Month-grid calendar with parent-owned state.
#[derive(IntoElement)]
pub struct Calendar {
    id: ElementId,
    year: i32,
    month: u8,
    selected: Option<CalendarDate>,
    min: Option<CalendarDate>,
    max: Option<CalendarDate>,
    size: CalendarSize,
    theme: Option<CalendarTheme>,
    design: Option<Arc<DesignSystem>>,
    on_select: Option<Rc<dyn Fn(CalendarDate, &mut Window, &mut App) + 'static>>,
    on_navigate: Option<Rc<dyn Fn(i32, u8, &mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Calendar {
    /// Create a calendar showing January 2000 with no selection.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let calendar = Calendar::new("departure").visible(2026, 10);
    /// ```
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            year: 2000,
            month: 1,
            selected: None,
            min: None,
            max: None,
            size: CalendarSize::default(),
            theme: None,
            design: None,
            on_select: None,
            on_navigate: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the visible month (month is clamped to 1-12 at render).
    pub fn visible(mut self, year: i32, month: u8) -> Self {
        self.year = year;
        self.month = month;
        self
    }

    /// Set the selected date; the visible month follows it.
    pub fn selected(mut self, date: CalendarDate) -> Self {
        self.year = date.year;
        self.month = date.month;
        self.selected = Some(date);
        self
    }

    /// Set the earliest selectable date.
    pub fn min(mut self, date: CalendarDate) -> Self {
        self.min = Some(date);
        self
    }

    /// Set the latest selectable date.
    pub fn max(mut self, date: CalendarDate) -> Self {
        self.max = Some(date);
        self
    }

    /// Set the calendar size.
    pub fn size(mut self, size: CalendarSize) -> Self {
        self.size = size;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: CalendarTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set the handler called with the newly selected date.
    pub fn on_select(
        mut self,
        handler: impl Fn(CalendarDate, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Set the handler called with the newly visible month.
    pub fn on_navigate(
        mut self,
        handler: impl Fn(i32, u8, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_navigate = Some(Rc::new(handler));
        self
    }

    /// Set an explicit ARIA label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Table).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Shift a date by `delta` days, rolling over months and years.
    ///
    /// Years clamp to 1-9999 so keyboard walks cannot leave the
    /// proleptic range.
    pub fn add_days(date: CalendarDate, delta: i32) -> CalendarDate {
        let mut year = date.year;
        let mut month = date.month;
        let mut day = i32::from(date.day) + delta;
        loop {
            let length = i32::from(CalendarDate::days_in_month(year, month));
            if day >= 1 && day <= length {
                break;
            }
            if day < 1 {
                if year <= 1 && month == 1 {
                    day = 1;
                    break;
                }
                (year, month) = CalendarDate::step_month(year, month, -1);
                day += i32::from(CalendarDate::days_in_month(year, month));
            } else {
                if year >= 9999 && month == 12 {
                    day = length;
                    break;
                }
                day -= length;
                (year, month) = CalendarDate::step_month(year, month, 1);
            }
        }
        // The loop only exits with `day` inside the month length (1-31).
        let day = u8::try_from(day).unwrap_or(date.day);
        CalendarDate::new(year, month, day).unwrap_or(date)
    }

    /// Clamp the visible month into range.
    fn clamped_visible(&self) -> (i32, u8) {
        (self.year.clamp(1, 9999), self.month.clamp(1, 12))
    }

    /// Build the calendar with theme.
    pub fn build_with_theme(self, theme: &CalendarTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the calendar with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &CalendarTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let (year, month) = self.clamped_visible();
        let root_id = self.id.clone();
        let mut root = div().id(self.id).flex().flex_col().gap(gpui::px(
            design.spacing.control_gap,
        ));
        root = match self.size {
            CalendarSize::Sm => root.text_xs(),
            CalendarSize::Md => root.text_sm(),
            CalendarSize::Lg => root.text_lg(),
        };
        let cell = match self.size {
            CalendarSize::Sm => design.spacing.grid_unit * 3.5,
            CalendarSize::Md => design.spacing.grid_unit * 4.0,
            CalendarSize::Lg => design.spacing.grid_unit * 5.0,
        };

        let title = format!(
            "{} {}",
            MONTH_NAMES[usize::from(month - 1)],
            year
        );
        let (prev_year, prev_month) = CalendarDate::step_month(year, month, -1);
        let (next_year, next_month) = CalendarDate::step_month(year, month, 1);
        let mut header = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(gpui::px(design.spacing.control_gap));
        header = header.child(Self::nav_control(
            (root_id.clone(), "calendar-prev"),
            "‹",
            (prev_year, prev_month),
            theme.nav,
            self.on_navigate.clone(),
        ));
        header = header.child(div().text_color(theme.title).child(title));
        header = header.child(Self::nav_control(
            (root_id.clone(), "calendar-next"),
            "›",
            (next_year, next_month),
            theme.nav,
            self.on_navigate.clone(),
        ));
        root = root.child(header);

        let mut weekdays = div().flex().flex_row();
        for label in WEEKDAY_HEADERS {
            weekdays = weekdays.child(
                div()
                    .w(gpui::px(cell))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.weekday)
                    .child(label),
            );
        }
        root = root.child(weekdays);

        let grid = CalendarDate::month_grid(year, month);
        let selected = self.selected;
        let min = self.min;
        let max = self.max;
        let select_handler = self.on_select.clone();
        let mut week = div().flex().flex_row();
        let mut weeks = 0;
        for (index, cell_date) in grid.iter().enumerate() {
            week = week.child(Self::day_cell(
                &root_id,
                *cell_date,
                cell,
                selected,
                min,
                max,
                select_handler.as_ref(),
                theme,
                design,
            ));
            if index % 7 == 6 {
                root = root.child(week);
                week = div().flex().flex_row();
                weeks += 1;
            }
        }
        if weeks * 7 < grid.len() {
            root = root.child(week);
        }

        let root = root.bg(theme.panel).rounded(gpui::px(design.corners.md));
        if let (Some(selected), Some(on_select)) = (self.selected, self.on_select.clone()) {
            let min = self.min;
            let max = self.max;
            let on_navigate = self.on_navigate.clone();
            let visible = (year, month);
            root.on_key_down(move |event: &KeyDownEvent, window, cx| {
                let delta = match event.keystroke.key.as_str() {
                    "left" => Some(-1),
                    "right" => Some(1),
                    "up" => Some(-7),
                    "down" => Some(7),
                    _ => None,
                };
                let Some(delta) = delta else {
                    return;
                };
                let next = Self::add_days(selected, delta);
                let in_bounds =
                    min.is_none_or(|min| next >= min) && max.is_none_or(|max| next <= max);
                if in_bounds {
                    on_select(next, window, cx);
                    if (next.year, next.month) != visible
                        && let Some(navigate) = &on_navigate
                    {
                        navigate(next.year, next.month, window, cx);
                    }
                    cx.stop_propagation();
                }
            })
        } else {
            root
        }
    }

    /// Previous/next month control reporting its target month.
    fn nav_control(
        id: impl Into<ElementId>,
        label: &'static str,
        target: (i32, u8),
        color: Rgba,
        on_navigate: Option<Rc<dyn Fn(i32, u8, &mut Window, &mut App) + 'static>>,
    ) -> Stateful<Div> {
        let mut control = div().id(id).text_color(color).child(label);
        match on_navigate {
            Some(handler) => {
                let key_handler = handler.clone();
                control = control
                    .cursor_pointer()
                    .on_click(move |_event: &ClickEvent, window, cx| {
                        handler(target.0, target.1, window, cx);
                    })
                    .on_key_down(move |event: &KeyDownEvent, window, cx| {
                        let key = event.keystroke.key.as_str();
                        if key == "enter" || key == "space" {
                            key_handler(target.0, target.1, window, cx);
                            cx.stop_propagation();
                        }
                    });
            }
            None => {
                control = control.opacity(0.4);
            }
        }
        control
    }

    /// One day cell: empty pad, disabled pad, or an active button.
    fn day_cell(
        root_id: &ElementId,
        date: Option<CalendarDate>,
        cell: f32,
        selected: Option<CalendarDate>,
        min: Option<CalendarDate>,
        max: Option<CalendarDate>,
        on_select: Option<&Rc<dyn Fn(CalendarDate, &mut Window, &mut App) + 'static>>,
        theme: &CalendarTheme,
        design: &DesignSystem,
    ) -> AnyElement {
        let sized = || {
            div()
                .w(gpui::px(cell))
                .h(gpui::px(cell))
                .flex()
                .items_center()
                .justify_center()
                .rounded(gpui::px(design.corners.sm))
        };
        let Some(date) = date else {
            return sized().into_any_element();
        };
        let id = (root_id.clone(), SharedString::from(date.to_ymd_string()));
        let in_bounds =
            min.is_none_or(|min| date >= min) && max.is_none_or(|max| date <= max);
        if !in_bounds {
            return sized()
                .id(id)
                .text_color(theme.day_disabled)
                .child(date.day.to_string())
                .into_any_element();
        }
        let current = selected == Some(date);
        let mut pad = sized().id(id).text_color(if current {
            theme.day_selected_text
        } else {
            theme.day
        });
        if current {
            pad = pad.bg(theme.day_selected);
        }
        if let Some(handler) = on_select {
            let select = handler.clone();
            pad = pad.cursor_pointer().on_click(
                move |_event: &ClickEvent, window, cx| {
                    select(date, window, cx);
                },
            );
        }
        pad.child(date.day.to_string()).into_any_element()
    }
}

impl RenderOnce for Calendar {
    /// Registers accessibility, resolves theme, and builds the grid.
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (year, month) = self.clamped_visible();
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_else(|| {
                format!("Calendar {} {}", MONTH_NAMES[usize::from(month - 1)], year).into()
            }),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Table)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| CalendarTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{Calendar, CalendarDate, CalendarSize, CalendarTheme};
    use gpui::rgba;

    #[test]
    fn calendar_defaults_to_january_2000_unselected() {
        let calendar = Calendar::new("departure");
        assert_eq!((calendar.year, calendar.month), (2000, 1));
        assert_eq!(calendar.selected, None);
        assert_eq!(calendar.size, CalendarSize::Md);
        assert!(calendar.on_select.is_none());
        assert!(calendar.on_navigate.is_none());
    }

    #[test]
    fn calendar_builders_store_month_selection_and_bounds() {
        let selected = CalendarDate::new(2026, 10, 7).unwrap();
        let calendar = Calendar::new("departure")
            .visible(2026, 10)
            .selected(selected)
            .min(CalendarDate::new(2026, 10, 1).unwrap())
            .max(CalendarDate::new(2026, 10, 31).unwrap())
            .size(CalendarSize::Lg);
        assert_eq!((calendar.year, calendar.month), (2026, 10));
        assert_eq!(calendar.selected, Some(selected));
        assert!(calendar.min.is_some());
        assert!(calendar.max.is_some());
        assert_eq!(calendar.size, CalendarSize::Lg);
    }

    #[test]
    fn add_days_rolls_over_months_years_and_leap_days() {
        let jan31 = CalendarDate::new(2026, 1, 31).unwrap();
        assert_eq!(
            Calendar::add_days(jan31, 1),
            CalendarDate::new(2026, 2, 1).unwrap()
        );
        assert_eq!(
            Calendar::add_days(jan31, -31),
            CalendarDate::new(2025, 12, 31).unwrap()
        );
        let feb28 = CalendarDate::new(2024, 2, 28).unwrap();
        assert_eq!(
            Calendar::add_days(feb28, 1),
            CalendarDate::new(2024, 2, 29).unwrap()
        );
        let new_year = CalendarDate::new(1, 1, 1).unwrap();
        assert_eq!(Calendar::add_days(new_year, -1), new_year);
    }

    #[test]
    fn visible_month_clamps_into_range() {
        let calendar = Calendar::new("departure").visible(0, 13);
        assert_eq!(calendar.clamped_visible(), (1, 12));
    }

    #[test]
    fn calendar_theme_defaults_match_compiled_tokens() {
        let theme = CalendarTheme::default();
        assert_eq!(theme.panel, rgba(0x1e1e1eff));
        assert_eq!(theme.title, rgba(0xffffffff));
        assert_eq!(theme.weekday, rgba(0xccccccff));
        assert_eq!(theme.day, rgba(0xffffffff));
        assert_eq!(theme.day_selected, rgba(0x007accff));
        assert_eq!(theme.day_selected_text, rgba(0xffffffff));
        assert_eq!(theme.day_disabled, rgba(0x777777ff));
        assert_eq!(theme.nav, rgba(0xccccccff));
    }
}
