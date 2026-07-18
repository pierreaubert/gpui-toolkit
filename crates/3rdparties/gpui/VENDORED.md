# Vendored: gpui

- Upstream: https://github.com/zed-industries/zed/tree/v1.9.0/crates/gpui
- Base ref: v1.9.0
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, gpui_web, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

- `Cargo.toml` limits `image` to the formats represented by GPUI's public
  `ImageFormat` (`bmp`, `gif`, `ico`, `jpeg`, `png`, `pnm`, `tiff`, and
  `webp`). This avoids compiling unrelated AVIF, DDS, EXR, farbfeld, HDR, QOI,
  TGA, and Rayon support while preserving GPUI's advertised decoding surface.

- `crates/assets/fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf`, `crates/assets/fonts/lilex/Lilex-Regular.ttf` (new files, outside this crate): restored from zed v1.9.0 `assets/fonts/` — `src/svg_renderer.rs` test module does `include_bytes!("../../../assets/fonts/...")`; the vendored layout (`crates/3rdparties/gpui/`) is one level deeper than upstream (`crates/gpui/`), so `../../../assets` resolves to `<repo>/crates/assets`. Sources are unmodified; only the font payloads were placed at the resolved path.

### Crate-root lint allows (clippy default lints, upstream code unchanged)

Added at the top of `src/gpui.rs` (Task 6, `just lint-host` gate with `-D warnings`),
one inner attribute per default-level lint that fires on unmodified upstream code
(representative trigger sites in parentheses):

- `#![allow(clippy::collapsible_if)]` — 20 sites, e.g. `src/bounds_tree.rs:339`, `src/elements/list.rs:381`
- `#![allow(clippy::collapsible_match)]` — `src/app.rs:1443`
- `#![allow(clippy::doc_lazy_continuation)]` — `src/platform/keystroke.rs:314`
- `#![allow(clippy::double_must_use)]` — `src/tab_stop.rs:438,447,456,467` (test module)
- `#![allow(clippy::excessive_precision)]` — `src/color.rs:496,506,516`
- `#![allow(clippy::field_reassign_with_default)]` — `src/elements/div.rs:4330` (test module)
- `#![allow(clippy::for_kv_map)]` — `src/app/entity_map.rs:1057`
- `#![allow(clippy::from_over_into)]` — `src/inspector.rs:12,48`
- `#![allow(clippy::if_same_then_else)]` — `src/elements/div.rs:3933,3950`
- `#![allow(clippy::legacy_numeric_constants)]` — `src/elements/uniform_list.rs:14`
- `#![allow(clippy::len_without_is_empty)]` — `src/elements/text.rs:918`, `src/scene.rs:57`
- `#![allow(clippy::len_zero)]` — `src/window.rs:3019`
- `#![allow(clippy::let_and_return)]` — `src/profiler.rs:276`
- `#![allow(clippy::let_unit_value)]` — `src/window.rs:1457`
- `#![allow(clippy::manual_async_fn)]` — `src/elements/svg.rs:266`
- `#![allow(clippy::manual_map)]` — `src/profiler.rs:135,187`
- `#![allow(clippy::match_like_matches_macro)]` — `src/interactive.rs:128,163`
- `#![allow(clippy::mem_replace_with_default)]` — `src/elements/image_cache.rs:244,290`
- `#![allow(clippy::needless_borrow)]` — `src/platform/keystroke.rs:774`, `src/tab_stop.rs:62,129,166`
- `#![allow(clippy::needless_borrows_for_generic_args)]` — `src/text_system.rs:685`
- `#![allow(clippy::new_ret_no_self)]` — `src/gpui.rs:170` (`AppContext::new` trait method)
- `#![allow(clippy::new_without_default)]` — `src/platform.rs:851`, `src/platform/test/platform.rs:501`
- `#![allow(clippy::question_mark)]` — `src/tab_stop.rs:209`
- `#![allow(clippy::redundant_closure)]` — `src/elements/svg.rs:271`, `src/platform_scheduler.rs:165`
- `#![allow(clippy::redundant_static_lifetimes)]` — `src/gpui.rs:9` (`GPUI_MANIFEST_DIR`)
- `#![allow(clippy::single_match)]` — `src/platform/app_menu.rs:177`
- `#![allow(clippy::too_many_arguments)]` — 7 sites, e.g. `src/element.rs:95`, `src/text_system/line.rs:334,580`
- `#![allow(clippy::unnecessary_map_or)]` — `src/app.rs:459`, `src/window.rs:4792,4844`, `src/platform/test/dispatcher.rs:29`
- `#![allow(unexpected_cfgs)]` — `src/styled.rs:19` uses `not(rust_analyzer)`, a custom cfg
  set by editors/Zed's own build, not declared as a Cargo feature or check-cfg here.
