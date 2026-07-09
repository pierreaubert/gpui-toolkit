# gpui-keybinding Tutorial

`gpui-keybinding` collects application keybindings by preset and produces both
GPUI bindings and documented help entries.

## 1. Add the crate

```toml
[dependencies]
gpui = { workspace = true }
gpui-keybinding = { workspace = true }
```

## 2. Define actions

```rust
use gpui::actions;

actions!(dashboard, [RefreshDashboard, ToggleSidebar]);
```

## 3. Implement a provider

```rust
use gpui::KeyBinding;
use gpui_keybinding::{
    DocumentedKeybinding, KeybindingCategory, KeybindingProvider, KeymapPreset,
    format_key_label,
};

struct DashboardBindings;

impl KeybindingProvider for DashboardBindings {
    fn bindings(&self, preset: KeymapPreset) -> Vec<KeyBinding> {
        match preset {
            KeymapPreset::Default | KeymapPreset::VSCode => vec![
                KeyBinding::new("secondary-r", RefreshDashboard, None),
                KeyBinding::new("secondary-b", ToggleSidebar, None),
            ],
            KeymapPreset::Vim => vec![KeyBinding::new("r", RefreshDashboard, None)],
            KeymapPreset::Emacs => vec![KeyBinding::new("ctrl-r", RefreshDashboard, None)],
        }
    }

    fn documented_bindings(&self, preset: KeymapPreset) -> Vec<DocumentedKeybinding> {
        let refresh_key = match preset {
            KeymapPreset::Default | KeymapPreset::VSCode => "secondary-r",
            KeymapPreset::Vim => "r",
            KeymapPreset::Emacs => "ctrl-r",
        };

        vec![
            DocumentedKeybinding::new(
                format_key_label(refresh_key).into_owned(),
                "Refresh dashboard data",
                KeybindingCategory::View,
            )
            .with_raw_key_spec(refresh_key),
        ]
    }
}
```

## 4. Register providers

```rust
use gpui_keybinding::{KeybindingRegistry, KeymapPreset};

let mut registry = KeybindingRegistry::new();
registry.register(DashboardBindings);

let bindings = registry.get_bindings(KeymapPreset::Default);
let docs = registry.get_documented(KeymapPreset::Default);
let conflicts = registry.detect_conflicts(KeymapPreset::Default);
```

Bind the returned `KeyBinding` values in your GPUI app and display `docs` in a
help surface or command palette.

## 5. Handle platform shortcuts

Use `secondary-` for normal application shortcuts. GPUI maps it to Command on
macOS and Control on Windows/Linux, while `format_key_label()` produces the
matching user-facing label.

Use literal `ctrl-`, `alt-`, and `cmd-` only when the command intentionally
requires that physical modifier. This keeps default and VSCode-style presets
platform-native while still allowing terminal-style and Emacs-style bindings.

## 6. Resolve conflicts

Run conflict detection for every preset before shipping a keymap:

```rust
for preset in [
    KeymapPreset::Default,
    KeymapPreset::Vim,
    KeymapPreset::Emacs,
    KeymapPreset::VSCode,
] {
    let conflicts = registry.detect_conflicts(preset);
    assert!(
        conflicts.is_empty(),
        "{preset:?} keybinding conflicts: {conflicts:#?}",
    );
}
```

If two commands collide, first decide whether they should be separated by GPUI
context. If they are both global commands, move the less common one behind a
chord and update its `DocumentedKeybinding` in the same change. Keep
`with_raw_key_spec()` populated so macOS `⌘+S` and Windows/Linux `Ctrl+S`
display labels still compare as the same underlying `secondary-s` shortcut.

## 7. Verify

```bash
cargo test -p gpui-keybinding
```
