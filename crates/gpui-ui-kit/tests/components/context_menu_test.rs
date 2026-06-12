//! ContextMenu component tests

use gpui_ui_kit::context_menu::ContextMenu;
use gpui_ui_kit::menu::MenuItem;

#[test]
fn test_context_menu_creation() {
    let items = vec![
        MenuItem::new("cut", "Cut"),
        MenuItem::new("copy", "Copy"),
        MenuItem::new("paste", "Paste"),
    ];
    let menu = ContextMenu::new("ctx-menu", items);
    drop(menu);
}

#[test]
fn test_context_menu_position() {
    let items = vec![MenuItem::new("item-1", "Item 1")];
    let menu =
        ContextMenu::new("ctx-menu", items).position(gpui::point(gpui::px(100.0), gpui::px(200.0)));
    drop(menu);
}

#[test]
fn test_context_menu_min_width() {
    let items = vec![MenuItem::new("item-1", "Item 1")];
    let menu = ContextMenu::new("ctx-menu", items).min_width(gpui::px(220.0));
    drop(menu);
}

#[test]
fn test_context_menu_focused_index() {
    let items = vec![
        MenuItem::new("item-1", "First"),
        MenuItem::new("item-2", "Second"),
        MenuItem::new("item-3", "Third"),
    ];
    let menu = ContextMenu::new("ctx-menu", items).focused_index(1);
    drop(menu);
}

#[test]
fn test_context_menu_on_select() {
    let items = vec![MenuItem::new("item-1", "Item 1")];
    let menu = ContextMenu::new("ctx-menu", items).on_select(|_id, _window, _cx| {});
    drop(menu);
}

#[test]
fn test_context_menu_on_close() {
    let items = vec![MenuItem::new("item-1", "Item 1")];
    let menu = ContextMenu::new("ctx-menu", items).on_close(|_window, _cx| {});
    drop(menu);
}

#[test]
fn test_context_menu_on_focus_change() {
    let items = vec![MenuItem::new("item-1", "Item 1")];
    let menu = ContextMenu::new("ctx-menu", items).on_focus_change(|_idx, _window, _cx| {});
    drop(menu);
}

#[test]
fn test_context_menu_full_configuration() {
    let items = vec![
        MenuItem::new("cut", "Cut"),
        MenuItem::new("copy", "Copy"),
        MenuItem::new("paste", "Paste"),
    ];
    let menu = ContextMenu::new("ctx-menu", items)
        .position(gpui::point(gpui::px(50.0), gpui::px(75.0)))
        .min_width(gpui::px(200.0))
        .focused_index(0)
        .on_select(|_id, _window, _cx| {})
        .on_close(|_window, _cx| {})
        .on_focus_change(|_idx, _window, _cx| {});
    drop(menu);
}
