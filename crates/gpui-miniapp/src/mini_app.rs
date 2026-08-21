use super::mini_app_config::MiniAppConfig;
use super::mini_app_shell::MiniAppShell;
use super::misc::current_platform;
use crate::{
    Quit, SetLanguageEnglish, SetLanguageFrench, SetLanguageGerman, SetLanguageJapanese,
    SetLanguageSpanish, ToggleTheme,
};
use gpui::*;
use gpui_design::{DesignLanguage, DesignSystem, DesignSystemState};
use gpui_ui_kit::accessibility::AccessibilityTree;
use gpui_ui_kit::i18n::{I18nState, Language};
use gpui_ui_kit::theme::{ThemeState, ThemeVariant};
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq)]
struct SetThemeVariant {
    variant: ThemeVariant,
}

impl Action for SetThemeVariant {
    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(*self)
    }

    fn partial_eq(&self, action: &dyn Action) -> bool {
        action.as_any().downcast_ref::<Self>() == Some(self)
    }

    fn name(&self) -> &'static str {
        Self::name_for_type()
    }

    fn name_for_type() -> &'static str
    where
        Self: Sized,
    {
        "miniapp::SetThemeVariant"
    }

    fn build(
        _value: gpui::private::serde_json::Value,
    ) -> gpui::private::anyhow::Result<Box<dyn Action>>
    where
        Self: Sized,
    {
        gpui::private::anyhow::bail!("SetThemeVariant is only constructed by MiniApp menus")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SetDesignLanguage {
    language: DesignLanguage,
}

impl Action for SetDesignLanguage {
    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(*self)
    }

    fn partial_eq(&self, action: &dyn Action) -> bool {
        action.as_any().downcast_ref::<Self>() == Some(self)
    }

    fn name(&self) -> &'static str {
        Self::name_for_type()
    }

    fn name_for_type() -> &'static str
    where
        Self: Sized,
    {
        "miniapp::SetDesignLanguage"
    }

    fn build(
        _value: gpui::private::serde_json::Value,
    ) -> gpui::private::anyhow::Result<Box<dyn Action>>
    where
        Self: Sized,
    {
        gpui::private::anyhow::bail!("SetDesignLanguage is only constructed by MiniApp menus")
    }
}

/// MiniApp provides a minimal application shell for GPUI examples and showcases
///
/// It handles:
/// - Application lifecycle
/// - Standard menu bar with Quit option
/// - Theme variant switching with menu and Cmd+T
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
    /// use gpui_miniapp::MiniApp;
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
        let config_rc = Rc::new(config);

        let platform = match current_platform() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("MiniApp platform error: {e}");
                return;
            }
        };

        let launch = move |cx: &mut App| {
            // Initialize theme state if enabled
            if config_rc.with_theme {
                cx.set_global(ThemeState::with_variant(config_rc.initial_theme));
            }

            // Always set design system global (platform-appropriate defaults)
            cx.set_global(DesignSystemState::new());

            // Initialize accessibility tree
            cx.set_global(AccessibilityTree::new());

            // Initialize i18n state if enabled
            if config_rc.with_i18n {
                let mut i18n = I18nState::new();
                i18n.set_language(config_rc.initial_language);
                cx.set_global(i18n);
            }

            // Register quit action
            cx.on_action::<Quit>(|_action, cx| {
                cx.quit();
            });

            // Register theme actions if enabled
            if config_rc.with_theme {
                cx.on_action::<ToggleTheme>(|_action, cx| {
                    cx.update_global::<ThemeState, _>(|state, _cx| {
                        state.toggle();
                    });
                    cx.refresh_windows();
                });

                cx.on_action::<SetThemeVariant>(|action, cx| {
                    Self::set_theme_variant(cx, action.variant);
                });
            }

            // Register design system actions
            cx.on_action::<SetDesignLanguage>(|action, cx| {
                Self::set_design_language(cx, action.language);
            });

            // Register language actions if enabled
            if config_rc.with_i18n {
                let config_for_lang = config_rc.clone();
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

                let config_for_lang = config_rc.clone();
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

                let config_for_lang = config_rc.clone();
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

                let config_for_lang = config_rc.clone();
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

                let config_for_lang = config_rc.clone();
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
                .unwrap_or(config_rc.initial_language);
            let menus = Self::build_menus_with_language(&config_rc, current_language);
            cx.set_menus(menus);

            // Bind keyboard shortcuts
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

            if config_rc.with_theme {
                cx.bind_keys([KeyBinding::new("cmd-t", ToggleTheme, None)]);
            }

            // Create window
            let bounds =
                Bounds::centered(None, size(px(config_rc.width), px(config_rc.height)), cx);

            let scrollable = config_rc.scrollable;
            if let Err(e) = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some(config_rc.title.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                move |_, cx| {
                    let inner_view = build_view(cx);
                    cx.new(|_| MiniAppShell {
                        inner: inner_view.into(),
                        scrollable,
                    })
                },
            ) {
                eprintln!("MiniApp window error: {e:?}");
                return;
            }

            cx.activate(true);

            #[cfg(target_family = "wasm")]
            crate::web_mark_ready();
        };

        let app = gpui::Application::with_platform(platform);
        #[cfg(target_family = "wasm")]
        {
            // `WebPlatform::run` returns immediately after scheduling the
            // launch callback, so keep the app alive explicitly for the
            // page's lifetime via the leaked `ApplicationHandle`.
            std::mem::forget(app.run_embedded(launch));
        }
        #[cfg(not(target_family = "wasm"))]
        app.run(launch);
    }

    /// Build the menu bar based on configuration and current language
    pub(super) fn build_menus_with_language(
        config: &MiniAppConfig,
        current_language: Language,
    ) -> Vec<Menu> {
        let mut menus = Vec::new();

        // App menu with Quit
        let quit_label: SharedString = format!("Quit {}", config.app_name).into();
        menus.push(Menu {
            name: config.app_name.clone(),
            items: vec![MenuItem::action(quit_label, Quit)],
            disabled: false,
        });

        // View menu with Theme and Design submenus
        {
            let mut view_items = Vec::new();

            if config.with_theme {
                let mut theme_items = ThemeVariant::all()
                    .iter()
                    .copied()
                    .map(Self::theme_menu_item)
                    .collect::<Vec<_>>();
                theme_items.push(MenuItem::separator());
                theme_items.push(MenuItem::action("Toggle Theme  Cmd+T", ToggleTheme));

                view_items.push(MenuItem::submenu(Menu {
                    name: "Theme".into(),
                    disabled: false,
                    items: theme_items,
                }));
            }

            view_items.push(MenuItem::submenu(Menu {
                name: "Design System".into(),
                disabled: false,
                items: DesignLanguage::all()
                    .iter()
                    .copied()
                    .map(Self::design_menu_item)
                    .collect(),
            }));

            menus.push(Menu {
                name: "View".into(),
                disabled: false,
                items: view_items,
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
                disabled: false,
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

    fn theme_menu_item(variant: ThemeVariant) -> MenuItem {
        MenuItem::action(variant.name(), SetThemeVariant { variant })
    }

    fn design_menu_item(language: DesignLanguage) -> MenuItem {
        MenuItem::action(language.label(), SetDesignLanguage { language })
    }

    fn set_theme_variant(cx: &mut App, variant: ThemeVariant) {
        cx.update_global::<ThemeState, _>(|state, _cx| {
            state.set_variant(variant);
        });
        cx.refresh_windows();
    }

    fn set_design_language(cx: &mut App, language: DesignLanguage) {
        cx.set_global(DesignSystemState::with_system(DesignSystem::for_language(
            language,
        )));
        cx.refresh_windows();
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
