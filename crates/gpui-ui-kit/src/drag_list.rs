//! DragList component
//!
//! A reorderable list via drag-and-drop. Items can be reordered by dragging.
//!
//! # Usage
//!
//! ```ignore
//! DragList::new("plugin-rack", vec![
//!     DragItem::new("eq", div().child("EQ")),
//!     DragItem::new("comp", div().child("Compressor")),
//!     DragItem::new("limiter", div().child("Limiter")),
//! ])
//! .on_reorder(|from, to, window, cx| { /* handle reorder */ })
//! ```

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    AnyElement, App, CursorStyle, Div, ElementId, MouseButton, Pixels, Rgba, SharedString,
    Stateful, Window, div, px,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Theme colors for drag list
#[derive(Debug, Clone, ComponentTheme)]
pub struct DragListTheme {
    /// Item background
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub item_bg: Rgba,
    /// Item hover background
    #[theme(default = 0x2a2a2aff, from = surface_hover)]
    pub item_hover: Rgba,
    /// Drag handle color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub handle_color: Rgba,
    /// Drop indicator line
    #[theme(default = 0x007accff, from = accent)]
    pub drop_indicator: Rgba,
    /// Item border
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Dragging item background (overlay)
    #[theme(default = 0x2a2a2aee, from = surface)]
    pub dragging_bg: Rgba,
}

/// An item in a drag list
pub struct DragItem {
    id: SharedString,
    content: AnyElement,
}

impl DragItem {
    /// Create a new drag item
    pub fn new(id: impl Into<SharedString>, content: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            content: content.into_any_element(),
        }
    }
}

/// Drag list orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DragListOrientation {
    /// Vertical list (default)
    #[default]
    Vertical,
    /// Horizontal list
    Horizontal,
}

// Thread-local drag state keyed by list element ID.
// Stores the source index of the item being dragged.
thread_local! {
    static DRAG_LIST_DRAG_STATE: RefCell<HashMap<ElementId, usize>> = RefCell::new(HashMap::new());
}

/// A reorderable drag list component
pub struct DragList {
    id: ElementId,
    items: Vec<DragItem>,
    orientation: DragListOrientation,
    show_handles: bool,
    gap: Pixels,
    on_reorder: Option<Rc<dyn Fn(usize, usize, &mut Window, &mut App) + 'static>>,
}

impl DragList {
    /// Create a new drag list
    pub fn new(id: impl Into<ElementId>, items: Vec<DragItem>) -> Self {
        Self {
            id: id.into(),
            items,
            orientation: DragListOrientation::default(),
            show_handles: true,
            gap: px(2.0),
            on_reorder: None,
        }
    }

    /// Set orientation
    pub fn orientation(mut self, orientation: DragListOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Show/hide drag handles
    pub fn show_handles(mut self, show: bool) -> Self {
        self.show_handles = show;
        self
    }

    /// Set gap between items
    pub fn gap(mut self, gap: Pixels) -> Self {
        self.gap = gap;
        self
    }

    /// Called when items are reordered (from_index, to_index)
    pub fn on_reorder(
        mut self,
        handler: impl Fn(usize, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_reorder = Some(Rc::new(handler));
        self
    }

    /// Build with theme
    pub fn build_with_theme(self, theme: &DragListTheme) -> Stateful<Div> {
        let mut container = div().id(self.id.clone()).flex();

        container = match self.orientation {
            DragListOrientation::Vertical => container.flex_col().gap(self.gap),
            DragListOrientation::Horizontal => container.flex_row().gap(self.gap),
        };

        let on_reorder = self.on_reorder;
        let list_id = self.id.clone();

        for (idx, item) in self.items.into_iter().enumerate() {
            let hover_bg = theme.item_hover;
            let item_id = item.id.clone();
            let mut row = div()
                .id(ElementId::from(item_id))
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .py_1()
                .bg(theme.item_bg)
                .rounded(px(4.0))
                .border_1()
                .border_color(theme.border)
                .cursor_pointer()
                .hover(move |s| s.bg(hover_bg));

            // Drag handle
            if self.show_handles {
                row = row.child(
                    div()
                        .text_color(theme.handle_color)
                        .cursor(CursorStyle::ClosedHand)
                        .child("\u{2630}"), // ☰ hamburger
                );
            }

            row = row.child(div().flex_1().child(item.content));

            // Attach drag logic when a reorder handler is provided.
            if let Some(ref handler) = on_reorder {
                let _handler_down = handler.clone();
                let list_id_down = list_id.clone();
                row = row.on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                    DRAG_LIST_DRAG_STATE.with(|state| {
                        state.borrow_mut().insert(list_id_down.clone(), idx);
                    });
                });

                let handler_up = handler.clone();
                let list_id_up = list_id.clone();
                row = row.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                    DRAG_LIST_DRAG_STATE.with(|state| {
                        let mut state = state.borrow_mut();
                        if let Some(source_idx) = state.get(&list_id_up).copied() {
                            if source_idx != idx {
                                handler_up(source_idx, idx, window, cx);
                            }
                            state.remove(&list_id_up);
                        }
                    });
                    cx.stop_propagation();
                });
            }

            container = container.child(row);
        }

        // If a drag is released outside any row, clear the state.
        if on_reorder.is_some() {
            let list_id_clear = list_id.clone();
            container = container.on_mouse_up(MouseButton::Left, move |_event, _window, _cx| {
                DRAG_LIST_DRAG_STATE.with(|state| {
                    state.borrow_mut().remove(&list_id_clear);
                });
            });
        }

        container
    }
}

impl RenderOnce for DragList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = DragListTheme::from(&global_theme);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for DragList {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{DragItem, DragList};
    use gpui::{ParentElement, div};

    #[test]
    fn test_drag_list_builds_with_reorder_handler() {
        let _el = DragList::new(
            "test-drag-list",
            vec![
                DragItem::new("a", div().child("A")),
                DragItem::new("b", div().child("B")),
            ],
        )
        .on_reorder(|_from, _to, _window, _cx| {})
        .build_with_theme(&super::DragListTheme::default());
    }
}
