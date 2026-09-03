# gpui-keybinding

Reusable keybinding framework with Vim/Emacs/VSCode presets for GPUI applications.

## What It Does

Provides a structured way to define, register, and manage keyboard shortcuts in GPUI applications. Ships with built-in presets (Default, Vim, Emacs, VSCode) so users can choose their preferred key mapping style. Includes conflict detection to catch duplicate bindings.

## Features

- **Multiple presets**: Default, Vim, Emacs, VSCode key mappings out of the box
- **Provider pattern**: Apps register keybinding providers; the registry collects them all
- **Conflict detection**: Finds duplicate documented key specs after GPUI parsing. It does not receive executable `KeyBinding` contexts.
- **Platform-aware formatting**: Shows Cmd on macOS, Ctrl on Windows/Linux
- **Documented bindings**: Each binding carries a human-readable description for help UI
- **Category system**: Organize bindings by category (Navigation, Editing, etc.)
- **Command palette backend**: Searchable entries derived from documented bindings
- **Which-key hints**: Next-key hint data for chorded bindings like `ctrl-k ctrl-s`

## Usage

```rust
use gpui::KeyBinding;
use gpui_keybinding::{
    DocumentedKeybinding, KeybindingCategory, KeybindingProvider, KeybindingRegistry,
    KeymapPreset, format_key_label,
};

// Implement the provider trait for your app
struct MyAppBindings;

impl KeybindingProvider for MyAppBindings {
    fn bindings(&self, preset: KeymapPreset) -> Vec<KeyBinding> {
        match preset {
            KeymapPreset::Default | KeymapPreset::VSCode => {
                vec![KeyBinding::new("secondary-p", OpenPalette, None)]
            }
            KeymapPreset::Vim => vec![KeyBinding::new("space p", OpenPalette, None)],
            KeymapPreset::Emacs => vec![KeyBinding::new("ctrl-x p", OpenPalette, None)],
        }
    }

    fn documented_bindings(&self, preset: KeymapPreset) -> Vec<DocumentedKeybinding> {
        let raw = match preset {
            KeymapPreset::Default | KeymapPreset::VSCode => "secondary-p",
            KeymapPreset::Vim => "space p",
            KeymapPreset::Emacs => "ctrl-x p",
        };

        vec![
            DocumentedKeybinding::new(
                format_key_label(raw).into_owned(),
                "Open command palette",
                KeybindingCategory::View,
            )
            .with_raw_key_spec(raw),
        ]
    }
}

// Register providers and query bindings
let mut registry = KeybindingRegistry::new();
registry.register(MyAppBindings);
let bindings = registry.get_bindings(KeymapPreset::Default);
let conflicts = registry.detect_conflicts(KeymapPreset::Default);
```

## When-Clause Contexts

`DocumentedKeybinding::with_context()` attaches an opaque VSCode-style
`when`-clause expression (e.g. `"editorTextFocus"`). The crate never executes
it — it is a first step toward context parity:

- Conflict detection groups by key *and* context, so the same key in different
  contexts is not reported as a conflict.
- Command-palette search indexes the context text.
- Cross-context shadowing still needs an executable context evaluator, which
  remains out of scope (evaluate GPUI `KeyBinding` contexts in the app).

## User Overrides (`keybindings.json`)

User remapping is JSON-based. Overrides match base bindings by description:

```rust
use gpui_keybinding::{apply_user_overrides, parse_user_overrides, serialize_user_overrides};

// Load: accepts a bare array or `{"bindings": [...]}`; blank input yields `[]`.
let overrides = parse_user_overrides(std::fs::read_to_string("keybindings.json")?.as_str())?;
let merged = apply_user_overrides(base_bindings, overrides);

// Save back to disk.
std::fs::write("keybindings.json", serialize_user_overrides(&merged)?)?;
```

An override replaces the base entry with the same description and is appended
otherwise; duplicates resolve last-wins.

## Discovery UI Data

For per-keystroke filtering, prefer the `_cached` variants
(`search_command_palette_cached`, `keybinding_hints_cached`): they return cheap
`Rc` slices with allocation-free cache hits. The `Vec`-returning twins share
the same matching logic but allocate on every call and exist for one-off
queries.

`gpui-keybinding` exposes backend data for command palettes and which-key style
overlays. UI crates can render these however they like without walking providers
or reimplementing chord parsing.

```rust
use gpui_keybinding::{KeybindingRegistry, KeymapPreset};

let registry = KeybindingRegistry::new();

// Searchable command palette rows.
let matches = registry.search_command_palette(KeymapPreset::Default, "save");

// Next-key hints after the user presses a chord prefix.
let hints = registry.keybinding_hints(KeymapPreset::Default, "ctrl-k");
```

## Conflict Resolution Workflow

Every provider should return both executable `KeyBinding` values and matching
`DocumentedKeybinding` entries. Keep the raw GPUI key spec in
`DocumentedKeybinding::with_raw_key_spec()` so conflict detection groups by the
actual binding instead of a platform-specific display label.

Recommended release gate for each preset:

```rust
use gpui_keybinding::{KeybindingRegistry, KeymapPreset};

fn assert_no_conflicts(registry: &KeybindingRegistry) {
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
}
```

The checker intentionally works only from `DocumentedKeybinding`: it cannot inspect the
GPUI context supplied to an executable `KeyBinding`. Filter or review reports by the
actual binding context before treating one as a release failure.

When a same-context conflict appears, prefer this order:

1. Move less common commands behind a chord such as `secondary-k secondary-s`.
2. Keep preset conventions intact. For example, do not steal Vim movement keys
   in Vim mode for global app actions.
3. Update the documented binding and command-palette entry in the same change
   as the executable `KeyBinding`.

## Platform Shortcut Policy

Use GPUI's `secondary-` modifier for ordinary app shortcuts that should follow
the host platform:

| Raw key spec | macOS display | Windows/Linux display | Intended use |
| --- | --- | --- | --- |
| `secondary-s` | `⌘+S` | `Ctrl+S` | Save, open, find, palette, and other standard app commands |
| `ctrl-s` | `Ctrl+S` | `Ctrl+S` | Literal Control shortcuts, terminal-style bindings, or Emacs presets |
| `cmd-s` | `⌘+S` | `⌘+S` | macOS-specific docs or commands only |
| `alt-left` | `Alt+←` | `Alt+←` | Cross-platform alternate navigation |

Display labels should be produced with `format_key_label(raw_spec)` instead of
handwritten strings. This keeps help surfaces, command palettes, and conflict
reports aligned across macOS, Windows, and Linux.

## Built-in Presets

| Preset | Style | Example Navigation |
|--------|-------|-------------------|
| Default | Standard shortcuts | Arrow keys, Cmd+Z |
| Vim | Modal editing | h/j/k/l, dd, yy |
| Emacs | Chord-based | C-f/C-b, C-n/C-p |
| VSCode | VS Code compatible | Cmd+Shift+P, Cmd+D |

## Architecture

```
src/
├── lib.rs       # Module exports and re-exports
├── preset.rs    # KeymapPreset enum
├── provider.rs  # KeybindingProvider trait, DocumentedKeybinding, KeybindingCategory
├── registry.rs  # KeybindingRegistry — collects and queries bindings
├── conflict.rs  # Conflict detection
├── platform.rs  # Platform-aware key label formatting
├── discovery.rs # Command palette entries and which-key hints
└── presets/     # Built-in preset definitions
    ├── default.rs
    ├── vim.rs
    ├── emacs.rs
    └── vscode.rs
```

## Testing

```bash
cargo test -p gpui-keybinding
```

## License

Part of the SOTF (Sound of the Future) project.
