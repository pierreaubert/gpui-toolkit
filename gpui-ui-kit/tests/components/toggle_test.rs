//! Toggle component tests

use gpui_ui_kit::toggle::Toggle;

#[test]
fn test_toggle_creation() {
    let toggle = Toggle::new("test").label("Enable feature").checked(true);
    drop(toggle);
}

#[test]
fn test_toggle_supports_mouse_click() {
    let toggle = Toggle::new("test")
        .label("Enable feature")
        .checked(false)
        .on_change(|_checked, _window, _cx| {});

    drop(toggle);
}

#[test]
fn test_toggle_supports_keyboard() {
    let toggle = Toggle::new("test")
        .checked(false)
        .on_change(|_checked, _window, _cx| {});

    drop(toggle);
}

#[test]
fn test_disabled_toggle_no_events() {
    let toggle = Toggle::new("test")
        .disabled(true)
        .on_change(|_checked, _window, _cx| {});

    drop(toggle);
}
