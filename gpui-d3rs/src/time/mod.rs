//! Time utilities (d3-time)
//!
//! This module provides time interval utilities for working with dates and times.
//! It follows the d3-time API pattern where intervals can be used to floor, ceil,
//! round, count, and generate ranges of dates.
//!
//! # Example
//!
//! ```
//! use d3rs::time::{TimeInterval, Interval};
//!
//! // Floor to start of day
//! let interval = TimeInterval::Day;
//! let date = 1701388800; // Dec 1, 2023 00:00:00 UTC
//! let floored = interval.floor(date);
//! ```

pub mod format;
mod interval;
mod scale;

pub use interval::{
    Interval, TimeInterval, time_day, time_hour, time_minute, time_monday, time_month, time_second,
    time_week, time_year,
};
pub use scale::TimeScale;

/// Duration constants in seconds
pub mod duration {
    /// One second in seconds
    pub const SECOND: i64 = 1;
    /// One minute in seconds
    pub const MINUTE: i64 = 60;
    /// One hour in seconds
    pub const HOUR: i64 = 3600;
    /// One day in seconds
    pub const DAY: i64 = 86400;
    /// One week in seconds
    pub const WEEK: i64 = 604800;
}

/// Milliseconds to timestamp (Unix epoch)
pub fn timestamp_from_millis(millis: i64) -> i64 {
    millis / 1000
}

/// Timestamp to milliseconds (Unix epoch)
pub fn millis_from_timestamp(timestamp: i64) -> i64 {
    timestamp * 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_constants() {
        assert_eq!(duration::MINUTE, 60);
        assert_eq!(duration::HOUR, 3600);
        assert_eq!(duration::DAY, 86400);
        assert_eq!(duration::WEEK, 604800);
    }

    #[test]
    fn test_millis_conversion() {
        assert_eq!(timestamp_from_millis(1000), 1);
        assert_eq!(millis_from_timestamp(1), 1000);
    }
}
