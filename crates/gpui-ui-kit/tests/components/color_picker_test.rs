//! ColorPicker component tests

use gpui_ui_kit::color::Color;
use gpui_ui_kit::color_picker::{ColorPickerMode, ColorPickerView};

#[test]
fn test_color_picker_mode_variants() {
    let modes = [ColorPickerMode::RGB, ColorPickerMode::HSL];
    for mode in &modes {
        // Verify all variants are accessible and Copy
        let _copy = *mode;
    }
}

#[test]
fn test_color_picker_mode_default() {
    let mode = ColorPickerMode::default();
    assert_eq!(mode, ColorPickerMode::RGB);
}

#[test]
fn test_color_picker_view_creation() {
    let color = Color::rgb(128, 64, 200);
    let view = ColorPickerView::new("Test Picker", color);
    assert_eq!(view.color(), color);
}

#[test]
fn test_color_picker_view_set_color() {
    let color1 = Color::rgb(100, 100, 100);
    let color2 = Color::rgb(200, 50, 25);
    let mut view = ColorPickerView::new("Picker", color1);
    assert_eq!(view.color(), color1);

    view.set_color(color2);
    assert_eq!(view.color(), color2);
}

#[test]
fn test_color_picker_view_with_alpha() {
    let color = Color::new(255, 0, 128, 200);
    let view = ColorPickerView::new("Alpha Picker", color);
    assert_eq!(view.color(), color);
}
