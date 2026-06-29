use super::prelude::*;

impl Showcase {
    pub(crate) fn render_table_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header("Table"))
            .child(
                Text::new("A powerful table component with sorting, selection, and pagination.")
                    .color(theme.text_secondary),
            )
            .child(
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .child(Text::new("Standard Table").size(TextSize::Lg))
                    .child(
                        div()
                            .h(px(300.0))
                            .border_1()
                            .border_color(theme.border)
                            .rounded_md()
                            .overflow_hidden()
                            .child(
                                Table::new("showcase-table", self.users.clone())
                                    .column(
                                        Column::new("id", "ID").width(px(60.0)).cell_render(
                                            |user: &User, _, _, _| user.id.to_string(),
                                        ),
                                    )
                                    .column(
                                        Column::new("name", "Name")
                                            .cell_render(|user: &User, _, _, _| user.name.clone()),
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
                                    .sort(self.sort_state.clone().unwrap_or(SortState {
                                        column_id: "name".into(),
                                        direction: SortDirection::Ascending,
                                    }))
                                    .on_sort(cx.listener(|this, state: &SortState, _window, cx| {
                                        this.sort_state = Some(state.clone());
                                        this.users.sort_by(|a, b| {
                                            let cmp = match state.column_id.as_ref() {
                                                "id" => a.id.cmp(&b.id),
                                                "name" => a.name.cmp(&b.name),
                                                "email" => a.email.cmp(&b.email),
                                                "role" => a.role.cmp(&b.role),
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
                                    .on_selection_change(cx.listener(
                                        |this, indices: &HashSet<usize>, _window, cx| {
                                            this.selected_users = indices.clone();
                                            cx.notify();
                                        },
                                    ))
                                    .pagination(self.pagination.clone())
                                    .on_page_change(cx.listener(
                                        |this, page: &usize, _window, cx| {
                                            this.pagination.current_page = *page;
                                            cx.notify();
                                        },
                                    ))
                                    .show_footer(true),
                            ),
                    )
                    .child(
                        HStack::new().spacing(StackSpacing::Md).child(
                            Text::new(format!("Selected: {} items", self.selected_users.len()))
                                .size(TextSize::Sm),
                        ),
                    ),
            )
    }
}
