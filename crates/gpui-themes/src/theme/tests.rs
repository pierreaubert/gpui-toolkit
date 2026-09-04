use super::accent_palette::AccentPalette;
use super::accessibility_palette::AccessibilityPalette;
use super::built_in_theme_preset::BuiltInThemePreset;
use super::community_theme_bundle::CommunityThemeBundle;
use super::community_theme_manifest::CommunityThemeManifest;
use super::editor_theme::EditorTheme;
use super::misc::COMMUNITY_THEME_SCHEMA_VERSION;
use super::misc::contrast_ratio;
use super::theme_gallery::ThemeGallery;
use super::theme_mode_preference::ThemeModePreference;
use super::theme_schedule::ThemeSchedule;
use super::theme_transition::ThemeTransition;
use super::time_of_day::TimeOfDay;
use super::tui_theme_preset::TuiThemePreset;
use super::types::AccentSource;
use super::types::ThemeAppearance;
pub use gpui_ui_kit::Color;

#[test]
fn community_bundle_replaces_a_gallery_entry_with_the_same_id() {
    let theme = EditorTheme::dark();
    let mut manifest = CommunityThemeManifest::for_theme(&theme);
    manifest.id = "nord".into();
    manifest.display_name = "Community Nord".into();
    let bundle = CommunityThemeBundle::new(manifest, theme);

    let gallery = ThemeGallery::from_built_ins().with_community_bundle(&bundle);
    let entries = gallery
        .entries
        .iter()
        .filter(|entry| entry.id == "nord")
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].display_name, "Community Nord");
}
#[test]
fn rust_export_escapes_metadata_and_preserves_separator_precision() {
    let mut theme = EditorTheme::dark();
    theme.name = "123 \\\"generated\\\"\\nname".into();
    theme.font_family = "Font \\\"Family\\\"".into();
    theme.design_language = "custom\\nmode".into();
    theme.separator_size = 20.25;

    let code = theme.to_rust_code();
    let function_name = code
        .lines()
        .find_map(|line| line.trim().strip_prefix("pub fn "))
        .and_then(|line| line.split('(').next())
        .expect("generated Rust must contain a function declaration");
    assert!(
        function_name
            .chars()
            .enumerate()
            .all(|(index, character)| character == '_'
                || character.is_ascii_alphanumeric()
                    && (index > 0 || character.is_ascii_alphabetic()))
    );
    assert!(code.contains(&format!("name: {:?}.to_string()", theme.name)));
    assert!(code.contains(&format!("font_family: {:?}.to_string()", theme.font_family)));
    assert!(code.contains(&format!(
        "design_language: {:?}.to_string()",
        theme.design_language
    )));
    assert!(code.contains("separator_size: 20.25,"));

    theme.separator_size = f32::NAN;
    assert!(theme.to_rust_code().contains("separator_size: f32::NAN,"));
}

#[test]
fn time_of_day_deserialization_rejects_invalid_ranges() {
    assert!(serde_json::from_str::<TimeOfDay>(r#"{"hour": 24, "minute": 0}"#).is_err());
    assert!(serde_json::from_str::<TimeOfDay>(r#"{"hour": 0, "minute": 60}"#).is_err());
    assert_eq!(
        serde_json::from_str::<TimeOfDay>(r#"{"hour": 23, "minute": 59}"#).unwrap(),
        TimeOfDay::new(23, 59)
    );
}

#[test]
fn terminal_presets_convert_to_accessible_editor_themes() {
    for preset in TuiThemePreset::all() {
        preset
            .palette()
            .to_editor_theme()
            .validate_accessibility()
            .unwrap_or_else(|error| panic!("{}: {error}", preset.name()));
    }
}
fn assert_rgba_eq(actual: gpui::Rgba, expected: gpui::Rgba) {
    assert!((actual.r - expected.r).abs() <= f32::EPSILON);
    assert!((actual.g - expected.g).abs() <= f32::EPSILON);
    assert!((actual.b - expected.b).abs() <= f32::EPSILON);
    assert!((actual.a - expected.a).abs() <= f32::EPSILON);
}

#[test]
fn test_theme_json_roundtrip() {
    let theme = EditorTheme::dark();
    let json = theme.to_json().unwrap();
    let loaded = EditorTheme::from_json(&json).unwrap();
    assert_eq!(loaded.name, theme.name);
    assert_eq!(loaded.background.r, theme.background.r);
}

#[test]
fn test_to_rust_code_includes_nested_structs() {
    let theme = EditorTheme::dark();
    let code = theme.to_rust_code();
    // Should contain nested struct initializations, not abbreviated comment.
    assert!(
        code.contains("PluginColors {"),
        "Rust code should include PluginColors initialization"
    );
    assert!(
        code.contains("GraphColors {"),
        "Rust code should include GraphColors initialization"
    );
    assert!(
        code.contains("EQCurveColors {"),
        "Rust code should include EQCurveColors initialization"
    );
    assert!(
        code.contains("SpectrumColors {"),
        "Rust code should include SpectrumColors initialization"
    );
    assert!(
        code.contains("MeterColors {"),
        "Rust code should include MeterColors initialization"
    );
    assert!(
        !code.contains("plugin_colors, graph_colors, etc."),
        "Rust code should not contain abbreviated placeholder"
    );
}

#[test]
fn test_to_rust_code_band_colors_are_indented() {
    let theme = EditorTheme::dark();
    let code = theme.to_rust_code();
    // Each band color should appear on its own indented line inside the vec![...].
    assert!(
        code.contains(",\n            Color::from_hex"),
        "Band colors should be formatted on separate indented lines"
    );
    assert!(
        code.contains("band_colors: vec!["),
        "Rust code should include band_colors vec initialization"
    );
}

#[test]
fn test_to_accordion_theme_maps_accent_fields() {
    let theme = EditorTheme::dark();
    let accordion = theme.to_accordion_theme();

    assert_rgba_eq(accordion.header_active_bg, theme.accent_muted.to_rgba());
    assert_rgba_eq(accordion.accent_tint, theme.accent_muted.to_rgba());
    assert_rgba_eq(accordion.accent, theme.accent.to_rgba());
}

#[test]
fn test_validate_band_colors_non_empty() {
    let mut theme = EditorTheme::dark();
    assert!(theme.validate().is_ok());
    theme.band_colors.clear();
    assert!(theme.validate().is_err());
}

#[test]
fn test_theme_schedule_resolves_day_and_night() {
    let schedule = ThemeSchedule::new(TimeOfDay::new(7, 30), TimeOfDay::new(18, 0));

    assert_eq!(schedule.resolve_at_minutes(8 * 60), ThemeAppearance::Light);
    assert_eq!(schedule.resolve_at_minutes(20 * 60), ThemeAppearance::Dark);
    assert_eq!(
        ThemeModePreference::Scheduled { schedule }.resolve(ThemeAppearance::Light, 23 * 60),
        ThemeAppearance::Dark
    );
}

#[test]
fn test_theme_schedule_supports_wraparound_light_period() {
    let schedule = ThemeSchedule::new(TimeOfDay::new(18, 0), TimeOfDay::new(7, 0));

    assert_eq!(schedule.resolve_at_minutes(23 * 60), ThemeAppearance::Light);
    assert_eq!(schedule.resolve_at_minutes(12 * 60), ThemeAppearance::Dark);
}

#[test]
fn test_accent_palette_applies_readable_text() {
    let seed = Color::from_hex(0xf0e442);
    let palette = AccentPalette::from_seed(seed, AccentSource::System, ThemeAppearance::Dark);
    let theme = EditorTheme::dark().with_accent_palette(palette);

    assert_eq!(theme.accent, seed);
    assert_eq!(theme.border_focused, seed);
    assert!(contrast_ratio(theme.text_on_accent, theme.accent) >= 4.5);
}

#[test]
fn test_color_blind_presets_are_accessible() {
    for preset in [
        BuiltInThemePreset::Protanopia,
        BuiltInThemePreset::Deuteranopia,
        BuiltInThemePreset::Tritanopia,
    ] {
        let theme = preset.to_theme();
        assert!(preset.accessibility().is_color_blind_safe());
        assert!(
            theme.validate_accessibility().is_ok(),
            "{} should meet core contrast requirements",
            preset.name()
        );
    }
}

#[test]
fn test_community_theme_bundle_roundtrip() {
    let mut manifest = CommunityThemeManifest::for_theme(&EditorTheme::dracula());
    manifest.author = "SOTF".to_string();
    manifest.tags = vec!["community".to_string(), "dark".to_string()];
    manifest.accessibility = AccessibilityPalette::Standard;

    let bundle = CommunityThemeBundle::new(manifest, EditorTheme::dracula());
    let json = bundle.to_json().unwrap();
    let loaded = CommunityThemeBundle::from_json(&json).unwrap();

    assert_eq!(
        loaded.manifest.schema_version,
        COMMUNITY_THEME_SCHEMA_VERSION
    );
    assert_eq!(loaded.manifest.id, "dracula");
    assert_eq!(loaded.manifest.author, "SOTF");
    assert_eq!(loaded.theme.name, "Dracula");
    assert!(loaded.validate().is_ok());
}

#[test]
fn test_transition_respects_reduce_motion() {
    let transition = ThemeTransition::default();

    assert_eq!(transition.effective_duration_ms(false), 220);
    assert_eq!(transition.effective_duration_ms(true), 0);
    assert_eq!(ThemeTransition::disabled().effective_duration_ms(false), 0);
}

#[test]
fn test_builtin_preset_lookup_accepts_friendly_ids() {
    assert_eq!(
        BuiltInThemePreset::from_id("High Contrast"),
        Some(BuiltInThemePreset::HighContrast)
    );
    assert_eq!(BuiltInThemePreset::from_id("tokyo-night"), None);
}

#[test]
fn test_tui_presets_have_full_ansi_palettes() {
    for preset in TuiThemePreset::all() {
        let palette = preset.palette();
        assert_eq!(palette.ansi.len(), 16);
        assert_eq!(palette.name, preset.name());
        assert_eq!(palette.to_editor_theme().name, preset.name());
    }
}

#[test]
fn test_theme_gallery_contains_builtins_and_community() {
    let mut manifest = CommunityThemeManifest::for_theme(&EditorTheme::nord());
    manifest.tags = vec!["cool".to_string()];
    let bundle = CommunityThemeBundle::new(manifest, EditorTheme::nord());

    let gallery = ThemeGallery::from_built_ins().with_community_bundle(&bundle);

    assert!(
        gallery
            .entries
            .iter()
            .any(|entry| entry.id == "high_contrast"
                && entry.accessibility == AccessibilityPalette::HighContrast)
    );
    assert!(gallery.entries.iter().any(|entry| entry.id == "nord"));
}

#[test]
fn test_default_functions() {
    assert_eq!(
        super::default::default_community_theme_schema_version(),
        COMMUNITY_THEME_SCHEMA_VERSION
    );
    assert_eq!(super::default::default_design_language(), "neutral");
}

#[test]
fn test_accessibility_palette() {
    assert_eq!(AccessibilityPalette::all().len(), 5);
    assert_eq!(AccessibilityPalette::HighContrast.name(), "High Contrast");
    assert!(AccessibilityPalette::Protanopia.is_color_blind_safe());
    assert!(!AccessibilityPalette::Standard.is_color_blind_safe());
}

#[test]
fn test_time_of_day() {
    assert_eq!(TimeOfDay::new(7, 30).minutes_after_midnight(), 450);
    assert_eq!(TimeOfDay::checked_new(23, 59), Some(TimeOfDay::new(23, 59)));
    assert_eq!(TimeOfDay::checked_new(24, 0), None);
    assert_eq!(TimeOfDay::checked_new(0, 60), None);
}

#[test]
fn test_theme_schedule_boundaries() {
    let schedule = ThemeSchedule::new(TimeOfDay::new(7, 0), TimeOfDay::new(18, 0));
    assert_eq!(schedule.resolve_at_minutes(7 * 60), ThemeAppearance::Light);
    assert_eq!(
        schedule.resolve_at_minutes(18 * 60 - 1),
        ThemeAppearance::Light
    );
    assert_eq!(schedule.resolve_at_minutes(18 * 60), ThemeAppearance::Dark);
    assert_eq!(schedule.resolve_at_minutes(6 * 60), ThemeAppearance::Dark);

    let equal = ThemeSchedule::new(TimeOfDay::new(12, 0), TimeOfDay::new(12, 0));
    assert_eq!(equal.resolve_at_minutes(12 * 60), ThemeAppearance::Dark);

    let wrap = ThemeSchedule::new(TimeOfDay::new(18, 0), TimeOfDay::new(7, 0));
    assert_eq!(wrap.resolve_at_minutes(25 * 60), ThemeAppearance::Light);
    assert_eq!(wrap.resolve_at_minutes(12 * 60), ThemeAppearance::Dark);
}

#[test]
fn test_theme_mode_preference_resolve() {
    assert_eq!(
        ThemeModePreference::FollowSystem.resolve(ThemeAppearance::Light, 0),
        ThemeAppearance::Light
    );
    assert_eq!(
        ThemeModePreference::Dark.resolve(ThemeAppearance::Light, 0),
        ThemeAppearance::Dark
    );
    let scheduled = ThemeSchedule::new(TimeOfDay::new(7, 0), TimeOfDay::new(18, 0));
    assert_eq!(
        ThemeModePreference::Scheduled {
            schedule: scheduled
        }
        .resolve(ThemeAppearance::Light, 20 * 60),
        ThemeAppearance::Dark
    );
}

#[test]
fn test_community_theme_manifest_validation_errors() {
    let mut manifest = CommunityThemeManifest::for_theme(&EditorTheme::dark());
    manifest.schema_version = 999;
    assert!(manifest.validate().is_err());

    let mut manifest = CommunityThemeManifest::for_theme(&EditorTheme::dark());
    manifest.id = "   ".to_string();
    assert!(manifest.validate().is_err());

    let mut manifest = CommunityThemeManifest::for_theme(&EditorTheme::dark());
    manifest.display_name = "".to_string();
    assert!(manifest.validate().is_err());
}

#[test]
fn test_community_theme_bundle_accepts_implicit_v1_schema_version() {
    let bundle = CommunityThemeBundle::from_theme(EditorTheme::nord());
    let mut value = serde_json::to_value(&bundle).unwrap();
    value
        .get_mut("manifest")
        .and_then(serde_json::Value::as_object_mut)
        .unwrap()
        .remove("schema_version");

    let json = serde_json::to_string(&value).unwrap();
    let loaded = CommunityThemeBundle::from_json(&json).unwrap();

    assert_eq!(
        loaded.manifest.schema_version,
        COMMUNITY_THEME_SCHEMA_VERSION
    );
    assert!(loaded.validate().is_ok());
}

#[test]
fn test_community_theme_bundle_from_theme() {
    let theme = EditorTheme::nord();
    let bundle = CommunityThemeBundle::from_theme(theme.clone());
    assert_eq!(bundle.theme.name, theme.name);
    assert!(!bundle.manifest.id.is_empty());
    assert!(bundle.validate().is_ok());
}

#[test]
fn test_misc_helpers() {
    use super::misc::{
        normalize_theme_id, readable_text_color, shift_lightness, slugify_theme_name,
    };
    assert_eq!(normalize_theme_id("My Theme!"), "my_theme");
    assert_eq!(slugify_theme_name("My Theme!"), "my-theme");
    assert_eq!(slugify_theme_name("!!!"), "custom-theme");
    let black = Color::from_hex(0x000000);
    let white = Color::from_hex(0xffffff);
    assert!(contrast_ratio(white, black) > 20.0);
    assert_eq!(readable_text_color(black), white);
    assert_eq!(readable_text_color(white), black);
    let shifted = shift_lightness(white, -1.0);
    assert_eq!(shifted, Color::from_hsl(0.0, 0.0, 0.0));
}

#[test]
fn test_theme_transition() {
    let default = ThemeTransition::default();
    assert_eq!(default.effective_duration_ms(false), 220);
    assert_eq!(default.effective_duration_ms(true), 0);
    assert_eq!(ThemeTransition::disabled().effective_duration_ms(false), 0);
}

#[test]
fn test_editor_theme_appearance_and_accessibility() {
    let dark = EditorTheme::dark();
    assert_eq!(dark.appearance(), ThemeAppearance::Dark);
    let light = EditorTheme::light();
    assert_eq!(light.appearance(), ThemeAppearance::Light);

    let protanopia = EditorTheme::accessibility_preset(AccessibilityPalette::Protanopia);
    assert!(protanopia.validate_accessibility().is_ok());

    let preset = EditorTheme::preset(BuiltInThemePreset::Nord);
    assert_eq!(preset.name, "Nord");
}

#[test]
fn test_editor_theme_conversions() {
    let theme = EditorTheme::dark();
    let _ = theme.to_button_theme();
    let _ = theme.to_slider_theme();
    let _ = theme.to_tabs_theme();
    let _ = theme.to_accordion_theme();
    let _ = theme.to_community_json().unwrap();
}

#[test]
fn test_editor_theme_validate_accessibility_fails_for_low_contrast() {
    let mut theme = EditorTheme::dark();
    theme.text_primary = theme.background;
    assert!(theme.validate_accessibility().is_err());
}

#[test]
fn test_editor_theme_with_accent_seed() {
    let theme =
        EditorTheme::light().with_accent_seed(Color::from_hex(0xf0e442), AccentSource::User);
    assert_eq!(theme.accent, Color::from_hex(0xf0e442));
}

#[test]
fn test_color_field_registry_is_cached_and_unique() {
    use super::color_group::ColorGroup;
    use crate::editor::color_field::{ColorField, all_color_fields};

    let first = all_color_fields();
    let second = all_color_fields();
    assert!(std::ptr::eq(first, second));
    assert!(!first.is_empty());

    let probe = ColorField::new(
        ColorGroup::Base,
        "Probe",
        |t| t.background,
        |t, c| {
            t.background = c;
        },
    );
    assert_eq!(probe.name, "Probe");
    assert_eq!(
        (probe.getter)(&EditorTheme::dark()),
        EditorTheme::dark().background
    );

    let mut names = std::collections::HashSet::new();
    for field in first {
        assert!(
            names.insert((field.group, field.name)),
            "duplicate field {:?}",
            field.name
        );
    }
}

#[test]
fn test_schedule_resolve_wraps_out_of_range_minutes() {
    use super::theme_schedule::ThemeSchedule;
    let schedule = ThemeSchedule::new(TimeOfDay::new(7, 0), TimeOfDay::new(18, 0));
    assert_eq!(schedule.resolve_at_minutes(24 * 60), ThemeAppearance::Dark);
    assert_eq!(
        schedule.resolve_at_minutes(u16::MAX),
        schedule.resolve_at_minutes(u16::MAX % (24 * 60))
    );
}

#[test]
fn test_rust_export_formats_alpha_and_non_finite_separator() {
    let mut theme = EditorTheme::dark();
    theme.text_on_accent_muted = Color::new(1, 2, 3, 128);
    let code = theme.to_rust_code();
    assert!(code.contains("Color::new(1, 2, 3, 128)"));
    assert!(code.contains("Color::from_hex("));

    theme.separator_size = f32::INFINITY;
    assert!(
        theme
            .to_rust_code()
            .contains("separator_size: f32::INFINITY,")
    );
    theme.separator_size = f32::NEG_INFINITY;
    assert!(
        theme
            .to_rust_code()
            .contains("separator_size: f32::NEG_INFINITY,")
    );
}

#[test]
fn test_token_aliases_resolve_and_reject_unknown() {
    use super::token_export::token_aliases;
    let theme = EditorTheme::dark();
    assert!(!token_aliases().is_empty());
    for (alias, key) in token_aliases() {
        assert_eq!(
            theme.resolve_token_alias(alias),
            theme.named_color(key),
            "alias {alias} must match field {key}"
        );
    }
    assert_eq!(
        theme.resolve_token_alias("palette.primary.main"),
        Some(theme.accent)
    );
    assert_eq!(theme.resolve_token_alias("palette.nope.nope"), None);
    assert_eq!(theme.named_color("bogus"), None);
}

#[test]
fn test_css_variables_export_covers_core_and_aliases() {
    let theme = EditorTheme::dark();
    let css = theme.to_css_variables();
    assert!(css.starts_with(":root {"));
    assert!(css.ends_with("}\n"));
    assert!(css.contains(&format!(
        "--color-accent: {};",
        theme.accent.to_hex_string()
    )));
    assert!(css.contains(&format!(
        "--palette-primary-main: {};",
        theme.accent.to_hex_string()
    )));
    assert!(css.contains("--palette-text-primary:"));
}

#[test]
fn test_style_dictionary_tokens_round_trip() {
    let theme = EditorTheme::nord();
    let tokens = theme.style_dictionary_tokens();
    assert!(!tokens.is_empty());
    let names: Vec<&str> = tokens.iter().map(|t| t.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);

    let accent = tokens
        .iter()
        .find(|t| t.name == "color.accent")
        .expect("accent token");
    assert_eq!(accent.path, vec!["color".to_string(), "accent".to_string()]);
    assert_eq!(accent.value, theme.accent.to_hex_string());
    assert_eq!(accent.token_type, "color");

    let json = theme.to_style_dictionary_json().unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let colors = value.get("color").and_then(|c| c.as_array()).unwrap();
    assert_eq!(colors.len(), tokens.len());
    for entry in colors {
        let name = entry.get("name").and_then(|n| n.as_str()).unwrap();
        let token = tokens.iter().find(|t| t.name == name).unwrap();
        assert_eq!(
            entry.get("value").and_then(|v| v.as_str()).unwrap(),
            token.value
        );
        assert_eq!(
            entry.get("type").and_then(|v| v.as_str()).unwrap(),
            token.token_type
        );
    }
    let back: Vec<super::token_export::ThemeDesignToken> =
        serde_json::from_value(serde_json::json!(tokens)).unwrap();
    assert_eq!(back, tokens);
}

#[test]
fn test_contrast_auto_fix_repairs_broken_theme() {
    use super::contrast_fix::{WCAG_AA_MIN_RATIO, nearest_passing_color};
    use super::misc::contrast_ratio;

    let mut broken = EditorTheme::dark();
    broken.text_primary = broken.background;
    broken.text_on_accent = broken.accent;
    assert!(broken.validate_accessibility().is_err());

    let issues = broken.accessibility_issues();
    assert!(!issues.is_empty());
    for issue in &issues {
        assert!(issue.ratio < WCAG_AA_MIN_RATIO);
        let (fg, bg) = match issue.pair {
            "text_on_accent/accent" => (broken.text_on_accent, broken.accent),
            _ => (broken.text_primary, broken.background),
        };
        assert!(contrast_ratio(issue.suggested, bg) >= WCAG_AA_MIN_RATIO);
        assert_eq!(
            broken.suggested_contrast_fix(issue.pair),
            Some(issue.suggested)
        );
        let _ = fg;
    }
    assert_eq!(broken.suggested_contrast_fix("bogus/pair"), None);
    assert!(broken.accessibility_badge().contains("issue"));

    let fixed = broken.clone().auto_fix_contrast();
    assert!(fixed.validate_accessibility().is_ok());
    assert_eq!(fixed.accessibility_badge(), "WCAG AA: pass");
    assert!(fixed.accessibility_issues().is_empty());

    let ok = EditorTheme::dark();
    assert!(ok.accessibility_issues().is_empty());
    assert_eq!(
        nearest_passing_color(ok.text_primary, ok.background),
        Some(ok.text_primary)
    );
}

#[test]
fn test_os_appearance_wiring_point() {
    use super::theme_mode_preference::ThemeModePreference;
    assert_eq!(
        ThemeAppearance::from_system_dark_flag(true),
        ThemeAppearance::Dark
    );
    assert_eq!(
        ThemeAppearance::from_system_dark_flag(false),
        ThemeAppearance::Light
    );
    assert!(ThemeModePreference::FollowSystem.follows_system());
    assert!(!ThemeModePreference::Dark.follows_system());
    assert_eq!(
        ThemeModePreference::FollowSystem.resolve_live(true, 12 * 60),
        ThemeAppearance::Dark
    );
    assert_eq!(
        ThemeModePreference::FollowSystem.resolve_live(false, 0),
        ThemeAppearance::Light
    );
    assert_eq!(
        ThemeModePreference::Light.resolve_live(true, 0),
        ThemeAppearance::Light
    );
    let scheduled = ThemeModePreference::Scheduled {
        schedule: ThemeSchedule::new(TimeOfDay::new(7, 0), TimeOfDay::new(18, 0)),
    };
    assert_eq!(
        scheduled.resolve_live(true, 12 * 60),
        ThemeAppearance::Light
    );
}

#[test]
fn test_transition_animation_gate_and_preview_progress() {
    use super::types::ThemeTransitionEasing;
    let transition = ThemeTransition::default();
    assert!(transition.is_animated(false));
    assert!(!transition.is_animated(true));
    assert!(!ThemeTransition::disabled().is_animated(false));

    assert_eq!(transition.preview_progress(0, false), 0.0);
    assert_eq!(transition.preview_progress(u16::MAX, false), 1.0);
    assert_eq!(transition.preview_progress(110, true), 1.0);

    let linear = ThemeTransition {
        duration_ms: 100,
        easing: ThemeTransitionEasing::Linear,
        cross_fade: true,
    };
    assert!((linear.preview_progress(50, false) - 0.5).abs() < 1e-6);
}
