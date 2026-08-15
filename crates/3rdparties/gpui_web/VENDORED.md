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
  callbacks throw "closure invoked after being dropped". The on-demand
  `frame_waker` rework (upstream PR #62327) was deliberately not ported: it
  requires `PlatformWindow` trait changes in gpui core and is a power
  optimization, not a correctness fix.
