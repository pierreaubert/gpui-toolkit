use gpui::KeyBinding;
use std::cell::RefCell;
use std::collections::HashMap;

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
    binding_cache: RefCell<HashMap<KeymapPreset, Vec<KeyBinding>>>,
    documented_cache: RefCell<HashMap<KeymapPreset, Vec<DocumentedKeybinding>>>,
    palette_cache: RefCell<HashMap<KeymapPreset, Vec<CommandPaletteEntry>>>,
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
    pub fn get_bindings(&self, preset: KeymapPreset) -> Vec<KeyBinding> {
        if let Some(bindings) = self.binding_cache.borrow().get(&preset) {
            return bindings.clone();
        }

        let mut bindings = Vec::new();
        for provider in &self.providers {
            bindings.extend(provider.bindings(preset));
        }
        self.binding_cache
            .borrow_mut()
            .insert(preset, bindings.clone());
        bindings
    }

    /// Get all documented keybindings for help/settings UI.
    pub fn get_documented(&self, preset: KeymapPreset) -> Vec<DocumentedKeybinding> {
        if let Some(docs) = self.documented_cache.borrow().get(&preset) {
            return docs.clone();
        }

        let mut docs = Vec::new();
        for provider in &self.providers {
            docs.extend(provider.documented_bindings(preset));
        }
        self.documented_cache
            .borrow_mut()
            .insert(preset, docs.clone());
        docs
    }

    /// Detect conflicting keybindings (same display key) within a preset.
    pub fn detect_conflicts(&self, preset: KeymapPreset) -> Vec<KeyConflict> {
        let docs = self.get_documented(preset);
        detect_conflicts(&docs)
    }

    pub(crate) fn get_palette_entries(&self, preset: KeymapPreset) -> Vec<CommandPaletteEntry> {
        if let Some(entries) = self.palette_cache.borrow().get(&preset) {
            return entries.clone();
        }

        let entries = command_palette_entries(&self.get_documented(preset));
        self.palette_cache
            .borrow_mut()
            .insert(preset, entries.clone());
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
