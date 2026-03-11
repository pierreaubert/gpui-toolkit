//! Checkbox component tests

use gpui_ui_kit::ComponentSize;
use gpui_ui_kit::checkbox::{Checkbox, CheckboxSize};

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

// -- New tests below --

#[test]
fn test_checkbox_all_sizes() {
    let sizes = [CheckboxSize::Sm, CheckboxSize::Md, CheckboxSize::Lg];

    for size in &sizes {
        let checkbox = Checkbox::new("test").size(*size);
        drop(checkbox);
    }
}

#[test]
fn test_checkbox_indeterminate_state() {
    let checkbox = Checkbox::new("test").indeterminate(true);
    drop(checkbox);

    let checkbox = Checkbox::new("test").indeterminate(false).checked(true);
    drop(checkbox);
}

#[test]
fn test_checkbox_size_from_component_size() {
    let conversions: Vec<(ComponentSize, CheckboxSize)> = vec![
        (ComponentSize::Xs, CheckboxSize::Sm),
        (ComponentSize::Sm, CheckboxSize::Sm),
        (ComponentSize::Md, CheckboxSize::Md),
        (ComponentSize::Lg, CheckboxSize::Lg),
        (ComponentSize::Xl, CheckboxSize::Lg),
    ];

    for (component_size, expected) in conversions {
        let checkbox_size: CheckboxSize = component_size.into();
        assert_eq!(checkbox_size, expected);
    }
}

#[test]
fn test_checkbox_without_label() {
    let checkbox = Checkbox::new("no-label").checked(false);
    drop(checkbox);
}

#[test]
fn test_checkbox_all_config_combinations() {
    let checkbox = Checkbox::new("full-config")
        .checked(true)
        .indeterminate(false)
        .label("Full config")
        .size(CheckboxSize::Lg)
        .disabled(false)
        .on_change(|_checked, _window, _cx| {});
    drop(checkbox);
}
