//! CSV and TSV parsing utilities
//!
//! High-level functions for parsing comma-separated and tab-separated values.

use super::dsv::{DsvParser, DsvRow};

/// Options for CSV/TSV parsing.
#[derive(Debug, Clone, Default)]
pub struct CsvOptions {
    /// Whether to skip empty lines (default: true)
    pub skip_empty_lines: bool,
    /// Whether to trim whitespace from values (default: true)
    pub trim_values: bool,
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

/// Parse a CSV string into rows.
///
/// # Example
///
/// ```
/// use d3rs::fetch::parse_csv;
///
/// let data = "name,value\nalice,10\nbob,20";
/// let rows = parse_csv(data);
/// assert_eq!(rows.len(), 2);
/// assert_eq!(rows[0].get("name"), Some(&"alice".to_string()));
/// assert_eq!(rows[0].get("value"), Some(&"10".to_string()));
/// ```
pub fn parse_csv(text: &str) -> Vec<DsvRow> {
    DsvParser::new(',').parse(text)
}

/// Parse a CSV string with options.
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
/// let rows = parse_csv_with_options(data, &options);
/// assert_eq!(rows[0].get("name"), Some(&"alice".to_string()));
/// ```
pub fn parse_csv_with_options(text: &str, options: &CsvOptions) -> Vec<DsvRow> {
    DsvParser::new(',')
        .skip_empty_lines(options.skip_empty_lines)
        .trim_values(options.trim_values)
        .parse(text)
}

/// Parse a TSV (tab-separated) string into rows.
///
/// # Example
///
/// ```
/// use d3rs::fetch::parse_tsv;
///
/// let data = "name\tvalue\nalice\t10\nbob\t20";
/// let rows = parse_tsv(data);
/// assert_eq!(rows.len(), 2);
/// assert_eq!(rows[0].get("name"), Some(&"alice".to_string()));
/// ```
pub fn parse_tsv(text: &str) -> Vec<DsvRow> {
    DsvParser::new('\t').parse(text)
}

/// Parse a TSV string with options.
pub fn parse_tsv_with_options(text: &str, options: &CsvOptions) -> Vec<DsvRow> {
    DsvParser::new('\t')
        .skip_empty_lines(options.skip_empty_lines)
        .trim_values(options.trim_values)
        .parse(text)
}

/// Format rows as CSV text.
///
/// # Example
///
/// ```
/// use d3rs::fetch::{parse_csv, format_csv};
///
/// let data = "name,value\nalice,10\nbob,20";
/// let rows = parse_csv(data);
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
        let rows = parse_csv(data);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("a"), Some(&"1".to_string()));
    }

    #[test]
    fn test_parse_tsv() {
        let data = "a\tb\tc\n1\t2\t3";
        let rows = parse_tsv(data);
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
}
