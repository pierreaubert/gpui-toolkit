# gpui-ui-kit-macros Tutorial

`gpui-ui-kit-macros` contains procedural macros used by toolkit components.

## 1. Add the crate

Most app code uses these through `gpui-ui-kit` re-exports:

```rust
use gpui_ui_kit::{ComponentBuilder, ComponentTheme, FormField};
```

If you are developing a lower-level crate:

```toml
[dependencies]
gpui-ui-kit-macros = { workspace = true }
```

## 2. Derive component themes

Use `ComponentTheme` for theme structs that should follow the toolkit's theme
derivation conventions.

```rust
use gpui_ui_kit::ComponentTheme;

#[derive(Clone, ComponentTheme)]
pub struct MyWidgetTheme {
    // fields here
}
```

## 3. Derive builders

Use `ComponentBuilder` to generate fluent setters for component configuration
types.

```rust
use gpui_ui_kit::ComponentBuilder;

#[derive(ComponentBuilder)]
pub struct MyWidgetProps {
    pub label: String,
}
```

## 4. Derive form fields

Use `FormField` for structs that represent validated form state.

## 5. Verify

```bash
cargo test -p gpui-ui-kit-macros
```

When changing macro output, add or update compile-focused tests in this crate.
