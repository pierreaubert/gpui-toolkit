use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::{
    DocumentedKeybinding, KeybindingCategory, KeymapPreset, format_key_label,
    registry::KeybindingRegistry,
};

/// A searchable command-palette row derived from documented keybindings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

fn hash_documented_bindings(bindings: &[DocumentedKeybinding]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for binding in bindings {
        binding.key.hash(&mut hasher);
        binding.raw_key_spec.hash(&mut hasher);
        binding.description.hash(&mut hasher);
        binding.category.hash(&mut hasher);
    }
    hasher.finish()
}

fn hash_palette_entries(entries: &[CommandPaletteEntry]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for entry in entries {
        entry.hash(&mut hasher);
    }
    hasher.finish()
}

type SearchKey = (u64, String);
type HintsKey = (u64, String);

thread_local! {
    static SEARCH_CACHE: RefCell<HashMap<SearchKey, Rc<[CommandPaletteEntry]>>> =
        RefCell::new(HashMap::new());
    static HINTS_CACHE: RefCell<HashMap<HintsKey, Rc<[KeybindingHint]>>> =
        RefCell::new(HashMap::new());
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
///
/// This is the convenience wrapper that returns a [`Vec`]; prefer
/// [`search_command_palette_cached`] to reuse the same allocation across calls.
pub fn search_command_palette(
    entries: &[CommandPaletteEntry],
    query: &str,
) -> Vec<CommandPaletteEntry> {
    search_command_palette_cached(entries, query).to_vec()
}

/// Cached version of [`search_command_palette`].
///
/// Results are stored in a thread-local cache keyed by a hash of the entries and
/// the normalized query, and returned as an [`Rc`] slice so callers can clone
/// the handle cheaply.
pub fn search_command_palette_cached(
    entries: &[CommandPaletteEntry],
    query: &str,
) -> Rc<[CommandPaletteEntry]> {
    let normalized = query.trim().to_ascii_lowercase();
    let entries_hash = hash_palette_entries(entries);
    let key = (entries_hash, normalized.clone());

    SEARCH_CACHE.with(|cache| {
        if let Some(cached) = cache.borrow().get(&key) {
            return Rc::clone(cached);
        }

        let result: Rc<[CommandPaletteEntry]> = if normalized.is_empty() {
            entries.to_vec().into()
        } else {
            let tokens: Vec<&str> = normalized.split_whitespace().collect();
            let mut matches: Vec<_> = entries
                .iter()
                .filter_map(|entry| {
                    score_entry(entry, &normalized, &tokens).map(|score| (score, entry))
                })
                .collect();

            matches.sort_by(|(score_a, a), (score_b, b)| {
                score_a
                    .cmp(score_b)
                    .then_with(|| a.category.name().cmp(b.category.name()))
                    .then_with(|| a.description.cmp(&b.description))
                    .then_with(|| a.key.cmp(&b.key))
            });

            matches
                .into_iter()
                .map(|(_, entry)| entry.clone())
                .collect::<Vec<_>>()
                .into()
        };

        cache.borrow_mut().insert(key, Rc::clone(&result));
        result
    })
}

/// Build which-key-style next-key hints for a chord prefix.
///
/// `prefix` should use GPUI key-spec syntax such as `"ctrl-k"` or
/// `"ctrl-k ctrl-s"`. Bindings without `raw_key_spec` are still considered,
/// but matching is necessarily based on their display string.
///
/// This is the convenience wrapper that returns a [`Vec`]; prefer
/// [`keybinding_hints_cached`] to reuse the same allocation across calls.
pub fn keybinding_hints(bindings: &[DocumentedKeybinding], prefix: &str) -> Vec<KeybindingHint> {
    keybinding_hints_cached(bindings, prefix).to_vec()
}

/// Cached version of [`keybinding_hints`].
///
/// Results are stored in a thread-local cache keyed by a hash of the bindings and
/// the normalized prefix, and returned as an [`Rc`] slice so callers can clone
/// the handle cheaply.
pub fn keybinding_hints_cached(
    bindings: &[DocumentedKeybinding],
    prefix: &str,
) -> Rc<[KeybindingHint]> {
    let prefix_normalized = prefix.trim().to_ascii_lowercase();
    let bindings_hash = hash_documented_bindings(bindings);
    let key = (bindings_hash, prefix_normalized);

    HINTS_CACHE.with(|cache| {
        if let Some(cached) = cache.borrow().get(&key) {
            return Rc::clone(cached);
        }

        let result = build_keybinding_hints(bindings, prefix);
        let result: Rc<[KeybindingHint]> = result.into();
        cache.borrow_mut().insert(key, Rc::clone(&result));
        result
    })
}

fn build_keybinding_hints(bindings: &[DocumentedKeybinding], prefix: &str) -> Vec<KeybindingHint> {
    let prefix_parts: Vec<&str> = prefix.split_whitespace().collect();
    let mut hints: BTreeMap<&str, KeybindingHint> = BTreeMap::new();

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
    pub fn command_palette_entries(
        &self,
        preset: KeymapPreset,
    ) -> std::rc::Rc<[CommandPaletteEntry]> {
        self.get_palette_entries(preset)
    }

    /// Search cached command-palette entries for a preset.
    ///
    /// This convenience method returns a [`Vec`]. Use
    /// [`search_command_palette_cached`] to reuse the same allocation.
    pub fn search_command_palette(
        &self,
        preset: KeymapPreset,
        query: &str,
    ) -> Vec<CommandPaletteEntry> {
        self.search_command_palette_cached(preset, query).to_vec()
    }

    /// Cached search of command-palette entries for a preset.
    ///
    /// Returns an [`Rc`] slice so callers can clone the result cheaply.
    pub fn search_command_palette_cached(
        &self,
        preset: KeymapPreset,
        query: &str,
    ) -> Rc<[CommandPaletteEntry]> {
        let normalized = query.trim().to_ascii_lowercase();
        let key = (preset, normalized);

        if let Some(cached) = self.search_cache.borrow().get(&key) {
            return Rc::clone(cached);
        }

        let entries = self.get_palette_entries(preset);
        let result = search_command_palette_cached(&entries, query);
        self.search_cache
            .borrow_mut()
            .insert(key, Rc::clone(&result));
        result
    }

    /// Build which-key-style next-key hints for a preset and chord prefix.
    ///
    /// This convenience method returns a [`Vec`]. Use
    /// [`keybinding_hints_cached`] to reuse the same allocation.
    pub fn keybinding_hints(&self, preset: KeymapPreset, prefix: &str) -> Vec<KeybindingHint> {
        self.keybinding_hints_cached(preset, prefix).to_vec()
    }

    /// Cached which-key-style next-key hints for a preset and chord prefix.
    ///
    /// Returns an [`Rc`] slice so callers can clone the result cheaply.
    pub fn keybinding_hints_cached(
        &self,
        preset: KeymapPreset,
        prefix: &str,
    ) -> Rc<[KeybindingHint]> {
        let prefix_normalized = prefix.trim().to_ascii_lowercase();
        let key = (preset, prefix_normalized);

        if let Some(cached) = self.hints_cache.borrow().get(&key) {
            return Rc::clone(cached);
        }

        let docs = self.get_documented(preset);
        let result = keybinding_hints_cached(&docs, prefix);
        self.hints_cache
            .borrow_mut()
            .insert(key, Rc::clone(&result));
        result
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
