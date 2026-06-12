//! DragList component tests

use gpui::div;
use gpui::prelude::ParentElement;
use gpui_ui_kit::drag_list::{DragItem, DragList, DragListOrientation};

#[test]
fn test_drag_list_creation() {
    let list = DragList::new(
        "dl-1",
        vec![
            DragItem::new("eq", div().child("EQ")),
            DragItem::new("comp", div().child("Compressor")),
        ],
    );
    drop(list);
}

#[test]
fn test_drag_list_orientation() {
    for orientation in [
        DragListOrientation::Vertical,
        DragListOrientation::Horizontal,
    ] {
        let list = DragList::new("dl-orient", vec![DragItem::new("a", div().child("A"))])
            .orientation(orientation);
        drop(list);
    }
}

#[test]
fn test_drag_list_show_handles() {
    let list =
        DragList::new("dl-handles", vec![DragItem::new("a", div().child("A"))]).show_handles(false);
    drop(list);
}

#[test]
fn test_drag_list_gap() {
    let list =
        DragList::new("dl-gap", vec![DragItem::new("a", div().child("A"))]).gap(gpui::px(8.0));
    drop(list);
}

#[test]
fn test_drag_list_on_reorder() {
    let list = DragList::new(
        "dl-reorder",
        vec![
            DragItem::new("a", div().child("A")),
            DragItem::new("b", div().child("B")),
        ],
    )
    .on_reorder(|_from, _to, _window, _cx| {});
    drop(list);
}

#[test]
fn test_drag_list_full_configuration() {
    let list = DragList::new(
        "dl-full",
        vec![
            DragItem::new("eq", div().child("EQ")),
            DragItem::new("comp", div().child("Compressor")),
            DragItem::new("limiter", div().child("Limiter")),
        ],
    )
    .orientation(DragListOrientation::Vertical)
    .show_handles(true)
    .gap(gpui::px(4.0))
    .on_reorder(|_from, _to, _window, _cx| {});
    drop(list);
}
