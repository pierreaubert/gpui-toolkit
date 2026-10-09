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

## `layout!`

The `layout!` function-like macro expands token-DSL trees to element
chains at compile time, sharing validation with the CLI's `layout
check` / `layout expand` (built-ins only — it cannot see
`toolkit.toml`). Nodes: `V`, `H`, `B`, `I`, `Tx`, `Hd`, `D`, with
Astryx-XLE-compatible styling: text size modifiers (`Tx.lg "Hi"`),
`weight` / `muted` text attrs, heading `[level = "2"]`, and stack
`[spacing = "lg"]` or fused `[g6]` gaps.

```rust
use gpui_ui_kit_macros::layout;

let _styled = layout! {
    V[g6] {
        Hd "Analytics" [level = "2"],
        H[g2] {
            Tx.lg "$42k",
            Tx.sm "Revenue" [weight = "bold", muted],
        },
    }
};
```

## Attribute Reference

- **Color fields (`Rgba`)**: `#[theme(default = 0xRRGGBB, from = <theme_field>)]`
- **Float fields (`f32`)**: `#[theme(default_f32 = <value>, from_expr = "<expr>")]`

This is a proc-macro crate. Use it via `gpui-ui-kit` which re-exports `ComponentTheme`.
