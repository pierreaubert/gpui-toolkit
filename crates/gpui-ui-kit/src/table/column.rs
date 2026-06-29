use gpui::prelude::IntoElement;
use gpui::{AnyElement, App, Pixels, SharedString, Window, div};

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
