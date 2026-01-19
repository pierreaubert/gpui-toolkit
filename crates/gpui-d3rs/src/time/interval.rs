//! Time interval implementation

use super::duration;

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
            TimeInterval::Minute => (timestamp / duration::MINUTE) * duration::MINUTE,
            TimeInterval::Hour => (timestamp / duration::HOUR) * duration::HOUR,
            TimeInterval::Day => (timestamp / duration::DAY) * duration::DAY,
            TimeInterval::Week => {
                // Week starts on Sunday (day 4 from Unix epoch which was Thursday)
                let days_since_epoch = timestamp / duration::DAY;
                let day_of_week = (days_since_epoch + 4) % 7; // 0 = Sunday
                (days_since_epoch - day_of_week) * duration::DAY
            }
            TimeInterval::Monday => {
                let days_since_epoch = timestamp / duration::DAY;
                let day_of_week = (days_since_epoch + 4) % 7;
                let days_to_monday = if day_of_week == 0 { 6 } else { day_of_week - 1 };
                (days_since_epoch - days_to_monday) * duration::DAY
            }
            TimeInterval::Month => {
                // Approximate: floor to first of month using UTC
                let days = timestamp / duration::DAY;
                // Very rough approximation - proper implementation needs full date library
                let approx_months = days / 30;
                approx_months * 30 * duration::DAY
            }
            TimeInterval::Year => {
                // Approximate: floor to first of year using UTC
                let days = timestamp / duration::DAY;
                let approx_years = days / 365;
                approx_years * 365 * duration::DAY
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
                // Approximate: add 30 days per month
                timestamp + step * 30 * duration::DAY
            }
            TimeInterval::Year => {
                // Approximate: add 365 days per year
                timestamp + step * 365 * duration::DAY
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
}
