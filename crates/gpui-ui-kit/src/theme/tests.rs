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

/// Linux keeps the classic look: `Dark`/`Light` values are frozen.
#[test]
fn classic_dark_light_values_are_frozen() {
    let dark = Theme::dark();
    assert_eq!(dark.background, gpui::rgb(0x1e1e1e));
    assert_eq!(dark.surface, gpui::rgb(0x2a2a2a));
    assert_eq!(dark.surface_hover, gpui::rgb(0x3a3a3a));
    assert_eq!(dark.muted, gpui::rgb(0x252525));
    assert_eq!(dark.text_primary, gpui::rgb(0xffffff));
    assert_eq!(dark.text_secondary, gpui::rgb(0xcccccc));
    assert_eq!(dark.text_muted, gpui::rgb(0x9c9c9c));
    assert_eq!(dark.text_on_accent, gpui::rgb(0xffffff));
    assert_eq!(dark.accent, gpui::rgb(0x007acc));
    assert_eq!(dark.accent_hover, gpui::rgb(0x0098ff));
    assert_eq!(dark.accent_muted, gpui::rgba(0x007acc33));
    assert_eq!(dark.success, gpui::rgb(0x22c55e));
    assert_eq!(dark.warning, gpui::rgb(0xf59e0b));
    assert_eq!(dark.error, gpui::rgb(0xf87171));
    assert_eq!(dark.info, gpui::rgb(0x3b82f6));
    assert_eq!(dark.border, gpui::rgb(0x3a3a3a));
    assert_eq!(dark.border_hover, gpui::rgb(0x555555));
    assert_eq!(dark.font_family.as_ref(), ".SystemUI");

    let light = Theme::light();
    assert_eq!(light.background, gpui::rgb(0xf5f5f5));
    assert_eq!(light.surface, gpui::rgb(0xffffff));
    assert_eq!(light.surface_hover, gpui::rgb(0xf0f0f0));
    assert_eq!(light.muted, gpui::rgb(0xeeeeee));
    assert_eq!(light.text_primary, gpui::rgb(0x1a1a1a));
    assert_eq!(light.text_secondary, gpui::rgb(0x4a4a4a));
    assert_eq!(light.text_muted, gpui::rgb(0x666666));
    assert_eq!(light.text_on_accent, gpui::rgb(0xffffff));
    assert_eq!(light.accent, gpui::rgb(0x0066cc));
    assert_eq!(light.accent_hover, gpui::rgb(0x0055aa));
    assert_eq!(light.accent_muted, gpui::rgba(0x0066cc22));
    assert_eq!(light.success, gpui::rgb(0x15803d));
    assert_eq!(light.warning, gpui::rgb(0xb45309));
    assert_eq!(light.error, gpui::rgb(0xb91c1c));
    assert_eq!(light.info, gpui::rgb(0x1d4ed8));
    assert_eq!(light.border, gpui::rgb(0xd4d4d4));
    assert_eq!(light.border_hover, gpui::rgb(0xaaaaaa));
    assert_eq!(light.font_family.as_ref(), ".SystemUI");
}

#[test]
fn platform_themes_use_native_anchors() {
    let macos_dark = Theme::for_variant(ThemeVariant::MacosDark);
    assert_eq!(macos_dark.background, gpui::rgb(0x1e1e1e));
    assert_eq!(macos_dark.accent, gpui::rgb(0x007aff));
    assert_eq!(macos_dark.success, gpui::rgb(0x30d158));
    assert_eq!(macos_dark.error, gpui::rgb(0xff4245));
    assert!(ThemeVariant::MacosDark.is_dark());

    let macos_light = Theme::for_variant(ThemeVariant::MacosLight);
    assert_eq!(macos_light.background, gpui::rgb(0xffffff));
    assert_eq!(macos_light.accent, gpui::rgb(0x007aff));
    assert_eq!(macos_light.success, gpui::rgb(0x34c759));
    assert_eq!(macos_light.error, gpui::rgb(0xff383c));
    assert!(!ThemeVariant::MacosLight.is_dark());

    let fluent_dark = Theme::for_variant(ThemeVariant::FluentDark);
    assert_eq!(fluent_dark.background, gpui::rgb(0x202020));
    assert_eq!(fluent_dark.accent, gpui::rgb(0x60cdff));
    assert_eq!(fluent_dark.text_on_accent, gpui::rgb(0x000000));
    assert_eq!(fluent_dark.font_family.as_ref(), "Segoe UI Variable");
    assert!(ThemeVariant::FluentDark.is_dark());

    let fluent_light = Theme::for_variant(ThemeVariant::FluentLight);
    assert_eq!(fluent_light.background, gpui::rgb(0xf3f3f3));
    assert_eq!(fluent_light.accent, gpui::rgb(0x0067c0));
    assert_eq!(fluent_light.text_on_accent, gpui::rgb(0xffffff));
    assert_eq!(fluent_light.font_family.as_ref(), "Segoe UI Variable");
    assert!(!ThemeVariant::FluentLight.is_dark());
}

#[test]
#[cfg(target_os = "macos")]
fn platform_appearance_selects_macos_family() {
    assert_eq!(
        ThemeVariant::for_platform_appearance(true),
        ThemeVariant::MacosDark
    );
    assert_eq!(
        ThemeVariant::for_platform_appearance(false),
        ThemeVariant::MacosLight
    );
}

#[test]
#[cfg(target_os = "windows")]
fn platform_appearance_selects_fluent_family() {
    assert_eq!(
        ThemeVariant::for_platform_appearance(true),
        ThemeVariant::FluentDark
    );
    assert_eq!(
        ThemeVariant::for_platform_appearance(false),
        ThemeVariant::FluentLight
    );
}

#[test]
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_appearance_selects_classic_family() {
    assert_eq!(
        ThemeVariant::for_platform_appearance(true),
        ThemeVariant::Dark
    );
    assert_eq!(
        ThemeVariant::for_platform_appearance(false),
        ThemeVariant::Light
    );
}

#[test]
fn window_appearance_selects_dark_or_light_side() {
    use gpui::WindowAppearance;
    let dark_sides = [WindowAppearance::Dark, WindowAppearance::VibrantDark];
    let light_sides = [WindowAppearance::Light, WindowAppearance::VibrantLight];
    for appearance in dark_sides {
        assert!(ThemeVariant::for_window_appearance(appearance).is_dark());
    }
    for appearance in light_sides {
        assert!(!ThemeVariant::for_window_appearance(appearance).is_dark());
    }
}

#[test]
fn system_appearance_change_stays_within_family() {
    assert_eq!(
        ThemeVariant::MacosDark.with_system_appearance(false),
        ThemeVariant::MacosLight
    );
    assert_eq!(
        ThemeVariant::MacosLight.with_system_appearance(true),
        ThemeVariant::MacosDark
    );
    assert_eq!(
        ThemeVariant::FluentDark.with_system_appearance(false),
        ThemeVariant::FluentLight
    );
    assert_eq!(
        ThemeVariant::Dark.with_system_appearance(false),
        ThemeVariant::Light
    );
    assert_eq!(
        ThemeVariant::Midnight.with_system_appearance(false),
        ThemeVariant::Midnight
    );
}

#[test]
fn follow_system_applies_live_appearance_only_when_enabled() {
    let mut state = ThemeState::with_variant(ThemeVariant::MacosDark);
    assert!(!state.follow_system);
    state.apply_system_appearance(false);
    assert_eq!(state.theme.variant, ThemeVariant::MacosDark);

    state.set_follow_system(true);
    state.apply_system_appearance(false);
    assert_eq!(state.theme.variant, ThemeVariant::MacosLight);
    state.apply_system_appearance(true);
    assert_eq!(state.theme.variant, ThemeVariant::MacosDark);
}
