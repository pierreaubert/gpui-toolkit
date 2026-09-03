use std::collections::HashMap;

use gpui::Keystroke;

use crate::DocumentedKeybinding;

/// A detected keybinding conflict: multiple bindings with the same display key.
#[derive(Debug, Clone)]
pub struct KeyConflict {
    /// The display key string that has duplicates.
    pub key: String,
    /// Descriptions of the conflicting bindings.
    pub descriptions: Vec<String>,
    /// Number of bindings with this key.
    pub count: usize,
}

#[derive(Debug, Eq, Hash, PartialEq)]
enum ConflictIdentity {
    Parsed(Vec<Keystroke>),
    Literal(String),
}

struct ConflictGroup<'a> {
    key: &'a str,
    descriptions: Vec<&'a str>,
}

fn conflict_identity(key_spec: &str) -> ConflictIdentity {
    match key_spec
        .split_whitespace()
        .map(Keystroke::parse)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(keystrokes) if !keystrokes.is_empty() => ConflictIdentity::Parsed(keystrokes),
        _ => ConflictIdentity::Literal(key_spec.to_ascii_lowercase()),
    }
}

/// Detect conflicting keybindings in a set of documented bindings.
///
/// Returns conflicts where the same raw key spec (or display key when no raw
/// spec is present) appears more than once *in the same `when`-clause
/// context*. Bindings scoped to different contexts never conflict with each
/// other; always-active bindings (no context) only conflict with other
/// always-active bindings. Cross-context shadowing still needs an executable
/// context evaluator, which is out of scope for this crate.
/// Operates on `DocumentedKeybinding` (which has the display key) rather than
/// GPUI `KeyBinding` (which doesn't expose its key spec).
pub fn detect_conflicts(bindings: &[DocumentedKeybinding]) -> Vec<KeyConflict> {
    let mut groups: HashMap<(ConflictIdentity, Option<String>), ConflictGroup<'_>> = HashMap::new();

    for binding in bindings {
        let conflict_key = binding.raw_key_spec.as_deref().unwrap_or(&binding.key);
        let context_key = binding.normalized_context().map(str::to_string);
        groups
            .entry((conflict_identity(conflict_key), context_key))
            .or_insert_with(|| ConflictGroup {
                key: conflict_key,
                descriptions: Vec::new(),
            })
            .descriptions
            .push(&binding.description);
    }

    groups
        .into_values()
        .filter(|group| group.descriptions.len() > 1)
        .map(|group| KeyConflict {
            count: group.descriptions.len(),
            key: group.key.to_string(),
            descriptions: group.descriptions.into_iter().map(str::to_string).collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KeybindingCategory;

    #[test]
    fn test_no_conflicts() {
        let bindings = vec![
            DocumentedKeybinding::new("Ctrl+A", "Select all", KeybindingCategory::Editing),
            DocumentedKeybinding::new("Ctrl+B", "Bold", KeybindingCategory::Formatting),
        ];
        assert!(detect_conflicts(&bindings).is_empty());
    }

    #[test]
    fn test_detects_conflict() {
        let bindings = vec![
            DocumentedKeybinding::new("Ctrl+F", "Find", KeybindingCategory::Search),
            DocumentedKeybinding::new("Ctrl+F", "Forward", KeybindingCategory::Navigation),
        ];
        let conflicts = detect_conflicts(&bindings);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].key, "Ctrl+F");
        assert_eq!(conflicts[0].count, 2);
    }

    #[test]
    fn test_detects_conflict_on_raw_key_spec() {
        // Two bindings with different display strings but same raw key spec.
        let bindings = vec![
            DocumentedKeybinding::new("⌘S", "Save", KeybindingCategory::FileOps)
                .with_raw_key_spec("secondary-s"),
            DocumentedKeybinding::new("Ctrl+S", "Save", KeybindingCategory::FileOps)
                .with_raw_key_spec("secondary-s"),
        ];
        let conflicts = detect_conflicts(&bindings);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].key, "secondary-s");
        assert_eq!(conflicts[0].count, 2);
    }

    #[test]
    fn test_detects_conflict_for_case_insensitive_raw_key_specs() {
        let bindings = vec![
            DocumentedKeybinding::new("⌘S", "Save", KeybindingCategory::FileOps)
                .with_raw_key_spec("Secondary-s"),
            DocumentedKeybinding::new("Ctrl+S", "Save copy", KeybindingCategory::FileOps)
                .with_raw_key_spec("secondary-s"),
        ];

        assert_eq!(detect_conflicts(&bindings).len(), 1);
    }

    #[test]
    fn test_uppercase_raw_key_is_distinct_from_unshifted_key() {
        let bindings = vec![
            DocumentedKeybinding::new("G", "Go to end", KeybindingCategory::Navigation)
                .with_raw_key_spec("G"),
            DocumentedKeybinding::new("g", "Go to start", KeybindingCategory::Navigation)
                .with_raw_key_spec("g"),
        ];

        assert!(detect_conflicts(&bindings).is_empty());
    }

    #[test]
    fn test_no_conflict_when_raw_key_spec_differs() {
        let bindings = vec![
            DocumentedKeybinding::new("⌘S", "Save", KeybindingCategory::FileOps)
                .with_raw_key_spec("secondary-s"),
            DocumentedKeybinding::new("Ctrl+O", "Open", KeybindingCategory::FileOps)
                .with_raw_key_spec("secondary-o"),
        ];
        assert!(detect_conflicts(&bindings).is_empty());
    }

    #[test]
    fn test_no_conflict_when_contexts_differ() {
        let bindings = vec![
            DocumentedKeybinding::new("Ctrl+S", "Save in editor", KeybindingCategory::FileOps)
                .with_context("editorTextFocus"),
            DocumentedKeybinding::new("Ctrl+S", "Save in terminal", KeybindingCategory::FileOps)
                .with_context("terminalFocus"),
        ];
        assert!(detect_conflicts(&bindings).is_empty());
    }

    #[test]
    fn test_no_conflict_between_scoped_and_unscoped_binding() {
        let bindings = vec![
            DocumentedKeybinding::new("Ctrl+S", "Save", KeybindingCategory::FileOps),
            DocumentedKeybinding::new("Ctrl+S", "Save in editor", KeybindingCategory::FileOps)
                .with_context("editorTextFocus"),
        ];
        assert!(detect_conflicts(&bindings).is_empty());
    }

    #[test]
    fn test_detects_conflict_within_same_context() {
        let bindings = vec![
            DocumentedKeybinding::new("Ctrl+S", "Save", KeybindingCategory::FileOps)
                .with_context("editorTextFocus"),
            DocumentedKeybinding::new("Ctrl+S", "Save all", KeybindingCategory::FileOps)
                .with_context("editorTextFocus"),
        ];
        let conflicts = detect_conflicts(&bindings);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].count, 2);
    }
}
