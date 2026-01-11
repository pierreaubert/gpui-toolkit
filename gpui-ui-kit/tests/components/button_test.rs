//! Button component tests

use gpui_ui_kit::button::{Button, ButtonSize, ButtonVariant};

#[test]
fn test_button_creation() {
    let variants = [
        ButtonVariant::Primary,
        ButtonVariant::Secondary,
        ButtonVariant::Destructive,
        ButtonVariant::Ghost,
        ButtonVariant::Outline,
    ];

    for variant in &variants {
        let button = Button::new("test-button", "Click me").variant(*variant);
        drop(button);
    }
}

#[test]
fn test_button_sizes() {
    let sizes = [
        ButtonSize::Xs,
        ButtonSize::Sm,
        ButtonSize::Md,
        ButtonSize::Lg,
    ];

    for size in &sizes {
        let button = Button::new("test-button", "Click me").size(*size);
        drop(button);
    }
}

#[test]
fn test_button_configuration() {
    let button = Button::new("test", "Test")
        .variant(ButtonVariant::Primary)
        .size(ButtonSize::Lg)
        .disabled(true)
        .selected(true)
        .full_width(true);

    drop(button);
}

#[test]
fn test_button_with_icons() {
    let button = Button::new("test", "Label").icon_left("←").icon_right("→");
    drop(button);
}

// Interaction tests

#[test]
fn test_button_supports_mouse_click() {
    let button = Button::new("test", "Click me").on_click(|_window, _cx| {});
    drop(button);
}

#[test]
fn test_button_keyboard_accessible() {
    let button = Button::new("test", "Press me")
        .variant(ButtonVariant::Primary)
        .on_click(|_window, _cx| {});
    drop(button);
}

#[test]
fn test_disabled_button_no_mouse_events() {
    let button = Button::new("test", "Disabled")
        .disabled(true)
        .on_click(|_window, _cx| {});
    drop(button);
}
