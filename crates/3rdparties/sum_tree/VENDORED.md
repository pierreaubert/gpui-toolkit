# Vendored: sum_tree

- Upstream: https://github.com/zed-industries/zed/tree/v1.9.0/crates/sum_tree
- Base ref: v1.9.0
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, gpui_web, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

- src/cursor.rs, src/sum_tree.rs: ztracing::instrument -> tracing::instrument — ztracing is GPL-3.0; tracing was already a dep
- src/sum_tree.rs: removed zlog::init_test() test call (and its now-empty `init_logger` ctor wrapper) — zlog is GPL-3.0
- Cargo.toml: dropped ztracing/zlog deps (GPL-3.0)
