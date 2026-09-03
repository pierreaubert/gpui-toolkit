use gpui::*;
use gpui_ui_kit::i18n::Language;
use gpui_ui_kit::theme::ThemeVariant;
use std::path::PathBuf;

/// Configuration for a MiniApp instance
#[derive(Clone, PartialEq)]
pub struct MiniAppConfig {
    /// Window title
    pub title: SharedString,
    /// Window width in pixels
    pub width: f32,
    /// Window height in pixels
    pub height: f32,
    /// Optional minimum native window size.
    pub min_size: Option<Size<Pixels>>,
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
    /// Optional file used to persist window size, theme, and language.
    ///
    /// When set, `MiniApp::run` loads stored values at startup (a missing or
    /// corrupt file falls back to the builder values) and saves the current
    /// values when a window closes. Off by default.
    pub state_file: Option<PathBuf>,
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
            min_size: None,
            app_name: title,
            scrollable: true,
            with_theme: false,
            with_i18n: false,
            initial_theme: ThemeVariant::default(),
            initial_language: Language::default(),
            state_file: None,
        }
    }

    /// Set the window size
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set the minimum native window size.
    ///
    /// This is enforced by GPUI's macOS, Windows, and Linux window backends.
    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        self.min_size = Some(size(px(width), px(height)));
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

    /// Enable theme variant switching
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

    /// Persist window size, theme, and language to `path` across runs.
    ///
    /// Values are loaded at startup and saved when a window closes. See the
    /// field documentation for fallback behavior.
    pub fn state_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.state_file = Some(path.into());
        self
    }
}

impl Default for MiniAppConfig {
    fn default() -> Self {
        Self::new("MiniApp")
    }
}
