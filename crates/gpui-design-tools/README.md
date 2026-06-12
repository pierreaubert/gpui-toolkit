# gpui-design-tools

Toolkit-owned design token tooling backed by `gpui_design::DesignSystem`.

## Commands

```bash
cargo run -p gpui-design-tools --bin gpui-export-design-tokens
cargo run -p gpui-design-tools --bin gpui-import-design-tokens -- --input tokens.json
cargo run -p gpui-design-tools --bin gpui-validate-design-tokens
```

`gpui-validate-design-tokens` supports CI report output:

```bash
cargo run -p gpui-design-tools --bin gpui-validate-design-tokens -- \
  --report-json target/gpui-conformance/design-tokens.json \
  --report-markdown target/gpui-conformance/design-tokens.md
```

This crate is generic toolkit infrastructure and must not depend on
`sotf-gpui`.
