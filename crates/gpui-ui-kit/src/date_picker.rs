//! DatePicker component
//!
//! A calendar popup primitive (Radix Calendar parity): month grid, month
//! navigation, single-date selection, and optional min/max bounds. State is
//! parent-owned like [`crate::Combobox`]: the visible month and the selected
//! date are props, every change reports through a callback.
//!
//! # Usage
//!
//! ```ignore
//! DatePicker::new("due-date")
//!     .selected(chosen)
//!     .visible_month(2026, 9)
//!     .on_select(|date, _window, _cx| { /* store date */ })
//!     .on_navigate(|year, month, _window, _cx| { /* store month */ })
//! ```

use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::{Theme, ThemeExt};
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{App, Div, ElementId, SharedString, Stateful, Window, div, px};
use std::rc::Rc;

/// A calendar date without timezone or clock dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarDate {
    /// Proleptic Gregorian year.
    pub year: i32,
    /// Month of year, 1-12.
    pub month: u8,
    /// Day of month, 1-31 subject to month length.
    pub day: u8,
}

impl CalendarDate {
    /// Build a date, returning `None` for out-of-range components.
    pub const fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if month < 1 || month > 12 || day < 1 {
            return None;
        }
        if day > Self::days_in_month(year, month) {
            return None;
        }
        Some(Self { year, month, day })
    }

    /// Whether `year` is a Gregorian leap year.
    pub const fn is_leap_year(year: i32) -> bool {
        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }

    /// Number of days in `month` of `year` (1-12; out-of-range months yield 0).
    pub const fn days_in_month(year: i32, month: u8) -> u8 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if Self::is_leap_year(year) => 29,
            2 => 28,
            _ => 0,
        }
    }

    /// Days since 1970-01-01 (Howard Hinnant's days-from-civil).
    const fn days_since_epoch(self) -> i64 {
        let y = if self.month <= 2 {
            self.year as i64 - 1
        } else {
            self.year as i64
        };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let mp = (self.month as i64 + 9) % 12;
        let doy = (153 * mp + 2) / 5 + self.day as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468
    }

    /// Weekday with Sunday first: 0 = Sunday .. 6 = Saturday.
    ///
    /// 1970-01-01 was a Thursday (index 4), which anchors the mapping.
    pub const fn weekday_sunday_first(self) -> u8 {
        (self.days_since_epoch() + 4).rem_euclid(7) as u8
    }

    /// Parse `"YYYY-MM-DD"`, returning `None` for malformed or impossible dates.
    pub fn parse_ymd(value: &str) -> Option<Self> {
        let (year, rest) = value.split_once('-')?;
        let (month, day) = rest.split_once('-')?;
        if year.len() != 4 || month.len() != 2 || day.len() != 2 {
            return None;
        }
        Self::new(year.parse().ok()?, month.parse().ok()?, day.parse().ok()?)
    }

    /// Format as `"YYYY-MM-DD"`.
    pub fn to_ymd_string(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// Step a visible `(year, month)` pair by `delta` months, clamping the
    /// year to a sane proleptic range.
    pub fn step_month(year: i32, month: u8, delta: i32) -> (i32, u8) {
        let total = year.saturating_mul(12) + month as i32 - 1 + delta;
        let stepped_year = total.div_euclid(12).clamp(1, 9999);
        let stepped_month = (total.rem_euclid(12) + 1) as u8;
        (stepped_year, stepped_month)
    }

    /// Sunday-first month grid: leading `None` cells pad the first week so
    /// day 1 lands under its weekday column, followed by every day.
    pub fn month_grid(year: i32, month: u8) -> Vec<Option<Self>> {
        let mut cells = Vec::new();
        let Some(first) = Self::new(year, month, 1) else {
            return cells;
        };
        cells.extend(std::iter::repeat_n(
            None,
            first.weekday_sunday_first() as usize,
        ));
        for day in 1..=Self::days_in_month(year, month) {
            // Validated by construction: `day` is within the month length.
            cells.push(Self::new(year, month, day));
        }
        cells
    }
}

/// Append a finished week row to the day grid.
fn push_week(grid: Div, week: Div) -> Div {
    grid.child(week)
}

/// English month names for the calendar title (i18n follows `TranslationKey`).
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

/// A calendar date picker with parent-owned state.
pub struct DatePicker {
    id: ElementId,
    selected: Option<CalendarDate>,
    visible_year: i32,
    visible_month: u8,
    min: Option<CalendarDate>,
    max: Option<CalendarDate>,
    on_select: Option<Rc<dyn Fn(CalendarDate, &mut Window, &mut App) + 'static>>,
    on_navigate: Option<Rc<dyn Fn(i32, u8, &mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
}

impl DatePicker {
    /// Create a date picker. The visible month defaults to 2000-01 until
    /// [`Self::visible_month`] or [`Self::selected`] sets it.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            selected: None,
            visible_year: 2000,
            visible_month: 1,
            min: None,
            max: None,
            on_select: None,
            on_navigate: None,
            aria_label: None,
        }
    }

    /// Set the selected date (defaults the visible month to it when unset).
    pub fn selected(mut self, date: Option<CalendarDate>) -> Self {
        if let Some(date) = date {
            self.visible_year = date.year;
            self.visible_month = date.month;
        }
        self.selected = date;
        self
    }

    /// Set the visible month (1-12; out-of-range values are ignored).
    pub fn visible_month(mut self, year: i32, month: u8) -> Self {
        if (1..=12).contains(&month) {
            self.visible_year = year;
            self.visible_month = month;
        }
        self
    }

    /// Reject dates before `min` (they render dimmed and inert).
    pub fn min(mut self, min: CalendarDate) -> Self {
        self.min = Some(min);
        self
    }

    /// Reject dates after `max` (they render dimmed and inert).
    pub fn max(mut self, max: CalendarDate) -> Self {
        self.max = Some(max);
        self
    }

    /// Called with the picked date.
    pub fn on_select(
        mut self,
        handler: impl Fn(CalendarDate, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Called with the new visible `(year, month)` on prev/next navigation.
    pub fn on_navigate(
        mut self,
        handler: impl Fn(i32, u8, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_navigate = Some(Rc::new(handler));
        self
    }

    /// Set an explicit ARIA label for the calendar grid.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    fn is_disabled(
        date: CalendarDate,
        min: Option<CalendarDate>,
        max: Option<CalendarDate>,
    ) -> bool {
        min.is_some_and(|min| date < min) || max.is_some_and(|max| date > max)
    }

    /// Build into an element with the given theme.
    pub fn build_with_theme(self, theme: &Theme) -> Stateful<Div> {
        let title = format!(
            "{} {}",
            MONTH_NAMES
                .get(self.visible_month as usize - 1)
                .copied()
                .unwrap_or("?"),
            self.visible_year
        );
        let selected = self.selected;
        let min = self.min;
        let max = self.max;
        let on_select = self.on_select;
        let on_navigate = self.on_navigate;
        let year = self.visible_year;
        let month = self.visible_month;

        let accent_bg = theme.accent;
        let day_text = theme.text_primary;
        let muted_text = theme.text_secondary;
        let hover_bg = theme.surface_hover;
        let border = theme.border;

        let picker_id = self.id.clone();
        let mut container = div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(theme.surface)
            .border_1()
            .border_color(border)
            .rounded(px(8.0))
            .w(px(280.0));

        // Header: prev | title | next, with the title centered.
        let mut header_row = div().flex().items_center().justify_between();
        let title_text: SharedString = title.into();
        let title_el = div()
            .flex_1()
            .text_center()
            .text_sm()
            .text_color(day_text)
            .child(title_text);
        let prev_delta = -1;
        let next_delta = 1;
        let nav_button = |label: &'static str, delta: i32| {
            let navigate = on_navigate.clone();
            let button_id = picker_id.clone();
            let nav_id: &'static str = if delta < 0 { "nav-prev" } else { "nav-next" };
            div()
                .id((button_id, nav_id))
                .px_2()
                .py_1()
                .rounded(px(4.0))
                .cursor_pointer()
                .text_color(day_text)
                .hover(move |s| s.bg(hover_bg))
                .on_click(move |_event, window, cx| {
                    if let Some(navigate) = navigate.as_ref() {
                        let (year, month) = CalendarDate::step_month(year, month, delta);
                        navigate(year, month, window, cx);
                    }
                })
                .child(label)
        };
        header_row = header_row
            .child(nav_button("‹", prev_delta))
            .child(title_el)
            .child(nav_button("›", next_delta));
        container = container.child(header_row);

        // Weekday header.
        let mut weekdays = div().flex().flex_row();
        for name in WEEKDAY_HEADERS {
            weekdays = weekdays.child(
                div()
                    .flex_1()
                    .text_center()
                    .text_xs()
                    .text_color(muted_text)
                    .child(name),
            );
        }
        container = container.child(weekdays);

        // Day grid.
        let mut grid = div().flex().flex_col().gap_1();
        let mut week = div().flex().flex_row().gap_1();
        let mut cells_in_week = 0;
        for cell in CalendarDate::month_grid(year, month) {
            if cells_in_week == 7 {
                grid = push_week(grid, week);
                week = div().flex().flex_row().gap_1();
                cells_in_week = 0;
            }
            match cell {
                None => {
                    week = week.child(div().flex_1());
                }
                Some(date) => {
                    let disabled = Self::is_disabled(date, min, max);
                    let is_selected = selected == Some(date);
                    let mut day = div()
                        .id((picker_id.clone(), SharedString::from(date.to_ymd_string())))
                        .flex_1()
                        .text_center()
                        .text_sm()
                        .py_1()
                        .rounded(px(4.0))
                        .text_color(if disabled { muted_text } else { day_text });
                    if is_selected {
                        day = day.bg(accent_bg).text_color(theme.surface);
                    } else if !disabled {
                        day = day.cursor_pointer().hover(move |s| s.bg(hover_bg));
                    }
                    if !disabled && let Some(select) = on_select.clone() {
                        day = day.on_click(move |_event, window, cx| {
                            select(date, window, cx);
                        });
                    }
                    week = week.child(day);
                }
            }
            cells_in_week += 1;
        }
        while cells_in_week < 7 {
            week = week.child(div().flex_1());
            cells_in_week += 1;
        }
        if cells_in_week > 0 {
            grid = push_week(grid, week);
        }
        container.child(grid)
    }
}

impl RenderOnce for DatePicker {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let label = self.aria_label.clone().unwrap_or_else(|| "Calendar".into());
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label,
            props: AriaProps::with_role(AriaRole::Group),
        });
        let global_theme = cx.theme();
        self.build_with_theme(&global_theme)
    }
}

impl IntoElement for DatePicker {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{CalendarDate, DatePicker};

    #[test]
    fn date_rejects_impossible_values() {
        assert!(CalendarDate::new(2026, 0, 1).is_none());
        assert!(CalendarDate::new(2026, 13, 1).is_none());
        assert!(CalendarDate::new(2026, 2, 30).is_none());
        assert!(CalendarDate::new(2025, 2, 29).is_none());
        assert!(CalendarDate::new(2024, 2, 29).is_some());
        assert!(CalendarDate::new(2000, 2, 29).is_some());
        assert!(CalendarDate::new(1900, 2, 29).is_none());
    }

    #[test]
    fn weekday_anchor_matches_known_dates() {
        // 1970-01-01 was a Thursday; 2026-09-03 is a Thursday.
        assert_eq!(
            CalendarDate::new(1970, 1, 1)
                .unwrap()
                .weekday_sunday_first(),
            4
        );
        assert_eq!(
            CalendarDate::new(2026, 9, 3)
                .unwrap()
                .weekday_sunday_first(),
            4
        );
        assert_eq!(
            CalendarDate::new(2026, 9, 6)
                .unwrap()
                .weekday_sunday_first(),
            0
        );
    }

    #[test]
    fn ymd_round_trip() {
        let date = CalendarDate::new(2026, 9, 3).unwrap();
        assert_eq!(date.to_ymd_string(), "2026-09-03");
        assert_eq!(CalendarDate::parse_ymd("2026-09-03"), Some(date));
        assert_eq!(CalendarDate::parse_ymd("2026-9-3"), None);
        assert_eq!(CalendarDate::parse_ymd("2026-13-01"), None);
        assert_eq!(CalendarDate::parse_ymd("not-a-date"), None);
    }

    #[test]
    fn month_grid_starts_on_weekday_and_covers_all_days() {
        // September 2026: the 1st is a Tuesday (2 leading blanks), 30 days.
        let grid = CalendarDate::month_grid(2026, 9);
        assert_eq!(grid.len(), 2 + 30);
        assert!(grid[..2].iter().all(Option::is_none));
        assert_eq!(grid[2], Some(CalendarDate::new(2026, 9, 1).unwrap()));
        assert_eq!(grid.iter().flatten().count(), 30);
    }

    #[test]
    fn step_month_wraps_years() {
        assert_eq!(CalendarDate::step_month(2026, 1, -1), (2025, 12));
        assert_eq!(CalendarDate::step_month(2026, 12, 1), (2027, 1));
        assert_eq!(CalendarDate::step_month(2026, 9, 0), (2026, 9));
    }

    #[test]
    fn picker_builder_records_state() {
        let date = CalendarDate::new(2026, 9, 3).unwrap();
        let picker = DatePicker::new("due")
            .selected(Some(date))
            .min(CalendarDate::new(2026, 9, 1).unwrap())
            .max(CalendarDate::new(2026, 9, 30).unwrap())
            .on_select(|_, _, _| {})
            .on_navigate(|_, _, _, _| {});

        assert_eq!(picker.selected, Some(date));
        assert_eq!((picker.visible_year, picker.visible_month), (2026, 9));
        assert!(picker.on_select.is_some());
        assert!(picker.on_navigate.is_some());
        assert!(!DatePicker::is_disabled(date, picker.min, picker.max));
        assert!(DatePicker::is_disabled(
            CalendarDate::new(2026, 10, 1).unwrap(),
            picker.min,
            picker.max
        ));
    }
}
