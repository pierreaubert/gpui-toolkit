# Vendored: gpui

- Upstream: https://github.com/zed-industries/zed/tree/v1.9.0/crates/gpui
- Base ref: v1.9.0
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, gpui_web, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

- `crates/assets/fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf`, `crates/assets/fonts/lilex/Lilex-Regular.ttf` (new files, outside this crate): restored from zed v1.9.0 `assets/fonts/` — `src/svg_renderer.rs` test module does `include_bytes!("../../../assets/fonts/...")`; the vendored layout (`crates/3rdparties/gpui/`) is one level deeper than upstream (`crates/gpui/`), so `../../../assets` resolves to `<repo>/crates/assets`. Sources are unmodified; only the font payloads were placed at the resolved path.
