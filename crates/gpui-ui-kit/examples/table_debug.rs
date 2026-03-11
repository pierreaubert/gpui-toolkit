use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::table::{Column, PaginationState, SelectionMode, SortDirection, SortState, Table};
use gpui_ui_kit::{HStack, MiniApp, MiniAppConfig, StackSpacing, Text, ThemeExt, VStack};
use std::collections::HashSet;

#[derive(Clone, Debug)]
struct User {
    id: usize,
    name: String,
    email: String,
    role: String,
    status: String,
}

struct TableDebug {
    users: Vec<User>,
    selected_users: HashSet<usize>,
    sort_state: Option<SortState>,
    pagination: PaginationState,
}

impl TableDebug {
    fn new(_cx: &mut Context<Self>) -> Self {
        let users = vec![
            User {
                id: 1,
                name: "Alice Smith".to_string(),
                email: "alice@example.com".to_string(),
                role: "Admin".to_string(),
                status: "Active".to_string(),
            },
            User {
                id: 2,
                name: "Bob Jones".to_string(),
                email: "bob@example.com".to_string(),
                role: "User".to_string(),
                status: "Inactive".to_string(),
            },
            User {
                id: 3,
                name: "Charlie Brown".to_string(),
                email: "charlie@example.com".to_string(),
                role: "Editor".to_string(),
                status: "Active".to_string(),
            },
            User {
                id: 4,
                name: "David Wilson".to_string(),
                email: "david@example.com".to_string(),
                role: "User".to_string(),
                status: "Pending".to_string(),
            },
            User {
                id: 5,
                name: "Eve Adams".to_string(),
                email: "eve@example.com".to_string(),
                role: "Admin".to_string(),
                status: "Active".to_string(),
            },
        ];

        Self {
            users: users.clone(),
            selected_users: HashSet::new(),
            sort_state: Some(SortState {
                column_id: "name".into(),
                direction: SortDirection::Ascending,
            }),
            pagination: PaginationState {
                current_page: 0,
                page_size: 10,
                total_items: users.len(),
            },
        }
    }
}

impl Render for TableDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let table = Table::new("user-table", self.users.clone())
            .column(
                Column::new("id", "ID")
                    .width(px(60.0))
                    .cell_render(|user: &User, _, _, _| user.id.to_string()),
            )
            .column(
                Column::new("name", "Name").cell_render(|user: &User, _, _, _| user.name.clone()),
            )
            .column(
                Column::new("email", "Email")
                    .cell_render(|user: &User, _, _, _| user.email.clone()),
            )
            .column(
                Column::new("role", "Role")
                    .width(px(100.0))
                    .cell_render(|user: &User, _, _, _| user.role.clone()),
            )
            .column(
                Column::new("status", "Status")
                    .width(px(100.0))
                    .cell_render(|user: &User, _, _, _| {
                        let color = match user.status.as_str() {
                            "Active" => rgb(0x22c55e),
                            "Inactive" => rgb(0xef4444),
                            _ => rgb(0xf59e0b),
                        };
                        div().text_color(color).child(user.status.clone())
                    }),
            )
            .sort(self.sort_state.clone().unwrap())
            .on_sort(cx.listener(|this, state: &SortState, _window, cx| {
                this.sort_state = Some(state.clone());
                // In a real app, you would sort the data here
                this.users.sort_by(|a, b| {
                    let cmp = match state.column_id.as_ref() {
                        "id" => a.id.cmp(&b.id),
                        "name" => a.name.cmp(&b.name),
                        "email" => a.email.cmp(&b.email),
                        "role" => a.role.cmp(&b.role),
                        "status" => a.status.cmp(&b.status),
                        _ => std::cmp::Ordering::Equal,
                    };
                    if state.direction == SortDirection::Ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });
                cx.notify();
            }))
            .selection_mode(SelectionMode::Multiple)
            .selected_indices(self.selected_users.clone())
            .on_selection_change(cx.listener(|this, indices: &HashSet<usize>, _window, cx| {
                this.selected_users = indices.clone();
                cx.notify();
            }))
            .pagination(self.pagination.clone())
            .on_page_change(cx.listener(|this, page: &usize, _window, cx| {
                this.pagination.current_page = *page;
                cx.notify();
            }))
            .show_footer(true);

        div()
            .size_full()
            .bg(theme.background)
            .child(
                VStack::new()
                    .full()
                    .spacing(StackSpacing::Lg)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Xs)
                            .child(Text::new("Table Component").size(gpui_ui_kit::TextSize::Xl))
                            .child(Text::new("A powerful table component with sorting, selection, and pagination.").size(gpui_ui_kit::TextSize::Sm).color(theme.text_secondary)),
                    )
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child(Text::new(format!("Selected: {} users", self.selected_users.len())).size(gpui_ui_kit::TextSize::Xs)),
                    )
                    .child(div().flex_1().border_1().border_color(theme.border).rounded_md().overflow_hidden().child(table))
            )
    }
}

fn main() {
    MiniApp::run(MiniAppConfig::new("Table Debug"), |cx| {
        cx.new(|cx| TableDebug::new(cx))
    });
}
