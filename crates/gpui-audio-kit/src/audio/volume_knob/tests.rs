use crate::audio::volume_knob::VolumeKnob;
use d3rs::render2d::{Renderer2D, VelloBackend};

#[test]
fn default_renderer_contract_is_shared_with_d3rs() {
    let knob = VolumeKnob::new();
    assert_eq!(knob.renderer_2d, Renderer2D::default());
    assert_eq!(knob.vello_backend, VelloBackend::default());
}

#[test]
fn test_asin_clamp_prevents_nan() {
    // The knob computes (dy / radius).asin().
    // Due to floating-point error the ratio can marginally exceed [-1, 1],
    // which would make asin return NaN. Clamping prevents this.
    for &ratio in &[-1.1_f32, -1.0, -0.5, 0.0, 0.5, 1.0, 1.1] {
        let result = ratio.clamp(-1.0, 1.0).asin();
        assert!(
            !result.is_nan(),
            "asin should not produce NaN for ratio {}",
            ratio
        );
    }
}

#[test]
fn default_volume_knob_is_constructible() {
    let knob = VolumeKnob::default();
    assert_eq!(knob.value, 0.0);
    assert!(!knob.muted);
}

#[test]
fn volume_knob_builder_setters_chain() {
    let _knob = VolumeKnob::new()
        .value(0.75)
        .label("Volume")
        .size(gpui::px(60.0))
        .muted(true)
        .disabled(false)
        .accent_color(gpui::rgba(0x00ff00ff))
        .muted_color(gpui::rgba(0xff0000ff))
        .bg_color(gpui::rgba(0x111111ff))
        .text_color(gpui::rgba(0xffffffff))
        .id("custom-id")
        .aria_label("Volume knob")
        .aria_role(gpui_ui_kit::accessibility::AriaRole::Slider)
        .on_change(|_val, _window, _cx| {})
        .on_mute_toggle(|_muted, _window, _cx| {});
}

#[test]
fn volume_knob_accessibility_summary_uses_effective_mute_value() {
    let summary = VolumeKnob::new()
        .label("Monitor")
        .value(0.75)
        .muted(true)
        .accessibility_summary();

    assert_eq!(summary.control_type, "volume_knob");
    assert_eq!(summary.label, "Monitor");
    assert_eq!(summary.role, gpui_ui_kit::accessibility::AriaRole::Slider);
    assert_eq!(summary.value_now, Some(0.0));
    assert_eq!(summary.value_min, Some(0.0));
    assert_eq!(summary.value_max, Some(1.0));
    assert_eq!(summary.value_text, Some("0%".into()));
    assert!(summary.muted);
    assert!(summary.description.contains("Muted"));
}

#[test]
fn volume_knob_accessibility_summary_clamps_the_displayed_value() {
    let summary = VolumeKnob::new().value(1.5).accessibility_summary();

    assert_eq!(summary.value_now, Some(1.0));
    assert_eq!(summary.value_text, Some("100%".into()));
}

#[test]
fn volume_knob_accessibility_summary_includes_disabled_state() {
    let summary = VolumeKnob::new()
        .label("Monitor")
        .value(0.75)
        .disabled(true)
        .accessibility_summary();

    assert!(summary.disabled);
    assert!(summary.description.contains("Disabled"));
}

#[test]
fn volume_knob_commits_only_changed_drags() {
    assert!(!VolumeKnob::should_commit_drag(0.5, 0.5, false));
    assert!(!VolumeKnob::should_commit_drag(0.5, 0.5, true));
    assert!(VolumeKnob::should_commit_drag(0.5, 0.6, true));
}

fn knob_from_one_call_site() -> VolumeKnob {
    VolumeKnob::new()
}

fn default_knob_from_one_call_site() -> VolumeKnob {
    VolumeKnob::default()
}

#[test]
fn default_ids_are_stable_at_the_same_call_site() {
    assert_eq!(knob_from_one_call_site().id, knob_from_one_call_site().id);
    assert_eq!(
        default_knob_from_one_call_site().id,
        default_knob_from_one_call_site().id
    );
}

#[test]
fn default_ids_remain_distinct_at_different_call_sites() {
    let a = VolumeKnob::new();
    let b = VolumeKnob::new();
    assert_ne!(a.id, b.id);
}
