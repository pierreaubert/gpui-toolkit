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
use gpui::{App, ElementId, FocusHandle, KeyDownEvent, Pixels, SharedString, Window, div, px};
use gpui_design::DesignSystem;
use std::collections::HashSet;
use std::sync::Arc;

mod column;
mod pagination_state;
mod sort_direction;
mod table_build;
mod types;

pub use column::Column;
pub use pagination_state::PaginationState;
pub use sort_direction::SortDirection;
use table_build::{
    TablePaddings, build_body, build_footer_row, build_header_row, build_pagination_bar,
};
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
        let pad = TablePaddings {
            cell_padding_x: px(design.spacing.card_padding),
            cell_padding_y: px(design.spacing.control_padding_y),
            compact_padding_x: px(design.spacing.control_padding_x * 0.5),
            compact_padding_y: px(design.spacing.control_padding_y * 0.5),
            resize_hit_width: px(design.interaction.min_touch_target * 0.25),
            control_radius: px(design.corners.sm),
        };
        let focus_handle = self
            .focus_handle
            .clone()
            .unwrap_or_else(|| cx.focus_handle());
        let row_count = self.rows.len();
        let focused_index = self.focused_index.filter(|index| *index < row_count);
        let mut container = div()
            .id(table_id.clone())
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.row_bg)
            .track_focus(&focus_handle)
            .focusable();

        // Move handler state first; section helpers then borrow the
        // remaining fields (disjoint from the moved ones).
        let on_sort = self.on_sort.map(std::rc::Rc::new);
        let selection_mode = self.selection_mode;
        let selected_indices = std::rc::Rc::new(self.selected_indices);
        let on_focus_change = self.on_focus_change.map(std::rc::Rc::new);
        let on_selection_change = self.on_selection_change.map(std::rc::Rc::new);
        let on_page_change = self.on_page_change.map(std::rc::Rc::new);

        let header = build_header_row(
            &self.columns,
            &self.id,
            &self.sort_state,
            &on_sort,
            &theme,
            &pad,
            window,
            cx,
        );
        let body = build_body(
            &self.rows,
            &self.columns,
            &self.id,
            selection_mode,
            self.focused_index.filter(|index| *index < self.rows.len()),
            &selected_indices,
            &on_selection_change,
            self.alternating_rows,
            self.virtual_window
                .unwrap_or_else(|| DataVirtualWindow::full(self.rows.len()))
                .with_total(self.rows.len()),
            self.virtual_row_height,
            &theme,
            &pad,
            window,
            cx,
        );
        let footer = self
            .show_footer
            .then(|| build_footer_row(&self.columns, &theme, &pad, window, cx));
        let pagination = self.pagination.as_ref().map(|pagination| {
            build_pagination_bar(pagination, &on_page_change, &theme, &pad, window, cx)
        });
        container = container.child(header);

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

        container = container.child(body);

        if let Some(footer) = footer {
            container = container.child(footer);
        }

        if let Some(bar) = pagination {
            container = container.child(bar);
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
