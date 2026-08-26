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

pub(super) const QUIT_KEYSTROKE: &str = "secondary-q";
pub(super) const TOGGLE_THEME_KEYSTROKE: &str = "secondary-t";

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
        let config = match Self::config_with_cli_window_min_size(config) {
            Ok(config) => config,
            Err(error) => {
                eprintln!("MiniApp argument error: {error}");
                #[cfg(not(target_family = "wasm"))]
                std::process::exit(2);
                #[cfg(target_family = "wasm")]
                return;
            }
        };
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
                let config_for_theme = config_rc.clone();
                cx.on_action::<ToggleTheme>(move |_action, cx| {
                    cx.update_global::<ThemeState, _>(|state, _cx| {
                        state.toggle();
                    });
                    Self::refresh_menus(cx, &config_for_theme);
                    cx.refresh_windows();
                });

                let config_for_theme = config_rc.clone();
                cx.on_action::<SetThemeVariant>(move |action, cx| {
                    Self::set_theme_variant(cx, action.variant);
                    Self::refresh_menus(cx, &config_for_theme);
                });
            }

            // Register design system actions
            let config_for_design = config_rc.clone();
            cx.on_action::<SetDesignLanguage>(move |action, cx| {
                Self::set_design_language(cx, action.language);
                Self::refresh_menus(cx, &config_for_design);
            });

            // Register language actions if enabled
            if config_rc.with_i18n {
                let config_for_lang = config_rc.clone();
                cx.on_action::<SetLanguageEnglish>(move |_action, cx| {
                    Self::set_language(cx, &config_for_lang, Language::English);
                });

                let config_for_lang = config_rc.clone();
                cx.on_action::<SetLanguageFrench>(move |_action, cx| {
                    Self::set_language(cx, &config_for_lang, Language::French);
                });

                let config_for_lang = config_rc.clone();
                cx.on_action::<SetLanguageGerman>(move |_action, cx| {
                    Self::set_language(cx, &config_for_lang, Language::German);
                });

                let config_for_lang = config_rc.clone();
                cx.on_action::<SetLanguageSpanish>(move |_action, cx| {
                    Self::set_language(cx, &config_for_lang, Language::Spanish);
                });

                let config_for_lang = config_rc.clone();
                cx.on_action::<SetLanguageJapanese>(move |_action, cx| {
                    Self::set_language(cx, &config_for_lang, Language::Japanese);
                });
            }

            // Build menu bar.
            Self::refresh_menus(cx, &config_rc);

            // Bind keyboard shortcuts
            cx.bind_keys([KeyBinding::new(QUIT_KEYSTROKE, Quit, None)]);

            if config_rc.with_theme {
                cx.bind_keys([KeyBinding::new(TOGGLE_THEME_KEYSTROKE, ToggleTheme, None)]);
            }

            // Create window
            let display_size = cx
                .primary_display()
                .map(|display| display.visible_bounds().size);
            let window_min_size = clamp_window_min_size(config_rc.min_size, display_size);
            let initial_size = size(px(config_rc.width), px(config_rc.height));
            let initial_size = window_min_size
                .as_ref()
                .map(|min_size| initial_size.max(min_size))
                .unwrap_or(initial_size);
            let initial_size = display_size
                .as_ref()
                .map(|display_size| initial_size.min(display_size))
                .unwrap_or(initial_size);
            let bounds = Bounds::centered(None, initial_size, cx);

            let scrollable = config_rc.scrollable;
            if let Err(e) = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size,
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
                #[cfg(not(target_family = "wasm"))]
                cx.quit();
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

    /// Apply the shared native-window CLI overrides to a configuration.
    ///
    /// `--window-min-size WIDTHxHEIGHT` takes precedence over the builder
    /// value, for example `--window-min-size 400x400`.
    #[cfg(not(target_family = "wasm"))]
    pub(super) fn config_with_cli_window_min_size(
        config: MiniAppConfig,
    ) -> Result<MiniAppConfig, String> {
        match parse_window_min_size(std::env::args().skip(1))? {
            Some((width, height)) => Ok(config.min_size(width, height)),
            None => Ok(config),
        }
    }

    #[cfg(target_family = "wasm")]
    pub(super) fn config_with_cli_window_min_size(
        config: MiniAppConfig,
    ) -> Result<MiniAppConfig, String> {
        Ok(config)
    }

    /// Build the menu bar based on configuration and current language
    #[cfg(test)]
    pub(super) fn build_menus_with_language(
        config: &MiniAppConfig,
        current_language: Language,
    ) -> Vec<Menu> {
        Self::build_menus(
            config,
            config.initial_theme,
            DesignSystem::platform_default().language,
            current_language,
        )
    }

    pub(super) fn build_menus(
        config: &MiniAppConfig,
        current_theme: ThemeVariant,
        current_design: DesignLanguage,
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
                    .map(|variant| Self::theme_menu_item(variant, current_theme))
                    .collect::<Vec<_>>();
                theme_items.push(MenuItem::separator());
                theme_items.push(MenuItem::action("Toggle Theme  Cmd+T", ToggleTheme));

                view_items.push(MenuItem::submenu(Menu {
                    name: "Theme".into(),
                    disabled: false,
                    items: theme_items,
                }));
            }

            #[cfg(not(target_os = "macos"))]
            {
                // `secondary-t` is Ctrl+T here, so do not show a macOS-only
                // accelerator in the native menu label.
                theme_items.pop();
                theme_items.push(MenuItem::action("Toggle Theme", ToggleTheme));
            }

            view_items.push(MenuItem::submenu(Menu {
                name: "Design System".into(),
                disabled: false,
                items: DesignLanguage::all()
                    .iter()
                    .copied()
                    .map(|language| Self::design_menu_item(language, current_design))
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
                    MenuItem::action("English", SetLanguageEnglish)
                        .checked(current_language == Language::English),
                    MenuItem::action("Français", SetLanguageFrench)
                        .checked(current_language == Language::French),
                    MenuItem::action("Deutsch", SetLanguageGerman)
                        .checked(current_language == Language::German),
                    MenuItem::action("Español", SetLanguageSpanish)
                        .checked(current_language == Language::Spanish),
                    MenuItem::action("日本語", SetLanguageJapanese)
                        .checked(current_language == Language::Japanese),
                ],
            });
        }

        menus
    }

    fn theme_menu_item(variant: ThemeVariant, current_theme: ThemeVariant) -> MenuItem {
        MenuItem::action(variant.name(), SetThemeVariant { variant })
            .checked(variant == current_theme)
    }

    fn design_menu_item(language: DesignLanguage, current_design: DesignLanguage) -> MenuItem {
        MenuItem::action(language.label(), SetDesignLanguage { language })
            .checked(language == current_design)
    }

    fn refresh_menus(cx: &mut App, config: &MiniAppConfig) {
        let current_theme = cx
            .try_global::<ThemeState>()
            .map(|state| state.theme.variant)
            .unwrap_or(config.initial_theme);
        let current_design = cx
            .try_global::<DesignSystemState>()
            .map(|state| state.system.language)
            .unwrap_or_else(|| DesignSystem::platform_default().language);
        let current_language = cx
            .try_global::<I18nState>()
            .map(|state| state.language)
            .unwrap_or(config.initial_language);
        cx.set_menus(Self::build_menus(
            config,
            current_theme,
            current_design,
            current_language,
        ));
    }

    fn set_language(cx: &mut App, config: &MiniAppConfig, language: Language) {
        cx.update_global::<I18nState, _>(|state, _cx| {
            state.set_language(language);
        });
        Self::refresh_menus(cx, config);
        cx.refresh_windows();
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

/// Clamp a requested minimum window size to the usable dimensions of the
/// display that will host the new window. If no display is known, preserve the
/// requested value and let the platform select the appropriate display.
pub(super) fn clamp_window_min_size(
    min_size: Option<Size<Pixels>>,
    display_size: Option<Size<Pixels>>,
) -> Option<Size<Pixels>> {
    match (min_size, display_size) {
        (Some(min_size), Some(display_size)) => Some(min_size.min(&display_size)),
        (min_size, _) => min_size,
    }
}

#[cfg(not(target_family = "wasm"))]
pub(super) fn parse_window_min_size<I>(args: I) -> Result<Option<(f32, f32)>, String>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let mut min_size = None;
    while let Some(argument) = args.next() {
        if argument != "--window-min-size" {
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| "--window-min-size requires WIDTHxHEIGHT".to_string())?;
        let (width, height) = value
            .split_once('x')
            .or_else(|| value.split_once('X'))
            .ok_or_else(|| {
                format!(
                    "invalid --window-min-size '{value}'; expected WIDTHxHEIGHT (for example 400x400)"
                )
            })?;
        let width = width.parse::<f32>().ok();
        let height = height.parse::<f32>().ok();
        match (width, height) {
            (Some(width), Some(height))
                if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 =>
            {
                min_size = Some((width, height));
            }
            _ => {
                return Err(format!(
                    "invalid --window-min-size '{value}'; expected positive WIDTHxHEIGHT (for example 400x400)"
                ));
            }
        }
    }
    Ok(min_size)
}
