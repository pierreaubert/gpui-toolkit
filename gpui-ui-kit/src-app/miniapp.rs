//! MiniApp - A minimal application template for GPUI examples and showcases
//!
//! Provides a reusable application shell with:
//! - Standard menu bar with Quit option (Cmd+Q on macOS)
//! - Theme switching (light/dark) with Cmd+T
//! - Language switching menu
//! - Configurable window title and size
//! - Extensible for additional default features
//!
//! # Example
//!
//! ```ignore
//! use gpui::*;
//! use gpui_ui_kit::miniapp::{MiniApp, MiniAppConfig};
//!
//! struct MyDemo;
//!
//! impl Render for MyDemo {
//!     fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
//!         div().child("Hello from MiniApp!")
//!     }
//! }
//!
//! fn main() {
//!     MiniApp::run(MiniAppConfig::new("My Demo"), |cx| cx.new(|_| MyDemo));
//! }
//! ```

use crate::i18n::{I18nState, Language};
use crate::theme::{ThemeState, ThemeVariant};
use gpui::*;

/// Configuration for a MiniApp instance
#[derive(Clone)]
pub struct MiniAppConfig {
    /// Window title
    pub title: SharedString,
    /// Window width in pixels
    pub width: f32,
    /// Window height in pixels
    pub height: f32,
    /// Application name shown in menu bar
    pub app_name: SharedString,
    /// Enable vertical scrollbar for content
    pub scrollable: bool,
    /// Enable theme support
    pub with_theme: bool,
    /// Enable i18n support
    pub with_i18n: bool,
    /// Initial theme variant
    pub initial_theme: ThemeVariant,
    /// Initial language
    pub initial_language: Language,
}

impl MiniAppConfig {
    /// Create a new configuration with the given title
    ///
    /// Uses default window size of 900x700 pixels.
    pub fn new(title: impl Into<SharedString>) -> Self {
        let title = title.into();
        Self {
            title: title.clone(),
            width: 900.0,
            height: 700.0,
            app_name: title,
            scrollable: true,
            with_theme: false,
            with_i18n: false,
            initial_theme: ThemeVariant::default(),
            initial_language: Language::default(),
        }
    }

    /// Set the window size
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set the application name shown in the menu bar
    ///
    /// By default, this is the same as the window title.
    pub fn app_name(mut self, name: impl Into<SharedString>) -> Self {
        self.app_name = name.into();
        self
    }

    /// Enable or disable vertical scrollbar for content
    ///
    /// By default, scrolling is enabled.
    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }

    /// Enable theme support with light/dark switching
    pub fn with_theme(mut self, enabled: bool) -> Self {
        self.with_theme = enabled;
        self
    }

    /// Enable i18n support with language switching
    pub fn with_i18n(mut self, enabled: bool) -> Self {
        self.with_i18n = enabled;
        self
    }

    /// Set initial theme variant
    pub fn initial_theme(mut self, theme: ThemeVariant) -> Self {
        self.initial_theme = theme;
        self
    }

    /// Set initial language
    pub fn initial_language(mut self, language: Language) -> Self {
        self.initial_language = language;
        self
    }
}

impl Default for MiniAppConfig {
    fn default() -> Self {
        Self::new("MiniApp")
    }
}

// Define actions for the menu
actions!(
    miniapp,
    [
        Quit,
        ToggleTheme,
        SetThemeDark,
        SetThemeLight,
        SetThemeMidnight,
        SetThemeForest,
        SetThemeBlackAndWhite,
        SetLanguageEnglish,
        SetLanguageFrench,
        SetLanguageGerman,
        SetLanguageSpanish,
        SetLanguageJapanese,
    ]
);

/// A wrapper view that adds vertical scrolling to its content
struct ScrollableWrapper {
    inner: AnyView,
}

impl Render for ScrollableWrapper {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("miniapp-scroll-container")
            .size_full()
            .overflow_y_scroll()
            .child(self.inner.clone())
    }
}

/// MiniApp provides a minimal application shell for GPUI examples and showcases
///
/// It handles:
/// - Application lifecycle
/// - Standard menu bar with Quit option
/// - Theme switching (light/dark) with menu and Cmd+T
/// - Language switching menu
/// - Window creation with configurable size
/// - Keyboard shortcut binding (Cmd+Q to quit)
pub struct MiniApp;

impl MiniApp {
    /// Run a MiniApp with the given configuration and view builder
    ///
    /// The `build_view` closure receives a `&mut Context<V>` and should return
    /// a `V` instance that implements `Render`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use gpui::*;
    /// use gpui_ui_kit::MiniApp;
    ///
    /// struct MyView;
    /// impl Render for MyView {
    ///     fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
    ///         div().child("Hello!")
    ///     }
    /// }
    ///
    /// MiniApp::run(MiniAppConfig::new("Demo"), |cx| cx.new(MyView::new));
    /// ```
    pub fn run<V, F>(config: MiniAppConfig, build_view: F)
    where
        V: Render + 'static,
        F: FnOnce(&mut App) -> Entity<V> + 'static,
    {
        let config_clone = config.clone();

        Application::new().run(move |cx: &mut App| {
            // Initialize theme state if enabled
            if config_clone.with_theme {
                cx.set_global(ThemeState::with_variant(config_clone.initial_theme));
            }

            // Initialize i18n state if enabled
            if config_clone.with_i18n {
                let mut i18n = I18nState::new();
                i18n.set_language(config_clone.initial_language);
                cx.set_global(i18n);
            }

            // Register quit action
            cx.on_action::<Quit>(|_action, cx| {
                cx.quit();
            });

            // Register theme actions if enabled
            if config_clone.with_theme {
                cx.on_action::<ToggleTheme>(|_action, cx| {
                    cx.update_global::<ThemeState, _>(|state, _cx| {
                        state.toggle();
                    });
                    cx.refresh_windows();
                });

                cx.on_action::<SetThemeDark>(|_action, cx| {
                    cx.update_global::<ThemeState, _>(|state, _cx| {
                        state.set_variant(ThemeVariant::Dark);
                    });
                    cx.refresh_windows();
                });

                cx.on_action::<SetThemeLight>(|_action, cx| {
                    cx.update_global::<ThemeState, _>(|state, _cx| {
                        state.set_variant(ThemeVariant::Light);
                    });
                    cx.refresh_windows();
                });

                cx.on_action::<SetThemeMidnight>(|_action, cx| {
                    cx.update_global::<ThemeState, _>(|state, _cx| {
                        state.set_variant(ThemeVariant::Midnight);
                    });
                    cx.refresh_windows();
                });

                cx.on_action::<SetThemeForest>(|_action, cx| {
                    cx.update_global::<ThemeState, _>(|state, _cx| {
                        state.set_variant(ThemeVariant::Forest);
                    });
                    cx.refresh_windows();
                });

                cx.on_action::<SetThemeBlackAndWhite>(|_action, cx| {
                    cx.update_global::<ThemeState, _>(|state, _cx| {
                        state.set_variant(ThemeVariant::BlackAndWhite);
                    });
                    cx.refresh_windows();
                });
            }

            // Register language actions if enabled
            if config_clone.with_i18n {
                let config_for_lang = config_clone.clone();
                cx.on_action::<SetLanguageEnglish>(move |_action, cx| {
                    cx.update_global::<I18nState, _>(|state, _cx| {
                        state.set_language(Language::English);
                    });
                    let current_language = cx
                        .try_global::<I18nState>()
                        .map(|state| state.language)
                        .unwrap_or(Language::English);
                    let menus = Self::build_menus_with_language(&config_for_lang, current_language);
                    cx.set_menus(menus);
                    cx.refresh_windows();
                });

                let config_for_lang = config_clone.clone();
                cx.on_action::<SetLanguageFrench>(move |_action, cx| {
                    cx.update_global::<I18nState, _>(|state, _cx| {
                        state.set_language(Language::French);
                    });
                    let current_language = cx
                        .try_global::<I18nState>()
                        .map(|state| state.language)
                        .unwrap_or(Language::English);
                    let menus = Self::build_menus_with_language(&config_for_lang, current_language);
                    cx.set_menus(menus);
                    cx.refresh_windows();
                });

                let config_for_lang = config_clone.clone();
                cx.on_action::<SetLanguageGerman>(move |_action, cx| {
                    cx.update_global::<I18nState, _>(|state, _cx| {
                        state.set_language(Language::German);
                    });
                    let current_language = cx
                        .try_global::<I18nState>()
                        .map(|state| state.language)
                        .unwrap_or(Language::English);
                    let menus = Self::build_menus_with_language(&config_for_lang, current_language);
                    cx.set_menus(menus);
                    cx.refresh_windows();
                });

                let config_for_lang = config_clone.clone();
                cx.on_action::<SetLanguageSpanish>(move |_action, cx| {
                    cx.update_global::<I18nState, _>(|state, _cx| {
                        state.set_language(Language::Spanish);
                    });
                    let current_language = cx
                        .try_global::<I18nState>()
                        .map(|state| state.language)
                        .unwrap_or(Language::English);
                    let menus = Self::build_menus_with_language(&config_for_lang, current_language);
                    cx.set_menus(menus);
                    cx.refresh_windows();
                });

                let config_for_lang = config_clone.clone();
                cx.on_action::<SetLanguageJapanese>(move |_action, cx| {
                    cx.update_global::<I18nState, _>(|state, _cx| {
                        state.set_language(Language::Japanese);
                    });
                    let current_language = cx
                        .try_global::<I18nState>()
                        .map(|state| state.language)
                        .unwrap_or(Language::English);
                    let menus = Self::build_menus_with_language(&config_for_lang, current_language);
                    cx.set_menus(menus);
                    cx.refresh_windows();
                });
            }

            // Build menu bar
            let current_language = cx
                .try_global::<I18nState>()
                .map(|state| state.language)
                .unwrap_or(config_clone.initial_language);
            let menus = Self::build_menus_with_language(&config_clone, current_language);
            cx.set_menus(menus);

            // Bind keyboard shortcuts
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

            if config_clone.with_theme {
                cx.bind_keys([KeyBinding::new("cmd-t", ToggleTheme, None)]);
            }

            // Create window
            let bounds = Bounds::centered(
                None,
                size(px(config_clone.width), px(config_clone.height)),
                cx,
            );

            if config_clone.scrollable {
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: Some(TitlebarOptions {
                            title: Some(config_clone.title.clone()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    move |_, cx| {
                        let inner_view = build_view(cx);
                        cx.new(|_| ScrollableWrapper {
                            inner: inner_view.into(),
                        })
                    },
                )
                .unwrap();
            } else {
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: Some(TitlebarOptions {
                            title: Some(config_clone.title.clone()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    |_, cx| build_view(cx),
                )
                .unwrap();
            }

            cx.activate(true);
        });
    }

    /// Build the menu bar based on configuration and current language
    fn build_menus_with_language(config: &MiniAppConfig, current_language: Language) -> Vec<Menu> {
        let mut menus = Vec::new();

        // App menu with Quit
        let quit_label: SharedString = format!("Quit {}", config.app_name).into();
        menus.push(Menu {
            name: config.app_name.clone(),
            items: vec![MenuItem::action(quit_label, Quit)],
        });

        // View menu with Theme submenu if enabled
        if config.with_theme {
            menus.push(Menu {
                name: "View".into(),
                items: vec![MenuItem::submenu(Menu {
                    name: "Theme".into(),
                    items: vec![
                        MenuItem::action("Dark", SetThemeDark),
                        MenuItem::action("Light", SetThemeLight),
                        MenuItem::action("Midnight", SetThemeMidnight),
                        MenuItem::action("Forest", SetThemeForest),
                        MenuItem::action("Black & White", SetThemeBlackAndWhite),
                        MenuItem::separator(),
                        MenuItem::action("Toggle Theme  Cmd+T", ToggleTheme),
                    ],
                })],
            });
        }

        // Language menu if i18n enabled
        if config.with_i18n {
            // Localize menu title based on current language
            let menu_title = match current_language {
                Language::English => "Language",
                Language::French => "Langue",
                Language::German => "Sprache",
                Language::Spanish => "Idioma",
                Language::Japanese => "言語",
            };

            menus.push(Menu {
                name: menu_title.into(),
                items: vec![
                    MenuItem::action("English", SetLanguageEnglish),
                    MenuItem::action("Français", SetLanguageFrench),
                    MenuItem::action("Deutsch", SetLanguageGerman),
                    MenuItem::action("Español", SetLanguageSpanish),
                    MenuItem::action("日本語", SetLanguageJapanese),
                ],
            });
        }

        menus
    }

    /// Run a MiniApp with default configuration
    ///
    /// Uses "MiniApp" as the default title and 900x700 window size.
    pub fn run_default<V, F>(build_view: F)
    where
        V: Render + 'static,
        F: FnOnce(&mut App) -> Entity<V> + 'static,
    {
        Self::run(MiniAppConfig::default(), build_view);
    }
}

#[cfg(test)]
mod tests {
    use super::MiniAppConfig;
    use crate::i18n::Language;
    use crate::theme::ThemeVariant;

    // ========================================================================
    // Basic Configuration Tests
    // ========================================================================

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
}
