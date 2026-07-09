//! Date and time formatting (d3-time-format)
//!
//! A lightweight UTC formatter compatible with the common D3/strftime
//! specifiers used by charts and release metadata.

const WEEKDAY_SHORT: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const WEEKDAY_LONG: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTH_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTH_LONG: [&str; 12] = [
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

/// Date format specifier
pub struct TimeFormat {
    pattern: String,
}

/// UTC date/time fields derived from a Unix timestamp in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeFormatParts {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    /// Sunday = 0, Monday = 1, ..., Saturday = 6.
    pub weekday: u32,
    /// Day of year, starting at 1.
    pub day_of_year: u32,
}

impl TimeFormat {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
        }
    }

    /// Format a timestamp (Unix seconds)
    pub fn format(&self, timestamp: i64) -> String {
        let parts = TimeFormatParts::from_unix_seconds(timestamp);
        let mut result = String::with_capacity(self.pattern.len() + 16);
        let mut chars = self.pattern.chars();

        while let Some(ch) = chars.next() {
            if ch != '%' {
                result.push(ch);
                continue;
            }

            let Some(specifier) = chars.next() else {
                result.push('%');
                break;
            };

            parts.push_specifier(specifier, timestamp, &mut result);
        }

        result
    }
}

impl TimeFormatParts {
    /// Convert a Unix timestamp in seconds to UTC calendar fields.
    pub fn from_unix_seconds(timestamp: i64) -> Self {
        let days = timestamp.div_euclid(86_400);
        let seconds_in_day = timestamp.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);

        Self {
            year,
            month,
            day,
            hour: (seconds_in_day / 3_600) as u32,
            minute: ((seconds_in_day % 3_600) / 60) as u32,
            second: (seconds_in_day % 60) as u32,
            weekday: (days + 4).rem_euclid(7) as u32,
            day_of_year: day_of_year(year, month, day),
        }
    }

    fn push_specifier(self, specifier: char, timestamp: i64, output: &mut String) {
        match specifier {
            '%' => output.push('%'),
            'a' => output.push_str(WEEKDAY_SHORT[self.weekday as usize]),
            'A' => output.push_str(WEEKDAY_LONG[self.weekday as usize]),
            'b' => output.push_str(MONTH_SHORT[(self.month - 1) as usize]),
            'B' => output.push_str(MONTH_LONG[(self.month - 1) as usize]),
            'c' => self.push_composite_datetime(output),
            'd' => output.push_str(&format!("{:02}", self.day)),
            'e' => output.push_str(&format!("{:2}", self.day)),
            'H' => output.push_str(&format!("{:02}", self.hour)),
            'I' => output.push_str(&format!("{:02}", self.hour_12())),
            'j' => output.push_str(&format!("{:03}", self.day_of_year)),
            'L' => output.push_str("000"),
            'm' => output.push_str(&format!("{:02}", self.month)),
            'M' => output.push_str(&format!("{:02}", self.minute)),
            'p' => output.push_str(if self.hour < 12 { "AM" } else { "PM" }),
            'Q' => output.push_str(&format!("{}", timestamp.saturating_mul(1000))),
            's' => output.push_str(&format!("{timestamp}")),
            'S' => output.push_str(&format!("{:02}", self.second)),
            'u' => output.push_str(&format!("{}", self.iso_weekday())),
            'w' => output.push_str(&format!("{}", self.weekday)),
            'x' => output.push_str(&format!(
                "{:02}/{:02}/{:04}",
                self.month, self.day, self.year
            )),
            'X' => output.push_str(&format!(
                "{:02}:{:02}:{:02}",
                self.hour, self.minute, self.second
            )),
            'y' => output.push_str(&format!("{:02}", self.year.rem_euclid(100))),
            'Y' => output.push_str(&format_year(self.year)),
            'Z' => output.push_str("+0000"),
            unknown => {
                output.push('%');
                output.push(unknown);
            }
        }
    }

    fn hour_12(self) -> u32 {
        let hour = self.hour % 12;
        if hour == 0 { 12 } else { hour }
    }

    fn iso_weekday(self) -> u32 {
        if self.weekday == 0 { 7 } else { self.weekday }
    }

    fn push_composite_datetime(self, output: &mut String) {
        output.push_str(WEEKDAY_SHORT[self.weekday as usize]);
        output.push(' ');
        output.push_str(MONTH_SHORT[(self.month - 1) as usize]);
        output.push(' ');
        output.push_str(&format!(
            "{:2} {:02}:{:02}:{:02} {}",
            self.day, self.hour, self.minute, self.second, self.year
        ));
    }
}

/// Helper to format date
pub fn format(pattern: &str, timestamp: i64) -> String {
    TimeFormat::new(pattern).format(timestamp)
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };

    (year, month as u32, day as u32)
}

fn day_of_year(year: i64, month: u32, day: u32) -> u32 {
    const COMMON_MONTH_STARTS: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let leap_day = u32::from(month > 2 && is_leap_year(year));
    COMMON_MONTH_STARTS[(month - 1) as usize] + day + leap_day
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn format_year(year: i64) -> String {
    if (0..=9999).contains(&year) {
        format!("{year:04}")
    } else {
        format!("{year}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_new() {
        let fmt = TimeFormat::new("%Y-%m-%d %H:%M:%S");
        let s = fmt.format(0);
        assert_eq!(s, "1970-01-01 00:00:00");
    }

    #[test]
    fn test_format_helper() {
        assert_eq!(format("%Y/%m/%d", 0), "1970/01/01");
    }

    #[test]
    fn test_format_specific_timestamp() {
        // 2024-06-21 12:13:38 UTC
        let ts = 1718972018;
        assert_eq!(format("%Y-%m-%d %H:%M:%S", ts), "2024-06-21 12:13:38");
    }

    #[test]
    fn test_format_partial_pattern() {
        assert_eq!(format("%H:%M", 3661), "01:01");
    }

    #[test]
    fn test_format_negative_timestamp() {
        assert_eq!(format("%Y-%m-%d %H:%M:%S", -7200), "1969-12-31 22:00:00");
    }

    #[test]
    fn test_format_no_tokens() {
        assert_eq!(format("hello world", 12345), "hello world");
    }

    #[test]
    fn formats_common_d3_time_specifiers() {
        let ts = 1718972018; // Friday, June 21 2024 12:13:38 UTC

        assert_eq!(
            format("%a %A %b %B %d %e %j %m %u %w %y %Y", ts),
            "Fri Friday Jun June 21 21 173 06 5 5 24 2024"
        );
        assert_eq!(
            format("%H %I %M %p %S %L %Z", ts),
            "12 12 13 PM 38 000 +0000"
        );
        assert_eq!(
            format("%x %X %c", ts),
            "06/21/2024 12:13:38 Fri Jun 21 12:13:38 2024"
        );
        assert_eq!(format("%% %s %Q", 42), "% 42 42000");
    }

    #[test]
    fn leaves_unknown_specifiers_literal() {
        assert_eq!(format("%q %Y", 0), "%q 1970");
    }

    #[test]
    fn exposes_calendar_parts_for_release_tests() {
        let parts = TimeFormatParts::from_unix_seconds(951_782_400);

        assert_eq!(
            parts,
            TimeFormatParts {
                year: 2000,
                month: 2,
                day: 29,
                hour: 0,
                minute: 0,
                second: 0,
                weekday: 2,
                day_of_year: 60,
            }
        );
    }
}
