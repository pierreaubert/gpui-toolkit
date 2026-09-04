//! Section builders extracted from `Table::build`: header, body, footer,
//! and pagination. Each helper takes explicit parameters so `build` stays a
//! thin composition over [`TablePaddings`].

use super::{
    Column, PaginationState, SelectionMode, SortDirection, SortState, TableTheme,
    virtual_spacer_height,
};
use crate::data_navigation::DataVirtualWindow;
use gpui::prelude::{InteractiveElement, ParentElement, StatefulInteractiveElement, Styled};
use gpui::{
    App, CursorStyle, Div, ElementId, FontWeight, MouseButton, Pixels, Stateful, Window, div,
};
use std::collections::HashSet;
use std::rc::Rc;

/// Cell and control spacing derived from the design system.
pub(super) struct TablePaddings {
    pub cell_padding_x: Pixels,
    pub cell_padding_y: Pixels,
    pub compact_padding_x: Pixels,
    pub compact_padding_y: Pixels,
    pub resize_hit_width: Pixels,
    pub control_radius: Pixels,
}

/// Header section: sortable/filterable column cells with resize handles.
pub(super) fn build_header_row<T: 'static>(
    columns: &[Column<T>],
    table_id: &ElementId,
    sort_state: &Option<SortState>,
    on_sort: &Option<Rc<Box<dyn Fn(&SortState, &mut Window, &mut App) + 'static>>>,
    theme: &TableTheme,
    pad: &TablePaddings,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let mut header_row = div()
        .flex()
        .w_full()
        .bg(theme.header_bg)
        .border_b_1()
        .border_color(theme.header_border);

    let sort_state = sort_state.clone();

    for column in columns {
        let column_id = column.id.clone();
        let is_sorted = sort_state
            .as_ref()
            .is_some_and(|s| s.column_id == column_id);
        let direction = if is_sorted {
            sort_state.as_ref().map(|s| s.direction)
        } else {
            None
        };

        let mut header_cell = div()
            .id(ElementId::from((table_id.clone(), column_id.clone())))
            .flex()
            .items_center()
            .gap_2()
            .px(pad.cell_padding_x)
            .py(pad.cell_padding_y)
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.header_text);

        if let Some(width) = column.width {
            header_cell = header_cell.w(width).flex_shrink_0();
        } else {
            header_cell = header_cell.flex_1();
        }

        if let Some(min_width) = column.min_width {
            header_cell = header_cell.min_w(min_width);
        }

        if column.sortable {
            header_cell = header_cell.cursor_pointer();
            if let Some(handler) = on_sort {
                let handler = handler.clone();
                let col_id = column_id.clone();
                let new_dir = match direction {
                    Some(SortDirection::Ascending) => SortDirection::Descending,
                    _ => SortDirection::Ascending,
                };
                header_cell =
                    header_cell.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                        handler(
                            &SortState {
                                column_id: col_id.clone(),
                                direction: new_dir,
                            },
                            window,
                            cx,
                        );
                    });
            }

            // Sort icon
            let icon_char = match direction {
                Some(SortDirection::Ascending) => "↑",
                Some(SortDirection::Descending) => "↓",
                None => "↕",
            };

            let icon_color = if is_sorted {
                theme.sort_icon_color
            } else {
                crate::color_tokens::with_alpha(theme.header_text, 0.3)
            };

            header_cell = header_cell.child(div().text_color(icon_color).child(icon_char));
        }

        if column.filterable {
            header_cell = header_cell.child(
                div()
                    .text_xs()
                    .text_color(crate::color_tokens::with_alpha(theme.header_text, 0.3))
                    .child("🔍"),
            );
        }

        if let Some(ref render) = column.header_render {
            header_cell = header_cell.child(render(window, cx));
        } else {
            header_cell = header_cell.child(column.header.clone());
        }

        if column.resizable {
            header_cell = header_cell.child(
                div()
                    .absolute()
                    .right_0()
                    .top_0()
                    .bottom_0()
                    .w(pad.resize_hit_width)
                    .cursor(CursorStyle::ResizeLeftRight)
                    .hover(|s| s.bg(theme.sort_icon_color)),
            );
        }

        header_row = header_row.child(header_cell);
    }
    header_row
}

/// Body section: virtual-windowed rows with selection and alternating fills.
///
/// Copies the (usually tiny) selection set once per render so `build` can
/// keep moving it into the keyboard handler with zero copies.
pub(super) fn build_body<T: 'static>(
    rows: &[T],
    columns: &[Column<T>],
    table_id: &ElementId,
    selection_mode: SelectionMode,
    focused_index: Option<usize>,
    selected_indices: &Rc<HashSet<usize>>,
    on_selection_change: &Option<Rc<Box<dyn Fn(&HashSet<usize>, &mut Window, &mut App) + 'static>>>,
    alternating_rows: bool,
    virtual_window: DataVirtualWindow,
    virtual_row_height: Option<f32>,
    theme: &TableTheme,
    pad: &TablePaddings,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let mut body = div()
        .id((table_id.clone(), "body"))
        .flex_1()
        .overflow_y_scroll()
        .flex()
        .flex_col();
    if let Some(height) = virtual_spacer_height(virtual_row_height, virtual_window.before_count()) {
        body = body.child(div().h(height).flex_shrink_0());
    }
    for (row_idx, row_data) in rows
        .iter()
        .enumerate()
        .skip(virtual_window.start)
        .take(virtual_window.len())
    {
        let is_selected = selected_indices.contains(&row_idx);
        let mut row_el = div()
            .id(ElementId::named_usize(table_id.to_string(), row_idx))
            .flex()
            .w_full()
            .border_b_1()
            .border_color(theme.cell_border);

        // Row styling
        if is_selected {
            row_el = row_el.bg(theme.row_selected_bg);
        } else if focused_index == Some(row_idx) {
            row_el = row_el.bg(theme.row_hover_bg);
        } else {
            let bg = if alternating_rows && row_idx % 2 != 0 {
                theme.row_alt_bg
            } else {
                theme.row_bg
            };
            let hover_bg = theme.row_hover_bg;
            row_el = row_el.bg(bg).hover(move |s| s.bg(hover_bg));
        }

        // Selection handler
        if selection_mode != SelectionMode::None {
            row_el = row_el.cursor_pointer();
            if let Some(handler) = on_selection_change {
                let handler = handler.clone();
                let current_selected = selected_indices.clone();
                row_el = row_el.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                    let mut next_selected = (*current_selected).clone();
                    match selection_mode {
                        SelectionMode::Single => {
                            next_selected.clear();
                            next_selected.insert(row_idx);
                        }
                        SelectionMode::Multiple => {
                            if next_selected.contains(&row_idx) {
                                next_selected.remove(&row_idx);
                            } else {
                                next_selected.insert(row_idx);
                            }
                        }
                        SelectionMode::None => {}
                    }
                    handler(&next_selected, window, cx);
                });
            }
        }

        for column in columns {
            let mut cell = div()
                .px(pad.cell_padding_x)
                .py(pad.cell_padding_y)
                .text_sm()
                .text_color(theme.cell_text)
                .flex()
                .items_center();

            if let Some(width) = column.width {
                cell = cell.w(width).flex_shrink_0();
            } else {
                cell = cell.flex_1();
            }

            if let Some(min_width) = column.min_width {
                cell = cell.min_w(min_width);
            }

            cell = cell.child((column.cell_render)(row_data, row_idx, window, cx));
            row_el = row_el.child(cell);
        }
        body = body.child(row_el);
    }
    if let Some(height) = virtual_spacer_height(virtual_row_height, virtual_window.after_count()) {
        body = body.child(div().h(height).flex_shrink_0());
    }
    body
}

/// Footer section (aggregate row); `None` when footers are hidden.
pub(super) fn build_footer_row<T: 'static>(
    columns: &[Column<T>],
    theme: &TableTheme,
    pad: &TablePaddings,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let mut footer_row = div()
        .flex()
        .w_full()
        .bg(theme.footer_bg)
        .border_t_1()
        .border_color(theme.header_border);

    for column in columns {
        let mut footer_cell = div()
            .px(pad.cell_padding_x)
            .py(pad.cell_padding_y)
            .text_xs()
            .text_color(theme.footer_text)
            .flex()
            .items_center();

        if let Some(width) = column.width {
            footer_cell = footer_cell.w(width).flex_shrink_0();
        } else {
            footer_cell = footer_cell.flex_1();
        }

        if let Some(ref render) = column.footer_render {
            footer_cell = footer_cell.child(render(window, cx));
        }

        footer_row = footer_row.child(footer_cell);
    }
    footer_row
}

/// Pagination bar; `None` when pagination is disabled.
pub(super) fn build_pagination_bar(
    pagination: &PaginationState,
    on_page_change: &Option<Rc<Box<dyn Fn(&usize, &mut Window, &mut App) + 'static>>>,
    theme: &TableTheme,
    pad: &TablePaddings,
    _window: &mut Window,
    _cx: &mut App,
) -> Div {
    let total_pages = pagination.total_pages();
    let current_page = pagination.current_page;

    let mut pagination_bar = div()
        .flex()
        .items_center()
        .justify_between()
        .px(pad.cell_padding_x)
        .py(pad.cell_padding_y)
        .bg(theme.header_bg)
        .border_t_1()
        .border_color(theme.header_border);

    // Page info
    let (start_item, end_item) = pagination.page_range();
    pagination_bar = pagination_bar.child(div().text_xs().text_color(theme.pagination_text).child(
        format!(
            "Showing {} to {} of {} items",
            start_item, end_item, pagination.total_items
        ),
    ));

    // Controls
    let mut controls = div().flex().items_center().gap_2();

    // Prev button
    let mut prev_btn = div()
        .px(pad.compact_padding_x)
        .py(pad.compact_padding_y)
        .text_xs()
        .rounded(pad.control_radius)
        .border_1()
        .border_color(theme.header_border)
        .text_color(theme.pagination_text);

    if current_page > 0 {
        prev_btn = prev_btn
            .cursor_pointer()
            .hover(|s| s.bg(theme.row_hover_bg));
        if let Some(handler) = on_page_change {
            let handler = handler.clone();
            let prev_page = current_page - 1;
            prev_btn = prev_btn.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                handler(&prev_page, window, cx);
            });
        }
    } else {
        prev_btn = prev_btn.opacity(0.5);
    }
    controls = controls.child(prev_btn.child("Previous"));

    // Page numbers
    controls = controls.child(
        div()
            .text_xs()
            .text_color(theme.pagination_text)
            .child(format!("Page {} of {}", current_page + 1, total_pages)),
    );

    // Next button
    let mut next_btn = div()
        .px(pad.compact_padding_x)
        .py(pad.compact_padding_y)
        .text_xs()
        .rounded(pad.control_radius)
        .border_1()
        .border_color(theme.header_border)
        .text_color(theme.pagination_text);

    if current_page + 1 < total_pages {
        next_btn = next_btn
            .cursor_pointer()
            .hover(|s| s.bg(theme.row_hover_bg));
        if let Some(handler) = on_page_change {
            let handler = handler.clone();
            let next_page = current_page + 1;
            next_btn = next_btn.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                handler(&next_page, window, cx);
            });
        }
    } else {
        next_btn = next_btn.opacity(0.5);
    }
    controls = controls.child(next_btn.child("Next"));

    pagination_bar = pagination_bar.child(controls);
    pagination_bar
}
