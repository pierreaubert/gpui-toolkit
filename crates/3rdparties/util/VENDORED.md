# Vendored: util

- Upstream: https://github.com/zed-industries/zed/tree/v1.9.0/crates/util
- Base ref: v1.9.0
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, gpui_web, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

- root `Cargo.toml` (not this crate): mirrored zed v1.9.0 `[patch.crates-io]` for `async-process` (rev 0b6d671) + `async-task` (rev b4486cd) — `src/command/darwin.rs` calls `smol::process::Child::adopt_raw_pid`, which exists only in zed's async-process fork; crates-io 2.5.0 fails with E0599
