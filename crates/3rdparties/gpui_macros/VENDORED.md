# Vendored: gpui_macros

- Upstream: https://github.com/zed-industries/zed/tree/v1.9.0/crates/gpui_macros
- Base ref: v1.9.0
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, gpui_web, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

### Crate-root lint allows (clippy default lints, upstream code unchanged)

Task 6, `just lint-host` gate with `-D warnings`:

- `#![allow(unexpected_cfgs)]` at the top of `tests/derive_inspector_reflection.rs:2`
  (the test target's crate root — a lib-root allow cannot cover integration tests).
  The file gates `derive_inspector_reflection` on `not(rust_analyzer)`, a custom cfg
  Zed sets for editor tooling; not declared as a Cargo feature or check-cfg here.
