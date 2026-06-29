use super::dsv_parser::DsvParser;
use super::types::DsvResult;
use super::types::DsvRow;

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

/// Parse a DSV string and return an empty vector if parsing fails.
pub fn parse_dsv_lossy(text: &str, delimiter: char) -> Vec<DsvRow> {
    DsvParser::new(delimiter).parse_lossy(text)
}
