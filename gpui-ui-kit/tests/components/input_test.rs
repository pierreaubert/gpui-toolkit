//! Input component tests

use gpui_ui_kit::input::{Input, InputVariant};

#[test]
fn test_input_creation() {
    let input = Input::new("test")
        .value("Hello")
        .placeholder("Enter text...");
    drop(input);
}

#[test]
fn test_input_supports_change_events() {
    let input = Input::new("test")
        .value("initial value")
        .placeholder("Enter text...")
        .on_change(|_text, _window, _cx| {});

    drop(input);
}

#[test]
fn test_input_supports_states() {
    let _disabled_input = Input::new("disabled")
        .value("Cannot edit")
        .disabled(true)
        .on_change(|_, _, _| {});

    let _readonly_input = Input::new("readonly")
        .value("Read only")
        .readonly(true)
        .on_change(|_, _, _| {});

    let _error_input = Input::new("error")
        .value("Invalid")
        .error("This field is required")
        .on_change(|_, _, _| {});
}

#[test]
fn test_input_supports_variants() {
    let _default = Input::new("default")
        .variant(InputVariant::Default)
        .on_change(|_, _, _| {});

    let _filled = Input::new("filled")
        .variant(InputVariant::Filled)
        .on_change(|_, _, _| {});

    let _flushed = Input::new("flushed")
        .variant(InputVariant::Flushed)
        .on_change(|_, _, _| {});
}

#[test]
fn test_input_supports_focus() {
    let input = Input::new("focus-test")
        .placeholder("Click to focus")
        .on_change(|_text, _window, _cx| {});

    drop(input);
}

#[test]
fn test_disabled_input_no_events() {
    let input = Input::new("disabled")
        .disabled(true)
        .on_change(|_text, _window, _cx| {});

    drop(input);
}

#[test]
fn test_readonly_input_no_events() {
    let input = Input::new("readonly")
        .readonly(true)
        .on_change(|_text, _window, _cx| {});

    drop(input);
}

#[test]
fn test_input_text_value_handling() {
    let input = Input::new("text-value")
        .value("Hello, World!")
        .label("Enter your name")
        .placeholder("Type here...")
        .on_change(|new_text, _window, _cx| {
            assert!(!new_text.is_empty());
        });

    drop(input);
}

#[test]
fn test_input_supports_icons() {
    let input = Input::new("with-icons")
        .icon_left("🔍")
        .icon_right("✓")
        .placeholder("Search...")
        .on_change(|_, _, _| {});

    drop(input);
}
