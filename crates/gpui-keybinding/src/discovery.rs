use std::collections::BTreeMap;

use crate::{
    DocumentedKeybinding, KeybindingCategory, KeymapPreset, format_key_label,
    registry::KeybindingRegistry,
};

/// A searchable command-palette row derived from documented keybindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteEntry {
    /// Display string for the key combination.
    pub key: String,
    /// Optional raw GPUI key spec, when the provider exposed one.
    pub raw_key_spec: Option<String>,
    /// Human-readable command description.
    pub description: String,
    /// Category used for grouping and search.
    pub category: KeybindingCategory,
    search_index: String,
    description_lower: String,
    key_lower: String,
    raw_lower: String,
    category_lower: String,
}

impl CommandPaletteEntry {
    /// Build a palette entry from a documented keybinding.
    pub fn from_binding(binding: &DocumentedKeybinding) -> Self {
        let mut search_index = String::new();
        push_search_text(&mut search_index, &binding.key);
        if let Some(raw) = &binding.raw_key_spec {
            push_search_text(&mut search_index, raw);
            push_search_text(&mut search_index, &format_key_label(raw));
        }
        push_search_text(&mut search_index, &binding.description);
        push_search_text(&mut search_index, binding.category.name());

        Self {
            key: binding.key.clone(),
            raw_key_spec: binding.raw_key_spec.clone(),
            description: binding.description.clone(),
            category: binding.category.clone(),
            search_index,
            description_lower: binding.description.to_ascii_lowercase(),
            key_lower: binding.key.to_ascii_lowercase(),
            raw_lower: binding
                .raw_key_spec
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase(),
            category_lower: binding.category.name().to_ascii_lowercase(),
        }
    }

    /// Precomputed lower-case text used by command-palette search.
    pub fn search_index(&self) -> &str {
        &self.search_index
    }
}

/// A which-key-style hint for the next key in a chord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeybindingHint {
    /// Display string for the next key.
    pub key: String,
    /// Raw key spec for the next key.
    pub raw_key_spec: String,
    /// Description when this key completes a command.
    pub description: Option<String>,
    /// Category of the terminal command, or the first child command.
    pub category: KeybindingCategory,
    /// True when pressing this key completes at least one command.
    pub is_terminal: bool,
    /// True when this key also prefixes longer chords.
    pub has_children: bool,
}

/// Build deterministic command-palette entries from documented keybindings.
pub fn command_palette_entries(bindings: &[DocumentedKeybinding]) -> Vec<CommandPaletteEntry> {
    let mut entries: Vec<_> = bindings
        .iter()
        .map(CommandPaletteEntry::from_binding)
        .collect();
    sort_palette_entries(&mut entries);
    entries
}

/// Search command-palette entries.
///
/// Empty queries return all entries in their existing order. Non-empty queries
/// split on whitespace and require every token to match the precomputed search
/// index.
pub fn search_command_palette(
    entries: &[CommandPaletteEntry],
    query: &str,
) -> Vec<CommandPaletteEntry> {
    let normalized = query.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return entries.to_vec();
    }

    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    let mut matches: Vec<_> = entries
        .iter()
        .filter_map(|entry| score_entry(entry, &normalized, &tokens).map(|score| (score, entry)))
        .collect();

    matches.sort_by(|(score_a, a), (score_b, b)| {
        score_a
            .cmp(score_b)
            .then_with(|| a.category.name().cmp(b.category.name()))
            .then_with(|| a.description.cmp(&b.description))
            .then_with(|| a.key.cmp(&b.key))
    });

    matches.into_iter().map(|(_, entry)| entry.clone()).collect()
}

/// Build which-key-style next-key hints for a chord prefix.
///
/// `prefix` should use GPUI key-spec syntax such as `"ctrl-k"` or
/// `"ctrl-k ctrl-s"`. Bindings without `raw_key_spec` are still considered,
/// but matching is necessarily based on their display string.
pub fn keybinding_hints<'a>(
    bindings: &'a [DocumentedKeybinding],
    prefix: &str,
) -> Vec<KeybindingHint> {
    let prefix_parts: Vec<&str> = prefix.split_whitespace().collect();
    let mut hints: BTreeMap<&'a str, KeybindingHint> = BTreeMap::new();

    for binding in bindings {
        let spec = binding.raw_key_spec.as_deref().unwrap_or(&binding.key);
        let parts: Vec<&str> = spec.split_whitespace().collect();
        if parts.is_empty() || !starts_with_parts(&parts, &prefix_parts) {
            continue;
        }

        if parts.len() == prefix_parts.len() {
            if let Some(last) = parts.last() {
                let entry = hints
                    .entry(*last)
                    .or_insert_with(|| hint_from_binding(last, binding, true, false));
                entry.is_terminal = true;
                entry.description = Some(binding.description.clone());
                entry.category = binding.category.clone();
            }
            continue;
        }

        let next = parts[prefix_parts.len()];
        let completes_command = parts.len() == prefix_parts.len() + 1;
        let entry = hints
            .entry(next)
            .or_insert_with(|| hint_from_binding(next, binding, completes_command, false));

        if completes_command {
            entry.is_terminal = true;
            entry.description = Some(binding.description.clone());
            entry.category = binding.category.clone();
        } else {
            entry.has_children = true;
        }
    }

    hints.into_values().collect()
}

impl KeybindingRegistry {
    /// Get cached command-palette entries for a preset.
    pub fn command_palette_entries(&self, preset: KeymapPreset) -> std::rc::Rc<[CommandPaletteEntry]> {
        self.get_palette_entries(preset)
    }

    /// Search cached command-palette entries for a preset.
    pub fn search_command_palette(
        &self,
        preset: KeymapPreset,
        query: &str,
    ) -> Vec<CommandPaletteEntry> {
        let entries = self.get_palette_entries(preset);
        search_command_palette(&entries, query)
    }

    /// Build which-key-style next-key hints for a preset and chord prefix.
    pub fn keybinding_hints(&self, preset: KeymapPreset, prefix: &str) -> Vec<KeybindingHint> {
        let docs = self.get_documented(preset);
        keybinding_hints(&docs, prefix)
    }
}

fn hint_from_binding(
    raw_key_spec: &str,
    binding: &DocumentedKeybinding,
    is_terminal: bool,
    has_children: bool,
) -> KeybindingHint {
    KeybindingHint {
        key: format_key_label(raw_key_spec),
        raw_key_spec: raw_key_spec.to_string(),
        description: is_terminal.then(|| binding.description.clone()),
        category: binding.category.clone(),
        is_terminal,
        has_children,
    }
}

fn starts_with_parts(parts: &[&str], prefix_parts: &[&str]) -> bool {
    prefix_parts.len() <= parts.len()
        && parts
            .iter()
            .zip(prefix_parts.iter())
            .all(|(part, prefix)| part == prefix)
}

fn push_search_text(out: &mut String, value: &str) {
    if !out.is_empty() {
        out.push(' ');
    }
    out.push_str(&value.to_ascii_lowercase());
}

fn score_entry(entry: &CommandPaletteEntry, normalized: &str, tokens: &[&str]) -> Option<usize> {
    if !tokens
        .iter()
        .all(|token| entry.search_index.contains(token))
    {
        return None;
    }

    let rank = if entry.description_lower.starts_with(normalized) {
        0
    } else if entry.key_lower.starts_with(normalized) {
        10
    } else if entry.raw_lower.starts_with(normalized) {
        20
    } else if entry.category_lower.starts_with(normalized) {
        30
    } else {
        40
    };

    Some(rank + entry.description_lower.len().min(1000))
}

fn sort_palette_entries(entries: &mut [CommandPaletteEntry]) {
    entries.sort_by(|a, b| {
        a.category
            .name()
            .cmp(b.category.name())
            .then_with(|| a.description.cmp(&b.description))
            .then_with(|| a.key.cmp(&b.key))
    });
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use gpui::KeyBinding;

    use super::*;
    use crate::{KeybindingProvider, KeymapPreset};

    fn docs() -> Vec<DocumentedKeybinding> {
        vec![
            DocumentedKeybinding::new("Ctrl+P", "Open command palette", KeybindingCategory::View)
                .with_raw_key_spec("ctrl-shift-p"),
            DocumentedKeybinding::new("Ctrl+K Ctrl+S", "Save all", KeybindingCategory::FileOps)
                .with_raw_key_spec("ctrl-k ctrl-s"),
            DocumentedKeybinding::new("Ctrl+K Ctrl+O", "Open folder", KeybindingCategory::FileOps)
                .with_raw_key_spec("ctrl-k ctrl-o"),
            DocumentedKeybinding::new(
                "Ctrl+K Ctrl+K",
                "Show keyboard shortcuts",
                KeybindingCategory::System,
            )
            .with_raw_key_spec("ctrl-k ctrl-k"),
        ]
    }

    #[test]
    fn command_palette_search_matches_description_category_and_key() {
        let entries = command_palette_entries(&docs());

        let by_description = search_command_palette(&entries, "palette");
        assert_eq!(by_description.len(), 1);
        assert_eq!(by_description[0].description, "Open command palette");

        let by_category = search_command_palette(&entries, "file save");
        assert_eq!(by_category.len(), 1);
        assert_eq!(by_category[0].description, "Save all");

        let by_raw_key = search_command_palette(&entries, "ctrl-shift-p");
        assert_eq!(by_raw_key.len(), 1);
        assert_eq!(by_raw_key[0].key, "Ctrl+P");
    }

    #[test]
    fn keybinding_hints_group_chord_prefixes() {
        let hints = keybinding_hints(&docs(), "ctrl-k");
        let keys: Vec<_> = hints
            .iter()
            .map(|hint| hint.raw_key_spec.as_str())
            .collect();

        assert_eq!(keys, vec!["ctrl-k", "ctrl-o", "ctrl-s"]);
        assert!(hints[0].is_terminal);
        assert_eq!(
            hints[0].description.as_deref(),
            Some("Show keyboard shortcuts")
        );
        assert!(hints[1].is_terminal);
        assert!(hints[2].is_terminal);
    }

    struct CountingProvider {
        documented_calls: Rc<Cell<usize>>,
    }

    impl KeybindingProvider for CountingProvider {
        fn bindings(&self, _preset: KeymapPreset) -> Vec<KeyBinding> {
            Vec::new()
        }

        fn documented_bindings(&self, _preset: KeymapPreset) -> Vec<DocumentedKeybinding> {
            self.documented_calls
                .set(self.documented_calls.get().saturating_add(1));
            docs()
        }
    }

    #[test]
    fn registry_reuses_cached_documented_bindings_for_discovery() {
        let calls = Rc::new(Cell::new(0));
        let mut registry = KeybindingRegistry::new();
        registry.register(CountingProvider {
            documented_calls: calls.clone(),
        });

        assert_eq!(
            registry
                .search_command_palette(KeymapPreset::Default, "palette")
                .len(),
            1
        );
        assert_eq!(
            registry
                .keybinding_hints(KeymapPreset::Default, "ctrl-k")
                .len(),
            3
        );
        assert_eq!(calls.get(), 1);
    }
}
