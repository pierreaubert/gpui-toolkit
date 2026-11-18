//! Retained row data for a committed Python table.

// Rust guideline compliant 2026-10-09

use gpui_python_runtime::ui_ir::{TableNode, TableRow};
use serde_json::Value;
use std::borrow::Cow;

enum Rows {
    Legacy {
        offset: usize,
        rows: Vec<Vec<String>>,
    },
    Typed(Vec<TableRow>),
}

pub(super) struct TableRows {
    rows: Rows,
    selected_row: Option<String>,
    selected_index: Option<usize>,
}

impl TableRows {
    pub(super) fn new(node: &TableNode, offset: usize, count: usize) -> Self {
        let rows = if node.typed_rows.is_empty() {
            Rows::Legacy {
                offset,
                rows: node.rows.iter().skip(offset).take(count).cloned().collect(),
            }
        } else {
            Rows::Typed(
                node.typed_rows
                    .iter()
                    .skip(offset)
                    .take(count)
                    .cloned()
                    .collect(),
            )
        };
        let mut result = Self {
            rows,
            selected_row: node.selected_row.clone(),
            selected_index: None,
        };
        result.selected_index = result.selected_row.as_deref().and_then(|selected| {
            (0..result.len()).position(|index| result.id(index).as_deref() == Some(selected))
        });
        result
    }

    pub(super) fn len(&self) -> usize {
        match &self.rows {
            Rows::Legacy { rows, .. } => rows.len(),
            Rows::Typed(rows) => rows.len(),
        }
    }

    pub(super) fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub(super) fn id(&self, index: usize) -> Option<Cow<'_, str>> {
        match &self.rows {
            Rows::Legacy { offset, rows } => rows
                .get(index)
                .map(|_| Cow::Owned(format!("row-{}", offset + index))),
            Rows::Typed(rows) => rows.get(index).map(|row| Cow::Borrowed(row.id.as_str())),
        }
    }

    pub(super) fn row(&self, index: usize) -> Option<(String, Vec<Value>, bool)> {
        match &self.rows {
            Rows::Legacy { offset, rows } => rows.get(index).map(|cells| {
                (
                    format!("row-{}", offset + index),
                    cells.iter().cloned().map(Value::String).collect(),
                    false,
                )
            }),
            Rows::Typed(rows) => rows.get(index).map(|row| {
                (
                    row.id.clone(),
                    row.cells.clone(),
                    self.selected_row.as_deref() == Some(row.id.as_str()),
                )
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TableRows;
    use gpui_python_runtime::ui_ir::TableNode;
    use serde_json::json;

    #[::core::prelude::v1::test]
    fn typed_rows_preserve_visible_values_and_selection() {
        let node: TableNode = serde_json::from_value(json!({
            "typed_rows": [
                {"id": "a", "cells": ["first"]},
                {"id": "b", "cells": [12, {"status": "ready"}]},
                {"id": "b", "cells": ["duplicate"]}
            ],
            "selected_row": "b"
        }))
        .unwrap();
        let rows = TableRows::new(&node, 1, 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows.selected_index(), Some(0));
        assert_eq!(
            rows.row(0),
            Some((
                "b".into(),
                vec![json!(12), json!({"status": "ready"})],
                true
            ))
        );
        assert_eq!(
            rows.row(1),
            Some(("b".into(), vec![json!("duplicate")], true))
        );
        assert_eq!(rows.id(2), None);
    }

    #[::core::prelude::v1::test]
    fn legacy_rows_keep_absolute_ids_and_keyboard_selection() {
        let node: TableNode = serde_json::from_value(json!({
            "rows": [["zero"], ["one"], ["two"]],
            "selected_row": "row-2"
        }))
        .unwrap();
        let rows = TableRows::new(&node, 1, 2);
        assert_eq!(rows.selected_index(), Some(1));
        assert_eq!(
            rows.row(0),
            Some(("row-1".into(), vec![json!("one")], false))
        );
        assert_eq!(rows.id(1).as_deref(), Some("row-2"));
        assert_eq!(TableRows::new(&node, 10, 2).len(), 0);
    }
}
