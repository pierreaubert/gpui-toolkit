# gpui-themes Tutorial

`gpui-themes` provides a theme editor and theme showcase for GPUI apps.

## 1. Add the crate

```toml
[dependencies]
gpui-themes = { workspace = true }
gpui-ui-kit = { workspace = true }
```

## 2. Run the editor

```bash
cargo run -p gpui-themes --bin theme-editor
```

Use it to inspect and adjust theme colors interactively.

## 3. Run the showcase

```bash
cargo run -p gpui-themes --bin theme-showcase
```

The showcase displays UI-kit components across built-in presets.

## 4. Use presets in code

```rust
use gpui_themes::BuiltInThemePreset;

let theme = BuiltInThemePreset::Dark.to_theme();
```

Install theme state with `gpui-miniapp` or set `ThemeState` manually in your
GPUI app.

## 5. Import community themes

```rust
use gpui_themes::CommunityThemeBundle;

let bundle = CommunityThemeBundle::from_json(json)?;
bundle.validate()?;
```

Community bundles are versioned by `manifest.schema_version`. Version `1` is the
current schema and is also the default for early exports that omitted the field.
Always validate after parsing so unsupported future schemas are rejected before
the theme is shown, saved, or re-exported.

## 6. Change the schema safely

Keep additive fields optional with serde defaults when they preserve v1 meaning.
Rename, removal, or semantic changes require a new schema version, a migration
path, and compatibility tests for older community bundles.

## 7. Verify

```bash
cargo check -p gpui-themes
cargo test -p gpui-themes
```
