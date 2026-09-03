# Code Review: gpui-au — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-au` (~3.8k LOC, macOS-only)

## 1. Purpose / role
macOS AUv3 `Platform` embedding GPUI in `NSView`: external `NSView` + `CAMetalLayer` + wgpu, frames driven by Swift `CVDisplayLink` → `gpui_au_request_frame()`. Files: `window/au_window.rs` (643), `ffi.rs` (605), `text_system/au_text_system_state.rs` (534), `platform.rs` (460), `safety_report.rs` (224). `#![cfg(target_os="macos")]` (`lib.rs:10`).

Public API (6 + `ffi`): `AuPlatform`, `PENDING_VIEW/PendingViewInfo`, `au_safety_report()/AuSafetyReport/Boundary/Status`; `ffi::{gpui_au_create:119, destroy:207, request_frame:223, resize:232, set_active/hovered, mouse_down/up/moved/dragged:262-329, scroll_wheel:352, key_down/up:452-477, insert/set_marked/unmark/delete:499-534}`.

## 2. SOTA gap analysis (vs AUv3/Dock)
1. **No `AUParameterTree`/`AUParameter`** — no automation, observe tokens, value strings.
2. **No factory presets / `AUPreset` / `fullState`** — host cannot save/recall UI state.
3. **No offline-render, render-quality, latency/tail-time, bus/sidechain declaration.**
4. **No MIDI/MPE input** — mouse/key/text FFI only.
5. **No host-transport bridge** (tempo/beat/playhead).
6. **No `auval`/Logic validation evidence** — `safety_report.rs:22` is self-declared.
7. **No resize constraints, fullscreen, tooltips, context menus** — cursor map (`platform.rs:336-346`) downgrades diagonals to avoid private selectors.

## 3. Performance evaluation
- `au_window.rs:84 new()` fan-out 49/fan-in 64 — CAMetalLayer (`:123-129`) + wgpu init (`:163-188` with `nslog!` per branch) synchronously on plugin-main thread; blocks AU instantiation.
- `draw:536` holds `renderer.lock()` across draw (`:537`); `sprite_atlas()` (`:543-546`) re-locks same mutex.
- `handle_resize:252` reconfigures surface on every host resize-drag (`:281` lock); no debounce.
- `platform.rs:78/82/86` locks executors/text-system on hot query paths.
- Externally clocked `request_frame` (`ffi.rs:223`) has no frame-skip/VSync back-pressure — `CVDisplayLink` overdrive queues redundant draws. `nslog!` on hot path (`au_window.rs:124,164,181,188`) violates AU sandbox expectations.

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Move wgpu creation off-main-thread; lazy-init placeholder | M | instantiation latency |
| 2 | `Arc`-clone renderer outside `draw()` lock; `try_lock` + frame-drop counter | S | realtime safety |
| 3 | Debounce `handle_resize` (~16 ms); gate `preferred_present_mode:None` (`:171`) for VSync | S | resize churn |
| 4 | Minimal `AUParameterTree` + `fullState` round-trip; extend `safety_report.rs:53` gates | M | host parity |
| 5 | Gate `nslog!` behind `debug_assert`/feature | S | sandbox hygiene |

## 5. Verdict
Good embedding shim, not yet a host-citizen plugin. Parameter/state/transport bridges are the SOTA gate; async init + lock hygiene are the perf gate.
