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

## 5. Verify

```bash
cargo check -p gpui-themes
```
