# Vendored: util

- Upstream: https://github.com/zed-industries/zed/tree/v1.9.0/crates/util
- Base ref: v1.9.0
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, gpui_web, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

- root `Cargo.toml` (not this crate): mirrored zed v1.9.0 `[patch.crates-io]` for `async-process` (rev 0b6d671) + `async-task` (rev b4486cd) — `src/command/darwin.rs` calls `smol::process::Child::adopt_raw_pid`, which exists only in zed's async-process fork; crates-io 2.5.0 fails with E0599

### Crate-root lint allows (clippy default lints, upstream code unchanged)

Added at the top of `src/util.rs` (Task 6, `just lint-host` gate with `-D warnings`),
one inner attribute per default-level lint that fires on unmodified upstream code
(representative trigger sites in parentheses):

- `#![allow(clippy::collapsible_if)]` — `src/paths.rs:212`, `src/shell.rs:607`
- `#![allow(clippy::collapsible_match)]` — `src/paths.rs:1254`
- `#![allow(clippy::comparison_to_empty)]` — `src/rel_path.rs:86`
- `#![allow(clippy::doc_lazy_continuation)]` — `src/shell.rs:172-176` (5 sites)
- `#![allow(clippy::from_over_into)]` — `src/rel_path.rs:362`
- `#![allow(clippy::if_same_then_else)]` — `src/rel_path.rs:163`
- `#![allow(clippy::needless_borrow)]` — `src/paths.rs:968`, `src/rel_path.rs:70,610`, `src/shell_builder.rs:201`
- `#![allow(clippy::needless_borrows_for_generic_args)]` — `src/shell.rs:151`
- `#![allow(clippy::new_without_default)]` — `src/rel_path.rs:310`
- `#![allow(clippy::obfuscated_if_else)]` — `src/paths.rs:1318,1321`, `src/shell_builder.rs:61`
- `#![allow(clippy::redundant_closure)]` — `src/shell.rs:88,112`
- `#![allow(clippy::result_unit_err)]` — `src/paths.rs:1517`
- `#![allow(clippy::single_char_add_str)]` — `src/shell.rs:309,314,332,337` (6 sites)
- `#![allow(clippy::too_many_arguments)]` — `src/command/darwin.rs:293`
- `#![allow(clippy::unnecessary_lazy_evaluations)]` — `src/shell.rs:196`
- `#![allow(clippy::unwrap_or_default)]` — `src/disambiguate.rs:39`
