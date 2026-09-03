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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentedKeybinding {
    /// Display string for the key combination (e.g. "Ctrl+B", "⌘B").
    pub key: String,
    /// Optional raw key spec used for conflict detection (e.g. "ctrl-shift-k").
    /// When present, conflict detection groups by this value instead of `key`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_key_spec: Option<String>,
    /// Description of what the keybinding does.
    pub description: String,
    /// Category for grouping in UI.
    pub category: KeybindingCategory,
    /// Optional VSCode-style `when`-clause context expression
    /// (e.g. `"editorTextFocus"`).
    ///
    /// Stored as an opaque string: this crate never executes it. Conflict
    /// detection treats bindings with different active contexts as
    /// non-conflicting, and command-palette search indexes the text.
    /// `None` (or a blank string) means the binding is always active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
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
            context: None,
        }
    }

    pub fn with_raw_key_spec(mut self, spec: impl Into<String>) -> Self {
        self.raw_key_spec = Some(spec.into());
        self
    }

    /// Attach a VSCode-style `when`-clause context expression.
    ///
    /// Empty or whitespace-only values normalize to `None` (always active).
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        let context = context.into();
        self.context = if context.trim().is_empty() {
            None
        } else {
            Some(context)
        };
        self
    }

    /// Normalized context used for conflict grouping and search.
    ///
    /// Returns `None` for always-active bindings (missing or blank context).
    pub fn normalized_context(&self) -> Option<&str> {
        self.context
            .as_deref()
            .map(str::trim)
            .filter(|context| !context.is_empty())
    }
}

#[derive(Deserialize)]
struct OverrideFile {
    #[serde(default)]
    bindings: Vec<DocumentedKeybinding>,
}

/// Parse user keybinding overrides from a `keybindings.json`-style document.
///
/// Accepts either a bare JSON array of [`DocumentedKeybinding`] values or an
/// object with a `"bindings"` array. A blank document yields an empty vec.
/// Malformed JSON, or a document of any other shape, returns `Err`.
pub fn parse_user_overrides(json: &str) -> Result<Vec<DocumentedKeybinding>, serde_json::Error> {
    if json.trim().is_empty() {
        return Ok(Vec::new());
    }
    match serde_json::from_str::<Vec<DocumentedKeybinding>>(json) {
        Ok(bindings) => Ok(bindings),
        Err(array_err) => match serde_json::from_str::<OverrideFile>(json) {
            Ok(file) => Ok(file.bindings),
            Err(_) => Err(array_err),
        },
    }
}

/// Serialize user keybinding overrides to a `keybindings.json`-style document.
pub fn serialize_user_overrides(
    bindings: &[DocumentedKeybinding],
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(bindings)
}

/// Apply user overrides on top of a base binding list.
///
/// An override replaces the first base entry with the same description;
/// overrides with no description match are appended. Duplicate overrides for
/// the same description resolve last-wins. An empty override list returns the
/// base list unchanged.
pub fn apply_user_overrides(
    mut base: Vec<DocumentedKeybinding>,
    overrides: Vec<DocumentedKeybinding>,
) -> Vec<DocumentedKeybinding> {
    for binding in overrides {
        if let Some(existing) = base
            .iter_mut()
            .find(|entry| entry.description == binding.description)
        {
            *existing = binding;
        } else {
            base.push(binding);
        }
    }
    base
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_category_name() {
        assert_eq!(
            KeybindingCategory::Custom("My Category".to_string()).name(),
            "My Category"
        );
    }

    #[test]
    fn context_builder_normalizes_blank_to_none() {
        let binding = DocumentedKeybinding::new("Ctrl+S", "Save", KeybindingCategory::FileOps);
        assert_eq!(binding.normalized_context(), None);

        let binding = DocumentedKeybinding::new("Ctrl+S", "Save", KeybindingCategory::FileOps)
            .with_context("  ");
        assert_eq!(binding.context, None);
        assert_eq!(binding.normalized_context(), None);

        let binding = DocumentedKeybinding::new("Ctrl+S", "Save", KeybindingCategory::FileOps)
            .with_context("editorTextFocus");
        assert_eq!(binding.normalized_context(), Some("editorTextFocus"));
    }

    #[test]
    fn normalized_context_trims_surrounding_whitespace() {
        let binding = DocumentedKeybinding::new("Ctrl+S", "Save", KeybindingCategory::FileOps)
            .with_context("  editorTextFocus  ");
        assert_eq!(binding.normalized_context(), Some("editorTextFocus"));
    }

    #[test]
    fn user_overrides_round_trip_through_json() {
        let bindings = vec![
            DocumentedKeybinding::new("Ctrl+S", "Save", KeybindingCategory::FileOps)
                .with_raw_key_spec("secondary-s")
                .with_context("editorTextFocus"),
            DocumentedKeybinding::new("Ctrl+P", "Palette", KeybindingCategory::View),
        ];
        let json = serialize_user_overrides(&bindings).expect("serializes");
        // Optional fields absent on the plain entry stay absent in JSON.
        assert!(!json.contains("raw_key_spec\": null"));
        let parsed = parse_user_overrides(&json).expect("parses");
        assert_eq!(parsed, bindings);
    }

    #[test]
    fn parse_user_overrides_accepts_bindings_object_and_blank() {
        assert_eq!(
            parse_user_overrides("").expect("blank is empty"),
            Vec::new()
        );
        assert_eq!(
            parse_user_overrides("   \n  ").expect("whitespace is empty"),
            Vec::new()
        );

        let binding = DocumentedKeybinding::new("Ctrl+S", "Save", KeybindingCategory::FileOps);
        let wrapped = format!(
            "{{\"bindings\": {}}}",
            serialize_user_overrides(&[binding]).unwrap()
        );
        let parsed = parse_user_overrides(&wrapped).expect("object form parses");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].description, "Save");

        let empty_object = parse_user_overrides("{}").expect("missing array defaults");
        assert!(empty_object.is_empty());
    }

    #[test]
    fn parse_user_overrides_rejects_malformed_documents() {
        assert!(parse_user_overrides("{not json").is_err());
        assert!(parse_user_overrides("{\"bindings\": 42}").is_err());
        assert!(parse_user_overrides("[{\"key\": 42}]").is_err());
    }

    #[test]
    fn apply_user_overrides_replaces_by_description_and_appends() {
        let base = vec![
            DocumentedKeybinding::new("Ctrl+S", "Save", KeybindingCategory::FileOps),
            DocumentedKeybinding::new("Ctrl+O", "Open", KeybindingCategory::FileOps),
        ];
        let overrides = vec![
            DocumentedKeybinding::new("Cmd+S", "Save", KeybindingCategory::FileOps)
                .with_raw_key_spec("secondary-s"),
            DocumentedKeybinding::new("Ctrl+N", "New", KeybindingCategory::FileOps),
        ];
        let merged = apply_user_overrides(base, overrides);
        assert_eq!(merged.len(), 3);
        let save = merged.iter().find(|b| b.description == "Save").unwrap();
        assert_eq!(save.key, "Cmd+S");
        assert_eq!(save.raw_key_spec.as_deref(), Some("secondary-s"));
        assert!(merged.iter().any(|b| b.description == "New"));
        assert!(merged.iter().any(|b| b.description == "Open"));
    }

    #[test]
    fn apply_user_overrides_last_wins_and_empty_is_noop() {
        let base = vec![DocumentedKeybinding::new(
            "Ctrl+S",
            "Save",
            KeybindingCategory::FileOps,
        )];
        let merged = apply_user_overrides(base.clone(), Vec::new());
        assert_eq!(merged, base);

        let overrides = vec![
            DocumentedKeybinding::new("A", "Save", KeybindingCategory::FileOps),
            DocumentedKeybinding::new("B", "Save", KeybindingCategory::FileOps)
                .with_context("editorTextFocus"),
        ];
        let merged = apply_user_overrides(base, overrides);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].key, "B");
        assert_eq!(merged[0].normalized_context(), Some("editorTextFocus"));
    }
}
