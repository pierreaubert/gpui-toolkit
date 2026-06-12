use gpui::KeyBinding;
use serde::{Deserialize, Serialize};

use crate::KeymapPreset;

/// Category for organizing keybindings in help/settings UI.
///
/// Provides built-in categories for common use cases plus a `Custom` variant
/// for application-specific categories.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeybindingCategory {
    Navigation,
    Editing,
    FileOps,
    Formatting,
    View,
    Search,
    Playback,
    System,
    /// Application-specific category with a custom name.
    Custom(String),
}

impl KeybindingCategory {
    /// Human-readable category name.
    pub fn name(&self) -> &str {
        match self {
            Self::Navigation => "Navigation",
            Self::Editing => "Editing",
            Self::FileOps => "File",
            Self::Formatting => "Formatting",
            Self::View => "View",
            Self::Search => "Search",
            Self::Playback => "Playback",
            Self::System => "System",
            Self::Custom(name) => name,
        }
    }
}

/// A human-readable keybinding entry for help display and settings UI.
#[derive(Debug, Clone)]
pub struct DocumentedKeybinding {
    /// Display string for the key combination (e.g. "Ctrl+B", "⌘B").
    pub key: String,
    /// Optional raw key spec used for conflict detection (e.g. "ctrl-shift-k").
    /// When present, conflict detection groups by this value instead of `key`.
    pub raw_key_spec: Option<String>,
    /// Description of what the keybinding does.
    pub description: String,
    /// Category for grouping in UI.
    pub category: KeybindingCategory,
}

impl DocumentedKeybinding {
    pub fn new(
        key: impl Into<String>,
        description: impl Into<String>,
        category: KeybindingCategory,
    ) -> Self {
        Self {
            key: key.into(),
            raw_key_spec: None,
            description: description.into(),
            category,
        }
    }

    pub fn with_raw_key_spec(mut self, spec: impl Into<String>) -> Self {
        self.raw_key_spec = Some(spec.into());
        self
    }
}

/// Trait for applications to provide keybindings per preset.
///
/// Implement this trait to register your application's action→key mappings.
/// Each provider can supply bindings for any or all presets.
///
/// # Example
///
/// ```ignore
/// struct MyAppBindings;
///
/// impl KeybindingProvider for MyAppBindings {
///     fn bindings(&self, preset: KeymapPreset) -> Vec<KeyBinding> {
///         match preset {
///             KeymapPreset::Default => vec![
///                 KeyBinding::new("secondary-s", SaveFile, None),
///                 KeyBinding::new("secondary-o", OpenFile, None),
///             ],
///             KeymapPreset::Vim => vec![
///                 KeyBinding::new(":w", SaveFile, Some("EditorView")),
///             ],
///             _ => vec![]
///         }
///     }
///
///     fn documented_bindings(&self, preset: KeymapPreset) -> Vec<DocumentedKeybinding> {
///         // Return human-readable descriptions for help UI
///         vec![]
///     }
/// }
/// ```
pub trait KeybindingProvider {
    /// Return GPUI `KeyBinding`s for the given preset.
    fn bindings(&self, preset: KeymapPreset) -> Vec<KeyBinding>;

    /// Return documented keybindings for help/settings UI.
    fn documented_bindings(&self, preset: KeymapPreset) -> Vec<DocumentedKeybinding>;
}
