//! Table component for displaying structured data
//!
//! Features:
//! - Column definitions with custom rendering
//! - Sorting (ascending/descending)
//! - Pagination
//! - Selection (none, single, multiple)
//! - Resizable columns (simulated with width callbacks)
//! - Alternating row colors
//! - Header and footer support
//! - Styling via TableTheme

use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::data_navigation::{DataNavigationAction, DataNavigationState, DataVirtualWindow};
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    App, CursorStyle, ElementId, FocusHandle, FontWeight, KeyDownEvent, MouseButton, Pixels,
    SharedString, Window, div, px,
};
use gpui_design::DesignSystem;
use std::collections::HashSet;
use std::sync::Arc;

mod column;
mod pagination_state;
mod sort_direction;
mod types;

pub use column::Column;
pub use pagination_state::PaginationState;
pub use sort_direction::SortDirection;
pub use types::{SelectionMode, SortState, TableTheme};
/// Table component
pub struct Table<T> {
    id: ElementId,
    columns: Vec<Column<T>>,
    rows: Vec<T>,
    sort_state: Option<SortState>,
    on_sort: Option<Box<dyn Fn(&SortState, &mut Window, &mut App) + 'static>>,
    selection_mode: SelectionMode,
    focused_index: Option<usize>,
    focus_handle: Option<FocusHandle>,
    virtual_window: Option<DataVirtualWindow>,
    virtual_row_height: Option<f32>,
    selected_indices: HashSet<usize>,
    on_focus_change: Option<Box<dyn Fn(Option<usize>, &mut Window, &mut App) + 'static>>,
    on_selection_change: Option<Box<dyn Fn(&HashSet<usize>, &mut Window, &mut App) + 'static>>,
    pagination: Option<PaginationState>,
    on_page_change: Option<Box<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
    on_resize: Option<Box<dyn Fn(&SharedString, Pixels, &mut Window, &mut App) + 'static>>,
    alternating_rows: bool,
    show_footer: bool,
    theme: Option<TableTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl<T: 'static> Table<T> {
    /// Create a new table
    pub fn new(id: impl Into<ElementId>, rows: Vec<T>) -> Self {
        Self {
            id: id.into(),
            columns: Vec::new(),
            rows,
            sort_state: None,
            on_sort: None,
            selection_mode: SelectionMode::None,
            focused_index: None,
            focus_handle: None,
            virtual_window: None,
            virtual_row_height: None,
            selected_indices: HashSet::new(),
            on_focus_change: None,
            on_selection_change: None,
            pagination: None,
            on_page_change: None,
            on_resize: None,
            alternating_rows: true,
            show_footer: false,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Add a column
    pub fn column(mut self, column: Column<T>) -> Self {
        self.columns.push(column);
        self
    }

    /// Set all columns
    pub fn columns(mut self, columns: Vec<Column<T>>) -> Self {
        self.columns = columns;
        self
    }

    /// Set sort state
    pub fn sort(mut self, state: SortState) -> Self {
        self.sort_state = Some(state);
        self
    }

    /// Set sort handler
    pub fn on_sort(
        mut self,
        handler: impl Fn(&SortState, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_sort = Some(Box::new(handler));
        self
    }

    /// Set selection mode
    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Set the currently keyboard-focused row index.
    pub fn focused_index(mut self, index: Option<usize>) -> Self {
        self.focused_index = index;
        self
    }

    /// Set the focus handle used for table keyboard navigation.
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Set a virtual row window for large tables.
    pub fn virtual_window(mut self, window: DataVirtualWindow) -> Self {
        self.virtual_window = Some(window);
        self
    }

    /// Set a virtual row window and fixed row height for scroll extent spacers.
    pub fn virtual_window_with_row_height(
        mut self,
        window: DataVirtualWindow,
        row_height: f32,
    ) -> Self {
        self.virtual_window = Some(window);
        self.virtual_row_height = Some(row_height);
        self
    }

    /// Compute and set a virtual row window from scroll geometry.
    pub fn virtual_viewport(
        mut self,
        scroll_offset: f32,
        row_height: f32,
        viewport_height: f32,
        overscan: usize,
    ) -> Self {
        self.virtual_window = Some(DataVirtualWindow::from_viewport(
            self.rows.len(),
            scroll_offset,
            row_height,
            viewport_height,
            overscan,
        ));
        self.virtual_row_height = Some(row_height);
        self
    }

    /// Set the focused-row change handler.
    pub fn on_focus_change(
        mut self,
        handler: impl Fn(Option<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_focus_change = Some(Box::new(handler));
        self
    }

    /// Set selected indices
    pub fn selected_indices(mut self, indices: HashSet<usize>) -> Self {
        self.selected_indices = indices;
        self
    }

    /// Set selection change handler
    pub fn on_selection_change(
        mut self,
        handler: impl Fn(&HashSet<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_selection_change = Some(Box::new(handler));
        self
    }

    /// Set pagination state
    pub fn pagination(mut self, state: PaginationState) -> Self {
        self.pagination = Some(state);
        self
    }

    /// Set page change handler
    pub fn on_page_change(
        mut self,
        handler: impl Fn(&usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_page_change = Some(Box::new(handler));
        self
    }

    /// Set column resize handler
    pub fn on_resize(
        mut self,
        handler: impl Fn(&SharedString, Pixels, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resize = Some(Box::new(handler));
        self
    }

    /// Enable/disable alternating row colors
    pub fn alternating_rows(mut self, alternating: bool) -> Self {
        self.alternating_rows = alternating;
        self
    }

    /// Enable/disable footer
    pub fn show_footer(mut self, show: bool) -> Self {
        self.show_footer = show;
        self
    }

    /// Set custom theme
    pub fn theme(mut self, theme: TableTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Override the design system used for table spacing and hit-area defaults.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit ARIA label
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Table)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    fn build(
        self,
        theme: TableTheme,
        design: Arc<DesignSystem>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let theme = std::rc::Rc::new(theme);
        let table_id = self.id.clone();
        let cell_padding_x = px(design.spacing.card_padding);
        let cell_padding_y = px(design.spacing.control_padding_y);
        let compact_padding_x = px(design.spacing.control_padding_x * 0.5);
        let compact_padding_y = px(design.spacing.control_padding_y * 0.5);
        let resize_hit_width = px(design.interaction.min_touch_target * 0.25);
        let control_radius = px(design.corners.sm);
        let focus_handle = self.focus_handle.unwrap_or_else(|| cx.focus_handle());
        let row_count = self.rows.len();
        let focused_index = self.focused_index.filter(|index| *index < row_count);
        let virtual_window = self
            .virtual_window
            .unwrap_or_else(|| DataVirtualWindow::full(row_count))
            .with_total(row_count);
        let virtual_row_height = self.virtual_row_height;
        let mut container = div()
            .id(table_id.clone())
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.row_bg)
            .track_focus(&focus_handle)
            .focusable();

        // Header
        let mut header_row = div()
            .flex()
            .w_full()
            .bg(theme.header_bg)
            .border_b_1()
            .border_color(theme.header_border);

        let sort_state = self.sort_state.clone();
        let on_sort = self.on_sort.map(std::rc::Rc::new);

        for column in &self.columns {
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
                .px(cell_padding_x)
                .py(cell_padding_y)
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
                if let Some(ref handler) = on_sort {
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
                        .w(resize_hit_width)
                        .cursor(CursorStyle::ResizeLeftRight)
                        .hover(|s| s.bg(theme.sort_icon_color)),
                );
            }

            header_row = header_row.child(header_cell);
        }
        container = container.child(header_row);

        // Body
        let mut body = div()
            .id("table-body")
            .flex_1()
            .overflow_y_scroll()
            .flex()
            .flex_col();

        let selection_mode = self.selection_mode;
        let selected_indices = std::rc::Rc::new(self.selected_indices);
        let on_focus_change = self.on_focus_change.map(std::rc::Rc::new);
        let on_selection_change = self.on_selection_change.map(std::rc::Rc::new);

        if on_focus_change.is_some() || on_selection_change.is_some() {
            let on_focus_change_for_key = on_focus_change.clone();
            let on_selection_change_for_key = on_selection_change.clone();
            let selected_indices_for_key = selected_indices.clone();
            let focus_handle_for_key = focus_handle.clone();
            container = container.on_key_down(
                move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
                    if !focus_handle_for_key.is_focused(window) {
                        return;
                    }

                    let Some(action) = DataNavigationAction::from_key(event.keystroke.key.as_str())
                    else {
                        return;
                    };

                    match action {
                        DataNavigationAction::Previous
                        | DataNavigationAction::Next
                        | DataNavigationAction::First
                        | DataNavigationAction::Last => {
                            let next = DataNavigationState::new(row_count)
                                .selected_index(focused_index)
                                .move_selection(action);
                            if next != focused_index {
                                cx.stop_propagation();
                                if let Some(ref handler) = on_focus_change_for_key {
                                    handler(next, window, cx);
                                }
                            }
                        }
                        DataNavigationAction::Activate => {
                            if let Some(index) = focused_index
                                && selection_mode != SelectionMode::None
                                && let Some(ref handler) = on_selection_change_for_key
                            {
                                let mut next_selected = (*selected_indices_for_key).clone();
                                match selection_mode {
                                    SelectionMode::Single => {
                                        next_selected.clear();
                                        next_selected.insert(index);
                                    }
                                    SelectionMode::Multiple => {
                                        if next_selected.contains(&index) {
                                            next_selected.remove(&index);
                                        } else {
                                            next_selected.insert(index);
                                        }
                                    }
                                    SelectionMode::None => {}
                                }
                                cx.stop_propagation();
                                handler(&next_selected, window, cx);
                            }
                        }
                        _ => {}
                    }
                },
            );
        }

        if let Some(height) =
            virtual_spacer_height(virtual_row_height, virtual_window.before_count())
        {
            body = body.child(div().h(height).flex_shrink_0());
        }

        for (row_idx, row_data) in self
            .rows
            .iter()
            .enumerate()
            .skip(virtual_window.start)
            .take(virtual_window.len())
        {
            let is_selected = selected_indices.contains(&row_idx);
            let mut row_el = div()
                .id(ElementId::from(row_idx))
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
                let bg = if self.alternating_rows && row_idx % 2 != 0 {
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
                if let Some(ref handler) = on_selection_change {
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

            for column in &self.columns {
                let mut cell = div()
                    .px(cell_padding_x)
                    .py(cell_padding_y)
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

        if let Some(height) =
            virtual_spacer_height(virtual_row_height, virtual_window.after_count())
        {
            body = body.child(div().h(height).flex_shrink_0());
        }
        container = container.child(body);

        // Footer
        if self.show_footer {
            let mut footer_row = div()
                .flex()
                .w_full()
                .bg(theme.footer_bg)
                .border_t_1()
                .border_color(theme.header_border);

            for column in &self.columns {
                let mut footer_cell = div()
                    .px(cell_padding_x)
                    .py(cell_padding_y)
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
            container = container.child(footer_row);
        }

        // Pagination
        if let Some(pagination) = self.pagination {
            let total_pages = pagination.total_pages();
            let current_page = pagination.current_page;
            let on_page_change = self.on_page_change.map(std::rc::Rc::new);

            let mut pagination_bar = div()
                .flex()
                .items_center()
                .justify_between()
                .px(cell_padding_x)
                .py(cell_padding_y)
                .bg(theme.header_bg)
                .border_t_1()
                .border_color(theme.header_border);

            // Page info
            let (start_item, end_item) = pagination.page_range();
            pagination_bar =
                pagination_bar.child(div().text_xs().text_color(theme.pagination_text).child(
                    format!(
                        "Showing {} to {} of {} items",
                        start_item, end_item, pagination.total_items
                    ),
                ));

            // Controls
            let mut controls = div().flex().items_center().gap_2();

            // Prev button
            let mut prev_btn = div()
                .px(compact_padding_x)
                .py(compact_padding_y)
                .text_xs()
                .rounded(control_radius)
                .border_1()
                .border_color(theme.header_border)
                .text_color(theme.pagination_text);

            if current_page > 0 {
                prev_btn = prev_btn
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.row_hover_bg));
                if let Some(ref handler) = on_page_change {
                    let handler = handler.clone();
                    let prev_page = current_page - 1;
                    prev_btn =
                        prev_btn.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
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
                .px(compact_padding_x)
                .py(compact_padding_y)
                .text_xs()
                .rounded(control_radius)
                .border_1()
                .border_color(theme.header_border)
                .text_color(theme.pagination_text);

            if current_page + 1 < total_pages {
                next_btn = next_btn
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.row_hover_bg));
                if let Some(ref handler) = on_page_change {
                    let handler = handler.clone();
                    let next_page = current_page + 1;
                    next_btn =
                        next_btn.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                            handler(&next_page, window, cx);
                        });
                }
            } else {
                next_btn = next_btn.opacity(0.5);
            }
            controls = controls.child(next_btn.child("Next"));

            pagination_bar = pagination_bar.child(controls);
            container = container.child(pagination_bar);
        }

        container
    }
}

impl<T: 'static> RenderOnce for Table<T> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Register in accessibility tree
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_default(),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Table)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| TableTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build(theme, design, window, cx)
    }
}

impl<T: 'static> IntoElement for Table<T> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

fn virtual_spacer_height(row_height: Option<f32>, row_count: usize) -> Option<Pixels> {
    if row_count == 0 {
        return None;
    }

    let row_height = row_height?;
    if !row_height.is_finite() || row_height <= 0.0 {
        return None;
    }

    let height = row_height * row_count as f32;
    if height.is_finite() && height > 0.0 {
        Some(px(height))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{DataVirtualWindow, SelectionMode, Table, virtual_spacer_height};
    use gpui::px;

    #[test]
    fn table_builder_records_keyboard_navigation_state() {
        let table = Table::new("rows", vec![1, 2, 3])
            .selection_mode(SelectionMode::Single)
            .focused_index(Some(1))
            .on_focus_change(|_, _, _| {})
            .on_selection_change(|_, _, _| {});

        assert_eq!(table.focused_index, Some(1));
        assert_eq!(table.selection_mode, SelectionMode::Single);
        assert!(table.on_focus_change.is_some());
        assert!(table.on_selection_change.is_some());
    }

    #[test]
    fn table_builder_records_virtual_window() {
        let table =
            Table::new("rows", vec![1, 2, 3, 4, 5]).virtual_window(DataVirtualWindow::new(5, 1, 4));

        assert_eq!(table.virtual_window, Some(DataVirtualWindow::new(5, 1, 4)));
        assert_eq!(table.virtual_row_height, None);
    }

    #[test]
    fn table_builder_records_virtual_window_with_row_height() {
        let table = Table::new("rows", vec![1, 2, 3, 4, 5])
            .virtual_window_with_row_height(DataVirtualWindow::new(5, 1, 4), 24.0);

        assert_eq!(table.virtual_window, Some(DataVirtualWindow::new(5, 1, 4)));
        assert_eq!(table.virtual_row_height, Some(24.0));
    }

    #[test]
    fn table_builder_computes_virtual_viewport() {
        let table =
            Table::new("rows", vec![1, 2, 3, 4, 5, 6]).virtual_viewport(20.0, 10.0, 20.0, 1);

        assert_eq!(table.virtual_window, Some(DataVirtualWindow::new(6, 1, 5)));
        assert_eq!(table.virtual_row_height, Some(10.0));
    }

    #[test]
    fn virtual_spacer_height_rejects_invalid_geometry() {
        assert_eq!(virtual_spacer_height(None, 3), None);
        assert_eq!(virtual_spacer_height(Some(0.0), 3), None);
        assert_eq!(virtual_spacer_height(Some(f32::NAN), 3), None);
        assert_eq!(virtual_spacer_height(Some(12.0), 0), None);
        assert_eq!(virtual_spacer_height(Some(12.0), 3), Some(px(36.0)));
    }
}
