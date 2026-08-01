use super::dsv_parse_error_kind::{DsvBudgetResource, DsvParseErrorKind};
use super::error::DsvParseError;
use super::parse::parse_dsv;
use std::collections::HashMap;

/// A row from a DSV file, stored as a HashMap of column name to value.
pub type DsvRow = HashMap<String, String>;

/// Result type for DSV parser operations.
pub type DsvResult<T> = Result<T, DsvParseError>;

/// Upper bounds for one DSV parse operation.
///
/// The limits protect both parser work and the row/cell allocations produced
/// by the high-level APIs. Use [`DsvBudget::unlimited`] only for trusted input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DsvBudget {
    pub max_input_bytes: usize,
    pub max_records: usize,
    pub max_columns: usize,
    pub max_field_bytes: usize,
    pub max_cells: usize,
}

impl DsvBudget {
    pub const fn new(
        max_input_bytes: usize,
        max_records: usize,
        max_columns: usize,
        max_field_bytes: usize,
        max_cells: usize,
    ) -> Self {
        Self {
            max_input_bytes,
            max_records,
            max_columns,
            max_field_bytes,
            max_cells,
        }
    }

    pub const fn unlimited() -> Self {
        Self::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX, usize::MAX)
    }

    pub(super) fn exceeded(
        self,
        resource: DsvBudgetResource,
        actual: usize,
        line: usize,
        column: usize,
        byte_offset: usize,
    ) -> Option<DsvParseError> {
        let limit = match resource {
            DsvBudgetResource::InputBytes => self.max_input_bytes,
            DsvBudgetResource::Records => self.max_records,
            DsvBudgetResource::Columns => self.max_columns,
            DsvBudgetResource::FieldBytes => self.max_field_bytes,
            DsvBudgetResource::Cells => self.max_cells,
        };
        (actual > limit).then(|| {
            DsvParseError::new(
                line,
                column,
                byte_offset,
                DsvParseErrorKind::BudgetExceeded {
                    resource,
                    limit,
                    actual,
                },
            )
        })
    }
}

impl Default for DsvBudget {
    fn default() -> Self {
        // Large enough for ordinary chart data while preventing accidental
        // unbounded allocations when parsing fetched or user-provided text.
        Self::new(
            16 * 1024 * 1024,
            1_000_000,
            1_024,
            4 * 1024 * 1024,
            10_000_000,
        )
    }
}

/// Policy for rows whose field count differs from the header count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnPolicy {
    /// Match D3's convenient behavior: missing cells become empty strings and
    /// extra cells are ignored by header-based row parsing.
    D3Compatible,
    /// Reject rows whose field count differs from the header count. Also
    /// rejects empty and duplicate headers because they cannot round-trip
    /// cleanly through `DsvRow`.
    Strict,
}

#[derive(Debug, Clone)]
pub(super) struct ParsedRecord {
    pub(super) line: usize,
    pub(super) byte_offset: usize,
    pub(super) fields: Vec<String>,
}

/// Compatibility alias for callers that still use the older fallible name.
pub fn try_parse_dsv(text: &str, delimiter: char) -> DsvResult<Vec<DsvRow>> {
    parse_dsv(text, delimiter)
}
