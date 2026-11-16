//! Theme tests

use gpui_ui_kit::button::ButtonTheme;
use gpui_ui_kit::theme::{Theme, ThemeVariant};

#[test]
fn test_every_theme_focus_ring_contrasts_with_accent_fills() {
    for variant in ThemeVariant::all() {
        let theme = Theme::for_variant(*variant);

        assert_ne!(
            theme.focus_ring, theme.accent,
            "{variant:?}: focus ring must be visible on a Primary fill"
        );
        assert_ne!(
            theme.focus_ring, theme.error,
            "{variant:?}: focus ring must be visible on a Destructive fill"
        );
        assert_eq!(
            ButtonTheme::from(&theme).focus_ring,
            theme.focus_ring,
            "{variant:?}: ButtonTheme must consume the global focus ring token"
        );
    }
}

#[test]
fn test_theme_creation() {
    let dark = Theme::dark();
    let light = Theme::light();

    // Themes should have different backgrounds
    assert_ne!(dark.background, light.background);

    // Dark theme should have darker background (lower luminance)
    let dark_lum = dark.background.r + dark.background.g + dark.background.b;
    let light_lum = light.background.r + light.background.g + light.background.b;
    assert!(
        dark_lum < light_lum,
        "Dark theme should be darker than light theme"
    );
}
