//! Automatic type inference for DSV data
//!
//! Provides d3.autoType-like functionality to automatically convert string
//! values to appropriate types (numbers, booleans, dates, etc.).

/// A value that has been automatically typed.
#[derive(Debug, Clone, PartialEq)]
pub enum AutoTyped {
    /// A null/empty value
    Null,
    /// A boolean value
    Bool(bool),
    /// An integer value
    Integer(i64),
    /// A floating-point value
    Float(f64),
    /// A string value (no conversion possible)
    String(String),
    /// A date string (ISO 8601 format detected)
    Date(String),
}

impl AutoTyped {
    /// Get as f64, if possible.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            AutoTyped::Integer(i) => Some(*i as f64),
            AutoTyped::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Get as i64, if possible.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            AutoTyped::Integer(i) => Some(*i),
            AutoTyped::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    /// Get as bool, if possible.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            AutoTyped::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as string reference.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            AutoTyped::String(s) => Some(s),
            AutoTyped::Date(s) => Some(s),
            _ => None,
        }
    }

    /// Check if this is a null value.
    pub fn is_null(&self) -> bool {
        matches!(self, AutoTyped::Null)
    }
}

/// Automatically infer and convert the type of a string value.
///
/// Follows d3.autoType conventions:
/// - Empty strings become null
/// - "true"/"false" become booleans
/// - "NaN" stays as NaN
/// - Numbers (including scientific notation) become integers or floats
/// - ISO 8601 date strings are detected
/// - Everything else stays as a string
///
/// # Example
///
/// ```
/// use d3rs::fetch::{auto_type, AutoTyped};
///
/// assert_eq!(auto_type("42"), AutoTyped::Integer(42));
/// assert_eq!(auto_type("3.14"), AutoTyped::Float(3.14));
/// assert_eq!(auto_type("true"), AutoTyped::Bool(true));
/// assert_eq!(auto_type(""), AutoTyped::Null);
/// assert_eq!(auto_type("hello"), AutoTyped::String("hello".to_string()));
/// ```
pub fn auto_type(value: &str) -> AutoTyped {
    let trimmed = value.trim();

    // Empty string is null
    if trimmed.is_empty() {
        return AutoTyped::Null;
    }

    // Boolean
    if trimmed.eq_ignore_ascii_case("true") {
        return AutoTyped::Bool(true);
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return AutoTyped::Bool(false);
    }

    // NaN
    if trimmed.eq_ignore_ascii_case("nan") {
        return AutoTyped::Float(f64::NAN);
    }

    // Infinity
    if trimmed.eq_ignore_ascii_case("infinity") || trimmed == "inf" {
        return AutoTyped::Float(f64::INFINITY);
    }
    if trimmed.eq_ignore_ascii_case("-infinity") || trimmed == "-inf" {
        return AutoTyped::Float(f64::NEG_INFINITY);
    }

    // Try to parse as integer first
    if let Ok(i) = trimmed.parse::<i64>() {
        return AutoTyped::Integer(i);
    }

    // Try to parse as float
    if let Ok(f) = trimmed.parse::<f64>() {
        return AutoTyped::Float(f);
    }

    // Check for ISO 8601 date format
    if is_iso_date(trimmed) {
        return AutoTyped::Date(trimmed.to_string());
    }

    // Default to string
    AutoTyped::String(value.to_string())
}

/// Check if a string looks like an ISO 8601 date.
fn is_iso_date(s: &str) -> bool {
    // Check for YYYY-MM-DD format
    if s.len() >= 10 {
        let bytes = s.as_bytes();
        if bytes.len() >= 10
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes[0..4].iter().all(|&b| b.is_ascii_digit())
            && bytes[5..7].iter().all(|&b| b.is_ascii_digit())
            && bytes[8..10].iter().all(|&b| b.is_ascii_digit())
        {
            return true;
        }
    }
    false
}

/// Convert a row of string values to auto-typed values.
///
/// # Example
///
/// ```
/// use d3rs::fetch::auto_type_row;
/// use std::collections::HashMap;
///
/// let mut row: HashMap<String, String> = HashMap::new();
/// row.insert("age".to_string(), "25".to_string());
/// row.insert("name".to_string(), "alice".to_string());
///
/// let typed = auto_type_row(&row);
/// assert!(matches!(typed.get("age"), Some(d3rs::fetch::AutoTyped::Integer(25))));
/// ```
pub fn auto_type_row(
    row: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, AutoTyped> {
    row.iter().map(|(k, v)| (k.clone(), auto_type(v))).collect()
}

/// Convert multiple rows to auto-typed values.
pub fn auto_type_rows(
    rows: &[std::collections::HashMap<String, String>],
) -> Vec<std::collections::HashMap<String, AutoTyped>> {
    rows.iter().map(auto_type_row).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_type_integer() {
        assert_eq!(auto_type("42"), AutoTyped::Integer(42));
        assert_eq!(auto_type("-123"), AutoTyped::Integer(-123));
        assert_eq!(auto_type("0"), AutoTyped::Integer(0));
    }

    #[test]
    fn test_auto_type_float() {
        assert_eq!(auto_type("3.14"), AutoTyped::Float(3.14));
        assert_eq!(auto_type("-0.5"), AutoTyped::Float(-0.5));
        assert_eq!(auto_type("1e10"), AutoTyped::Float(1e10));
        assert_eq!(auto_type("1.5e-3"), AutoTyped::Float(1.5e-3));
    }

    #[test]
    fn test_auto_type_bool() {
        assert_eq!(auto_type("true"), AutoTyped::Bool(true));
        assert_eq!(auto_type("false"), AutoTyped::Bool(false));
        assert_eq!(auto_type("TRUE"), AutoTyped::Bool(true));
        assert_eq!(auto_type("False"), AutoTyped::Bool(false));
    }

    #[test]
    fn test_auto_type_null() {
        assert_eq!(auto_type(""), AutoTyped::Null);
        assert_eq!(auto_type("  "), AutoTyped::Null);
    }

    #[test]
    fn test_auto_type_special() {
        assert!(matches!(auto_type("NaN"), AutoTyped::Float(f) if f.is_nan()));
        assert_eq!(auto_type("infinity"), AutoTyped::Float(f64::INFINITY));
        assert_eq!(auto_type("-infinity"), AutoTyped::Float(f64::NEG_INFINITY));
    }

    #[test]
    fn test_auto_type_date() {
        assert_eq!(
            auto_type("2023-01-15"),
            AutoTyped::Date("2023-01-15".to_string())
        );
        assert_eq!(
            auto_type("2023-12-31T10:30:00"),
            AutoTyped::Date("2023-12-31T10:30:00".to_string())
        );
    }

    #[test]
    fn test_auto_type_string() {
        assert_eq!(auto_type("hello"), AutoTyped::String("hello".to_string()));
        assert_eq!(
            auto_type("foo bar"),
            AutoTyped::String("foo bar".to_string())
        );
    }

    #[test]
    fn test_auto_typed_as_methods() {
        assert_eq!(AutoTyped::Integer(42).as_f64(), Some(42.0));
        assert_eq!(AutoTyped::Float(3.14).as_i64(), Some(3));
        assert_eq!(AutoTyped::Bool(true).as_bool(), Some(true));
        assert_eq!(AutoTyped::String("test".to_string()).as_str(), Some("test"));
    }
}
