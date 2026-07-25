//! Time interval implementation

use super::duration;

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    // Howard Hinnant's proleptic-Gregorian civil calendar algorithm.
    let z = days_since_epoch as i128 + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i128::from(month <= 2);
    (year as i64, month as u32, day as u32)
}

fn days_from_civil(mut year: i64, month: u32, day: u32) -> i64 {
    year -= i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_prime = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn is_leap_year(year: i64) -> bool {
    year.rem_euclid(4) == 0 && (year.rem_euclid(100) != 0 || year.rem_euclid(400) == 0)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn timestamp_from_civil(year: i64, month: u32, day: u32, seconds_in_day: i64) -> i64 {
    days_from_civil(year, month, day)
        .saturating_mul(duration::DAY)
        .saturating_add(seconds_in_day)
}

/// Common time interval operations trait
pub trait Interval {
    /// Floor to the start of the interval containing the given timestamp
    fn floor(&self, timestamp: i64) -> i64;

    /// Ceil to the start of the next interval after the given timestamp
    fn ceil(&self, timestamp: i64) -> i64 {
        let floored = self.floor(timestamp);
        if floored == timestamp {
            timestamp
        } else {
            self.offset(floored, 1)
        }
    }

    /// Round to the nearest interval boundary
    fn round(&self, timestamp: i64) -> i64 {
        let floor = self.floor(timestamp);
        let ceil = self.ceil(timestamp);
        if timestamp - floor < ceil - timestamp {
            floor
        } else {
            ceil
        }
    }

    /// Offset the timestamp by the given number of intervals
    fn offset(&self, timestamp: i64, step: i64) -> i64;

    /// Count the number of intervals between two timestamps
    fn count(&self, start: i64, end: i64) -> i64 {
        let start = self.floor(start);
        let end = self.floor(end);
        let mut count = 0;
        let mut current = start;
        while current < end {
            current = self.offset(current, 1);
            count += 1;
        }
        count
    }

    /// Generate a range of timestamps at interval boundaries
    fn range(&self, start: i64, stop: i64, step: i64) -> Vec<i64> {
        let step = step.max(1);
        let mut result = Vec::new();
        let mut current = self.ceil(start);
        let mut i = 0;
        while current < stop {
            if i % step == 0 {
                result.push(current);
            }
            current = self.offset(current, 1);
            i += 1;
        }
        result
    }
}

/// Time interval types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInterval {
    /// Every second
    Second,
    /// Every minute
    Minute,
    /// Every hour
    Hour,
    /// Every day
    Day,
    /// Every week (starting Sunday)
    Week,
    /// Every week (starting Monday)
    Monday,
    /// Every month
    Month,
    /// Every year
    Year,
}

impl Interval for TimeInterval {
    fn floor(&self, timestamp: i64) -> i64 {
        match self {
            TimeInterval::Second => timestamp,
            TimeInterval::Minute => timestamp.div_euclid(duration::MINUTE) * duration::MINUTE,
            TimeInterval::Hour => timestamp.div_euclid(duration::HOUR) * duration::HOUR,
            TimeInterval::Day => timestamp.div_euclid(duration::DAY) * duration::DAY,
            TimeInterval::Week => {
                // Week starts on Sunday (day 4 from Unix epoch which was Thursday)
                let days_since_epoch = timestamp.div_euclid(duration::DAY);
                let day_of_week = (days_since_epoch + 4).rem_euclid(7); // 0 = Sunday
                (days_since_epoch - day_of_week) * duration::DAY
            }
            TimeInterval::Monday => {
                let days_since_epoch = timestamp.div_euclid(duration::DAY);
                let day_of_week = (days_since_epoch + 4).rem_euclid(7);
                let days_to_monday = if day_of_week == 0 { 6 } else { day_of_week - 1 };
                (days_since_epoch - days_to_monday) * duration::DAY
            }
            TimeInterval::Month => {
                let (year, month, _) = civil_from_days(timestamp.div_euclid(duration::DAY));
                timestamp_from_civil(year, month, 1, 0)
            }
            TimeInterval::Year => {
                let (year, _, _) = civil_from_days(timestamp.div_euclid(duration::DAY));
                timestamp_from_civil(year, 1, 1, 0)
            }
        }
    }

    fn offset(&self, timestamp: i64, step: i64) -> i64 {
        match self {
            TimeInterval::Second => timestamp + step * duration::SECOND,
            TimeInterval::Minute => timestamp + step * duration::MINUTE,
            TimeInterval::Hour => timestamp + step * duration::HOUR,
            TimeInterval::Day => timestamp + step * duration::DAY,
            TimeInterval::Week | TimeInterval::Monday => timestamp + step * duration::WEEK,
            TimeInterval::Month => {
                let days = timestamp.div_euclid(duration::DAY);
                let seconds_in_day = timestamp.rem_euclid(duration::DAY);
                let (year, month, day) = civil_from_days(days);
                let month_index = i128::from(year) * 12 + i128::from(month - 1) + i128::from(step);
                let target_year = month_index.div_euclid(12) as i64;
                let target_month = month_index.rem_euclid(12) as u32 + 1;
                timestamp_from_civil(
                    target_year,
                    target_month,
                    day.min(days_in_month(target_year, target_month)),
                    seconds_in_day,
                )
            }
            TimeInterval::Year => {
                let days = timestamp.div_euclid(duration::DAY);
                let seconds_in_day = timestamp.rem_euclid(duration::DAY);
                let (year, month, day) = civil_from_days(days);
                let target_year = (i128::from(year) + i128::from(step)) as i64;
                timestamp_from_civil(
                    target_year,
                    month,
                    day.min(days_in_month(target_year, month)),
                    seconds_in_day,
                )
            }
        }
    }
}

impl TimeInterval {
    /// Get a human-readable format string for this interval
    pub fn format_pattern(&self) -> &'static str {
        match self {
            TimeInterval::Second => "%H:%M:%S",
            TimeInterval::Minute => "%H:%M",
            TimeInterval::Hour => "%H:00",
            TimeInterval::Day => "%b %d",
            TimeInterval::Week | TimeInterval::Monday => "%b %d",
            TimeInterval::Month => "%B",
            TimeInterval::Year => "%Y",
        }
    }

    /// Get the duration of this interval in seconds (approximate for month/year)
    pub fn duration(&self) -> i64 {
        match self {
            TimeInterval::Second => duration::SECOND,
            TimeInterval::Minute => duration::MINUTE,
            TimeInterval::Hour => duration::HOUR,
            TimeInterval::Day => duration::DAY,
            TimeInterval::Week | TimeInterval::Monday => duration::WEEK,
            TimeInterval::Month => 30 * duration::DAY,
            TimeInterval::Year => 365 * duration::DAY,
        }
    }

    /// Find the best interval for a given time span
    pub fn for_span(span_seconds: i64) -> Self {
        if span_seconds < 60 {
            TimeInterval::Second
        } else if span_seconds < 3600 {
            TimeInterval::Minute
        } else if span_seconds < 86400 {
            TimeInterval::Hour
        } else if span_seconds < 604800 {
            TimeInterval::Day
        } else if span_seconds < 2592000 {
            TimeInterval::Week
        } else if span_seconds < 31536000 {
            TimeInterval::Month
        } else {
            TimeInterval::Year
        }
    }
}

/// Shorthand functions for common intervals
pub fn time_second() -> TimeInterval {
    TimeInterval::Second
}

pub fn time_minute() -> TimeInterval {
    TimeInterval::Minute
}

pub fn time_hour() -> TimeInterval {
    TimeInterval::Hour
}

pub fn time_day() -> TimeInterval {
    TimeInterval::Day
}

pub fn time_week() -> TimeInterval {
    TimeInterval::Week
}

pub fn time_monday() -> TimeInterval {
    TimeInterval::Monday
}

pub fn time_month() -> TimeInterval {
    TimeInterval::Month
}

pub fn time_year() -> TimeInterval {
    TimeInterval::Year
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floor_day() {
        let interval = TimeInterval::Day;
        // Dec 1, 2023 12:30:45 UTC = 1701432645
        let timestamp = 1701432645;
        let floored = interval.floor(timestamp);
        // Should floor to Dec 1, 2023 00:00:00 UTC = 1701388800
        assert_eq!(floored, 1701388800);
    }

    #[test]
    fn test_floor_hour() {
        let interval = TimeInterval::Hour;
        let timestamp = 1701432645; // Dec 1, 2023 12:30:45 UTC
        let floored = interval.floor(timestamp);
        // Should floor to Dec 1, 2023 12:00:00 UTC = 1701432000
        assert_eq!(floored, 1701432000);
    }

    #[test]
    fn test_offset_day() {
        let interval = TimeInterval::Day;
        let start = 1701388800; // Dec 1, 2023 00:00:00 UTC
        let next = interval.offset(start, 1);
        // Should be Dec 2, 2023 00:00:00 UTC
        assert_eq!(next, start + duration::DAY);
    }

    #[test]
    fn month_floor_uses_utc_calendar_boundaries() {
        let leap_day_noon = 1_709_210_096; // 2024-02-29 12:34:56 UTC
        assert_eq!(
            TimeInterval::Month.floor(leap_day_noon),
            1_706_745_600 // 2024-02-01 00:00:00 UTC
        );
        assert_eq!(
            TimeInterval::Year.floor(leap_day_noon),
            1_704_067_200 // 2024-01-01 00:00:00 UTC
        );
    }

    #[test]
    fn calendar_offsets_handle_leap_years_and_clamp_month_ends() {
        let january_31_2024 = 1_706_659_200;
        let february_29_2024 = 1_709_164_800;
        let february_28_2025 = 1_740_700_800;

        assert_eq!(
            TimeInterval::Month.offset(january_31_2024, 1),
            february_29_2024
        );
        assert_eq!(
            TimeInterval::Year.offset(february_29_2024, 1),
            february_28_2025
        );
        assert_eq!(
            TimeInterval::Month.offset(february_29_2024, -1),
            1_704_067_200 + 28 * duration::DAY // 2024-01-29
        );
    }

    #[test]
    fn calendar_ranges_and_counts_follow_real_month_boundaries() {
        let january_2024 = 1_704_067_200;
        let may_2024 = 1_714_521_600;
        assert_eq!(
            TimeInterval::Month.range(january_2024, may_2024, 1),
            vec![1_704_067_200, 1_706_745_600, 1_709_251_200, 1_711_929_600,]
        );
        assert_eq!(TimeInterval::Month.count(january_2024, may_2024), 4);
    }

    #[test]
    fn floors_before_unix_epoch_are_calendar_correct() {
        assert_eq!(TimeInterval::Day.floor(-1), -duration::DAY);
        assert_eq!(TimeInterval::Month.floor(-1), -2_678_400); // 1969-12-01
        assert_eq!(TimeInterval::Year.floor(-1), -31_536_000); // 1969-01-01
    }

    #[test]
    fn test_range() {
        let interval = TimeInterval::Day;
        let start = 1701388800; // Dec 1, 2023
        let stop = start + 5 * duration::DAY; // Dec 6, 2023
        let range = interval.range(start, stop, 1);
        assert_eq!(range.len(), 5);
        assert_eq!(range[0], start);
        assert_eq!(range[4], start + 4 * duration::DAY);
    }

    #[test]
    fn test_count() {
        let interval = TimeInterval::Day;
        let start = 1701388800; // Dec 1, 2023
        let end = start + 7 * duration::DAY; // Dec 8, 2023
        let count = interval.count(start, end);
        assert_eq!(count, 7);
    }

    #[test]
    fn test_for_span() {
        assert_eq!(TimeInterval::for_span(30), TimeInterval::Second);
        assert_eq!(TimeInterval::for_span(300), TimeInterval::Minute);
        assert_eq!(TimeInterval::for_span(7200), TimeInterval::Hour);
        assert_eq!(TimeInterval::for_span(172800), TimeInterval::Day);
        assert_eq!(TimeInterval::for_span(1209600), TimeInterval::Week);
    }

    #[test]
    fn all_intervals_expose_consistent_floor_offset_duration_and_format() {
        let cases = [
            (TimeInterval::Second, duration::SECOND, "%H:%M:%S"),
            (TimeInterval::Minute, duration::MINUTE, "%H:%M"),
            (TimeInterval::Hour, duration::HOUR, "%H:00"),
            (TimeInterval::Day, duration::DAY, "%b %d"),
            (TimeInterval::Week, duration::WEEK, "%b %d"),
            (TimeInterval::Monday, duration::WEEK, "%b %d"),
            (TimeInterval::Month, 30 * duration::DAY, "%B"),
            (TimeInterval::Year, 365 * duration::DAY, "%Y"),
        ];
        let timestamp = 40 * duration::DAY + 12_345;
        for (interval, expected_duration, pattern) in cases {
            assert_eq!(interval.duration(), expected_duration);
            assert_eq!(interval.format_pattern(), pattern);
            if !matches!(interval, TimeInterval::Month | TimeInterval::Year) {
                assert_eq!(
                    interval.offset(timestamp, 2),
                    timestamp + 2 * expected_duration
                );
            }
            assert!(interval.floor(timestamp) <= timestamp);
        }
    }

    #[test]
    fn interval_round_ceil_range_and_span_boundaries_are_deterministic() {
        let minute = TimeInterval::Minute;
        assert_eq!(minute.ceil(duration::MINUTE), duration::MINUTE);
        assert_eq!(minute.ceil(duration::MINUTE + 1), 2 * duration::MINUTE);
        assert_eq!(minute.round(duration::MINUTE + 29), duration::MINUTE);
        assert_eq!(minute.round(duration::MINUTE + 30), 2 * duration::MINUTE);
        assert_eq!(
            minute.range(0, 6 * duration::MINUTE, 2),
            vec![0, 2 * duration::MINUTE, 4 * duration::MINUTE]
        );
        assert_eq!(
            minute.range(0, 3 * duration::MINUTE, 0),
            vec![0, duration::MINUTE, 2 * duration::MINUTE]
        );

        assert_eq!(TimeInterval::for_span(59), TimeInterval::Second);
        assert_eq!(TimeInterval::for_span(60), TimeInterval::Minute);
        assert_eq!(TimeInterval::for_span(3_600), TimeInterval::Hour);
        assert_eq!(TimeInterval::for_span(86_400), TimeInterval::Day);
        assert_eq!(TimeInterval::for_span(604_800), TimeInterval::Week);
        assert_eq!(TimeInterval::for_span(2_592_000), TimeInterval::Month);
        assert_eq!(TimeInterval::for_span(31_536_000), TimeInterval::Year);
    }

    #[test]
    fn shorthand_constructors_cover_every_interval() {
        assert_eq!(time_second(), TimeInterval::Second);
        assert_eq!(time_minute(), TimeInterval::Minute);
        assert_eq!(time_hour(), TimeInterval::Hour);
        assert_eq!(time_day(), TimeInterval::Day);
        assert_eq!(time_week(), TimeInterval::Week);
        assert_eq!(time_monday(), TimeInterval::Monday);
        assert_eq!(time_month(), TimeInterval::Month);
        assert_eq!(time_year(), TimeInterval::Year);
    }
}
