# gpui-ui-kit-macros

Procedural macros for `gpui-ui-kit`.

Provides the `ComponentTheme` derive macro which generates `Default` and `From<&Theme>` implementations for component theme structs, reducing repetitive boilerplate.

## Usage

```rust
use gpui_ui_kit_macros::ComponentTheme;

#[derive(Debug, Clone, ComponentTheme)]
pub struct MyComponentTheme {
    #[theme(default = 0x007acc, from = accent)]
    pub primary_color: Rgba,

    #[theme(default = 0xffffff, from = text_primary)]
    pub text_color: Rgba,

    #[theme(default_f32 = 1.0, from_expr = "1.0")]
    pub opacity: f32,
}
```

This generates:
- `impl Default for MyComponentTheme` using the hex/literal `default` values
- `impl From<&Theme> for MyComponentTheme` mapping fields from the global theme (`from = none` keeps the default)

## Attribute Reference

- **Color fields (`Rgba`)**: `#[theme(default = 0xRRGGBB, from = <theme_field>)]`
- **Float fields (`f32`)**: `#[theme(default_f32 = <value>, from_expr = "<expr>")]`

This is a proc-macro crate. Use it via `gpui-ui-kit` which re-exports `ComponentTheme`.
