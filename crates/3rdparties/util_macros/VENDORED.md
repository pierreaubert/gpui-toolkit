# Vendored: util_macros

- Upstream: https://github.com/zed-industries/zed/tree/v1.9.0/crates/util_macros
- Base ref: v1.9.0
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, gpui_web, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

### Crate-root lint allows (clippy default lints, upstream code unchanged)

Added at the top of `src/util_macros.rs` (Task 6, `just lint-host` gate with `-D warnings`):

- `#![allow(unexpected_cfgs)]` — upstream gates code on `cfg!(perf_enabled)` (line 204),
  a custom cfg Zed sets via RUSTFLAGS in their own builds; not declared as a
  Cargo feature or check-cfg here, so rustc's `unexpected_cfgs` fires.
