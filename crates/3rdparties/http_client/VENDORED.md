# Vendored: http_client

- Upstream: https://github.com/zed-industries/zed/tree/v1.9.0/crates/http_client
- Base ref: v1.9.0
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, gpui_web, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

### Crate-root lint allows (clippy default lints, upstream code unchanged)

Added at the top of `src/http_client.rs` (Task 6, `just lint-host` gate with `-D warnings`):

- `#![allow(clippy::new_without_default)]` — `BlockedHttpClient::new()`.
