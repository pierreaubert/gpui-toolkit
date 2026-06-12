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

    fn documented_bindings(&self, _: KeymapPreset) -> Vec<DocumentedKeybinding> {
        vec![DocumentedKeybinding::new(
            "Refresh",
            "Refresh dashboard data",
            KeybindingCategory::View,
        )]
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

## 5. Verify

```bash
cargo test -p gpui-keybinding
```
