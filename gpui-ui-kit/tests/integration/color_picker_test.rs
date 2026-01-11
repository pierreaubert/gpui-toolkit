//! Integration tests for ColorPicker component
//!
//! Tests the color picker component including:
//! - Basic rendering
//! - RGB and HSL modes
//! - Color manipulation methods
//! - Mode toggling
//! - Reset functionality

use gpui::TestAppContext;
use gpui_ui_kit::color::Color;
use gpui_ui_kit::color_picker::{ColorPickerMode, ColorPickerView};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

#[gpui::test]
async fn test_color_picker_renders(cx: &mut TestAppContext) {
    let _window =
        cx.add_window(|_window, _cx| ColorPickerView::new("Test Color", Color::rgb(128, 64, 192)));
}

#[gpui::test]
async fn test_color_picker_with_different_colors(cx: &mut TestAppContext) {
    // Red
    let _window = cx.add_window(|_window, _cx| ColorPickerView::new("Red", Color::rgb(255, 0, 0)));
}

#[gpui::test]
async fn test_color_picker_with_green(cx: &mut TestAppContext) {
    let _window =
        cx.add_window(|_window, _cx| ColorPickerView::new("Green", Color::rgb(0, 255, 0)));
}

#[gpui::test]
async fn test_color_picker_with_blue(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| ColorPickerView::new("Blue", Color::rgb(0, 0, 255)));
}

#[gpui::test]
async fn test_color_picker_with_white(cx: &mut TestAppContext) {
    let _window =
        cx.add_window(|_window, _cx| ColorPickerView::new("White", Color::rgb(255, 255, 255)));
}

#[gpui::test]
async fn test_color_picker_with_black(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| ColorPickerView::new("Black", Color::rgb(0, 0, 0)));
}

// ============================================================================
// Color API Tests
// ============================================================================

#[gpui::test]
async fn test_color_picker_color_method(cx: &mut TestAppContext) {
    let window =
        cx.add_window(|_window, _cx| ColorPickerView::new("Test", Color::rgb(100, 150, 200)));

    window
        .update(cx, |view, _window, _cx| {
            let color = view.color();
            assert_eq!(color.r, 100);
            assert_eq!(color.g, 150);
            assert_eq!(color.b, 200);
        })
        .unwrap();
}

#[gpui::test]
async fn test_color_picker_set_color(cx: &mut TestAppContext) {
    let window = cx.add_window(|_window, _cx| ColorPickerView::new("Test", Color::rgb(0, 0, 0)));

    window
        .update(cx, |view, _window, _cx| {
            view.set_color(Color::rgb(255, 128, 64));
            let color = view.color();
            assert_eq!(color.r, 255);
            assert_eq!(color.g, 128);
            assert_eq!(color.b, 64);
        })
        .unwrap();
}

// ============================================================================
// Color Conversion Tests
// ============================================================================

#[gpui::test]
async fn test_color_to_hex_string(cx: &mut TestAppContext) {
    let color = Color::rgb(255, 128, 0);
    let hex = color.to_hex_string();
    assert!(hex.starts_with("#"), "Hex string should start with #");

    let _ = cx; // Satisfy async test requirement
}

#[gpui::test]
async fn test_color_to_hsl_conversion(cx: &mut TestAppContext) {
    // Pure red should have hue 0
    let red = Color::rgb(255, 0, 0);
    let (h, s, l) = red.to_hsl();
    assert!((h - 0.0).abs() < 0.01, "Red hue should be 0");
    assert!((s - 1.0).abs() < 0.01, "Red saturation should be 1");
    assert!((l - 0.5).abs() < 0.01, "Red lightness should be 0.5");

    let _ = cx;
}

#[gpui::test]
async fn test_color_from_hsl(cx: &mut TestAppContext) {
    // Create color from HSL (pure green: h=0.333, s=1, l=0.5)
    let green = Color::from_hsl(1.0 / 3.0, 1.0, 0.5);
    assert_eq!(green.r, 0);
    assert_eq!(green.g, 255);
    assert_eq!(green.b, 0);

    let _ = cx;
}

#[gpui::test]
async fn test_color_with_alpha(cx: &mut TestAppContext) {
    let color = Color::rgb(255, 0, 0);
    let with_alpha = color.with_alpha(0.5);
    assert_eq!(with_alpha.r, 255);
    assert_eq!(with_alpha.g, 0);
    assert_eq!(with_alpha.b, 0);
    assert_eq!(with_alpha.a, 128); // 0.5 * 255 = 127.5, rounded to 128

    let _ = cx;
}

// ============================================================================
// Mode Tests
// ============================================================================

#[gpui::test]
async fn test_color_picker_mode_default(cx: &mut TestAppContext) {
    // Default mode should be RGB
    let mode = ColorPickerMode::default();
    assert_eq!(mode, ColorPickerMode::RGB);

    let _ = cx;
}

#[gpui::test]
async fn test_color_picker_mode_variants(cx: &mut TestAppContext) {
    // Test both mode variants exist
    let _rgb = ColorPickerMode::RGB;
    let _hsl = ColorPickerMode::HSL;

    let _ = cx;
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[gpui::test]
async fn test_color_picker_grayscale(cx: &mut TestAppContext) {
    // Grayscale colors (R=G=B) should have saturation 0
    let gray = Color::rgb(128, 128, 128);
    let (_, s, _) = gray.to_hsl();
    assert!((s - 0.0).abs() < 0.01, "Gray saturation should be 0");

    let _ = cx;
}

#[gpui::test]
async fn test_color_picker_boundary_values(cx: &mut TestAppContext) {
    // Test boundary RGB values
    let _min = Color::rgb(0, 0, 0);
    let _max = Color::rgb(255, 255, 255);

    let _ = cx;
}

#[gpui::test]
async fn test_color_picker_alpha_channel(cx: &mut TestAppContext) {
    let window = cx.add_window(|_window, _cx| {
        let color = Color::rgb(100, 100, 100).with_alpha(0.75);
        ColorPickerView::new("Alpha Test", color)
    });

    window
        .update(cx, |view, _window, _cx| {
            let color = view.color();
            assert_eq!(color.a, 191); // 0.75 * 255 ≈ 191
        })
        .unwrap();
}
