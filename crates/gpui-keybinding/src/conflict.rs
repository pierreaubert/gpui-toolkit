use std::collections::HashMap;

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

/// Detect conflicting keybindings in a set of documented bindings.
///
/// Returns conflicts where the same raw key spec (or display key when no raw
/// spec is present) appears more than once.
/// Operates on `DocumentedKeybinding` (which has the display key) rather than
/// GPUI `KeyBinding` (which doesn't expose its key spec).
pub fn detect_conflicts(bindings: &[DocumentedKeybinding]) -> Vec<KeyConflict> {
    let mut groups: HashMap<&str, Vec<&str>> = HashMap::new();

    for binding in bindings {
        let conflict_key = binding.raw_key_spec.as_deref().unwrap_or(&binding.key);
        groups
            .entry(conflict_key)
            .or_default()
            .push(&binding.description);
    }

    groups
        .into_iter()
        .filter(|(_, descs)| descs.len() > 1)
        .map(|(key, descriptions)| KeyConflict {
            count: descriptions.len(),
            key: key.to_string(),
            descriptions: descriptions.into_iter().map(str::to_string).collect(),
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
    fn test_no_conflict_when_raw_key_spec_differs() {
        let bindings = vec![
            DocumentedKeybinding::new("⌘S", "Save", KeybindingCategory::FileOps)
                .with_raw_key_spec("secondary-s"),
            DocumentedKeybinding::new("Ctrl+O", "Open", KeybindingCategory::FileOps)
                .with_raw_key_spec("secondary-o"),
        ];
        assert!(detect_conflicts(&bindings).is_empty());
    }
}
