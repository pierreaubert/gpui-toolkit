use gpui::KeyBinding;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::{
    CommandPaletteEntry, DocumentedKeybinding, KeyConflict, KeybindingProvider, KeymapPreset,
    command_palette_entries, detect_conflicts,
};

/// Collects keybindings from multiple providers and aggregates them.
///
/// Register providers for different parts of your application, then
/// retrieve the combined bindings for any preset.
///
/// # Example
///
/// ```ignore
/// let mut registry = KeybindingRegistry::new();
/// registry.register(MyAppBindings);
/// registry.register(PluginBindings);
///
/// let bindings = registry.get_bindings(KeymapPreset::Vim);
/// cx.bind_keys(bindings);
/// ```
pub struct KeybindingRegistry {
    providers: Vec<Box<dyn KeybindingProvider>>,
    binding_cache: RefCell<HashMap<KeymapPreset, Rc<[KeyBinding]>>>,
    documented_cache: RefCell<HashMap<KeymapPreset, Rc<[DocumentedKeybinding]>>>,
    palette_cache: RefCell<HashMap<KeymapPreset, Rc<[CommandPaletteEntry]>>>,
}

impl KeybindingRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            binding_cache: RefCell::new(HashMap::new()),
            documented_cache: RefCell::new(HashMap::new()),
            palette_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Register a keybinding provider.
    pub fn register(&mut self, provider: impl KeybindingProvider + 'static) {
        self.providers.push(Box::new(provider));
        self.clear_caches();
    }

    /// Get all GPUI `KeyBinding`s for a preset, collected from all providers.
    pub fn get_bindings(&self, preset: KeymapPreset) -> Rc<[KeyBinding]> {
        if let Some(bindings) = self.binding_cache.borrow().get(&preset) {
            return Rc::clone(bindings);
        }

        let mut bindings = Vec::new();
        for provider in &self.providers {
            bindings.extend(provider.bindings(preset));
        }
        let bindings: Rc<[KeyBinding]> = bindings.into();
        self.binding_cache
            .borrow_mut()
            .insert(preset, Rc::clone(&bindings));
        bindings
    }

    /// Get all documented keybindings for help/settings UI.
    pub fn get_documented(&self, preset: KeymapPreset) -> Rc<[DocumentedKeybinding]> {
        if let Some(docs) = self.documented_cache.borrow().get(&preset) {
            return Rc::clone(docs);
        }

        let mut docs = Vec::new();
        for provider in &self.providers {
            docs.extend(provider.documented_bindings(preset));
        }
        let docs: Rc<[DocumentedKeybinding]> = docs.into();
        self.documented_cache
            .borrow_mut()
            .insert(preset, Rc::clone(&docs));
        docs
    }

    /// Detect conflicting keybindings (same display key) within a preset.
    pub fn detect_conflicts(&self, preset: KeymapPreset) -> Vec<KeyConflict> {
        let docs = self.get_documented(preset);
        detect_conflicts(&docs)
    }

    pub(crate) fn get_palette_entries(&self, preset: KeymapPreset) -> Rc<[CommandPaletteEntry]> {
        if let Some(entries) = self.palette_cache.borrow().get(&preset) {
            return Rc::clone(entries);
        }

        let entries = command_palette_entries(&self.get_documented(preset));
        let entries: Rc<[CommandPaletteEntry]> = entries.into();
        self.palette_cache
            .borrow_mut()
            .insert(preset, Rc::clone(&entries));
        entries
    }

    fn clear_caches(&mut self) {
        self.binding_cache.get_mut().clear();
        self.documented_cache.get_mut().clear();
        self.palette_cache.get_mut().clear();
    }
}

impl Default for KeybindingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocumentedKeybinding, KeybindingCategory, KeybindingProvider, KeymapPreset};

    struct StaticProvider;

    impl KeybindingProvider for StaticProvider {
        fn bindings(&self, _preset: KeymapPreset) -> Vec<gpui::KeyBinding> {
            Vec::new()
        }

        fn documented_bindings(&self, _preset: KeymapPreset) -> Vec<DocumentedKeybinding> {
            vec![DocumentedKeybinding::new(
                "Ctrl+P",
                "Open command palette",
                KeybindingCategory::View,
            )]
        }
    }

    #[test]
    fn caches_return_shared_rc_slices() {
        let mut registry = KeybindingRegistry::new();
        registry.register(StaticProvider);

        let bindings_a = registry.get_bindings(KeymapPreset::Default);
        let bindings_b = registry.get_bindings(KeymapPreset::Default);
        assert_eq!(bindings_a.len(), bindings_b.len());
        assert!(std::rc::Rc::ptr_eq(&bindings_a, &bindings_b));

        let docs_a = registry.get_documented(KeymapPreset::Default);
        let docs_b = registry.get_documented(KeymapPreset::Default);
        assert!(std::rc::Rc::ptr_eq(&docs_a, &docs_b));

        let palette_a = registry.get_palette_entries(KeymapPreset::Default);
        let palette_b = registry.get_palette_entries(KeymapPreset::Default);
        assert!(std::rc::Rc::ptr_eq(&palette_a, &palette_b));
    }
}
