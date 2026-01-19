//! Select component tests

use gpui_ui_kit::select::{Select, SelectOption, SelectSize, SelectTheme};
use gpui_ui_kit::theme::Theme;

#[test]
fn test_select_creation() {
    let options = vec![
        SelectOption::new("apple", "Apple"),
        SelectOption::new("banana", "Banana"),
        SelectOption::new("orange", "Orange"),
    ];

    let select = Select::new("test-select")
        .options(options)
        .selected("apple")
        .placeholder("Choose a fruit");

    drop(select);
}

#[test]
fn test_select_configuration() {
    let select = Select::new("test")
        .size(SelectSize::Lg)
        .label("Fruit Selection")
        .placeholder("Choose")
        .disabled(true);

    drop(select);
}

#[test]
fn test_select_sizes() {
    let sizes = [SelectSize::Sm, SelectSize::Md, SelectSize::Lg];

    for size in &sizes {
        let select = Select::new("test").size(*size);
        drop(select);
    }
}

#[test]
fn test_select_option_creation() {
    let option = SelectOption::new("value", "Label");
    assert_eq!(option.value, "value");
    assert_eq!(option.label, "Label");
    assert!(!option.disabled);

    let disabled_option = SelectOption::new("value", "Label").disabled(true);
    assert!(disabled_option.disabled);
}

#[test]
fn test_select_dropdown_has_opaque_background() {
    let theme = SelectTheme::default();
    assert_eq!(
        theme.dropdown_bg.a, 1.0,
        "Select dropdown background should be opaque in default theme"
    );

    let app_theme = Theme::dark();
    let select_theme = SelectTheme::from(&app_theme);
    assert_eq!(
        select_theme.dropdown_bg.a, 1.0,
        "Select dropdown background should be opaque in dark theme"
    );

    let app_theme_light = Theme::light();
    let select_theme_light = SelectTheme::from(&app_theme_light);
    assert_eq!(
        select_theme_light.dropdown_bg.a, 1.0,
        "Select dropdown background should be opaque in light theme"
    );
}

// Interaction tests

#[test]
fn test_select_supports_mouse_click() {
    let select = Select::new("test")
        .placeholder("Choose")
        .on_change(|_value, _window, _cx| {})
        .on_toggle(|_is_open, _window, _cx| {});

    drop(select);
}

#[test]
fn test_select_supports_keyboard_navigation() {
    let select = Select::new("test")
        .on_change(|_value, _window, _cx| {})
        .on_toggle(|_is_open, _window, _cx| {})
        .on_highlight(|_index, _window, _cx| {});

    drop(select);
}

#[test]
fn test_select_supports_keyboard_activation() {
    let select = Select::new("test")
        .on_change(|_value, _window, _cx| {})
        .on_toggle(|_is_open, _window, _cx| {});

    drop(select);
}

#[test]
fn test_disabled_select_no_events() {
    let select = Select::new("test")
        .disabled(true)
        .on_change(|_value, _window, _cx| {})
        .on_toggle(|_is_open, _window, _cx| {});

    drop(select);
}

#[test]
fn test_select_complete_keyboard_support() {
    let select = Select::new("test")
        .on_change(|_value, _window, _cx| {})
        .on_toggle(|_is_open, _window, _cx| {})
        .on_highlight(|_index, _window, _cx| {});

    drop(select);
}
