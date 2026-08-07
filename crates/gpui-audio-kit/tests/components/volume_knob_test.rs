//! VolumeKnob component tests

use gpui_audio_kit::audio::volume_knob::VolumeKnob;

#[test]
fn test_volume_knob_creation() {
    let knob = VolumeKnob::new();
    drop(knob);
}

#[test]
fn test_volume_knob_configuration() {
    let knob = VolumeKnob::new()
        .id("main-vol")
        .value(0.75)
        .label("Master")
        .size(gpui::px(48.0))
        .muted(false);

    drop(knob);
}

#[test]
fn test_volume_knob_muted() {
    let knob = VolumeKnob::new().id("muted-vol").value(0.5).muted(true);

    drop(knob);
}

#[test]
fn test_volume_knob_custom_colors() {
    let knob = VolumeKnob::new()
        .id("themed-vol")
        .accent_color(gpui::rgba(0x00FF00FF))
        .muted_color(gpui::rgba(0xFF0000FF))
        .bg_color(gpui::rgba(0x333333FF))
        .text_color(gpui::rgba(0xFFFFFFFF));

    drop(knob);
}

#[test]
fn test_volume_knob_handlers() {
    let knob = VolumeKnob::new()
        .id("vol")
        .on_change(|_val, _window, _cx| {})
        .on_commit(|_val, _window, _cx| {})
        .on_mute_toggle(|_muted, _window, _cx| {});

    drop(knob);
}

#[test]
fn test_volume_knob_zero_volume() {
    let knob = VolumeKnob::new().id("zero-vol").value(0.0);

    drop(knob);
}

#[test]
fn test_volume_knob_max_volume() {
    let knob = VolumeKnob::new().id("max-vol").value(1.0);

    drop(knob);
}

#[test]
fn test_volume_knob_custom_size() {
    let knob = VolumeKnob::new().id("large-vol").size(gpui::px(96.0));

    drop(knob);
}
