use super::Theme;
use super::theme_ext::ThemeExt;
use super::theme_variant::ThemeVariant;
use gpui::{App, Global};
use std::sync::Arc;

/// Global state for theme management
pub struct ThemeState {
    pub theme: Arc<Theme>,
    /// Track live OS appearance changes within the current family.
    pub follow_system: bool,
}

impl Global for ThemeState {}

impl ThemeState {
    /// Create new theme state with default (dark) theme
    pub fn new() -> Self {
        Self {
            theme: Arc::new(Theme::default()),
            follow_system: false,
        }
    }

    /// Create theme state with specific variant
    pub fn with_variant(variant: ThemeVariant) -> Self {
        Self {
            theme: Arc::new(Theme::for_variant(variant)),
            follow_system: false,
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

    /// Enable or disable tracking of OS appearance changes.
    pub fn set_follow_system(&mut self, follow: bool) {
        self.follow_system = follow;
    }

    /// Apply a live OS dark-mode flag when following the system.
    ///
    /// No-op unless [`ThemeState::set_follow_system`] enabled tracking;
    /// a manual [`ThemeState::set_variant`] choice is never overridden.
    pub fn apply_system_appearance(&mut self, system_dark: bool) {
        if self.follow_system {
            self.set_variant(self.theme.variant.with_system_appearance(system_dark));
        }
    }
}

impl Default for ThemeState {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeExt for App {
    fn theme(&self) -> Arc<Theme> {
        self.try_global::<ThemeState>().map_or_else(
            || {
                // The fallback theme is allocated once and reused across calls.
                static FALLBACK: std::sync::OnceLock<Arc<Theme>> = std::sync::OnceLock::new();
                FALLBACK.get_or_init(|| Arc::new(Theme::dark())).clone()
            },
            |s| s.theme.clone(),
        )
    }
}
