//! Date and time formatting (d3-time-format)
//!
//! A simplified implementation of date formatting string generation.
//! Full strftime support would require a heavy dependency like `chrono`.
//! Here we provide a lightweight formatter compatible with standard D3 expectations.

// use super::interval::{Interval, TimeInterval};

/// Date format specifier
pub struct TimeFormat {
    pattern: String,
}

impl TimeFormat {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
        }
    }

    /// Format a timestamp (Unix seconds)
    pub fn format(&self, timestamp: i64) -> String {
        // We use the `time` crate or `chrono` if available, or a simple implementation
        // Since we don't want to add heavy dependencies for this specific demo unless requested,
        // we'll defer to a minimal implementation or assuming `chrono` is available if common.
        // However, `gpui-d3rs` Cargo.toml doesn't show chrono.

        // Let's implement basic ISO formatting and simple tokens.
        // %Y, %m, %d, %H, %M, %S

        // Convert timestamp to components (UTC)
        let days = timestamp / 86400;
        let seconds_in_day = timestamp % 86400;
        let hour = seconds_in_day / 3600;
        let minute = (seconds_in_day % 3600) / 60;
        let second = seconds_in_day % 60;

        // Simplified Gregorian calendar
        let z = days + 719468;
        let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = mp + (if mp < 10 { 3 } else { -9 });
        let year = y + (if m <= 2 { 1 } else { 0 });

        let mut result = self.pattern.clone();

        result = result.replace("%Y", &format!("{:04}", year));
        result = result.replace("%m", &format!("{:02}", m));
        result = result.replace("%d", &format!("{:02}", d));
        result = result.replace("%H", &format!("{:02}", hour));
        result = result.replace("%M", &format!("{:02}", minute));
        result = result.replace("%S", &format!("{:02}", second));

        result
    }
}

/// Helper to format date
pub fn format(pattern: &str, timestamp: i64) -> String {
    TimeFormat::new(pattern).format(timestamp)
}
