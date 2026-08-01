use super::dsv_parser::DsvParser;
use super::types::DsvRow;
use super::types::{DsvBudget, DsvResult};

/// Parse a DSV string with the given delimiter.
///
/// # Example
///
/// ```
/// use d3rs::fetch::parse_dsv;
///
/// let data = "name|age\nalice|30\nbob|25";
/// let rows = parse_dsv(data, '|').unwrap();
/// assert_eq!(rows.len(), 2);
/// assert_eq!(rows[0].get("age"), Some(&"30".to_string()));
/// ```
pub fn parse_dsv(text: &str, delimiter: char) -> DsvResult<Vec<DsvRow>> {
    DsvParser::new(delimiter).parse(text)
}

/// Parse DSV with explicit size, allocation, and record limits.
pub fn parse_dsv_with_budget(
    text: &str,
    delimiter: char,
    budget: DsvBudget,
) -> DsvResult<Vec<DsvRow>> {
    DsvParser::new(delimiter).parse_with_budget(text, budget)
}

/// Parse a DSV string and return an empty vector if parsing fails.
pub fn parse_dsv_lossy(text: &str, delimiter: char) -> Vec<DsvRow> {
    DsvParser::new(delimiter).parse_lossy(text)
}
