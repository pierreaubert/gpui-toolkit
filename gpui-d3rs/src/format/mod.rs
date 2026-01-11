//! Number formatting module (d3-format)
//!
//! This module provides a flexible number formatting system based on D3.js's d3-format.
//!
//! ## Format Specifiers
//!
//! Format specifiers follow the pattern: `[[fill]align][sign][symbol][0][width][,][.precision][~][type]`
//!
//! - **fill**: Any character to use for padding
//! - **align**: `<` (left), `>` (right), `^` (center), `=` (pad after sign)
//! - **sign**: `-` (minus only), `+` (plus/minus), ` ` (space/minus), `(` (parens for negative)
//! - **symbol**: `$` (currency), `#` (alternate form)
//! - **0**: Zero-pad
//! - **width**: Minimum field width
//! - **,**: Use grouping separator
//! - **precision**: Number of digits
//! - **~**: Trim insignificant trailing zeros
//! - **type**: `e`, `f`, `g`, `r`, `s`, `%`, `p`, `b`, `o`, `d`, `x`, `X`, `c`
//!
//! # Example
//!
//! ```
//! use d3rs::format::{format, format_prefix};
//!
//! // Basic number formatting
//! assert_eq!(format(".2f")(3.14159), "3.14");
//! assert_eq!(format(",.0f")(1234567.0), "1,234,567");
//!
//! // SI prefix formatting
//! assert_eq!(format(".4s")(1234.0), "1.2340k");
//! ```

mod locale;
mod prefix;
mod specifier;

pub use locale::{DEFAULT_LOCALE, Locale};
pub use prefix::{format_prefix, prefix_exponent};
pub use specifier::{Align, FormatSpecifier, FormatType, Sign};

use specifier::parse_specifier;

/// Create a formatter function for the given specifier string
///
/// # Example
///
/// ```
/// use d3rs::format::format;
///
/// let fmt = format(".2f");
/// assert_eq!(fmt(3.14159), "3.14");
///
/// let fmt = format("+.1%");
/// assert_eq!(fmt(0.255), "+25.5%");
/// ```
pub fn format(specifier: &str) -> impl Fn(f64) -> String {
    let spec = parse_specifier(specifier);
    let locale = DEFAULT_LOCALE;
    move |value| locale.format(&spec, value)
}

/// Create a formatter using a specific locale
///
/// # Example
///
/// ```
/// use d3rs::format::{format_locale, Locale};
///
/// let french = Locale::new(",", " ", Some("€"), Some("€"));
/// let fmt = format_locale(&french, ",.2f");
/// assert_eq!(fmt(1234.56), "1 234,56");
/// ```
pub fn format_locale<'a>(locale: &'a Locale, specifier: &str) -> impl Fn(f64) -> String + 'a {
    let spec = parse_specifier(specifier);
    move |value| locale.format(&spec, value)
}

/// Format a single value with a specifier
///
/// Convenience function when you don't need to reuse the formatter.
pub fn format_value(specifier: &str, value: f64) -> String {
    format(specifier)(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_basic() {
        assert_eq!(format("d")(42.0), "42");
        assert_eq!(format("f")(3.14), "3.140000");
        assert_eq!(format(".2f")(3.14159), "3.14");
        assert_eq!(format(".0f")(3.9), "4");
    }

    #[test]
    fn test_format_precision() {
        assert_eq!(format(".1f")(1.0), "1.0");
        assert_eq!(format(".2f")(1.0), "1.00");
        assert_eq!(format(".3f")(1.0), "1.000");
    }

    #[test]
    fn test_format_grouping() {
        assert_eq!(format(",")(1234.0), "1,234");
        assert_eq!(format(",")(1234567.0), "1,234,567");
        assert_eq!(format(",.2f")(1234567.89), "1,234,567.89");
    }

    #[test]
    fn test_format_sign() {
        assert_eq!(format("+d")(42.0), "+42");
        assert_eq!(format("+d")(-42.0), "-42");
        assert_eq!(format(" d")(42.0), " 42");
        assert_eq!(format(" d")(-42.0), "-42");
    }

    #[test]
    fn test_format_percentage() {
        assert_eq!(format(".0%")(0.5), "50%");
        assert_eq!(format(".1%")(0.255), "25.5%");
        assert_eq!(format(".2%")(0.01), "1.00%");
    }

    #[test]
    fn test_format_si_prefix() {
        // Rust's formatting gives more precision
        assert_eq!(format(".2s")(1234.0), "1.23k");
        assert_eq!(format(".2s")(1234567.0), "1.23M");
        assert_eq!(format(".2s")(0.00123), "1.23m");
    }

    #[test]
    fn test_format_exponential() {
        // Rust uses lowercase 'e' without + sign for positive exponents
        assert_eq!(format(".2e")(1234.0), "1.23e3");
        assert_eq!(format(".2e")(0.00123), "1.23e-3");
    }

    #[test]
    fn test_format_width() {
        assert_eq!(format("8d")(42.0), "      42");
        assert_eq!(format("08d")(42.0), "00000042");
        assert_eq!(format("<8d")(42.0), "42      ");
        assert_eq!(format("^8d")(42.0), "   42   ");
    }

    #[test]
    fn test_format_hex() {
        assert_eq!(format("x")(255.0), "ff");
        assert_eq!(format("X")(255.0), "FF");
        assert_eq!(format("#x")(255.0), "0xff");
    }

    #[test]
    fn test_format_binary_octal() {
        assert_eq!(format("b")(10.0), "1010");
        assert_eq!(format("o")(64.0), "100");
    }
}
