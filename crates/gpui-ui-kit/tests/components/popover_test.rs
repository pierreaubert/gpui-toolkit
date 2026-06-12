//! Popover component tests

use gpui::div;
use gpui::prelude::{IntoElement, ParentElement};
use gpui_ui_kit::popover::{Popover, PopoverPlacement};

#[test]
fn test_popover_creation() {
    let popover = Popover::new("my-popover");
    drop(popover);
}

#[test]
fn test_popover_all_placements() {
    let placements = [
        PopoverPlacement::Top,
        PopoverPlacement::Bottom,
        PopoverPlacement::Left,
        PopoverPlacement::Right,
        PopoverPlacement::TopStart,
        PopoverPlacement::TopEnd,
        PopoverPlacement::BottomStart,
        PopoverPlacement::BottomEnd,
    ];

    for placement in &placements {
        let popover = Popover::new("test").placement(*placement);
        drop(popover);
    }
}

#[test]
fn test_popover_width() {
    let popover = Popover::new("test").width(gpui::px(300.0));
    drop(popover);
}

#[test]
fn test_popover_show_backdrop() {
    let popover = Popover::new("test").show_backdrop(true);
    drop(popover);

    let popover = Popover::new("test").show_backdrop(false);
    drop(popover);
}

#[test]
fn test_popover_content() {
    let popover = Popover::new("test").content(div().child("Popover body"));
    drop(popover);
}

#[test]
fn test_popover_content_with_factory() {
    let popover = Popover::new("test")
        .content_with(|_theme| div().child("Themed content").into_any_element());
    drop(popover);
}

#[test]
fn test_popover_on_close() {
    let popover = Popover::new("test").on_close(|_window, _cx| {});
    drop(popover);
}

#[test]
fn test_popover_full_configuration() {
    let popover = Popover::new("device-picker")
        .placement(PopoverPlacement::BottomStart)
        .width(gpui::px(250.0))
        .show_backdrop(true)
        .content(div().child("Device list"))
        .on_close(|_window, _cx| {});
    drop(popover);
}
