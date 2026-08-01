use super::dsv_parse_error_kind::{DsvBudgetResource, DsvParseErrorKind};
use super::error::DsvParseError;
use super::types::DsvResult;
use super::types::DsvRow;
use super::types::ParsedRecord;
use super::types::{ColumnPolicy, DsvBudget};
use std::collections::HashSet;

/// A DSV parser that can be configured with any delimiter.
///
/// # Example
///
/// ```
/// use d3rs::fetch::DsvParser;
///
/// let parser = DsvParser::new(',');
/// let data = "name,age\nalice,30\nbob,25";
/// let rows = parser.parse(data).unwrap();
/// assert_eq!(rows.len(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct DsvParser {
    pub(super) delimiter: char,
    pub(super) skip_empty_lines: bool,
    pub(super) trim_values: bool,
    pub(super) column_policy: ColumnPolicy,
    pub(super) budget: DsvBudget,
}

impl DsvParser {
    /// Create a new parser with the given delimiter.
    pub fn new(delimiter: char) -> Self {
        Self {
            delimiter,
            skip_empty_lines: true,
            trim_values: true,
            column_policy: ColumnPolicy::D3Compatible,
            budget: DsvBudget::default(),
        }
    }

    /// Set whether to skip empty lines.
    pub fn skip_empty_lines(mut self, skip: bool) -> Self {
        self.skip_empty_lines = skip;
        self
    }

    /// Set whether to trim whitespace from values.
    pub fn trim_values(mut self, trim: bool) -> Self {
        self.trim_values = trim;
        self
    }

    /// Set row/header width validation policy.
    pub fn column_policy(mut self, policy: ColumnPolicy) -> Self {
        self.column_policy = policy;
        self
    }

    /// Set resource limits for parses performed by this parser.
    pub fn budget(mut self, budget: DsvBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Parse a DSV string into rows.
    ///
    /// The first record is treated as the header row.
    pub fn parse(&self, text: &str) -> DsvResult<Vec<DsvRow>> {
        self.parse_with_budget_and_cancel(text, self.budget, || false)
    }

    /// Parse with explicit resource limits.
    pub fn parse_with_budget(&self, text: &str, budget: DsvBudget) -> DsvResult<Vec<DsvRow>> {
        self.parse_with_budget_and_cancel(text, budget, || false)
    }

    /// Parse with resource limits and a cooperative cancellation callback.
    ///
    /// The callback is checked while scanning the input, so callers can abort
    /// work on a background task without waiting for a large field or file to
    /// finish parsing.
    pub fn parse_with_budget_and_cancel<F: FnMut() -> bool>(
        &self,
        text: &str,
        budget: DsvBudget,
        mut cancelled: F,
    ) -> DsvResult<Vec<DsvRow>> {
        let mut records = self.parse_records_with_budget(text, budget, &mut cancelled)?;
        if records.is_empty() {
            return Ok(Vec::new());
        }

        let header = records.remove(0);
        self.validate_headers(&header.fields, header.line, header.byte_offset)?;

        let cell_count = header.fields.len().saturating_mul(records.len());
        if let Some(error) = budget.exceeded(DsvBudgetResource::Cells, cell_count, 1, 1, 0) {
            return Err(error);
        }

        let mut rows = Vec::with_capacity(records.len());
        for record in records {
            if self.column_policy == ColumnPolicy::Strict
                && record.fields.len() != header.fields.len()
            {
                return Err(DsvParseError::new(
                    record.line,
                    1,
                    record.byte_offset,
                    DsvParseErrorKind::HeaderColumnMismatch {
                        expected: header.fields.len(),
                        actual: record.fields.len(),
                    },
                ));
            }

            let mut row = DsvRow::new();
            for (i, name) in header.fields.iter().enumerate() {
                row.insert(
                    name.clone(),
                    record.fields.get(i).cloned().unwrap_or_default(),
                );
            }
            rows.push(row);
        }
        Ok(rows)
    }

    /// Parse a DSV string into rows and return an empty vector if parsing fails.
    ///
    /// Prefer [`Self::parse`] for data ingestion. This helper exists for
    /// D3-compatible demos where malformed static input should render as no data.
    pub fn parse_lossy(&self, text: &str) -> Vec<DsvRow> {
        self.parse(text).unwrap_or_default()
    }

    /// Compatibility alias for callers that still use the older fallible name.
    pub fn try_parse(&self, text: &str) -> DsvResult<Vec<DsvRow>> {
        self.parse(text)
    }

    /// Parse a DSV string without headers (returns arrays of strings).
    pub fn parse_rows(&self, text: &str) -> DsvResult<Vec<Vec<String>>> {
        let mut cancelled = || false;
        let records = self.parse_records_with_budget(text, self.budget, &mut cancelled)?;
        let cells = records.iter().try_fold(0usize, |total, record| {
            total.checked_add(record.fields.len())
        });
        let cells = cells.unwrap_or(usize::MAX);
        if let Some(error) = self
            .budget
            .exceeded(DsvBudgetResource::Cells, cells, 1, 1, 0)
        {
            return Err(error);
        }
        Ok(records.into_iter().map(|record| record.fields).collect())
    }

    /// Parse rows without headers and return an empty vector if parsing fails.
    pub fn parse_rows_lossy(&self, text: &str) -> Vec<Vec<String>> {
        self.parse_rows(text).unwrap_or_default()
    }

    /// Compatibility alias for callers that still use the older fallible name.
    pub fn try_parse_rows(&self, text: &str) -> DsvResult<Vec<Vec<String>>> {
        self.parse_rows(text)
    }

    pub(super) fn parse_records_with_budget<F: FnMut() -> bool>(
        &self,
        text: &str,
        budget: DsvBudget,
        cancelled: &mut F,
    ) -> DsvResult<Vec<ParsedRecord>> {
        self.validate_delimiter()?;

        if let Some(error) = budget.exceeded(DsvBudgetResource::InputBytes, text.len(), 1, 1, 0) {
            return Err(error);
        }

        if text.is_empty() {
            return Ok(Vec::new());
        }

        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let mut records = Vec::new();
        let mut record = Vec::new();
        let mut field = String::new();
        let mut in_quotes = false;
        let mut at_field_start = true;
        let mut after_quote = false;
        let mut last_was_record_terminator = false;
        let mut record_has_content = false;
        let mut line = 1usize;
        let mut column = 1usize;
        let mut record_start_line = 1usize;
        let mut record_start_byte = 0usize;
        let mut i = 0usize;

        while i < chars.len() {
            if cancelled() {
                return Err(DsvParseError::new(
                    line,
                    column,
                    chars[i].0,
                    DsvParseErrorKind::Cancelled,
                ));
            }
            if let Some(error) = budget.exceeded(
                DsvBudgetResource::FieldBytes,
                field.len(),
                line,
                column,
                chars[i].0,
            ) {
                return Err(error);
            }
            if let Some(error) = budget.exceeded(
                DsvBudgetResource::Columns,
                record.len() + 1,
                line,
                column,
                chars[i].0,
            ) {
                return Err(error);
            }
            let (byte_offset, ch) = chars[i];
            last_was_record_terminator = false;

            if in_quotes {
                if ch == '"' {
                    if chars.get(i + 1).map(|(_, c)| *c) == Some('"') {
                        field.push('"');
                        i += 2;
                        column += 2;
                    } else {
                        in_quotes = false;
                        after_quote = true;
                        i += 1;
                        column += 1;
                    }
                    continue;
                }

                if ch == '\r' || ch == '\n' {
                    field.push('\n');
                    if ch == '\r' && chars.get(i + 1).map(|(_, c)| *c) == Some('\n') {
                        i += 2;
                    } else {
                        i += 1;
                    }
                    line += 1;
                    column = 1;
                    continue;
                }

                field.push(ch);
                i += 1;
                column += 1;
                continue;
            }

            if after_quote {
                if ch == self.delimiter {
                    self.finish_field(&mut record, &mut field);
                    after_quote = false;
                    at_field_start = true;
                    i += 1;
                    column += 1;
                    continue;
                }

                if ch == '\r' || ch == '\n' {
                    self.finish_field(&mut record, &mut field);
                    self.finish_record(
                        &mut records,
                        &mut record,
                        record_start_line,
                        record_start_byte,
                        record_has_content,
                    );
                    if let Some(error) = budget.exceeded(
                        DsvBudgetResource::Records,
                        records.len(),
                        record_start_line,
                        1,
                        record_start_byte,
                    ) {
                        return Err(error);
                    }
                    after_quote = false;
                    at_field_start = true;
                    if ch == '\r' && chars.get(i + 1).map(|(_, c)| *c) == Some('\n') {
                        i += 2;
                    } else {
                        i += 1;
                    }
                    line += 1;
                    column = 1;
                    record_start_line = line;
                    record_start_byte = chars.get(i).map(|(idx, _)| *idx).unwrap_or(text.len());
                    record_has_content = false;
                    last_was_record_terminator = true;
                    continue;
                }

                if self.trim_values && ch.is_whitespace() {
                    i += 1;
                    column += 1;
                    continue;
                }

                return Err(DsvParseError::new(
                    line,
                    column,
                    byte_offset,
                    DsvParseErrorKind::UnexpectedQuote,
                ));
            }

            if ch == '"' {
                if at_field_start || (self.trim_values && field.trim().is_empty()) {
                    if self.trim_values {
                        field.clear();
                    }
                    record_has_content = true;
                    in_quotes = true;
                    at_field_start = false;
                    i += 1;
                    column += 1;
                    continue;
                }

                return Err(DsvParseError::new(
                    line,
                    column,
                    byte_offset,
                    DsvParseErrorKind::UnexpectedQuote,
                ));
            }

            if ch == self.delimiter {
                record_has_content = true;
                self.finish_field(&mut record, &mut field);
                at_field_start = true;
                i += 1;
                column += 1;
                continue;
            }

            if ch == '\r' || ch == '\n' {
                self.finish_field(&mut record, &mut field);
                self.finish_record(
                    &mut records,
                    &mut record,
                    record_start_line,
                    record_start_byte,
                    record_has_content,
                );
                if let Some(error) = budget.exceeded(
                    DsvBudgetResource::Records,
                    records.len(),
                    record_start_line,
                    1,
                    record_start_byte,
                ) {
                    return Err(error);
                }
                at_field_start = true;
                if ch == '\r' && chars.get(i + 1).map(|(_, c)| *c) == Some('\n') {
                    i += 2;
                } else {
                    i += 1;
                }
                line += 1;
                column = 1;
                record_start_line = line;
                record_start_byte = chars.get(i).map(|(idx, _)| *idx).unwrap_or(text.len());
                record_has_content = false;
                last_was_record_terminator = true;
                continue;
            }

            if !ch.is_whitespace() {
                record_has_content = true;
            }
            field.push(ch);
            at_field_start = false;
            i += 1;
            column += 1;
        }

        if in_quotes {
            return Err(DsvParseError::new(
                line,
                column,
                text.len(),
                DsvParseErrorKind::UnterminatedQuotedField,
            ));
        }

        if let Some(error) = budget.exceeded(
            DsvBudgetResource::FieldBytes,
            field.len(),
            line,
            column,
            text.len(),
        ) {
            return Err(error);
        }

        if cancelled() {
            return Err(DsvParseError::new(
                line,
                column,
                text.len(),
                DsvParseErrorKind::Cancelled,
            ));
        }

        if !last_was_record_terminator || !record.is_empty() || !field.is_empty() {
            self.finish_field(&mut record, &mut field);
            self.finish_record(
                &mut records,
                &mut record,
                record_start_line,
                record_start_byte,
                record_has_content,
            );
            if let Some(error) = budget.exceeded(
                DsvBudgetResource::Records,
                records.len(),
                record_start_line,
                1,
                record_start_byte,
            ) {
                return Err(error);
            }
        }

        Ok(records)
    }

    pub(super) fn validate_delimiter(&self) -> DsvResult<()> {
        if matches!(self.delimiter, '"' | '\r' | '\n') {
            Err(DsvParseError::new(
                1,
                1,
                0,
                DsvParseErrorKind::InvalidDelimiter,
            ))
        } else {
            Ok(())
        }
    }

    pub(super) fn finish_field(&self, record: &mut Vec<String>, field: &mut String) {
        let value = std::mem::take(field);
        if self.trim_values {
            record.push(value.trim().to_string());
        } else {
            record.push(value);
        }
    }

    pub(super) fn finish_record(
        &self,
        records: &mut Vec<ParsedRecord>,
        record: &mut Vec<String>,
        line: usize,
        byte_offset: usize,
        record_has_content: bool,
    ) {
        if self.skip_empty_lines && !record_has_content {
            record.clear();
            return;
        }

        records.push(ParsedRecord {
            line,
            byte_offset,
            fields: std::mem::take(record),
        });
    }

    pub(super) fn validate_headers(
        &self,
        headers: &[String],
        line: usize,
        byte_offset: usize,
    ) -> DsvResult<()> {
        if self.column_policy != ColumnPolicy::Strict {
            return Ok(());
        }

        let mut seen = HashSet::new();
        for (index, header) in headers.iter().enumerate() {
            if header.is_empty() {
                return Err(DsvParseError::new(
                    line,
                    index + 1,
                    byte_offset,
                    DsvParseErrorKind::EmptyHeader { index },
                ));
            }
            if !seen.insert(header) {
                return Err(DsvParseError::new(
                    line,
                    index + 1,
                    byte_offset,
                    DsvParseErrorKind::DuplicateHeader {
                        name: header.clone(),
                    },
                ));
            }
        }

        Ok(())
    }

    /// Format rows as DSV text.
    pub fn format(&self, rows: &[DsvRow], columns: &[&str]) -> String {
        let mut result = String::new();

        // Header
        result.push_str(&columns.join(&self.delimiter.to_string()));
        result.push('\n');

        // Data rows
        for row in rows {
            let values: Vec<String> = columns
                .iter()
                .map(|col| {
                    let value = row.get(*col).cloned().unwrap_or_default();
                    self.format_value(&value)
                })
                .collect();
            result.push_str(&values.join(&self.delimiter.to_string()));
            result.push('\n');
        }

        result
    }

    /// Format rows from arrays as DSV text.
    pub fn format_rows(&self, rows: &[Vec<String>]) -> String {
        rows.iter()
            .map(|row| {
                row.iter()
                    .map(|v| self.format_value(v))
                    .collect::<Vec<_>>()
                    .join(&self.delimiter.to_string())
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Format a value, quoting if necessary.
    pub(super) fn format_value(&self, value: &str) -> String {
        if value.contains(self.delimiter) || value.contains('"') || value.contains('\n') {
            format!("\"{}\"", value.replace('"', "\"\""))
        } else {
            value.to_string()
        }
    }
}
