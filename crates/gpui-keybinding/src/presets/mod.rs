//! Generic navigation presets for different keymap styles.
//!
//! These provide the standard key mappings for common navigation actions
//! in each preset. Applications can use these to build their keybinding
//! tables without hardcoding preset-specific knowledge.

mod default;
mod emacs;
mod vim;
mod vscode;

pub use default::DEFAULT_NAVIGATION;
pub use emacs::EMACS_NAVIGATION;
pub use vim::VIM_NAVIGATION;
pub use vscode::VSCODE_NAVIGATION;

use crate::KeymapPreset;

/// Standard navigation actions that any application can map.
///
/// These represent common UI navigation patterns. Use [`navigation_key`]
/// to get the key spec for a given action in a given preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavigationAction {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    First,
    Last,
    Enter,
    Cancel,
    Expand,
    Collapse,
    NextTab,
    PrevTab,
    Search,
    Delete,
    Undo,
    Redo,
}

/// A key mapping entry: action to key spec(s).
#[derive(Debug, Clone)]
pub struct NavigationMapping {
    pub action: NavigationAction,
    /// Primary key spec (GPUI format, e.g., "ctrl-p", "k").
    pub primary: &'static str,
    /// Optional alternative key spec.
    pub alt: Option<&'static str>,
    /// Human-readable display string.
    pub display: &'static str,
}

/// Get the primary key spec for a navigation action in a given preset.
///
/// Returns the GPUI key spec string (e.g., "k" for Vim Up, "ctrl-p" for Emacs Up).
///
/// # Example
///
/// ```ignore
/// use gpui_keybinding::presets::{NavigationAction, navigation_key};
/// use gpui_keybinding::KeymapPreset;
///
/// let key = navigation_key(NavigationAction::Up, KeymapPreset::Vim);
/// assert_eq!(key, "k");
/// ```
pub fn navigation_key(action: NavigationAction, preset: KeymapPreset) -> Option<&'static str> {
    let mappings = navigation_mappings(preset);
    mappings
        .iter()
        .find(|m| m.action == action)
        .map(|m| m.primary)
}

/// Get all navigation mappings for a preset.
pub fn navigation_mappings(preset: KeymapPreset) -> &'static [NavigationMapping] {
    match preset {
        KeymapPreset::Default => DEFAULT_NAVIGATION,
        KeymapPreset::Vim => VIM_NAVIGATION,
        KeymapPreset::Emacs => EMACS_NAVIGATION,
        KeymapPreset::VSCode => VSCODE_NAVIGATION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_presets_have_up() {
        for preset in KeymapPreset::all() {
            let key = navigation_key(NavigationAction::Up, *preset);
            assert!(key.is_some(), "preset {:?} missing Up mapping", preset);
        }
    }

    #[test]
    fn test_vim_navigation() {
        assert_eq!(
            navigation_key(NavigationAction::Up, KeymapPreset::Vim),
            Some("k")
        );
        assert_eq!(
            navigation_key(NavigationAction::Down, KeymapPreset::Vim),
            Some("j")
        );
        assert_eq!(
            navigation_key(NavigationAction::Left, KeymapPreset::Vim),
            Some("h")
        );
        assert_eq!(
            navigation_key(NavigationAction::Right, KeymapPreset::Vim),
            Some("l")
        );
    }

    #[test]
    fn test_emacs_navigation() {
        assert_eq!(
            navigation_key(NavigationAction::Up, KeymapPreset::Emacs),
            Some("ctrl-p")
        );
        assert_eq!(
            navigation_key(NavigationAction::Down, KeymapPreset::Emacs),
            Some("ctrl-n")
        );
    }
}
