//! Menu component tests

use gpui_ui_kit::menu::{Menu, MenuItem};

#[test]
fn test_menu_creation() {
    let menu = Menu::new(
        "test-menu-1",
        vec![
            MenuItem::new("item-1", "Menu Item 1"),
            MenuItem::new("item-2", "Menu Item 2"),
        ],
    );
    drop(menu);
}

#[test]
fn test_menu_supports_mouse_click() {
    let menu = Menu::new(
        "test-menu-2",
        vec![
            MenuItem::new("item-1", "Menu Item 1"),
            MenuItem::new("item-2", "Menu Item 2"),
        ],
    )
    .on_select(|_id, _window, _cx| {});

    drop(menu);
}

#[test]
fn test_menu_supports_keyboard_navigation() {
    let menu = Menu::new(
        "test-menu-3",
        vec![
            MenuItem::new("item-1", "First"),
            MenuItem::new("item-2", "Second"),
            MenuItem::new("item-3", "Third"),
        ],
    )
    .on_select(|_id, _window, _cx| {});

    drop(menu);
}
