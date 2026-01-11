//! IconButton component tests

use gpui_ui_kit::icon_button::IconButton;

#[test]
fn test_icon_button_creation() {
    let icon_button = IconButton::new("test", "🔍");
    drop(icon_button);
}

#[test]
fn test_icon_button_supports_mouse_click() {
    let icon_button = IconButton::new("test", "🔍").on_click(|_window, _cx| {});
    drop(icon_button);
}
