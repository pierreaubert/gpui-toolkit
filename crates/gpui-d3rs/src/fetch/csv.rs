//! CSV and TSV parsing utilities
//!
//! High-level functions for parsing comma-separated and tab-separated values.

use super::dsv::{DsvBudget, DsvParseError, DsvParser, DsvResult, DsvRow};

/// Options for CSV/TSV parsing.
#[derive(Debug, Clone)]
pub struct CsvOptions {
    /// Whether to skip empty lines (default: true)
    pub skip_empty_lines: bool,
    /// Whether to trim whitespace from values (default: true)
    pub trim_values: bool,
}

impl Default for CsvOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl CsvOptions {
    /// Create default options.
    pub fn new() -> Self {
        Self {
            skip_empty_lines: true,
            trim_values: true,
        }
    }
}

/// Parse a CSV string into rows, returning structured errors.
///
/// # Example
///
/// ```
/// use d3rs::fetch::parse_csv;
///
/// let data = "name,value\nalice,10\nbob,20";
/// let rows = parse_csv(data).unwrap();
/// assert_eq!(rows.len(), 2);
/// assert_eq!(rows[0].get("name"), Some(&"alice".to_string()));
/// assert_eq!(rows[0].get("value"), Some(&"10".to_string()));
/// ```
pub fn parse_csv(text: &str) -> DsvResult<Vec<DsvRow>> {
    DsvParser::new(',').parse(text)
}

/// Parse CSV with explicit size, allocation, and record limits.
pub fn parse_csv_with_budget(text: &str, budget: DsvBudget) -> DsvResult<Vec<DsvRow>> {
    DsvParser::new(',').parse_with_budget(text, budget)
}

/// Parse CSV with limits and a cooperative cancellation callback.
pub fn parse_csv_with_budget_and_cancel<F: FnMut() -> bool>(
    text: &str,
    budget: DsvBudget,
    cancelled: F,
) -> DsvResult<Vec<DsvRow>> {
    DsvParser::new(',').parse_with_budget_and_cancel(text, budget, cancelled)
}

/// Parse a CSV string and return an empty vector if parsing fails.
pub fn parse_csv_lossy(text: &str) -> Vec<DsvRow> {
    DsvParser::new(',').parse_lossy(text)
}

/// Compatibility alias for callers that still use the older fallible name.
pub fn try_parse_csv(text: &str) -> Result<Vec<DsvRow>, DsvParseError> {
    parse_csv(text)
}

/// Parse a CSV string with options, returning structured errors.
///
/// # Example
///
/// ```
/// use d3rs::fetch::{parse_csv_with_options, CsvOptions};
///
/// let options = CsvOptions {
///     skip_empty_lines: true,
///     trim_values: true,
/// };
///
/// let data = "name,value\n alice , 10 \nbob,20";
/// let rows = parse_csv_with_options(data, &options).unwrap();
/// assert_eq!(rows[0].get("name"), Some(&"alice".to_string()));
/// ```
pub fn parse_csv_with_options(text: &str, options: &CsvOptions) -> DsvResult<Vec<DsvRow>> {
    DsvParser::new(',')
        .skip_empty_lines(options.skip_empty_lines)
        .trim_values(options.trim_values)
        .parse(text)
}

/// Parse a CSV string with options and return an empty vector if parsing fails.
pub fn parse_csv_lossy_with_options(text: &str, options: &CsvOptions) -> Vec<DsvRow> {
    DsvParser::new(',')
        .skip_empty_lines(options.skip_empty_lines)
        .trim_values(options.trim_values)
        .parse_lossy(text)
}

/// Compatibility alias for callers that still use the older fallible name.
pub fn try_parse_csv_with_options(
    text: &str,
    options: &CsvOptions,
) -> Result<Vec<DsvRow>, DsvParseError> {
    parse_csv_with_options(text, options)
}

/// Parse a TSV (tab-separated) string into rows, returning structured errors.
///
/// # Example
///
/// ```
/// use d3rs::fetch::parse_tsv;
///
/// let data = "name\tvalue\nalice\t10\nbob\t20";
/// let rows = parse_tsv(data).unwrap();
/// assert_eq!(rows.len(), 2);
/// assert_eq!(rows[0].get("name"), Some(&"alice".to_string()));
/// ```
pub fn parse_tsv(text: &str) -> DsvResult<Vec<DsvRow>> {
    DsvParser::new('\t').parse(text)
}

/// Parse TSV with explicit size, allocation, and record limits.
pub fn parse_tsv_with_budget(text: &str, budget: DsvBudget) -> DsvResult<Vec<DsvRow>> {
    DsvParser::new('\t').parse_with_budget(text, budget)
}

/// Parse TSV with limits and a cooperative cancellation callback.
pub fn parse_tsv_with_budget_and_cancel<F: FnMut() -> bool>(
    text: &str,
    budget: DsvBudget,
    cancelled: F,
) -> DsvResult<Vec<DsvRow>> {
    DsvParser::new('\t').parse_with_budget_and_cancel(text, budget, cancelled)
}

/// Parse a TSV string and return an empty vector if parsing fails.
pub fn parse_tsv_lossy(text: &str) -> Vec<DsvRow> {
    DsvParser::new('\t').parse_lossy(text)
}

/// Compatibility alias for callers that still use the older fallible name.
pub fn try_parse_tsv(text: &str) -> Result<Vec<DsvRow>, DsvParseError> {
    parse_tsv(text)
}

/// Parse a TSV string with options, returning structured errors.
pub fn parse_tsv_with_options(text: &str, options: &CsvOptions) -> DsvResult<Vec<DsvRow>> {
    DsvParser::new('\t')
        .skip_empty_lines(options.skip_empty_lines)
        .trim_values(options.trim_values)
        .parse(text)
}

/// Parse a TSV string with options and return an empty vector if parsing fails.
pub fn parse_tsv_lossy_with_options(text: &str, options: &CsvOptions) -> Vec<DsvRow> {
    DsvParser::new('\t')
        .skip_empty_lines(options.skip_empty_lines)
        .trim_values(options.trim_values)
        .parse_lossy(text)
}

/// Compatibility alias for callers that still use the older fallible name.
pub fn try_parse_tsv_with_options(
    text: &str,
    options: &CsvOptions,
) -> Result<Vec<DsvRow>, DsvParseError> {
    parse_tsv_with_options(text, options)
}

/// Format rows as CSV text.
///
/// # Example
///
/// ```
/// use d3rs::fetch::{parse_csv, format_csv};
///
/// let data = "name,value\nalice,10\nbob,20";
/// let rows = parse_csv(data).unwrap();
/// let output = format_csv(&rows, &["name", "value"]);
/// assert!(output.contains("alice,10"));
/// ```
pub fn format_csv(rows: &[DsvRow], columns: &[&str]) -> String {
    DsvParser::new(',').format(rows, columns)
}

/// Format rows as TSV text.
pub fn format_tsv(rows: &[DsvRow], columns: &[&str]) -> String {
    DsvParser::new('\t').format(rows, columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv() {
        let data = "a,b,c\n1,2,3\n4,5,6";
        let rows = parse_csv(data).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("a"), Some(&"1".to_string()));
    }

    #[test]
    fn test_parse_tsv() {
        let data = "a\tb\tc\n1\t2\t3";
        let rows = parse_tsv(data).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("b"), Some(&"2".to_string()));
    }

    #[test]
    fn test_format_csv() {
        let mut row = DsvRow::new();
        row.insert("x".to_string(), "1".to_string());
        row.insert("y".to_string(), "2".to_string());
        let result = format_csv(&[row], &["x", "y"]);
        assert!(result.starts_with("x,y"));
        assert!(result.contains("1,2"));
    }

    #[test]
    fn test_try_parse_csv_reports_errors() {
        let err = try_parse_csv("name,value\nalice,\"broken").unwrap_err();
        assert_eq!(err.line, 2);
    }

    #[test]
    fn test_lossy_parse_csv_swallow_errors_explicitly() {
        assert!(parse_csv_lossy("name,value\nalice,\"broken").is_empty());
    }
}
