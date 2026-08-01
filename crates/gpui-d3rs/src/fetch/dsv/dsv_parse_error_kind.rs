/// Structured DSV parse error kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DsvParseErrorKind {
    UnterminatedQuotedField,
    UnexpectedQuote,
    InvalidDelimiter,
    HeaderColumnMismatch {
        expected: usize,
        actual: usize,
    },
    EmptyHeader {
        index: usize,
    },
    DuplicateHeader {
        name: String,
    },
    BudgetExceeded {
        resource: DsvBudgetResource,
        limit: usize,
        actual: usize,
    },
    Cancelled,
}

/// Resource categories guarded by [`DsvBudget`](super::types::DsvBudget).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsvBudgetResource {
    InputBytes,
    Records,
    Columns,
    FieldBytes,
    Cells,
}

impl std::fmt::Display for DsvParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnterminatedQuotedField => write!(f, "unterminated quoted field"),
            Self::UnexpectedQuote => write!(f, "unexpected quote in unquoted field"),
            Self::InvalidDelimiter => {
                write!(f, "delimiter cannot be quote, carriage return, or newline")
            }
            Self::HeaderColumnMismatch { expected, actual } => {
                write!(
                    f,
                    "row has {actual} columns but header has {expected} columns"
                )
            }
            Self::EmptyHeader { index } => write!(f, "header at index {index} is empty"),
            Self::DuplicateHeader { name } => write!(f, "duplicate header {name:?}"),
            Self::BudgetExceeded {
                resource,
                limit,
                actual,
            } => write!(f, "DSV {resource:?} budget exceeded: {actual} > {limit}"),
            Self::Cancelled => write!(f, "DSV parsing cancelled"),
        }
    }
}
