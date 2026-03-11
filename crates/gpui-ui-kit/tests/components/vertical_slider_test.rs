//! VerticalSlider component tests

use gpui_ui_kit::audio::vertical_slider::{VerticalSlider, VerticalSliderSize};
use gpui_ui_kit::scale::Scale;

#[test]
fn test_vertical_slider_creation() {
    let slider = VerticalSlider::new("slider-1");
    drop(slider);
}

#[test]
fn test_vertical_slider_all_sizes() {
    let sizes = [
        VerticalSliderSize::Sm,
        VerticalSliderSize::Md,
        VerticalSliderSize::Lg,
    ];
    for size in &sizes {
        let slider = VerticalSlider::new("slider").size(*size);
        drop(slider);
    }
}

#[test]
fn test_vertical_slider_size_default() {
    let size = VerticalSliderSize::default();
    assert_eq!(size, VerticalSliderSize::Md);
}

#[test]
fn test_vertical_slider_configuration() {
    let slider = VerticalSlider::new("vol-slider")
        .value(-6.0)
        .min(-60.0)
        .max(12.0)
        .unit("dB")
        .label("Volume")
        .shortcut_key('v')
        .size(VerticalSliderSize::Lg)
        .scale(Scale::Linear)
        .height(200.0)
        .with_ticks()
        .selected(false)
        .disabled(false);

    drop(slider);
}

#[test]
fn test_vertical_slider_with_peak() {
    let slider = VerticalSlider::new("meter")
        .value(-12.0)
        .min(-60.0)
        .max(0.0)
        .peak(Some(-6.0));

    drop(slider);
}

#[test]
fn test_vertical_slider_no_peak() {
    let slider = VerticalSlider::new("meter").peak(None);

    drop(slider);
}

#[test]
fn test_vertical_slider_disabled() {
    let slider = VerticalSlider::new("disabled-slider").disabled(true);
    drop(slider);
}

#[test]
fn test_vertical_slider_selected() {
    let slider = VerticalSlider::new("sel-slider").selected(true);
    drop(slider);
}

#[test]
fn test_vertical_slider_handlers() {
    let slider = VerticalSlider::new("slider")
        .on_change(|_val, _window, _cx| {})
        .on_drag_start(|_pos, _val, _window, _cx| {})
        .on_select(|_window, _cx| {})
        .on_reset(|_window, _cx| {});

    drop(slider);
}

#[test]
fn test_vertical_slider_log_scale() {
    let slider = VerticalSlider::new("freq-slider")
        .value(1000.0)
        .min(20.0)
        .max(20000.0)
        .scale(Scale::Logarithmic);

    drop(slider);
}
