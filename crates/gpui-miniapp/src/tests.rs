use super::mini_app::{MiniApp, QUIT_KEYSTROKE, TOGGLE_THEME_KEYSTROKE};
use super::mini_app_config::MiniAppConfig;
use super::mini_app_shell::MiniAppShell;
use super::misc::current_platform;
use super::misc::decode_query_component;
use gpui::{Menu, MenuItem, px, size};
use gpui_design::DesignLanguage;
use gpui_ui_kit::i18n::Language;
use gpui_ui_kit::theme::ThemeVariant;

// ========================================================================
// Basic Configuration Tests
// ========================================================================

#[test]
fn keyboard_shortcuts_use_platform_secondary_modifier() {
    assert_eq!(QUIT_KEYSTROKE, "secondary-q");
    assert_eq!(TOGGLE_THEME_KEYSTROKE, "secondary-t");
}

#[test]
fn query_components_percent_decode_utf8_and_reject_malformed_escapes() {
    assert_eq!(
        decode_query_component("stories%20with%20spaces%20%F0%9F%8C%8D"),
        Some("stories with spaces 🌍".to_owned())
    );
    assert_eq!(
        decode_query_component("plus+is+form+encoded"),
        Some("plus is form encoded".to_owned())
    );
    assert_eq!(decode_query_component("bad%2"), None);
    assert_eq!(decode_query_component("bad%zz"), None);
}

fn find_submenu<'a>(menu: &'a Menu, name: &str) -> &'a Menu {
    menu.items
        .iter()
        .find_map(|item| match item {
            MenuItem::Submenu(submenu) if submenu.name.as_ref() == name => Some(submenu),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{name} submenu should exist"))
}

fn action_names(items: &[MenuItem]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|item| match item {
            MenuItem::Action { name, .. } => Some(name.as_ref()),
            _ => None,
        })
        .collect()
}

fn checked_action_names(items: &[MenuItem]) -> Vec<&str> {
    items
        .iter()
        .filter(|item| item.is_checked())
        .filter_map(|item| match item {
            MenuItem::Action { name, .. } => Some(name.as_ref()),
            _ => None,
        })
        .collect()
}

#[test]
fn test_config_new() {
    let config = MiniAppConfig::new("Test App");
    assert_eq!(config.title.as_ref(), "Test App");
    assert_eq!(config.app_name.as_ref(), "Test App");
    assert_eq!(config.width, 900.0);
    assert_eq!(config.height, 700.0);
}

#[test]
fn test_config_size() {
    let config = MiniAppConfig::new("Test").size(1200.0, 800.0);
    assert_eq!(config.width, 1200.0);
    assert_eq!(config.height, 800.0);
}

#[test]
fn test_config_min_size() {
    let config = MiniAppConfig::new("Test").min_size(640.0, 480.0);
    assert_eq!(config.min_size, Some(size(px(640.0), px(480.0))));
}

#[test]
fn test_window_min_size_is_clamped_to_visible_display() {
    assert_eq!(
        super::mini_app::clamp_window_min_size(
            Some(size(px(400.0), px(300.0))),
            Some(size(px(1920.0), px(1080.0))),
        ),
        Some(size(px(400.0), px(300.0)))
    );
    assert_eq!(
        super::mini_app::clamp_window_min_size(
            Some(size(px(3000.0), px(1200.0))),
            Some(size(px(1920.0), px(1080.0))),
        ),
        Some(size(px(1920.0), px(1080.0)))
    );
    assert_eq!(
        super::mini_app::clamp_window_min_size(None, Some(size(px(1920.0), px(1080.0)))),
        None
    );
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn test_window_min_size_cli_parser() {
    assert_eq!(
        super::mini_app::parse_window_min_size(["--window-min-size".into(), "400x300".into()]),
        Ok(Some((400.0, 300.0)))
    );
    assert_eq!(
        super::mini_app::parse_window_min_size(["--window-min-size".into(), "400X300".into()]),
        Ok(Some((400.0, 300.0)))
    );
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn test_window_min_size_cli_parser_rejects_invalid_dimensions() {
    let error =
        super::mini_app::parse_window_min_size(["--window-min-size".into(), "400-by-300".into()])
            .expect_err("a malformed dimension must be rejected");
    assert!(error.contains("WIDTHxHEIGHT"));

    for value in ["0x300", "400x0", "-1x300", "400x-1", "NaNx300"] {
        let error =
            super::mini_app::parse_window_min_size(["--window-min-size".into(), value.into()])
                .expect_err("invalid dimensions must be rejected");
        assert!(error.contains("positive WIDTHxHEIGHT"));
    }

    let error = super::mini_app::parse_window_min_size(["--window-min-size".into()])
        .expect_err("a missing dimension must be rejected");
    assert!(error.contains("requires WIDTHxHEIGHT"));
}

#[test]
fn test_config_app_name() {
    let config = MiniAppConfig::new("Window Title").app_name("Menu Name");
    assert_eq!(config.title.as_ref(), "Window Title");
    assert_eq!(config.app_name.as_ref(), "Menu Name");
}

#[test]
fn test_config_default() {
    let config = MiniAppConfig::default();
    assert_eq!(config.title.as_ref(), "MiniApp");
}

#[test]
fn test_config_builder_chain() {
    let config = MiniAppConfig::new("Demo")
        .size(1000.0, 600.0)
        .app_name("My Demo App");

    assert_eq!(config.title.as_ref(), "Demo");
    assert_eq!(config.width, 1000.0);
    assert_eq!(config.height, 600.0);
    assert_eq!(config.app_name.as_ref(), "My Demo App");
}

#[test]
fn test_config_with_theme() {
    let config = MiniAppConfig::new("Test").with_theme(true);
    assert!(config.with_theme);
}

#[test]
fn test_config_with_i18n() {
    let config = MiniAppConfig::new("Test").with_i18n(true);
    assert!(config.with_i18n);
}

// ========================================================================
// Scrollable Configuration Tests
// ========================================================================

#[test]
fn test_config_scrollable_default() {
    let config = MiniAppConfig::new("Test");
    assert!(config.scrollable, "scrollable should be true by default");
}

#[test]
fn test_config_scrollable_disabled() {
    let config = MiniAppConfig::new("Test").scrollable(false);
    assert!(!config.scrollable);
}

#[test]
fn test_config_scrollable_enabled() {
    let config = MiniAppConfig::new("Test").scrollable(true);
    assert!(config.scrollable);
}

#[cfg(feature = "builder")]
#[test]
fn test_miniapp_shell_solves_content_slot_with_builder() {
    let solved = MiniAppShell::solve_content_layout(800.0, 600.0, 8.0);
    let content = solved.find("content").expect("content slot should exist");

    assert_eq!(content.width, 800.0);
    assert_eq!(content.height, 600.0);
    assert!(content.visible);
}

// ========================================================================
// Theme Configuration Tests
// ========================================================================

#[test]
fn test_config_with_theme_default_false() {
    let config = MiniAppConfig::new("Test");
    assert!(!config.with_theme, "with_theme should be false by default");
}

#[test]
fn test_config_with_theme_disabled() {
    let config = MiniAppConfig::new("Test").with_theme(false);
    assert!(!config.with_theme);
}

#[test]
fn test_config_initial_theme_dark() {
    let config = MiniAppConfig::new("Test").initial_theme(ThemeVariant::Dark);
    assert_eq!(config.initial_theme, ThemeVariant::Dark);
}

#[test]
fn test_config_initial_theme_light() {
    let config = MiniAppConfig::new("Test").initial_theme(ThemeVariant::Light);
    assert_eq!(config.initial_theme, ThemeVariant::Light);
}

#[test]
fn test_config_initial_theme_midnight() {
    let config = MiniAppConfig::new("Test").initial_theme(ThemeVariant::Midnight);
    assert_eq!(config.initial_theme, ThemeVariant::Midnight);
}

#[test]
fn test_config_initial_theme_forest() {
    let config = MiniAppConfig::new("Test").initial_theme(ThemeVariant::Forest);
    assert_eq!(config.initial_theme, ThemeVariant::Forest);
}

#[test]
fn test_config_initial_theme_black_and_white() {
    let config = MiniAppConfig::new("Test").initial_theme(ThemeVariant::BlackAndWhite);
    assert_eq!(config.initial_theme, ThemeVariant::BlackAndWhite);
}

// ========================================================================
// Language Configuration Tests
// ========================================================================

#[test]
fn test_config_with_i18n_default_false() {
    let config = MiniAppConfig::new("Test");
    assert!(!config.with_i18n, "with_i18n should be false by default");
}

#[test]
fn test_config_with_i18n_disabled() {
    let config = MiniAppConfig::new("Test").with_i18n(false);
    assert!(!config.with_i18n);
}

#[test]
fn test_config_initial_language_english() {
    let config = MiniAppConfig::new("Test").initial_language(Language::English);
    assert_eq!(config.initial_language, Language::English);
}

#[test]
fn test_config_initial_language_french() {
    let config = MiniAppConfig::new("Test").initial_language(Language::French);
    assert_eq!(config.initial_language, Language::French);
}

#[test]
fn test_config_initial_language_german() {
    let config = MiniAppConfig::new("Test").initial_language(Language::German);
    assert_eq!(config.initial_language, Language::German);
}

#[test]
fn test_config_initial_language_spanish() {
    let config = MiniAppConfig::new("Test").initial_language(Language::Spanish);
    assert_eq!(config.initial_language, Language::Spanish);
}

#[test]
fn test_config_initial_language_japanese() {
    let config = MiniAppConfig::new("Test").initial_language(Language::Japanese);
    assert_eq!(config.initial_language, Language::Japanese);
}

// ========================================================================
// Full Builder Chain Tests
// ========================================================================

#[test]
fn test_config_full_builder_chain() {
    let config = MiniAppConfig::new("Full Demo")
        .size(1920.0, 1080.0)
        .app_name("Full Demo App")
        .scrollable(false)
        .with_theme(true)
        .with_i18n(true)
        .initial_theme(ThemeVariant::Midnight)
        .initial_language(Language::Japanese);

    assert_eq!(config.title.as_ref(), "Full Demo");
    assert_eq!(config.width, 1920.0);
    assert_eq!(config.height, 1080.0);
    assert_eq!(config.app_name.as_ref(), "Full Demo App");
    assert!(!config.scrollable);
    assert!(config.with_theme);
    assert!(config.with_i18n);
    assert_eq!(config.initial_theme, ThemeVariant::Midnight);
    assert_eq!(config.initial_language, Language::Japanese);
}

#[test]
fn test_config_clone() {
    let config1 = MiniAppConfig::new("Clone Test")
        .size(800.0, 600.0)
        .with_theme(true);

    let config2 = config1.clone();

    assert_eq!(config1.title.as_ref(), config2.title.as_ref());
    assert_eq!(config1.width, config2.width);
    assert_eq!(config1.height, config2.height);
    assert_eq!(config1.with_theme, config2.with_theme);
}

// ========================================================================
// Edge Case Tests
// ========================================================================

#[test]
fn test_config_empty_title() {
    let config = MiniAppConfig::new("");
    assert_eq!(config.title.as_ref(), "");
    assert_eq!(config.app_name.as_ref(), "");
}

#[test]
fn test_config_unicode_title() {
    let config = MiniAppConfig::new("音楽プレーヤー");
    assert_eq!(config.title.as_ref(), "音楽プレーヤー");
}

#[test]
fn test_config_emoji_title() {
    let config = MiniAppConfig::new("🎵 Music Player");
    assert_eq!(config.title.as_ref(), "🎵 Music Player");
}

#[test]
fn test_config_zero_size() {
    let config = MiniAppConfig::new("Test").size(0.0, 0.0);
    assert_eq!(config.width, 0.0);
    assert_eq!(config.height, 0.0);
}

#[test]
fn test_config_large_size() {
    let config = MiniAppConfig::new("Test").size(7680.0, 4320.0); // 8K resolution
    assert_eq!(config.width, 7680.0);
    assert_eq!(config.height, 4320.0);
}

// ========================================================================
// Default Value Verification Tests
// ========================================================================

#[test]
fn test_config_all_defaults() {
    let config = MiniAppConfig::new("Test");

    // Verify all default values
    assert_eq!(config.width, 900.0);
    assert_eq!(config.height, 700.0);
    assert!(config.scrollable);
    assert!(!config.with_theme);
    assert!(!config.with_i18n);
    assert_eq!(config.initial_theme, ThemeVariant::default());
    assert_eq!(config.initial_language, Language::default());
}

#[test]
fn test_config_default_matches_new() {
    let config_default = MiniAppConfig::default();
    let config_new = MiniAppConfig::new("MiniApp");

    assert_eq!(config_default.title.as_ref(), config_new.title.as_ref());
    assert_eq!(config_default.width, config_new.width);
    assert_eq!(config_default.height, config_new.height);
    assert_eq!(config_default.scrollable, config_new.scrollable);
    assert_eq!(config_default.with_theme, config_new.with_theme);
    assert_eq!(config_default.with_i18n, config_new.with_i18n);
}

#[test]
fn test_current_platform_returns_ok() {
    // On supported platforms current_platform should succeed.
    let result = current_platform();
    assert!(
        result.is_ok(),
        "current_platform failed: {:?}",
        result.err()
    );
}

// ========================================================================
// Performance behavior tests
// ========================================================================

#[cfg(feature = "builder")]
#[test]
fn content_size_fills_the_shell_without_retained_cache_state() {
    let first = MiniAppShell::content_size(800.0, 600.0);
    let second = MiniAppShell::content_size(800.0, 600.0);
    assert_eq!(first, second);

    let third = MiniAppShell::content_size(1024.0, 768.0);
    assert_ne!(first, third);
}

#[test]
fn test_build_menus_with_language_basic() {
    let config = MiniAppConfig::new("Menu Test")
        .with_theme(true)
        .with_i18n(true);

    let menus = MiniApp::build_menus_with_language(&config, Language::English);

    // App, View, and Language menus when theme + i18n are enabled.
    assert_eq!(menus.len(), 3);
    assert_eq!(menus[0].name.as_ref(), "Menu Test");
    assert_eq!(menus[1].name.as_ref(), "View");
    assert_eq!(menus[2].name.as_ref(), "Language");

    let theme_menu = find_submenu(&menus[1], "Theme");
    #[allow(
        unused_mut,
        reason = "non-macOS replaces the platform-specific last label below"
    )]
    let mut expected_theme_names = ThemeVariant::all()
        .iter()
        .map(ThemeVariant::name)
        .chain(["Toggle Theme  Cmd+T"])
        .collect::<Vec<_>>();
    #[cfg(not(target_os = "macos"))]
    {
        expected_theme_names.pop();
        expected_theme_names.push("Toggle Theme");
    }
    assert_eq!(action_names(&theme_menu.items), expected_theme_names);

    let design_menu = find_submenu(&menus[1], "Design System");
    let expected_design_names = DesignLanguage::all()
        .iter()
        .map(DesignLanguage::label)
        .collect::<Vec<_>>();
    assert_eq!(action_names(&design_menu.items), expected_design_names);

    let no_i18n = MiniApp::build_menus_with_language(
        &MiniAppConfig::new("Menu Test").with_theme(true),
        Language::English,
    );
    assert_eq!(no_i18n.len(), 2);
}

#[test]
fn menus_mark_the_active_theme_design_and_language() {
    let config = MiniAppConfig::new("Menu Test")
        .with_theme(true)
        .with_i18n(true);
    let current_theme = ThemeVariant::all()[0];
    let current_design = DesignLanguage::all()[0];
    let menus = MiniApp::build_menus(&config, current_theme, current_design, Language::French);

    let theme_menu = find_submenu(&menus[1], "Theme");
    assert_eq!(
        checked_action_names(&theme_menu.items),
        [current_theme.name()]
    );

    let design_menu = find_submenu(&menus[1], "Design System");
    assert_eq!(
        checked_action_names(&design_menu.items),
        [current_design.label()]
    );

    assert_eq!(menus[2].name.as_ref(), "Langue");
    assert_eq!(checked_action_names(&menus[2].items), ["Français"]);
}

// ========================================================================
// Session State Persistence Tests
// ========================================================================

use super::mini_app_state::{
    MiniAppState, language_from_code, load_miniapp_state, save_miniapp_state, theme_from_name,
};
use std::path::PathBuf;

fn unique_state_path(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "miniapp-state-test-{}-{}-{tag}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ))
}

#[test]
fn theme_names_round_trip_all_variants_case_insensitively() {
    for variant in ThemeVariant::all() {
        assert_eq!(theme_from_name(variant.name()), Some(*variant));
        assert_eq!(
            theme_from_name(&variant.name().to_ascii_lowercase()),
            Some(*variant)
        );
        assert_eq!(
            theme_from_name(&format!("  {}  ", variant.name())),
            Some(*variant)
        );
    }
    assert_eq!(
        theme_from_name("Black & White"),
        Some(ThemeVariant::BlackAndWhite)
    );
    assert_eq!(
        theme_from_name("black & white"),
        Some(ThemeVariant::BlackAndWhite)
    );
    assert_eq!(theme_from_name("Not A Theme"), None);
    assert_eq!(theme_from_name(""), None);
}

#[test]
fn language_codes_round_trip_all_languages_case_insensitively() {
    for language in Language::all() {
        assert_eq!(language_from_code(language.code()), Some(*language));
        assert_eq!(
            language_from_code(&language.code().to_ascii_uppercase()),
            Some(*language)
        );
        assert_eq!(
            language_from_code(&format!(" {} ", language.code())),
            Some(*language)
        );
    }
    assert_eq!(language_from_code("xx"), None);
    assert_eq!(language_from_code(""), None);
}

#[test]
fn state_snapshot_and_is_empty() {
    let full = MiniAppState::snapshot(800.0, 600.0, ThemeVariant::Light, Language::French);
    assert!(!full.is_empty());
    assert_eq!(full.width, Some(800.0));
    assert_eq!(full.theme, Some(ThemeVariant::Light));
    assert!(MiniAppState::default().is_empty());
}

#[test]
fn state_save_and_load_round_trip() {
    let path = unique_state_path("roundtrip");
    let state = MiniAppState::snapshot(1024.0, 768.0, ThemeVariant::Forest, Language::Japanese);
    save_miniapp_state(&path, &state).expect("save should succeed");
    let loaded = load_miniapp_state(&path).expect("load should succeed");
    assert_eq!(loaded, state);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn state_load_missing_file_is_none() {
    let path = unique_state_path("missing");
    assert_eq!(load_miniapp_state(&path), None);
}

#[test]
fn state_load_ignores_garbage_but_applies_valid_lines() {
    let path = unique_state_path("garbage");
    std::fs::write(
        &path,
        "# comment\n\nno-equals-here\nwidth=-5\nwidth=NaN\nheight=0\ntheme=Bogus\nlanguage=xx\nunknown_key=1\n  language  =  fr  \nwidth=640\n",
    )
    .expect("fixture write should succeed");
    let loaded = load_miniapp_state(&path).expect("load should succeed");
    assert_eq!(loaded.width, Some(640.0));
    assert_eq!(loaded.height, None);
    assert_eq!(loaded.theme, None);
    assert_eq!(loaded.language, Some(Language::French));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn state_load_empty_file_yields_empty_state() {
    let path = unique_state_path("empty");
    std::fs::write(&path, "# only a comment\n\n").expect("fixture write should succeed");
    let loaded = load_miniapp_state(&path).expect("load should succeed");
    assert!(loaded.is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn state_save_creates_parent_directories() {
    let dir = unique_state_path("parentdir");
    let path = dir.join("nested").join("state.txt");
    let state = MiniAppState::snapshot(900.0, 700.0, ThemeVariant::Dark, Language::English);
    save_miniapp_state(&path, &state).expect("save should create parents");
    assert_eq!(load_miniapp_state(&path), Some(state));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn config_state_file_defaults_to_none_and_builder_sets_path() {
    let config = MiniAppConfig::new("Test");
    assert_eq!(config.state_file, None);
    let path = unique_state_path("builder");
    let config = MiniAppConfig::new("Test").state_file(&path);
    assert_eq!(config.state_file, Some(path));
}

#[test]
fn config_with_persisted_state_applies_file_and_ignores_missing() {
    // Missing file: config untouched.
    let missing = MiniAppConfig::new("Test")
        .size(111.0, 222.0)
        .state_file(unique_state_path("absent"));
    let kept = MiniApp::config_with_persisted_state(missing.clone());
    assert_eq!(kept.width, 111.0);
    assert_eq!(kept.height, 222.0);

    // Present file: valid fields override, builder values kept otherwise.
    let path = unique_state_path("apply");
    std::fs::write(&path, "width=1280\ntheme=Light\nlanguage=ja\n")
        .expect("fixture write should succeed");
    let config = MiniAppConfig::new("Test")
        .size(111.0, 222.0)
        .initial_theme(ThemeVariant::Dark)
        .state_file(&path);
    let applied = MiniApp::config_with_persisted_state(config);
    assert_eq!(applied.width, 1280.0);
    assert_eq!(applied.height, 222.0);
    assert_eq!(applied.initial_theme, ThemeVariant::Light);
    assert_eq!(applied.initial_language, Language::Japanese);
    let _ = std::fs::remove_file(&path);
}
