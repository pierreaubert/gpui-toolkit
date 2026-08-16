# Vendored: gpui_web

- Upstream: https://github.com/zed-industries/zed/tree/v1.9.0/crates/gpui_web
- Base ref: v1.9.0
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

- `src/window.rs` ports the reentrancy-safe callback invocation from upstream
  zed PR #61707 (commit c97b7c0ea4): new `WebWindowInner::with_callback`
  (take/call/restore so `callbacks` is not borrowed while GPUI code runs),
  applied to the rAF `request_frame`, ResizeObserver `resize` (both call
  sites), `active_status_change`, and `appearance_changed` invocations;
  plus `raf_id` tracking and a `Drop for WebWindow` that cancels the pending
  rAF, disconnects the ResizeObserver, breaks the DPR media-query Rc cycle,
  and removes the canvas/input elements — otherwise a dropped window's pending
  callbacks throw "closure invoked after being dropped".
- `src/dispatcher.rs` ports the `MainThreadMailbox::run_waker_loop` fixes from
  the same commit: the `waitAsync` result is now only awaited when
  `is_async` is true (a synchronous "not-equal" return is a benign race, not
  a fatal error that kills the waker loop), and the mailbox is drained again
  after re-arming the signal with `Atomics::store(0)` so items posted between
  the previous drain and the re-arm are not lost. Also ported from the same
  file: `shared_memory_supported` is no longer `cfg`-gated on the
  `multithreaded` feature, and the `supports_threads` selection uses
  `cfg!(feature = "multithreaded")` so the SharedArrayBuffer warning is only
  emitted when threading was actually requested.

## Deliberately not ported (from upstream zed PR #61707, commit c97b7c0ea4)

- `src/platform.rs` WebGPU-failure DOM message (`show_webgpu_unavailable_message`,
  replaces `on_finish_launching` on init failure) — not ported, follow-up.
- `src/window.rs` `fullscreenchange` listener / `toggle_fullscreen` rework —
  not ported: the vendored `toggle_fullscreen` flips a local flag, so an
  Esc-exit desyncs the stale flag; follow-up.
- `src/window.rs` clamped logical size hunk (recompute logical size from the
  clamped physical size) — not ported; only matters beyond
  `max_texture_dimension_2d`.
- `src/window.rs` `handle` field removal — not ported; the field stays with a
  harmless `#[allow(dead_code)]`.
- `src/events.rs` `EventListenerHandle` rework (drop-unregisters listeners,
  removes the `smallvec` dependency) — not ported; the leak only matters on
  window drop.

## Deliberately not ported (other upstream changes)

- The on-demand `frame_waker` rework (upstream PR #62327) was deliberately
  not ported: it requires `PlatformWindow` trait changes in gpui core and is
  a power optimization, not a correctness fix.
