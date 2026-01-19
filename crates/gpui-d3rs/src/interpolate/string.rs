//! String interpolation functions
//!
//! Provides interpolation for strings containing numbers.

use regex::Regex;
use std::sync::LazyLock;

// Regex to find numbers in strings
static NUMBER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-?(?:\d+\.?\d*|\d*\.?\d+)(?:[eE][+-]?\d+)?").unwrap());

/// Interpolate between two strings containing numbers.
///
/// Numbers embedded in strings are interpolated; other characters are taken
/// from the ending string.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate_string;
///
/// let interp = interpolate_string("10px", "20px");
/// assert_eq!(interp(0.0), "10px");
/// assert_eq!(interp(0.5), "15px");
/// assert_eq!(interp(1.0), "20px");
/// ```
pub fn interpolate_string(a: &str, b: &str) -> impl Fn(f64) -> String {
    let a_numbers: Vec<f64> = NUMBER_RE
        .find_iter(a)
        .filter_map(|m| m.as_str().parse().ok())
        .collect();

    let b_numbers: Vec<f64> = NUMBER_RE
        .find_iter(b)
        .filter_map(|m| m.as_str().parse().ok())
        .collect();

    // Get the parts between numbers from string b
    let b_parts: Vec<String> = NUMBER_RE.split(b).map(|s| s.to_string()).collect();

    move |t| {
        let mut result = String::new();
        let n = a_numbers.len().min(b_numbers.len());

        for (i, part) in b_parts.iter().enumerate() {
            result.push_str(part);
            if i < n {
                let val = a_numbers[i] + (b_numbers[i] - a_numbers[i]) * t;
                // Format with appropriate precision
                if val.fract().abs() < 1e-10 {
                    result.push_str(&format!("{}", val as i64));
                } else {
                    result.push_str(
                        format!("{:.6}", val)
                            .trim_end_matches('0')
                            .trim_end_matches('.'),
                    );
                }
            } else if i < b_numbers.len() {
                // Use b's number directly if a doesn't have enough numbers
                result.push_str(&format!("{}", b_numbers[i]));
            }
        }

        result
    }
}

/// Interpolate CSS transform strings.
///
/// Handles translate, rotate, scale, and skew transforms.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate_transform_css;
///
/// let interp = interpolate_transform_css(
///     "translate(0px, 0px) rotate(0deg)",
///     "translate(100px, 50px) rotate(180deg)"
/// );
///
/// let mid = interp(0.5);
/// assert!(mid.contains("50")); // Should contain interpolated values
/// ```
pub fn interpolate_transform_css(a: &str, b: &str) -> impl Fn(f64) -> String {
    // Parse transforms from both strings
    let a_transforms = parse_css_transforms(a);
    let b_transforms = parse_css_transforms(b);

    move |t| {
        let mut result = Vec::new();

        // Match transforms by type
        for bt in &b_transforms {
            let at = a_transforms.iter().find(|at| at.name == bt.name);
            let interp_values: Vec<f64> = match at {
                Some(at) => bt
                    .values
                    .iter()
                    .zip(at.values.iter())
                    .map(|(bv, av)| av + (bv - av) * t)
                    .collect(),
                None => bt.values.clone(),
            };
            result.push(format_transform(&bt.name, &interp_values, &bt.units));
        }

        result.join(" ")
    }
}

#[derive(Debug, Clone)]
struct CssTransform {
    name: String,
    values: Vec<f64>,
    units: Vec<String>,
}

fn parse_css_transforms(s: &str) -> Vec<CssTransform> {
    static TRANSFORM_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(\w+)\(([^)]+)\)").unwrap());

    TRANSFORM_RE
        .captures_iter(s)
        .map(|cap| {
            let name = cap[1].to_string();
            let args = &cap[2];

            let mut values = Vec::new();
            let mut units = Vec::new();

            for part in args.split(',') {
                let part = part.trim();
                // Extract number and unit
                let num_str: String = part
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                    .collect();
                let unit: String = part.chars().skip(num_str.len()).collect();

                if let Ok(num) = num_str.parse::<f64>() {
                    values.push(num);
                    units.push(unit);
                }
            }

            CssTransform {
                name,
                values,
                units,
            }
        })
        .collect()
}

fn format_transform(name: &str, values: &[f64], units: &[String]) -> String {
    let args: Vec<String> = values
        .iter()
        .zip(units.iter())
        .map(|(v, u)| {
            if v.fract().abs() < 1e-10 {
                format!("{}{}", *v as i64, u)
            } else {
                format!("{:.2}{}", v, u)
            }
        })
        .collect();
    format!("{}({})", name, args.join(", "))
}

/// Interpolate between two dates (as timestamps).
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate_date;
///
/// // Timestamps in milliseconds
/// let start = 0.0; // Jan 1, 1970
/// let end = 86400000.0; // Jan 2, 1970 (24 hours later)
///
/// let interp = interpolate_date(start, end);
/// assert_eq!(interp(0.5), 43200000.0); // 12 hours later
/// ```
pub fn interpolate_date(a: f64, b: f64) -> impl Fn(f64) -> f64 {
    move |t| a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_string_simple() {
        let interp = interpolate_string("10px", "20px");
        assert_eq!(interp(0.0), "10px");
        assert_eq!(interp(0.5), "15px");
        assert_eq!(interp(1.0), "20px");
    }

    #[test]
    fn test_interpolate_string_multiple() {
        let interp = interpolate_string("0 0", "100 200");
        let result = interp(0.5);
        assert!(result.contains("50"));
        assert!(result.contains("100"));
    }

    #[test]
    fn test_interpolate_transform_css() {
        let interp = interpolate_transform_css("translate(0px, 0px)", "translate(100px, 50px)");

        let mid = interp(0.5);
        assert!(mid.contains("50"));
        assert!(mid.contains("25"));
    }

    #[test]
    fn test_parse_css_transforms() {
        let transforms = parse_css_transforms("translate(10px, 20px) rotate(45deg)");
        assert_eq!(transforms.len(), 2);
        assert_eq!(transforms[0].name, "translate");
        assert_eq!(transforms[0].values, vec![10.0, 20.0]);
        assert_eq!(transforms[1].name, "rotate");
        assert_eq!(transforms[1].values, vec![45.0]);
    }
}
