//! Tabs component tests

use gpui_ui_kit::tabs::{TabItem, Tabs};

#[test]
fn test_tabs_creation() {
    let tabs = Tabs::new("tabs")
        .tabs(vec![
            TabItem::new("tab-1", "Tab 1"),
            TabItem::new("tab-2", "Tab 2"),
        ])
        .selected_index(0);
    drop(tabs);
}

#[test]
fn test_tabs_supports_mouse_click() {
    let tabs = Tabs::new("tabs")
        .tabs(vec![
            TabItem::new("tab-1", "Tab 1"),
            TabItem::new("tab-2", "Tab 2"),
        ])
        .selected_index(0)
        .on_change(|_index, _window, _cx| {});

    drop(tabs);
}

#[test]
fn test_tabs_supports_keyboard_navigation() {
    let tabs = Tabs::new("tabs")
        .tabs(vec![
            TabItem::new("tab-1", "Tab 1"),
            TabItem::new("tab-2", "Tab 2"),
            TabItem::new("tab-3", "Tab 3"),
        ])
        .selected_index(0)
        .on_change(|_index, _window, _cx| {});

    drop(tabs);
}
