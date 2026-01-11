//! Time scale implementation

use super::duration;
use super::interval::{Interval, TimeInterval};
use crate::scale::{Scale, nice_number};

/// A time scale maps temporal domain to continuous range
///
/// Unlike linear scales, time scales use appropriate time intervals
/// for tick generation (seconds, minutes, hours, days, etc.).
///
/// # Example
///
/// ```
/// use d3rs::time::TimeScale;
/// use d3rs::scale::Scale;
///
/// let scale = TimeScale::new()
///     .domain(0, 86400) // One day in seconds
///     .range(0.0, 800.0);
///
/// // Half a day maps to the middle of the range
/// assert!((scale.scale(43200) - 400.0).abs() < 1e-6);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeScale {
    domain_min: i64,
    domain_max: i64,
    range_min: f64,
    range_max: f64,
    clamped: bool,
}

impl Default for TimeScale {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeScale {
    /// Create a new time scale with default domain and range
    pub fn new() -> Self {
        Self {
            domain_min: 0,
            domain_max: 1,
            range_min: 0.0,
            range_max: 1.0,
            clamped: false,
        }
    }

    /// Set the domain (timestamps in seconds since epoch)
    pub fn domain(mut self, min: i64, max: i64) -> Self {
        self.domain_min = min;
        self.domain_max = max;
        self
    }

    /// Set the range (output values)
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.range_min = min;
        self.range_max = max;
        self
    }

    /// Enable or disable clamping
    pub fn clamp(mut self, enabled: bool) -> Self {
        self.clamped = enabled;
        self
    }

    /// Adjust the domain to nice time boundaries
    pub fn nice(mut self, count: Option<usize>) -> Self {
        let span = self.domain_max - self.domain_min;
        let interval = TimeInterval::for_span(span);
        self.domain_min = interval.floor(self.domain_min);
        self.domain_max = interval.ceil(self.domain_max);

        // Optionally further adjust based on count
        if let Some(count) = count {
            let step = (span as f64) / (count as f64);
            let nice_step = nice_number(step, true) as i64;
            self.domain_min = (self.domain_min / nice_step) * nice_step;
            self.domain_max = ((self.domain_max + nice_step - 1) / nice_step) * nice_step;
        }

        self
    }

    /// Get the domain minimum
    pub fn domain_min(&self) -> i64 {
        self.domain_min
    }

    /// Get the domain maximum
    pub fn domain_max(&self) -> i64 {
        self.domain_max
    }

    /// Create a copy of this scale
    pub fn copy(&self) -> Self {
        *self
    }

    /// Get time-appropriate ticks
    pub fn time_ticks(&self, count: usize) -> Vec<i64> {
        let span = self.domain_max - self.domain_min;
        let interval = self.best_interval(span, count);
        interval.range(self.domain_min, self.domain_max + 1, 1)
    }

    /// Get the best interval for tick generation
    fn best_interval(&self, span: i64, target_count: usize) -> TimeInterval {
        // Find interval that gives approximately target_count ticks
        let intervals = [
            (TimeInterval::Second, duration::SECOND),
            (TimeInterval::Minute, duration::MINUTE),
            (TimeInterval::Hour, duration::HOUR),
            (TimeInterval::Day, duration::DAY),
            (TimeInterval::Week, duration::WEEK),
            (TimeInterval::Month, 30 * duration::DAY),
            (TimeInterval::Year, 365 * duration::DAY),
        ];

        for (interval, dur) in intervals.iter().rev() {
            let count = span / dur;
            if count as usize >= target_count {
                return *interval;
            }
        }

        TimeInterval::Second
    }

    /// Get the appropriate time interval for the current domain
    pub fn interval(&self) -> TimeInterval {
        TimeInterval::for_span(self.domain_max - self.domain_min)
    }
}

impl Scale<i64, f64> for TimeScale {
    fn scale(&self, value: i64) -> f64 {
        let value = if self.clamped {
            value.clamp(
                self.domain_min.min(self.domain_max),
                self.domain_min.max(self.domain_max),
            )
        } else {
            value
        };

        let t = (value - self.domain_min) as f64 / (self.domain_max - self.domain_min) as f64;
        self.range_min + t * (self.range_max - self.range_min)
    }

    fn invert(&self, value: f64) -> Option<i64> {
        let value = if self.clamped {
            value.clamp(
                self.range_min.min(self.range_max),
                self.range_min.max(self.range_max),
            )
        } else {
            value
        };

        let t = (value - self.range_min) / (self.range_max - self.range_min);
        Some(self.domain_min + ((self.domain_max - self.domain_min) as f64 * t) as i64)
    }

    fn ticks(&self, count: usize) -> Vec<i64> {
        self.time_ticks(count)
    }

    fn domain(&self) -> (i64, i64) {
        (self.domain_min, self.domain_max)
    }

    fn range(&self) -> (f64, f64) {
        (self.range_min, self.range_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_scale_basic() {
        let scale = TimeScale::new()
            .domain(0, 86400) // One day
            .range(0.0, 100.0);

        assert!((scale.scale(0) - 0.0).abs() < 1e-6);
        assert!((scale.scale(43200) - 50.0).abs() < 1e-6);
        assert!((scale.scale(86400) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_time_scale_invert() {
        let scale = TimeScale::new().domain(0, 86400).range(0.0, 100.0);

        assert_eq!(scale.invert(0.0).unwrap(), 0);
        assert_eq!(scale.invert(50.0).unwrap(), 43200);
        assert_eq!(scale.invert(100.0).unwrap(), 86400);
    }

    #[test]
    fn test_time_scale_clamped() {
        let scale = TimeScale::new()
            .domain(0, 86400)
            .range(0.0, 100.0)
            .clamp(true);

        assert!((scale.scale(-1000) - 0.0).abs() < 1e-6);
        assert!((scale.scale(100000) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_time_scale_ticks() {
        let scale = TimeScale::new()
            .domain(0, 86400 * 7) // One week
            .range(0.0, 700.0);

        let ticks = scale.time_ticks(7);
        assert!(!ticks.is_empty());
        assert!(ticks.len() <= 14); // Reasonable number of ticks
    }

    #[test]
    fn test_time_scale_interval() {
        // 86400 seconds = 1 day, for_span returns Day for spans >= 1 day
        let day_scale = TimeScale::new().domain(0, 86400);
        assert_eq!(day_scale.interval(), TimeInterval::Day);

        // 7 days returns Week
        let week_scale = TimeScale::new().domain(0, 86400 * 7);
        assert_eq!(week_scale.interval(), TimeInterval::Week);
    }
}
