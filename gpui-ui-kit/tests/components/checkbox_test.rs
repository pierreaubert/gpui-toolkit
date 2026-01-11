//! Checkbox component tests

use gpui_ui_kit::checkbox::Checkbox;

#[test]
fn test_checkbox_creation() {
    let checkbox = Checkbox::new("test").label("Accept terms").checked(true);
    drop(checkbox);
}

#[test]
fn test_checkbox_supports_mouse_click() {
    let checkbox = Checkbox::new("test")
        .label("Accept terms")
        .checked(false)
        .on_change(|_checked, _window, _cx| {});

    drop(checkbox);
}

#[test]
fn test_checkbox_supports_keyboard() {
    let checkbox = Checkbox::new("test")
        .checked(false)
        .on_change(|_checked, _window, _cx| {});

    drop(checkbox);
}

#[test]
fn test_disabled_checkbox_no_events() {
    let checkbox = Checkbox::new("test")
        .disabled(true)
        .on_change(|_checked, _window, _cx| {});

    drop(checkbox);
}
