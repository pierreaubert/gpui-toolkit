//! Proves the annotated theme template compiles and stays complete.
//!
//! Including the template proves the documented customization builds
//! against the real crates; the key drift test fails when `EditorTheme`
//! gains, loses, or renames a field; the coverage test fails when a
//! field has no `theme.<field>` home in the template.

#[path = "../templates/theme_custom.rs"]
mod theme_template_inc;

use gpui_themes::EditorTheme;
use gpui_toolkit_cli::template_show;
use theme_template_inc::custom_theme;

/// Every `EditorTheme` field the template must document.
const EXPECTED_KEYS: &[&str] = &[
    "name",
    "background",
    "background_secondary",
    "background_tertiary",
    "surface",
    "surface_hover",
    "surface_selected",
    "text_primary",
    "text_secondary",
    "text_muted",
    "text_disabled",
    "border",
    "border_focused",
    "accent",
    "accent_hover",
    "accent_muted",
    "text_on_accent",
    "text_on_accent_muted",
    "success",
    "warning",
    "error",
    "info",
    "meter_normal",
    "meter_warning",
    "meter_clip",
    "button_mute_active",
    "button_solo_active",
    "button_dim_active",
    "progress_bar_bg",
    "progress_bar_fill",
    "toast_success_bg",
    "toast_error_bg",
    "toast_info_bg",
    "toast_warning_bg",
    "plugin_colors",
    "graph_colors",
    "band_colors",
    "eq_curve_colors",
    "spectrum_colors",
    "meter_colors",
    "peak_indicator",
    "drag_over_highlight",
    "drag_over_border",
    "neutral_indicator",
    "warning_background",
    "knob_color",
    "optimization_color",
    "grid_color",
    "separator_size",
    "font_family",
    "design_language",
];

#[test]
fn theme_template_constructs_and_validates() {
    let theme = custom_theme();
    assert_eq!(theme.name, "Custom Dark");
    theme.validate_accessibility().unwrap();
}

#[test]
fn editor_theme_keys_match_template_contract() {
    let value = serde_json::to_value(EditorTheme::default()).unwrap();
    let mut keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    let mut expected: Vec<&str> = EXPECTED_KEYS.to_vec();
    expected.sort_unstable();
    assert_eq!(keys, expected);
}

#[test]
fn theme_template_documents_every_key() {
    let shown = template_show("theme-custom").unwrap();
    assert_eq!(shown.kind, "theme");
    let missing: Vec<&&str> = EXPECTED_KEYS
        .iter()
        .filter(|key| !mentions_field(&shown.source, key))
        .collect();
    assert!(missing.is_empty(), "undocumented theme fields: {missing:?}");
}

/// Whether `theme.<key>` appears with an identifier boundary after it.
///
/// The boundary keeps `theme.border_focused` from covering
/// `theme.border`: only a standalone mention counts.
fn mentions_field(source: &str, key: &str) -> bool {
    let needle = format!("theme.{key}");
    let mut start = 0;
    while let Some(pos) = source[start..].find(&needle) {
        let end = start + pos + needle.len();
        let boundary = source
            .as_bytes()
            .get(end)
            .is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_');
        if boundary {
            return true;
        }
        start = end;
    }
    false
}
