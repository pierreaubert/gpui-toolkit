//! Theme tests

use gpui_ui_kit::theme::Theme;

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
