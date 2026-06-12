# gpui-keybinding

Reusable keybinding framework with Vim/Emacs/VSCode presets for GPUI applications.

## Architecture

- `preset.rs` — `KeymapPreset` enum: Default, Vim, Emacs, VSCode
- `provider.rs` — `KeybindingProvider` trait (apps register bindings per preset), `DocumentedKeybinding`, `KeybindingCategory`
- `registry.rs` — `KeybindingRegistry`: collects bindings from multiple providers, queries by preset/category
- `conflict.rs` — `detect_conflicts()`: finds duplicate key+context bindings, returns `Vec<KeyConflict>`
- `platform.rs` — `format_key_label()`, `platform_modifier()`, `platform_modifier_symbol()`: platform-aware key formatting (Cmd on macOS, Ctrl on others)
- `presets/` — Built-in preset definitions:
  - `default.rs`, `vim.rs`, `emacs.rs`, `vscode.rs` — preset-specific navigation mappings
  - `mod.rs` — `NavigationAction` enum, `navigation_key()`, `navigation_mappings()`

## Key Public API

- `KeymapPreset` — enum of available presets (`preset.rs`)
- `KeybindingProvider` trait — `fn keybindings(&self, preset: KeymapPreset) -> Vec<DocumentedKeybinding>` (`provider.rs`)
- `KeybindingRegistry` — `register(provider)`, `bindings_for(preset, category)` (`registry.rs`)
- `DocumentedKeybinding` — key combo + action + human-readable description (`provider.rs`)
- `KeybindingCategory` — category enum for organizing bindings in help UI (`provider.rs`)
- `detect_conflicts(bindings) -> Vec<KeyConflict>` — conflict detection (`conflict.rs`)
- `NavigationAction` / `navigation_key()` — generic navigation actions with preset-specific keys (`presets/`)

## Testing

```bash
cargo test -p gpui-keybinding
```

## Important Notes

- Depends on `gpui` for keystroke types
- Presets define key mappings, not behavior — the consuming app implements the actual actions
- Conflict detection checks for duplicate key+context combinations across all registered providers
