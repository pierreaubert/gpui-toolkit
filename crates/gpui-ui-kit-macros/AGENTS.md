# gpui-ui-kit-macros (proc-macro, version: 0.9.6)

Procedural macros for the gpui-ui-kit component library.

## Purpose

Provides derive macros and attribute macros used by `gpui-ui-kit` components.

## Dependencies

- `proc-macro2` - Token stream manipulation
- `quote` - Quasi-quoting for code generation
- `syn` - Rust syntax parsing

## Testing

```bash
cargo check -p gpui-ui-kit-macros && cargo clippy -p gpui-ui-kit-macros
```

## Notes

- This is a proc-macro crate — it can only export procedural macros
- Changes here affect all components in `gpui-ui-kit`
