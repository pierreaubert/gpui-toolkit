use super::dsv_parse_error_kind::DsvParseErrorKind;
use super::dsv_parser::DsvParser;
use super::parse::parse_dsv;
use super::types::DsvRow;
use super::types::{ColumnPolicy, DsvBudget};

#[test]
fn test_parse_simple() {
    let data = "name,value\nalice,10\nbob,20";
    let rows = parse_dsv(data, ',').unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get("name"), Some(&"alice".to_string()));
    assert_eq!(rows[0].get("value"), Some(&"10".to_string()));
}

#[test]
fn test_parse_quoted() {
    let data = "name,message\nalice,\"hello, world\"\nbob,\"say \"\"hi\"\"\"";
    let rows = parse_dsv(data, ',').unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get("message"), Some(&"hello, world".to_string()));
    assert_eq!(rows[1].get("message"), Some(&"say \"hi\"".to_string()));
}

#[test]
fn test_parse_quoted_newline() {
    let data = "name,message\nalice,\"hello\nworld\"\nbob,done";
    let rows = parse_dsv(data, ',').unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get("message"), Some(&"hello\nworld".to_string()));
    assert_eq!(rows[1].get("message"), Some(&"done".to_string()));
}

#[test]
fn test_parse_crlf() {
    let data = "name,value\r\nalice,10\r\nbob,20\r\n";
    let rows = parse_dsv(data, ',').unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].get("name"), Some(&"bob".to_string()));
}

#[test]
fn test_parse_empty_values() {
    let data = "a,b,c\n1,,3\n,2,";
    let rows = parse_dsv(data, ',').unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get("b"), Some(&String::new()));
    assert_eq!(rows[1].get("a"), Some(&String::new()));
}

#[test]
fn test_parse_rows() {
    let parser = DsvParser::new(',');
    let data = "1,2,3\n4,5,6";
    let rows = parser.parse_rows(data).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], vec!["1", "2", "3"]);
}

#[test]
fn test_empty_field_records_are_not_empty_lines() {
    let parser = DsvParser::new(',');
    let rows = parser.parse_rows(",\n\"\"\n   ").unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], vec!["", ""]);
    assert_eq!(rows[1], vec![""]);
}

#[test]
fn test_try_parse_reports_unclosed_quote() {
    let parser = DsvParser::new(',');
    let err = parser
        .parse("name,value\nalice,\"broken")
        .expect_err("unterminated quote should be an error");

    assert_eq!(err.line, 2);
    assert!(err.message.contains("unterminated"));
    assert_eq!(err.kind, DsvParseErrorKind::UnterminatedQuotedField);
    assert!(parser.parse_lossy("name,value\nalice,\"broken").is_empty());
}

#[test]
fn test_unexpected_quote_reports_location() {
    let parser = DsvParser::new(',');
    let err = parser.parse("name,value\nalice,br\"oken").unwrap_err();

    assert_eq!(err.line, 2);
    assert_eq!(err.column, 9);
    assert_eq!(err.kind, DsvParseErrorKind::UnexpectedQuote);
}

#[test]
fn test_strict_column_mismatch() {
    let parser = DsvParser::new(',').column_policy(ColumnPolicy::Strict);
    let err = parser.parse("a,b\n1,2,3").unwrap_err();

    assert_eq!(
        err.kind,
        DsvParseErrorKind::HeaderColumnMismatch {
            expected: 2,
            actual: 3
        }
    );
}

#[test]
fn test_strict_empty_and_duplicate_headers() {
    let empty = DsvParser::new(',')
        .column_policy(ColumnPolicy::Strict)
        .parse("a,,c\n1,2,3")
        .unwrap_err();
    assert_eq!(empty.kind, DsvParseErrorKind::EmptyHeader { index: 1 });

    let duplicate = DsvParser::new(',')
        .column_policy(ColumnPolicy::Strict)
        .parse("a,b,a\n1,2,3")
        .unwrap_err();
    assert_eq!(
        duplicate.kind,
        DsvParseErrorKind::DuplicateHeader {
            name: "a".to_string()
        }
    );
}

#[test]
fn test_invalid_delimiter() {
    let err = DsvParser::new('\n').parse("a,b\n1,2").unwrap_err();
    assert_eq!(err.kind, DsvParseErrorKind::InvalidDelimiter);
}

#[test]
fn test_format() {
    let parser = DsvParser::new(',');
    let mut row1 = DsvRow::new();
    row1.insert("name".to_string(), "alice".to_string());
    row1.insert("value".to_string(), "10".to_string());

    let mut row2 = DsvRow::new();
    row2.insert("name".to_string(), "bob".to_string());
    row2.insert("value".to_string(), "20".to_string());

    let result = parser.format(&[row1, row2], &["name", "value"]);
    assert!(result.contains("alice,10"));
    assert!(result.contains("bob,20"));
}

#[test]
fn test_format_quoted() {
    let parser = DsvParser::new(',');
    let mut row = DsvRow::new();
    row.insert("text".to_string(), "hello, \"world\"".to_string());

    let result = parser.format(&[row], &["text"]);
    assert!(result.contains("\"hello, \"\"world\"\"\""));
}

#[test]
fn test_try_parse_dsv_alias() {
    let rows = super::types::try_parse_dsv("a,b\n1,2", ',').unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("a"), Some(&"1".to_string()));
}

#[test]
fn test_column_policy_traits() {
    let policy = ColumnPolicy::D3Compatible;
    assert_eq!(format!("{policy:?}"), "D3Compatible");
    assert_eq!(policy, ColumnPolicy::D3Compatible);
    assert_ne!(policy, ColumnPolicy::Strict);
    let cloned = policy;
    assert_eq!(policy, cloned);
}

#[test]
fn test_budget_rejects_large_input_before_scanning() {
    let parser = DsvParser::new(',');
    let err = parser
        .parse_with_budget("a,b\n1,2", DsvBudget::new(3, 10, 10, 10, 100))
        .unwrap_err();

    assert!(matches!(err.kind, DsvParseErrorKind::BudgetExceeded { .. }));
}

#[test]
fn test_budget_cancellation_is_cooperative() {
    let parser = DsvParser::new(',');
    let mut checks = 0;
    let err = parser
        .parse_with_budget_and_cancel("a,b\n1,2\n3,4", DsvBudget::unlimited(), || {
            checks += 1;
            checks > 1
        })
        .unwrap_err();

    assert_eq!(err.kind, DsvParseErrorKind::Cancelled);
    assert!(checks >= 2);
}

#[test]
fn test_budget_limits_output_cells() {
    let parser = DsvParser::new(',');
    let err = parser
        .parse_with_budget("a,b\n1,2\n3,4", DsvBudget::new(100, 10, 10, 10, 2))
        .unwrap_err();

    assert!(matches!(err.kind, DsvParseErrorKind::BudgetExceeded { .. }));
}
