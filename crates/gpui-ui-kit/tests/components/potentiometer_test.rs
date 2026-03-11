//! Potentiometer component tests

use gpui_ui_kit::audio::potentiometer::{Potentiometer, PotentiometerSize};
use gpui_ui_kit::scale::Scale;

#[test]
fn test_potentiometer_creation() {
    let pot = Potentiometer::new("pot-1");
    drop(pot);
}

#[test]
fn test_potentiometer_all_sizes() {
    let sizes = [
        PotentiometerSize::Sm,
        PotentiometerSize::Md,
        PotentiometerSize::Lg,
    ];
    for size in &sizes {
        let pot = Potentiometer::new("pot").size(*size);
        drop(pot);
    }
}

#[test]
fn test_potentiometer_size_default() {
    let size = PotentiometerSize::default();
    assert_eq!(size, PotentiometerSize::Md);
}

#[test]
fn test_potentiometer_configuration() {
    let pot = Potentiometer::new("freq-knob")
        .value(1000.0)
        .min(20.0)
        .max(20000.0)
        .unit("Hz")
        .label("Frequency")
        .shortcut_key('f')
        .size(PotentiometerSize::Lg)
        .scale(Scale::Logarithmic)
        .selected(true)
        .disabled(false);

    drop(pot);
}

#[test]
fn test_potentiometer_linear_scale() {
    let pot = Potentiometer::new("gain")
        .value(0.0)
        .min(-24.0)
        .max(24.0)
        .unit("dB")
        .label("Gain")
        .scale(Scale::Linear);

    drop(pot);
}

#[test]
fn test_potentiometer_disabled() {
    let pot = Potentiometer::new("disabled-pot").disabled(true);
    drop(pot);
}

#[test]
fn test_potentiometer_selected() {
    let pot = Potentiometer::new("sel-pot").selected(true);
    drop(pot);
}

#[test]
fn test_potentiometer_handlers() {
    let pot = Potentiometer::new("pot")
        .on_change(|_val, _window, _cx| {})
        .on_drag_start(|_pos, _val, _window, _cx| {})
        .on_select(|_window, _cx| {})
        .on_reset(|_window, _cx| {});

    drop(pot);
}

#[test]
fn test_scale_variants() {
    let scales = [Scale::Linear, Scale::Logarithmic];
    for scale in &scales {
        let _copy = *scale;
    }
}

#[test]
fn test_scale_default() {
    let scale = Scale::default();
    assert_eq!(scale, Scale::Linear);
}
