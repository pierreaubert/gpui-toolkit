//! SI prefix formatting utilities

use super::locale::DEFAULT_LOCALE;
use super::specifier::parse_specifier;

/// SI prefix symbols
const SI_PREFIXES: &[(&str, i32)] = &[
    ("y", -24),
    ("z", -21),
    ("a", -18),
    ("f", -15),
    ("p", -12),
    ("n", -9),
    ("µ", -6),
    ("m", -3),
    ("", 0),
    ("k", 3),
    ("M", 6),
    ("G", 9),
    ("T", 12),
    ("P", 15),
    ("E", 18),
    ("Z", 21),
    ("Y", 24),
];

/// Calculate the SI prefix exponent for a value
///
/// Returns the power of 1000 to use (e.g., 3 for kilo, 6 for mega)
///
/// # Example
///
/// ```
/// use d3rs::format::prefix_exponent;
///
/// assert_eq!(prefix_exponent(1.0), 0);
/// assert_eq!(prefix_exponent(1000.0), 3);
/// assert_eq!(prefix_exponent(1_000_000.0), 6);
/// assert_eq!(prefix_exponent(0.001), -3);
/// ```
pub fn prefix_exponent(value: f64) -> i32 {
    if value == 0.0 {
        return 0;
    }

    let exp = (value.abs().log10() / 3.0).floor() as i32;
    exp.clamp(-8, 8) * 3
}

/// Get the SI prefix symbol for a given exponent
fn prefix_symbol(exp: i32) -> &'static str {
    let index = ((exp / 3) + 8) as usize;
    if index < SI_PREFIXES.len() {
        SI_PREFIXES[index].0
    } else {
        ""
    }
}

/// Create a formatter that uses a fixed SI prefix
///
/// This is useful when you want to format multiple values with the same prefix,
/// such as when labeling an axis.
///
/// # Example
///
/// ```
/// use d3rs::format::format_prefix;
///
/// // Format using kilo prefix
/// let fmt = format_prefix(".2", 1e3);
/// assert_eq!(fmt(1000.0), "1.00k");
/// assert_eq!(fmt(2500.0), "2.50k");
///
/// // Format using mega prefix
/// let fmt = format_prefix(".2", 1e6);
/// assert_eq!(fmt(1_000_000.0), "1.00M");
/// assert_eq!(fmt(2_500_000.0), "2.50M");
/// ```
pub fn format_prefix(specifier: &str, value: f64) -> impl Fn(f64) -> String {
    let spec = parse_specifier(specifier);
    let exp = prefix_exponent(value);
    let prefix = prefix_symbol(exp).to_string();
    let scale = 10_f64.powi(-exp);

    move |v: f64| {
        let scaled = v * scale;
        let formatted = DEFAULT_LOCALE.format(&spec, scaled);
        format!("{}{}", formatted, prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_exponent() {
        assert_eq!(prefix_exponent(0.0), 0);
        assert_eq!(prefix_exponent(1.0), 0);
        assert_eq!(prefix_exponent(10.0), 0);
        assert_eq!(prefix_exponent(100.0), 0);
        assert_eq!(prefix_exponent(1000.0), 3);
        assert_eq!(prefix_exponent(10000.0), 3);
        assert_eq!(prefix_exponent(1_000_000.0), 6);
        assert_eq!(prefix_exponent(0.001), -3);
        assert_eq!(prefix_exponent(0.000001), -6);
    }

    #[test]
    fn test_format_prefix_kilo() {
        let fmt = format_prefix(".2", 1e3);
        assert_eq!(fmt(1000.0), "1.00k");
        assert_eq!(fmt(2000.0), "2.00k");
        assert_eq!(fmt(2500.0), "2.50k");
    }

    #[test]
    fn test_format_prefix_mega() {
        let fmt = format_prefix(".2", 1e6);
        assert_eq!(fmt(1_000_000.0), "1.00M");
        assert_eq!(fmt(2_500_000.0), "2.50M");
    }

    #[test]
    fn test_format_prefix_milli() {
        let fmt = format_prefix(".2", 1e-3);
        assert_eq!(fmt(0.001), "1.00m");
        assert_eq!(fmt(0.0025), "2.50m");
    }

    #[test]
    fn test_format_prefix_micro() {
        let fmt = format_prefix(".2", 1e-6);
        assert_eq!(fmt(0.000001), "1.00µ");
    }
}
