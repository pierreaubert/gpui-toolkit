//! Toolbar component tests

use gpui::div;
use gpui::prelude::ParentElement;
use gpui_ui_kit::toolbar::{Toolbar, ToolbarItem};

#[test]
fn test_toolbar_creation() {
    let toolbar = Toolbar::new("tb-1");
    drop(toolbar);
}

#[test]
fn test_toolbar_item() {
    let toolbar = Toolbar::new("tb-item")
        .item(ToolbarItem::button("bold", "B"))
        .item(ToolbarItem::button("italic", "I"));
    drop(toolbar);
}

#[test]
fn test_toolbar_separator() {
    let toolbar = Toolbar::new("tb-sep")
        .item(ToolbarItem::button("bold", "B"))
        .separator()
        .item(ToolbarItem::button("align", "<"));
    drop(toolbar);
}

#[test]
fn test_toolbar_bordered() {
    let toolbar = Toolbar::new("tb-border").bordered(false);
    drop(toolbar);
}

#[test]
fn test_toolbar_button_active() {
    let toolbar = Toolbar::new("tb-active").item(ToolbarItem::button("bold", "B").active(true));
    drop(toolbar);
}

#[test]
fn test_toolbar_button_disabled() {
    let toolbar =
        Toolbar::new("tb-disabled").item(ToolbarItem::button("redo", "Redo").disabled(true));
    drop(toolbar);
}

#[test]
fn test_toolbar_button_on_click() {
    let toolbar = Toolbar::new("tb-click")
        .item(ToolbarItem::button("save", "Save").on_click(|_window, _cx| {}));
    drop(toolbar);
}

#[test]
fn test_toolbar_custom_item() {
    let toolbar = Toolbar::new("tb-custom").item(ToolbarItem::custom(div().child("Custom widget")));
    drop(toolbar);
}

#[test]
fn test_toolbar_full_configuration() {
    let toolbar = Toolbar::new("tb-full")
        .bordered(true)
        .item(ToolbarItem::button("bold", "B").active(true))
        .item(ToolbarItem::button("italic", "I").on_click(|_window, _cx| {}))
        .separator()
        .item(ToolbarItem::button("undo", "Undo").disabled(true))
        .item(ToolbarItem::custom(div().child("Zoom: 100%")));
    drop(toolbar);
}
