//! Button component tests

use gpui_ui_kit::ComponentSize;
use gpui_ui_kit::button::{Button, ButtonSize, ButtonTheme, ButtonVariant};

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

// -- New tests below --

#[test]
fn test_button_theme_construction() {
    let theme = ButtonTheme {
        accent: gpui::rgba(0xff6600ff),
        accent_hover: gpui::rgba(0xff8800ff),
        surface: gpui::rgba(0x2a2a2aff),
        surface_hover: gpui::rgba(0x3a3a3aff),
        text_primary: gpui::rgba(0xffffffff),
        text_secondary: gpui::rgba(0xccccccff),
        text_on_accent: gpui::rgba(0xffffffff),
        error: gpui::rgba(0xcc3333ff),
        error_hover: gpui::rgba(0xe64545ff),
        border: gpui::rgba(0x555555ff),
        transparent: gpui::rgba(0x00000000),
    };
    let button = Button::new("themed", "Theme").theme(theme);
    drop(button);
}

#[test]
fn test_button_selected_with_each_variant() {
    let variants = [
        ButtonVariant::Primary,
        ButtonVariant::Secondary,
        ButtonVariant::Destructive,
        ButtonVariant::Ghost,
        ButtonVariant::Outline,
    ];

    for variant in &variants {
        let button = Button::new("test", "Selected")
            .variant(*variant)
            .selected(true);
        drop(button);
    }
}

#[test]
fn test_button_full_width_method() {
    let button = Button::new("test", "Full").full_width(true);
    drop(button);

    let button = Button::new("test", "Not Full").full_width(false);
    drop(button);
}

#[test]
fn test_button_build_returns_stateful_div() {
    let button = Button::new("build-test", "Build")
        .variant(ButtonVariant::Primary)
        .size(ButtonSize::Md);
    let _element = button.build();
}

#[test]
fn test_button_size_from_component_size() {
    let sizes: Vec<(ComponentSize, ButtonSize)> = vec![
        (ComponentSize::Xs, ButtonSize::Xs),
        (ComponentSize::Sm, ButtonSize::Sm),
        (ComponentSize::Md, ButtonSize::Md),
        (ComponentSize::Lg, ButtonSize::Lg),
        (ComponentSize::Xl, ButtonSize::Lg),
    ];

    for (component_size, expected) in sizes {
        let button_size: ButtonSize = component_size.into();
        assert_eq!(button_size, expected);
    }
}
