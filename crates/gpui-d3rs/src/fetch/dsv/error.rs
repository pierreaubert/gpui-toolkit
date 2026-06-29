use super::dsv_parse_error_kind::DsvParseErrorKind;

/// Recoverable DSV parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DsvParseError {
    pub line: usize,
    pub column: usize,
    pub byte_offset: usize,
    pub kind: DsvParseErrorKind,
    pub message: String,
}

impl DsvParseError {
    pub fn new(line: usize, column: usize, byte_offset: usize, kind: DsvParseErrorKind) -> Self {
        let message = kind.to_string();
        Self {
            line,
            column,
            byte_offset,
            kind,
            message,
        }
    }
}

impl std::fmt::Display for DsvParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for DsvParseError {}
