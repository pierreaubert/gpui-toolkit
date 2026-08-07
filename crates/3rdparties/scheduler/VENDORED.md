# Vendored: scheduler

- Upstream: https://github.com/zed-industries/zed/tree/v1.9.0/crates/scheduler
- Base ref: v1.9.0
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, gpui_web, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

- `src/executor.rs`, `src/scheduler.rs`, and `Cargo.toml` replace the three
  blocking `flume` channels with equivalent standard-library MPSC channels.
  This removes the transitive dependency on yanked `spin` 0.9.8 without
  changing scheduler APIs or channel capacity.

### Crate-root lint allows (clippy default lints, upstream code unchanged)

Added at the top of `src/scheduler.rs` (Task 6, `just lint-host` gate with `-D warnings`):

- `#![allow(clippy::new_without_default)]` — `TestClock::new()` in `src/clock.rs`.
- `#![allow(clippy::type_complexity)]` — boxed `FnOnce` types in `src/executor.rs` and `src/scheduler.rs`.
- `#![allow(clippy::nonminimal_bool)]` — `!env::var(...).is_ok()` in `src/test_scheduler.rs`.
- `#![allow(clippy::unnecessary_map_or)]` — `.map_or(false, ...)` in `src/test_scheduler.rs` (2 sites).
- `#![allow(clippy::let_unit_value)]` — `let _ = ...` on unit-valued expressions in `src/tests.rs:301,337`.
