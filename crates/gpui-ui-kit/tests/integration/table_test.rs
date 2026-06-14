//! Integration tests for the Table component

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div};
use gpui_ui_kit::table::{Column, SelectionMode, Table};

struct TableTestView;

impl Render for TableTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            Table::new("test-table", vec!["Row 1", "Row 2", "Row 3"])
                .column(
                    Column::new("name", "Name")
                        .cell_render(|item: &&'static str, _idx, _window, _cx| *item),
                )
                .selection_mode(SelectionMode::Multiple),
        )
    }
}

#[gpui::test]
async fn test_table_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| TableTestView);
}
