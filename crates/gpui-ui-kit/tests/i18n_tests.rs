//! I18n integration tests
//!
//! Tests that verify translations work correctly across all components.

use gpui_ui_kit::i18n::{I18nState, Language, TranslationKey};

#[test]
fn test_all_languages_have_app_title() {
    let translations = gpui_ui_kit::i18n::Translations::new();

    for lang in Language::all() {
        let title = translations.get(*lang, TranslationKey::AppTitle);
        assert_ne!(title, "???", "Language {:?} missing AppTitle", lang);
        assert!(!title.is_empty(), "Language {:?} has empty AppTitle", lang);
    }
}

#[test]
fn test_all_languages_have_menu_translations() {
    let translations = gpui_ui_kit::i18n::Translations::new();

    let menu_keys = [
        TranslationKey::MenuFile,
        TranslationKey::MenuEdit,
        TranslationKey::MenuView,
        TranslationKey::MenuHelp,
        TranslationKey::MenuQuit,
        TranslationKey::MenuTheme,
        TranslationKey::MenuLanguage,
        TranslationKey::MenuSettings,
    ];

    for lang in Language::all() {
        for key in &menu_keys {
            let text = translations.get(*lang, *key);
            assert_ne!(
                text, "???",
                "Language {:?} missing translation for {:?}",
                lang, key
            );
            assert!(
                !text.is_empty(),
                "Language {:?} has empty translation for {:?}",
                lang,
                key
            );
        }
    }
}

#[test]
fn test_all_languages_have_section_translations() {
    let translations = gpui_ui_kit::i18n::Translations::new();

    let section_keys = [
        TranslationKey::SectionButtons,
        TranslationKey::SectionTypography,
        TranslationKey::SectionBadges,
        TranslationKey::SectionAvatars,
        TranslationKey::SectionFormControls,
        TranslationKey::SectionProgress,
        TranslationKey::SectionAlerts,
        TranslationKey::SectionTabs,
        TranslationKey::SectionCards,
        TranslationKey::SectionBreadcrumbs,
        TranslationKey::SectionSpinners,
        TranslationKey::SectionLayout,
        TranslationKey::SectionIconButtons,
        TranslationKey::SectionToasts,
        TranslationKey::SectionDialogs,
        TranslationKey::SectionMenus,
        TranslationKey::SectionTooltips,
        TranslationKey::SectionPotentiometers,
        TranslationKey::SectionAccordion,
    ];

    for lang in Language::all() {
        for key in &section_keys {
            let text = translations.get(*lang, *key);
            assert_ne!(
                text, "???",
                "Language {:?} missing translation for {:?}",
                lang, key
            );
            assert!(
                !text.is_empty(),
                "Language {:?} has empty translation for {:?}",
                lang,
                key
            );
        }
    }
}

#[test]
fn test_all_languages_have_button_translations() {
    let translations = gpui_ui_kit::i18n::Translations::new();

    let button_keys = [
        TranslationKey::ButtonPrimary,
        TranslationKey::ButtonSecondary,
        TranslationKey::ButtonDestructive,
        TranslationKey::ButtonGhost,
        TranslationKey::ButtonOutline,
        TranslationKey::ButtonCancel,
        TranslationKey::ButtonSave,
        TranslationKey::ButtonConfirm,
    ];

    for lang in Language::all() {
        for key in &button_keys {
            let text = translations.get(*lang, *key);
            assert_ne!(
                text, "???",
                "Language {:?} missing translation for {:?}",
                lang, key
            );
            assert!(
                !text.is_empty(),
                "Language {:?} has empty translation for {:?}",
                lang,
                key
            );
        }
    }
}

#[test]
fn test_all_languages_have_alert_translations() {
    let translations = gpui_ui_kit::i18n::Translations::new();

    let alert_keys = [
        TranslationKey::AlertInfo,
        TranslationKey::AlertSuccess,
        TranslationKey::AlertWarning,
        TranslationKey::AlertError,
        TranslationKey::AlertInfoMessage,
        TranslationKey::AlertSuccessMessage,
        TranslationKey::AlertWarningMessage,
        TranslationKey::AlertErrorMessage,
    ];

    for lang in Language::all() {
        for key in &alert_keys {
            let text = translations.get(*lang, *key);
            assert_ne!(
                text, "???",
                "Language {:?} missing translation for {:?}",
                lang, key
            );
            assert!(
                !text.is_empty(),
                "Language {:?} has empty translation for {:?}",
                lang,
                key
            );
        }
    }
}

#[test]
fn test_all_languages_have_label_translations() {
    let translations = gpui_ui_kit::i18n::Translations::new();

    let label_keys = [
        TranslationKey::LabelVariants,
        TranslationKey::LabelSizes,
        TranslationKey::LabelStates,
        TranslationKey::LabelToggles,
        TranslationKey::LabelCheckboxes,
        TranslationKey::LabelSlider,
        TranslationKey::LabelInput,
        TranslationKey::LabelSmall,
        TranslationKey::LabelMedium,
        TranslationKey::LabelLarge,
        TranslationKey::LabelDisabled,
        TranslationKey::LabelSelected,
    ];

    for lang in Language::all() {
        for key in &label_keys {
            let text = translations.get(*lang, *key);
            assert_ne!(
                text, "???",
                "Language {:?} missing translation for {:?}",
                lang, key
            );
            assert!(
                !text.is_empty(),
                "Language {:?} has empty translation for {:?}",
                lang,
                key
            );
        }
    }
}

#[test]
fn test_language_switching() {
    let mut state = I18nState::new();

    // Default should be English
    assert_eq!(state.language, Language::English);
    assert_eq!(state.t(TranslationKey::AppTitle), "GPUI UI Kit Showcase");

    // Switch to French
    state.set_language(Language::French);
    assert_eq!(state.language, Language::French);
    assert_eq!(state.t(TranslationKey::AppTitle), "Vitrine du UI Kit GPUI");

    // Switch to German
    state.set_language(Language::German);
    assert_eq!(state.language, Language::German);
    assert_eq!(state.t(TranslationKey::AppTitle), "GPUI UI Kit Showcase");

    // Switch to Spanish
    state.set_language(Language::Spanish);
    assert_eq!(state.language, Language::Spanish);
    assert_eq!(state.t(TranslationKey::AppTitle), "Galeria del UI Kit GPUI");

    // Switch to Japanese
    state.set_language(Language::Japanese);
    assert_eq!(state.language, Language::Japanese);
    assert_eq!(state.t(TranslationKey::AppTitle), "GPUI UI Kit Showcase");
}

#[test]
fn test_fallback_to_english() {
    let translations = gpui_ui_kit::i18n::Translations::new();

    // All languages should fall back to English if a translation is missing
    // (though in practice all translations should be present)
    for lang in Language::all() {
        let text = translations.get(*lang, TranslationKey::AppTitle);
        assert_ne!(
            text, "???",
            "Language {:?} should fall back to English",
            lang
        );
    }
}

#[test]
fn test_translation_consistency() {
    let translations = gpui_ui_kit::i18n::Translations::new();

    // Verify that certain translations maintain expected structure
    // For example, button labels should be short
    let button_keys = [
        TranslationKey::ButtonPrimary,
        TranslationKey::ButtonSecondary,
        TranslationKey::ButtonCancel,
        TranslationKey::ButtonSave,
    ];

    for lang in Language::all() {
        for key in &button_keys {
            let text = translations.get(*lang, *key);
            assert!(
                text.len() < 30,
                "Button text too long for {:?} in {:?}: '{}'",
                key,
                lang,
                text
            );
        }
    }
}

#[test]
fn test_language_metadata() {
    // Test language code mapping
    assert_eq!(Language::English.code(), "en");
    assert_eq!(Language::French.code(), "fr");
    assert_eq!(Language::German.code(), "de");
    assert_eq!(Language::Spanish.code(), "es");
    assert_eq!(Language::Japanese.code(), "ja");

    // Test flag mapping
    assert_eq!(Language::English.flag(), "GB");
    assert_eq!(Language::French.flag(), "FR");
    assert_eq!(Language::German.flag(), "DE");
    assert_eq!(Language::Spanish.flag(), "ES");
    assert_eq!(Language::Japanese.flag(), "JP");

    // Test native names
    assert_eq!(Language::English.native_name(), "English");
    assert_eq!(Language::French.native_name(), "Francais");
    assert_eq!(Language::German.native_name(), "Deutsch");
    assert_eq!(Language::Spanish.native_name(), "Espanol");
    assert_eq!(Language::Japanese.native_name(), "Nihongo");
}

#[test]
fn test_all_translation_keys_have_entries() {
    let translations = gpui_ui_kit::i18n::Translations::new();

    // Create a set of all translation keys we expect
    let all_keys = vec![
        // App
        TranslationKey::AppTitle,
        TranslationKey::AppSubtitle,
        // Menu
        TranslationKey::MenuFile,
        TranslationKey::MenuEdit,
        TranslationKey::MenuView,
        TranslationKey::MenuHelp,
        TranslationKey::MenuQuit,
        TranslationKey::MenuTheme,
        TranslationKey::MenuLanguage,
        TranslationKey::MenuSettings,
        // Theme
        TranslationKey::ThemeDark,
        TranslationKey::ThemeLight,
        // Sections
        TranslationKey::SectionButtons,
        TranslationKey::SectionTypography,
        TranslationKey::SectionBadges,
        TranslationKey::SectionAvatars,
        TranslationKey::SectionFormControls,
        TranslationKey::SectionProgress,
        TranslationKey::SectionAlerts,
        TranslationKey::SectionTabs,
        TranslationKey::SectionCards,
        TranslationKey::SectionBreadcrumbs,
        TranslationKey::SectionSpinners,
        TranslationKey::SectionLayout,
        TranslationKey::SectionIconButtons,
        TranslationKey::SectionToasts,
        TranslationKey::SectionDialogs,
        TranslationKey::SectionMenus,
        TranslationKey::SectionTooltips,
        TranslationKey::SectionPotentiometers,
        TranslationKey::SectionAccordion,
        // Labels
        TranslationKey::LabelVariants,
        TranslationKey::LabelSizes,
        TranslationKey::LabelStates,
        TranslationKey::LabelToggles,
        TranslationKey::LabelCheckboxes,
        TranslationKey::LabelSlider,
        TranslationKey::LabelInput,
        TranslationKey::LabelSmall,
        TranslationKey::LabelMedium,
        TranslationKey::LabelLarge,
        TranslationKey::LabelDisabled,
        TranslationKey::LabelSelected,
        // Buttons
        TranslationKey::ButtonPrimary,
        TranslationKey::ButtonSecondary,
        TranslationKey::ButtonDestructive,
        TranslationKey::ButtonGhost,
        TranslationKey::ButtonOutline,
        TranslationKey::ButtonCancel,
        TranslationKey::ButtonSave,
        TranslationKey::ButtonConfirm,
        // Dialog
        TranslationKey::DialogConfirmTitle,
        TranslationKey::DialogConfirmMessage,
        // Alerts
        TranslationKey::AlertInfo,
        TranslationKey::AlertSuccess,
        TranslationKey::AlertWarning,
        TranslationKey::AlertError,
        TranslationKey::AlertInfoMessage,
        TranslationKey::AlertSuccessMessage,
        TranslationKey::AlertWarningMessage,
        TranslationKey::AlertErrorMessage,
        // Accordion
        TranslationKey::AccordionGettingStarted,
        TranslationKey::AccordionFeatures,
        TranslationKey::AccordionConfiguration,
    ];

    // Verify every key has a translation in every language
    for lang in Language::all() {
        let mut missing = Vec::new();
        for key in &all_keys {
            let text = translations.get(*lang, *key);
            if text == "???" {
                missing.push(key);
            }
        }
        assert!(
            missing.is_empty(),
            "Language {:?} missing translations for: {:?}",
            lang,
            missing
        );
    }
}
