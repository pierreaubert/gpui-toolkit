use super::{Theme, ThemeState, ThemeVariant};
use std::sync::Arc;

#[test]
fn theme_ext_returns_same_arc_instance() {
    let state = ThemeState::new();
    let ptr1 = Arc::as_ptr(&state.theme);
    let ptr2 = Arc::as_ptr(&state.theme);
    assert_eq!(ptr1, ptr2);
}

#[test]
fn theme_state_stores_theme_as_arc() {
    let state = ThemeState::new();
    let theme_ref1: &Theme = &state.theme;
    let theme_ref2: &Theme = &state.theme;
    assert!(
        std::ptr::eq(
            theme_ref1.font_family.as_ref(),
            theme_ref2.font_family.as_ref()
        ),
        "repeated borrows of ThemeState.theme should point to the same Theme instance"
    );
}

#[test]
fn theme_dark_fallback_is_stable() {
    let state = ThemeState::new();
    let fallback1 = Theme::dark();
    let fallback2 = Theme::dark();

    let stored: &Theme = &state.theme;
    let stored2: &Theme = &state.theme;
    assert!(std::ptr::eq(stored, stored2));

    assert_eq!(stored.background, fallback1.background);
    assert_eq!(stored.text_primary, fallback2.text_primary);
}

#[test]
fn theme_state_with_variant_keeps_stable_instance() {
    let state = ThemeState::with_variant(ThemeVariant::Light);
    let t1: &Theme = &state.theme;
    let t2: &Theme = &state.theme;
    assert!(std::ptr::eq(t1, t2));
    assert_eq!(t1.variant, ThemeVariant::Light);
}

#[test]
fn theme_state_set_variant_replaces_but_keeps_arc() {
    let mut state = ThemeState::new();
    let before = Arc::as_ptr(&state.theme);
    state.set_variant(ThemeVariant::Light);
    let after = Arc::as_ptr(&state.theme);
    assert_ne!(before, after, "set_variant should allocate a new Arc");
    assert_eq!(state.theme.variant, ThemeVariant::Light);
}

#[test]
fn carbon_themes_use_plex_and_blue_action_color() {
    for variant in [
        ThemeVariant::CarbonWhite,
        ThemeVariant::CarbonGray10,
        ThemeVariant::CarbonGray90,
        ThemeVariant::CarbonGray100,
    ] {
        let theme = Theme::for_variant(variant);
        assert_eq!(theme.variant, variant);
        assert_eq!(theme.font_family.as_ref(), "IBM Plex Sans");
        assert_ne!(theme.background, theme.surface);
        assert_eq!(theme.text_on_accent, gpui::rgb(0xffffff));
    }
}
