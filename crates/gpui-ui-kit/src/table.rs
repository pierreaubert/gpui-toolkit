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

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::*;
use gpui::*;
use std::collections::HashSet;

/// Theme colors for table styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct TableTheme {
    /// Header background color
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub header_bg: Rgba,
    /// Header text color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub header_text: Rgba,
    /// Header border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub header_border: Rgba,
    /// Row background color
    #[theme(default = 0x1e1e1eff, from = background)]
    pub row_bg: Rgba,
    /// Alternating row background color
    #[theme(default = 0x252525ff, from = muted)]
    pub row_alt_bg: Rgba,
    /// Row background color on hover
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub row_hover_bg: Rgba,
    /// Row background color when selected
    #[theme(default = 0x007acc33, from = accent_muted)]
    pub row_selected_bg: Rgba,
    /// Cell text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub cell_text: Rgba,
    /// Cell border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub cell_border: Rgba,
    /// Sort icon color
    #[theme(default = 0x007accff, from = accent)]
    pub sort_icon_color: Rgba,
    /// Pagination controls text color
    #[theme(default = 0x888888ff, from = text_muted)]
    pub pagination_text: Rgba,
    /// Footer background color
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub footer_bg: Rgba,
    /// Footer text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub footer_text: Rgba,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    /// Toggle direction
    pub fn toggle(&self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

/// Sort state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortState {
    pub column_id: SharedString,
    pub direction: SortDirection,
}

/// Selection mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    #[default]
    None,
    Single,
    Multiple,
}

/// Column definition
pub struct Column<T> {
    pub id: SharedString,
    pub header: SharedString,
    pub width: Option<Pixels>,
    pub min_width: Option<Pixels>,
    pub sortable: bool,
    pub filterable: bool,
    pub resizable: bool,
    pub cell_render: Box<dyn Fn(&T, usize, &mut Window, &mut App) -> AnyElement>,
    pub header_render: Option<Box<dyn Fn(&mut Window, &mut App) -> AnyElement>>,
    pub footer_render: Option<Box<dyn Fn(&mut Window, &mut App) -> AnyElement>>,
}

impl<T> Column<T> {
    /// Create a new column
    pub fn new(id: impl Into<SharedString>, header: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            header: header.into(),
            width: None,
            min_width: None,
            sortable: true,
            filterable: false,
            resizable: true,
            cell_render: Box::new(|_, _, _, _| div().into_any_element()),
            header_render: None,
            footer_render: None,
        }
    }

    /// Set fixed width
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    /// Set minimum width
    pub fn min_width(mut self, min_width: Pixels) -> Self {
        self.min_width = Some(min_width);
        self
    }

    /// Set if column is sortable
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    /// Set if column is filterable
    pub fn filterable(mut self, filterable: bool) -> Self {
        self.filterable = filterable;
        self
    }

    /// Set if column is resizable
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Set custom cell renderer
    pub fn cell_render<F, E>(mut self, render: F) -> Self
    where
        F: Fn(&T, usize, &mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.cell_render = Box::new(move |item, row_idx, window, cx| {
            render(item, row_idx, window, cx).into_any_element()
        });
        self
    }

    /// Set custom header renderer
    pub fn header_render<F, E>(mut self, render: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.header_render = Some(Box::new(move |window, cx| {
            render(window, cx).into_any_element()
        }));
        self
    }

    /// Set custom footer renderer
    pub fn footer_render<F, E>(mut self, render: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.footer_render = Some(Box::new(move |window, cx| {
            render(window, cx).into_any_element()
        }));
        self
    }
}

/// Pagination state
#[derive(Debug, Clone, Default)]
pub struct PaginationState {
    pub current_page: usize,
    pub page_size: usize,
    pub total_items: usize,
}

impl PaginationState {
    /// Calculate total pages
    pub fn total_pages(&self) -> usize {
        if self.page_size == 0 {
            0
        } else {
            self.total_items.div_ceil(self.page_size)
        }
    }
}

/// Table component
pub struct Table<T> {
    id: ElementId,
    columns: Vec<Column<T>>,
    rows: Vec<T>,
    sort_state: Option<SortState>,
    on_sort: Option<Box<dyn Fn(&SortState, &mut Window, &mut App) + 'static>>,
    selection_mode: SelectionMode,
    selected_indices: HashSet<usize>,
    on_selection_change: Option<Box<dyn Fn(&HashSet<usize>, &mut Window, &mut App) + 'static>>,
    pagination: Option<PaginationState>,
    on_page_change: Option<Box<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
    on_resize: Option<Box<dyn Fn(&SharedString, Pixels, &mut Window, &mut App) + 'static>>,
    alternating_rows: bool,
    show_footer: bool,
    theme: Option<TableTheme>,
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
            selected_indices: HashSet::new(),
            on_selection_change: None,
            pagination: None,
            on_page_change: None,
            on_resize: None,
            alternating_rows: true,
            show_footer: false,
            theme: None,
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

    fn build(self, theme: TableTheme, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = std::rc::Rc::new(theme);
        let mut container = div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.row_bg);

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
                .id(SharedString::from(format!("header-{}", column_id)))
                .flex()
                .items_center()
                .gap_2()
                .px_4()
                .py_2()
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
                        .w(px(4.0))
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
        let selected_indices = self.selected_indices.clone();
        let on_selection_change = self.on_selection_change.map(std::rc::Rc::new);

        for (row_idx, row_data) in self.rows.iter().enumerate() {
            let is_selected = selected_indices.contains(&row_idx);
            let mut row_el = div()
                .id(SharedString::from(format!("row-{}", row_idx)))
                .flex()
                .w_full()
                .border_b_1()
                .border_color(theme.cell_border);

            // Row styling
            if is_selected {
                row_el = row_el.bg(theme.row_selected_bg);
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
                        let mut next_selected = current_selected.clone();
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
                    .px_4()
                    .py_2()
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
                    .px_4()
                    .py_2()
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
                .px_4()
                .py_2()
                .bg(theme.header_bg)
                .border_t_1()
                .border_color(theme.header_border);

            // Page info
            let start_item = current_page * pagination.page_size + 1;
            let end_item = ((current_page + 1) * pagination.page_size).min(pagination.total_items);
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
                .px_2()
                .py_1()
                .text_xs()
                .rounded_md()
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
                .px_2()
                .py_1()
                .text_xs()
                .rounded_md()
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
        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| TableTheme::from(&global_theme));
        self.build(theme, window, cx)
    }
}

impl<T: 'static> IntoElement for Table<T> {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
