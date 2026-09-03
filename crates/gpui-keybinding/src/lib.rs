//! gpui-keybinding — Reusable keybinding framework for GPUI applications.
//!
//! Provides:
//! - [`KeymapPreset`] — preset identifiers (Default, Vim, Emacs, VSCode)
//! - [`KeybindingCategory`] — categories for organizing bindings in help UI
//! - [`DocumentedKeybinding`] — human-readable binding descriptions with an
//!   optional VSCode-style `when`-clause [`DocumentedKeybinding::context`]
//! - [`KeybindingProvider`] — trait for apps to register bindings per preset
//! - [`KeybindingRegistry`] — collects bindings from multiple providers
//! - [`CommandPaletteEntry`] + [`keybinding_hints`] — discovery data for
//!   command palettes and chord hint overlays
//! - [`NavigationAction`] + [`navigation_key`] — generic navigation presets
//! - `platform` — platform-aware key label formatting
//! - `conflict` — conflict detection for duplicate key+context bindings
//! - [`parse_user_overrides`] / [`serialize_user_overrides`] /
//!   [`apply_user_overrides`] — `keybindings.json`-style user remapping
//!
//! ## Hot paths
//!
//! For per-keystroke filtering, prefer the `_cached` discovery variants
//! ([`search_command_palette_cached`], [`keybinding_hints_cached`]) over the
//! [`Vec`]-returning twins: the uncached wrappers exist for one-off queries
//! and share the same matching logic, but they allocate on every call while
//! the cached variants return cheap [`Rc`] handles with allocation-free hits.

mod conflict;
mod discovery;
mod platform;
mod preset;
mod provider;
mod registry;

pub mod presets;

pub use conflict::{KeyConflict, detect_conflicts};
pub use discovery::{
    CommandPaletteEntry, KeybindingHint, command_palette_entries, keybinding_hints,
    keybinding_hints_cached, search_command_palette, search_command_palette_cached,
};
pub use platform::{format_key_label, platform_modifier, platform_modifier_symbol};
pub use preset::KeymapPreset;
pub use presets::{NavigationAction, NavigationMapping, navigation_key, navigation_mappings};
pub use provider::{
    DocumentedKeybinding, KeybindingCategory, KeybindingProvider, apply_user_overrides,
    parse_user_overrides, serialize_user_overrides,
};
pub use registry::KeybindingRegistry;
