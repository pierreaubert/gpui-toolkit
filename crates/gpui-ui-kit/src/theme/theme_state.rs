use super::Theme;
use super::theme_ext::ThemeExt;
use super::theme_variant::ThemeVariant;
use gpui::{App, Global};
use std::sync::Arc;

/// Global state for theme management
pub struct ThemeState {
    pub theme: Arc<Theme>,
}

impl Global for ThemeState {}

impl ThemeState {
    /// Create new theme state with default (dark) theme
    pub fn new() -> Self {
        Self {
            theme: Arc::new(Theme::default()),
        }
    }

    /// Create theme state with specific variant
    pub fn with_variant(variant: ThemeVariant) -> Self {
        Self {
            theme: Arc::new(Theme::for_variant(variant)),
        }
    }

    /// Set theme variant
    pub fn set_variant(&mut self, variant: ThemeVariant) {
        self.theme = Arc::new(Theme::for_variant(variant));
    }

    /// Toggle between light and dark themes
    pub fn toggle(&mut self) {
        self.set_variant(self.theme.variant.toggle());
    }
}

impl Default for ThemeState {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeExt for App {
    fn theme(&self) -> Arc<Theme> {
        self.try_global::<ThemeState>()
            .map(|s| s.theme.clone())
            .unwrap_or_else(|| {
                // The fallback theme is allocated once and reused across calls.
                static FALLBACK: std::sync::OnceLock<Arc<Theme>> = std::sync::OnceLock::new();
                FALLBACK.get_or_init(|| Arc::new(Theme::dark())).clone()
            })
    }
}
