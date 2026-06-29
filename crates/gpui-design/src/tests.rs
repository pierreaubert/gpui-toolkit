use super::animation_rules::AnimationRules;
use super::audio_control_rules::AudioControlRules;
use super::corner_radii::CornerRadii;
use super::design_conformance_matrix::DesignConformanceMatrix;
use super::design_language::DesignLanguage;
use super::design_platform::DesignPlatform;
use super::design_system::DesignSystem;
use super::design_system::all_design_presets;
use super::design_token_export::DesignTokenExport;
use super::elevation_rules::ElevationRules;
use super::layout_thresholds::LayoutThresholds;
use super::spacing_rules::SpacingRules;
use super::types::CornerRadiusStyle;
use super::types::GroupSeparatorStyle;
use super::typography_rules::TypographyRules;

#[test]
fn test_neutral_matches_current_solver_constants() {
    let ds = DesignSystem::neutral();
    // These must match the constants in layout_solver.rs exactly
    assert_eq!(ds.layout.vertical_threshold, 400.0);
    assert_eq!(ds.layout.group_stack_threshold, 500.0);
    assert_eq!(ds.layout.compact_slider_threshold, 700.0);
    assert_eq!(ds.layout.hide_viz_threshold, 600.0);
    assert_eq!(ds.layout.compact_knob_threshold, 400.0);
    assert_eq!(ds.layout.large_knob_threshold, 800.0);
    assert_eq!(ds.layout.slider_height_normal, 180.0);
    assert_eq!(ds.layout.slider_height_compact, 120.0);
}

#[test]
fn test_all_presets_construct() {
    let _ = DesignSystem::neutral();
    let _ = DesignSystem::apple_hig();
    let _ = DesignSystem::material3();
    let _ = DesignSystem::fluent();
    let _ = DesignSystem::adwaita();
    let _ = DesignSystem::breeze();
    let _ = DesignSystem::carbon();
}

#[test]
fn test_platform_default_returns_valid() {
    let ds = DesignSystem::platform_default();
    // Should be one of the known languages
    assert!(matches!(
        ds.language,
        DesignLanguage::AppleHig
            | DesignLanguage::Material3
            | DesignLanguage::Fluent
            | DesignLanguage::Adwaita
            | DesignLanguage::Breeze
            | DesignLanguage::Carbon
            | DesignLanguage::Neutral
    ));
}

#[test]
fn test_platform_mapping_covers_main_os_families() {
    assert_eq!(
        DesignSystem::for_platform(DesignPlatform::Macos).language,
        DesignLanguage::AppleHig
    );
    assert_eq!(
        DesignSystem::for_platform(DesignPlatform::Ios).language,
        DesignLanguage::AppleHig
    );
    assert_eq!(
        DesignSystem::for_platform(DesignPlatform::Windows).language,
        DesignLanguage::Fluent
    );
    assert_eq!(
        DesignSystem::for_platform(DesignPlatform::Android).language,
        DesignLanguage::Material3
    );
    assert_eq!(
        DesignSystem::for_platform(DesignPlatform::Linux).language,
        DesignLanguage::Adwaita
    );
    assert_eq!(
        DesignSystem::for_platform(DesignPlatform::Other).language,
        DesignLanguage::Neutral
    );
}

#[test]
fn test_design_language_ids_select_presets() {
    for language in [
        DesignLanguage::AppleHig,
        DesignLanguage::Material3,
        DesignLanguage::Fluent,
        DesignLanguage::Adwaita,
        DesignLanguage::Breeze,
        DesignLanguage::Carbon,
        DesignLanguage::Neutral,
    ] {
        assert_eq!(DesignLanguage::from_id(language.as_str()), Some(language));
        assert_eq!(
            DesignSystem::from_language_id(language.as_str())
                .unwrap()
                .language,
            language
        );
        assert!(!language.label().is_empty());
    }

    assert_eq!(DesignLanguage::from_id("system"), None);
    assert_eq!(DesignSystem::from_language_id("system"), None);
    assert_eq!(
        DesignLanguage::from_id("gtk"),
        Some(DesignLanguage::Adwaita)
    );
    assert_eq!(
        DesignLanguage::from_id("gnome"),
        Some(DesignLanguage::Adwaita)
    );
    assert_eq!(DesignLanguage::from_id("kde"), Some(DesignLanguage::Breeze));
}

#[test]
fn test_presets_use_origin_typography() {
    for (preset_id, system) in all_design_presets() {
        let expected = match system.language {
            DesignLanguage::AppleHig | DesignLanguage::Material3 | DesignLanguage::Neutral => {
                ".SystemUIFont"
            }
            DesignLanguage::Fluent => "Segoe UI Variable",
            DesignLanguage::Adwaita => "Adwaita Sans",
            DesignLanguage::Breeze => "Noto Sans",
            DesignLanguage::Carbon => "IBM Plex Sans",
        };
        assert_eq!(system.typography.font_family, expected, "{preset_id}");
    }
}

#[test]
fn test_presets_differ() {
    let neutral = DesignSystem::neutral();
    let apple = DesignSystem::apple_hig();
    let material = DesignSystem::material3();
    let fluent = DesignSystem::fluent();
    let adwaita = DesignSystem::adwaita();
    let breeze = DesignSystem::breeze();
    let carbon = DesignSystem::carbon();

    // Each preset should have a different language
    assert_ne!(neutral.language, apple.language);
    assert_ne!(apple.language, material.language);
    assert_ne!(material.language, fluent.language);
    assert_ne!(fluent.language, adwaita.language);
    assert_ne!(adwaita.language, breeze.language);
    assert_ne!(breeze.language, carbon.language);

    // Key differentiators
    assert_ne!(neutral.toggle_variant, material.toggle_variant);
    assert_ne!(apple.corners.style, material.corners.style);
    assert_ne!(neutral.label_position, fluent.label_position);
    assert_ne!(neutral.group_separator, material.group_separator);
    assert_ne!(adwaita.corners.md, breeze.corners.md);
    assert_eq!(carbon.corners.md, 0.0);
}

#[test]
fn test_apple_uses_larger_touch_targets() {
    let apple = DesignSystem::apple_hig();
    let neutral = DesignSystem::neutral();
    assert!(apple.interaction.min_touch_target > neutral.interaction.min_touch_target);
    assert_eq!(apple.group_separator, GroupSeparatorStyle::None);
    assert!(apple.corners.md > neutral.corners.md);
}

#[test]
fn test_material_uses_cards() {
    let material = DesignSystem::material3();
    assert_eq!(material.group_separator, GroupSeparatorStyle::Card);
}

#[test]
fn test_fluent_is_compact() {
    let fluent = DesignSystem::fluent();
    let neutral = DesignSystem::neutral();
    assert!(fluent.spacing.section_gap <= neutral.spacing.section_gap);
    assert!(fluent.layout.slider_height_normal < neutral.layout.slider_height_normal);
    assert_eq!(fluent.corners.md, 4.0);
    assert_eq!(fluent.corners.lg, 8.0);
    assert_eq!(fluent.typography.font_family, "Segoe UI Variable");
}

#[test]
fn test_linux_presets_follow_adwaita_and_breeze_conventions() {
    let adwaita = DesignSystem::adwaita();
    let breeze = DesignSystem::breeze();

    assert_eq!(adwaita.typography.font_family, "Adwaita Sans");
    assert_eq!(breeze.typography.font_family, "Noto Sans");
    assert!(adwaita.corners.md > breeze.corners.md);
    assert!(adwaita.interaction.min_touch_target > breeze.interaction.min_touch_target);
    assert_eq!(adwaita.group_separator, GroupSeparatorStyle::Divider);
    assert_eq!(breeze.group_separator, GroupSeparatorStyle::Border);
}

#[test]
fn test_carbon_is_square_and_flat() {
    let carbon = DesignSystem::carbon();
    assert_eq!(carbon.corners.sm, 0.0);
    assert_eq!(carbon.corners.md, 0.0);
    assert_eq!(carbon.spacing.grid_unit, 2.0);
    assert_eq!(carbon.elevation.level_1_blur, 0.0);
    assert!(!carbon.animation.prefer_spring);
}

#[test]
fn test_serializable() {
    let ds = DesignSystem::neutral();
    let json = serde_json::to_string(&ds).unwrap();
    assert!(json.contains("\"language\":\"Neutral\""));
    assert!(json.contains("\"vertical_threshold\":400.0"));
}

#[test]
fn test_design_language_as_str() {
    assert_eq!(DesignLanguage::AppleHig.as_str(), "apple_hig");
    assert_eq!(DesignLanguage::Material3.as_str(), "material3");
    assert_eq!(DesignLanguage::Fluent.as_str(), "fluent");
    assert_eq!(DesignLanguage::Adwaita.as_str(), "adwaita");
    assert_eq!(DesignLanguage::Breeze.as_str(), "breeze");
    assert_eq!(DesignLanguage::Carbon.as_str(), "carbon");
    assert_eq!(DesignLanguage::Neutral.as_str(), "neutral");
}

#[test]
fn style_dictionary_tokens_include_platform_and_motion() {
    let ds = DesignSystem::apple_hig();
    let tokens = ds.style_dictionary_tokens();

    assert!(tokens.iter().any(|token| token.name() == "design.language"));
    assert!(
        tokens
            .iter()
            .any(|token| token.name() == "motion.duration_ms")
    );
    assert!(
        tokens
            .iter()
            .any(|token| token.name() == "interaction.min_touch_target")
    );
}

#[test]
fn style_dictionary_tokens_are_cached_and_borrowable() {
    let ds = DesignSystem::neutral();

    let owned = ds.style_dictionary_tokens();
    let borrowed = ds.style_dictionary_tokens_ref();

    assert_eq!(owned.len(), borrowed.len());
    assert!(
        owned
            .iter()
            .zip(borrowed)
            .all(|(a, b)| a.name() == b.name())
    );

    // A second call should return the same cached slice.
    assert_eq!(ds.style_dictionary_tokens_ref().as_ptr(), borrowed.as_ptr());
}

#[test]
fn design_token_name_is_precomputed() {
    let ds = DesignSystem::neutral();
    let tokens = ds.style_dictionary_tokens();
    let token = tokens
        .iter()
        .find(|t| t.path == vec!["spacing", "grid_unit"])
        .unwrap();

    assert_eq!(token.name(), "spacing.grid_unit");
    assert_eq!(token.name(), token.path.join("."));
}

#[test]
fn conformance_and_motion_reports_are_stable() {
    let ds = DesignSystem::apple_hig();

    assert!(ds.conformance_report(false).passed());
    assert_eq!(ds.motion_spec(true).duration_ms, 0);
    assert_eq!(ds.motion_spec(false).duration_ms, ds.animation.duration_ms);
}

#[test]
fn conformance_matrix_covers_all_presets_and_motion_modes() {
    let matrix = DesignConformanceMatrix::all_presets();

    assert_eq!(matrix.cases.len(), all_design_presets().len() * 2);
    assert!(matrix.passed(), "{}", matrix.to_markdown_table());
    assert!(
        matrix
            .cases
            .iter()
            .any(|case| case.preset_id == "apple_hig" && case.reduced_motion)
    );
    assert!(
        matrix
            .to_markdown_table()
            .contains("| apple_hig | reduced |")
    );
}

#[test]
fn conformance_report_catches_mutated_public_fields() {
    let mut ds = DesignSystem::neutral();
    ds.spacing.grid_unit = 0.0;
    ds.elevation.shadow_opacity = 1.5;
    ds.typography.large_size = ds.typography.base_size - 1.0;
    ds.animation.fast_ms = ds.animation.slow_ms + 1;

    let report = ds.conformance_report(false);
    let ids: Vec<_> = report.findings.iter().map(|finding| finding.id).collect();

    assert!(ids.contains(&"spacing.grid_unit"));
    assert!(ids.contains(&"elevation.shadow_opacity"));
    assert!(ids.contains(&"typography.scale"));
    assert!(ids.contains(&"motion.duration_order"));
}

#[test]
fn style_dictionary_export_is_serializable_and_complete() {
    let export = DesignTokenExport::for_all_presets();
    let json = serde_json::to_string(&export).unwrap();

    assert!(json.contains("\"apple_hig\""));
    assert!(json.contains("\"interaction.min_touch_target\""));
    assert!(json.contains("\"audio.knob_arc_sweep_deg\""));
}

#[test]
fn test_corner_radii_new_validates() {
    let c = CornerRadii::new(4.0, 8.0, 12.0, 16.0, CornerRadiusStyle::Circular);
    assert_eq!(c.sm, 4.0);
}

#[test]
#[should_panic(expected = "sm must be >= 0")]
fn test_corner_radii_new_rejects_negative() {
    CornerRadii::new(-1.0, 8.0, 12.0, 16.0, CornerRadiusStyle::Circular);
}

#[test]
fn test_spacing_rules_new_validates() {
    let s = SpacingRules::new(4.0, 12.0, 8.0, 8.0, 16.0, 12.0);
    assert_eq!(s.grid_unit, 4.0);
}

#[test]
#[should_panic(expected = "grid_unit must be >= 0")]
fn test_spacing_rules_new_rejects_negative() {
    SpacingRules::new(-1.0, 12.0, 8.0, 8.0, 16.0, 12.0);
}

#[test]
fn test_elevation_rules_new_validates_opacity() {
    let e = ElevationRules::new(0.0, 4.0, 16.0, 0.15, 2.0);
    assert_eq!(e.shadow_opacity, 0.15);
}

#[test]
#[should_panic(expected = "shadow_opacity must be in [0, 1]")]
fn test_elevation_rules_new_rejects_invalid_opacity() {
    ElevationRules::new(0.0, 4.0, 16.0, 1.5, 2.0);
}

#[test]
fn test_animation_rules_new_validates() {
    let a = AnimationRules::new(200, 100, 400, false, 170.0, 26.0);
    assert_eq!(a.duration_ms, 200);
}

#[test]
#[should_panic(expected = "duration_ms must be > 0")]
fn test_animation_rules_new_rejects_zero_duration() {
    AnimationRules::new(0, 100, 400, false, 170.0, 26.0);
}

#[test]
fn test_typography_rules_new_validates() {
    let t = TypographyRules::new(".SystemUIFont", false, 14.0, 11.0, 18.0);
    assert_eq!(t.base_size, 14.0);
}

#[test]
#[should_panic(expected = "base_size must be > 0")]
fn test_typography_rules_new_rejects_zero_size() {
    TypographyRules::new(".SystemUIFont", false, 0.0, 11.0, 18.0);
}

#[test]
fn test_layout_thresholds_new_validates() {
    let l = LayoutThresholds::new(400.0, 500.0, 700.0, 600.0, 400.0, 800.0, 180.0, 120.0);
    assert_eq!(l.vertical_threshold, 400.0);
}

#[test]
#[should_panic(expected = "vertical_threshold must be > 0")]
fn test_layout_thresholds_new_rejects_zero() {
    LayoutThresholds::new(0.0, 500.0, 700.0, 600.0, 400.0, 800.0, 180.0, 120.0);
}

#[test]
fn test_audio_control_rules_new_validates() {
    let a = AudioControlRules::new(135.0, 270.0, 2.5, 48, 2.0, [14.0, 18.0, 24.0]);
    assert_eq!(a.knob_arc_segments, 48);
}

#[test]
#[should_panic(expected = "knob_arc_segments must be > 0")]
fn test_audio_control_rules_new_rejects_zero_segments() {
    AudioControlRules::new(135.0, 270.0, 2.5, 0, 2.0, [14.0, 18.0, 24.0]);
}
