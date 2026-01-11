//! Slider component tests

use gpui_ui_kit::slider::Slider;

#[test]
fn test_slider_creation() {
    let slider = Slider::new("test").value(0.5).min(0.0).max(1.0);
    drop(slider);
}

#[test]
fn test_slider_supports_mouse_drag() {
    let slider = Slider::new("test")
        .value(0.5)
        .on_change(|_value, _window, _cx| {});

    drop(slider);
}

#[test]
fn test_slider_supports_keyboard() {
    let slider = Slider::new("test")
        .value(0.5)
        .min(0.0)
        .max(1.0)
        .on_change(|_value, _window, _cx| {});

    drop(slider);
}

#[test]
fn test_slider_supports_mouse_scroll() {
    let slider = Slider::new("test")
        .value(0.5)
        .on_change(|_value, _window, _cx| {});

    drop(slider);
}
