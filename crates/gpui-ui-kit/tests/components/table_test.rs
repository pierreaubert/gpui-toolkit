//! Table component tests

use gpui_ui_kit::table::{Column, SelectionMode, Table};

#[derive(Clone)]
struct TestItem {
    id: usize,
    name: String,
}

#[test]
fn test_table_creation() {
    let rows = vec![
        TestItem {
            id: 1,
            name: "Item 1".to_string(),
        },
        TestItem {
            id: 2,
            name: "Item 2".to_string(),
        },
    ];

    let table = Table::new("table", rows)
        .column(Column::new("id", "ID").cell_render(|item: &TestItem, _, _, _| item.id.to_string()))
        .column(
            Column::new("name", "Name").cell_render(|item: &TestItem, _, _, _| item.name.clone()),
        );

    drop(table);
}

#[test]
fn test_table_with_sorting() {
    let rows = vec![TestItem {
        id: 1,
        name: "Item 1".to_string(),
    }];

    let table = Table::new("table", rows)
        .column(Column::new("id", "ID").cell_render(|item: &TestItem, _, _, _| item.id.to_string()))
        .on_sort(|_state, _window, _cx| {});

    drop(table);
}

#[test]
fn test_table_with_selection() {
    let rows = vec![TestItem {
        id: 1,
        name: "Item 1".to_string(),
    }];

    let table = Table::new("table", rows)
        .column(Column::new("id", "ID").cell_render(|item: &TestItem, _, _, _| item.id.to_string()))
        .selection_mode(SelectionMode::Single)
        .on_selection_change(|_indices, _window, _cx| {});

    drop(table);
}

#[test]
fn test_table_with_pagination() {
    let rows = vec![TestItem {
        id: 1,
        name: "Item 1".to_string(),
    }];

    let table = Table::new("table", rows)
        .column(Column::new("id", "ID").cell_render(|item: &TestItem, _, _, _| item.id.to_string()))
        .on_page_change(|_page, _window, _cx| {});

    drop(table);
}
