# Perf review: gpui-ios

Date: 2026-08-22

## Role and hot paths

iOS/tvOS platform backend: UIKit window + CAMetalLayer, rendering delegated to the
vendored `gpui_wgpu` renderer (`crates/3rdparties/gpui_wgpu`, Metal backend), CoreText
text system, touch→GPUI event translation, momentum scrolling, UIKit accessibility bridge.

Hot paths:

- **Per frame**: `CADisplayLink` → `gpui_ios_request_frame` (`src/ios/ffi/gpui_mod.rs:161`) →
  `request_frame_for_window` (`src/ios/ffi/misc.rs:18`, pumps momentum, then invokes GPUI's
  frame callback) → `IosWindow::draw` (`src/ios/window/ios_window.rs:2023`) →
  `WgpuRenderer::draw` (`crates/3rdparties/gpui_wgpu/src/wgpu_renderer.rs:1066`).
- **Per touch event**: `handle_touches` (`src/ios/window/handle.rs:64`) →
  `IosWindow::handle_touch` (`src/ios/window/ios_window.rs:671`) → event dispatch +
  `request_forced_frame`.
- **Text**: `IosTextSystem::layout_line` / `rasterize_glyph`
  (`src/ios/text_system/ios_text_system.rs:126-136`).
- **Accessibility**: `refresh_accessibility` (`src/ios/window/ios_window.rs:1120`) — diff is
  already scratch-buffer based and zero-alloc-contract-tested
  (`tests/accessibility_allocation_contracts.rs:47`, `benches/accessibility_diff.rs`).

No `map_async`/readback in the interactive path (readback exists only in the headless QA
path, `wgpu_renderer.rs:2170-2183`). Touch state and velocity tracking already use fixed
arrays (`src/ios/window/types.rs:88-91`, `src/momentum.rs:21-24`). No TODO/FIXME in-crate.

## Findings

1. **[Alloc] `input_diag_log` does file I/O on every UIKit event, ungated.**
   `src/ios/window/register.rs:49-60`: each call does `format!`, `log::info!`, `eprintln!`,
   `std::env::temp_dir()` and **open+append+close of a log file**. It is called from the
   `GPUIWindow sendEvent:` override (`register.rs:77`, i.e. *every* UIKit event), from
   `handle_touches` (`src/ios/window/handle.rs:82-85`), from the indirect-scroll delegate
   callbacks (`register.rs:239, 259, 279`; `handle.rs:108-110`), and on every scroll
   start/move/end (`src/ios/window/ios_window.rs:764, 847, 911, 989`). No
   `debug_assertions`/env gate, so release builds pay file I/O at up to 120 Hz during
   scrolls. Highest-impact finding in the crate.

2. **[Alloc] Pencil sampling runs on every touch, with a per-event `Sel::register`.**
   `handle_touch` calls `dispatch_pointer_sample` unconditionally
   (`ios_window.rs:681`); it does `Sel::register("type")` (`ios_window.rs:1035` — an
   objc-runtime lookup with internal locking per event), five `msg_send`s, then
   `dispatch_pencil_sample` locks a global `Mutex` (`src/pencil.rs:96`) even when no
   callback is registered. Should early-out when no pencil/hover callback is set and cache
   the selector in a `static`.

3. **[Alloc] Grayscale glyph rasterization allocates a fresh `Vec` per glyph; the cached
   CoreText context machinery is dead for text.** `rasterize_glyph` early-returns through
   font_kit's `Canvas::new` for non-emoji (`src/ios/text_system/ios_text_system_state.rs:315-336`),
   so `GLYPH_TEXT_CONTEXT_CACHE`/`GLYPH_BITMAP_SCRATCH` (`ios_text_system_state.rs:50-58`)
   only ever serve emoji — the `!is_emoji` grayscale branch at lines 363-368 is unreachable.
   Each rasterized glyph also gets copied again on atlas upload: `upload_texture` does
   `bytes.to_vec()` (R8 monochrome hits the catch-all in `swizzle_upload_data`,
   `crates/3rdparties/gpui_wgpu/src/wgpu_atlas.rs:237-244, 372-383`) before
   `queue.write_texture`. Net: 2 heap copies per new glyph on top of rasterization.

4. **[Alloc] Text layout cache is unbounded and clones on every hit.**
   `layout_cache: HashMap<LayoutCacheKey, Arc<LineLayout>>`
   (`ios_text_system_state.rs:145`) has no eviction — every distinct (text, size, runs)
   lives for the process lifetime (a music player with ever-changing track/playlist strings
   grows it indefinitely). Hits deep-clone all `Vec<ShapedRun>`/glyph vectors via
   `clone_layout` (`ios_text_system_state.rs:487-511`), so the `Arc` buys nothing over
   storing `LineLayout` directly; and `layout_line` takes a **write** lock even for cache
   hits (`src/ios/text_system/ios_text_system.rs:135`).

5. **[Alloc] Uncached layout recreates CTFonts and re-derives emoji-ness per glyph.**
   `layout_line_uncached` calls `clone_with_font_size` per font run
   (`ios_text_system_state.rs:547`) and `is_emoji` (HashMap lookup + string compare) per
   glyph inside the inner loop (`ios_text_system_state.rs:591`). Only on cache misses, but
   misses are every new string. Moderate.

6. **[Roundtrip] Blocking `device.poll(Wait)` on the main thread during resize.**
   `handle_layout_change` → `renderer.update_drawable_size` (`ios_window.rs:1775`) →
   `wgpu_renderer.rs:949-956` polls with `PollType::Wait { timeout: None }` before
   reconfiguring the surface. Full GPU pipeline stall on the UI thread on every
   rotation/split-view change; also the synchronous-poll pattern the campaign flags as a
   wasm hazard. Infrequent but user-visible (rotation jank).

7. **[Alloc] Signpost instrumentation buffer grows without bound.**
   Every `emit_signpost` pushes into a global `Mutex<Vec<IosSignpostEvent>>` that is never
   trimmed (`src/instrumentation.rs:37-47`); `refresh_accessibility` allocates a `format!`ed
   `Arc<str>` per refresh (`ios_window.rs:1128-1131`). Slow leak plus a lock+alloc on an
   app-visible path.

8. **[GPU] Startup builds the Metal surface twice.** `IosWindow::new` creates a surface to
   build `WgpuContext`, drops it, then `WgpuRenderer::new` creates its own
   (`ios_window.rs:405-413`, comment at line 411 acknowledges it). One-time cost; also two
   adapter/device selections would occur if the shared-context pre-population ever missed.

9. **[GPU] `request_forced_frame(force_render: true)` on every touch-move/scroll tick**
   (`ios_window.rs:585-592`, call sites 720, 776, 859, 923, 1003, 1411). Touch-move delivery
   can exceed vsync rate (ProMotion 120 Hz, coalesced touches); forcing a render per event
   risks more `WgpuRenderer::draw` calls than display frames. (needs profiling — GPUI may
   already coalesce; measure draw calls per vsync during a fling.)

Positive notes: atlas uploads are batched per frame via `pending_uploads`
(`wgpu_atlas.rs:39, 245`); `sprite_atlas()` is cached behind a cheap lock
(`ios_window.rs:2032-2050`); CPU glyph raster + atlas upload matches upstream GPUI design —
no offscreen→readback→re-upload anti-pattern here.

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | Gate `input_diag_log` behind an env var or `debug_assertions`; drop the file-append in hot builds | 1 | S | Removes per-event file I/O during scroll |
| 2 | Early-out `dispatch_pointer_sample` when no pencil callback; cache `Sel` in a static | 2 | S | Removes 6 msg_sends + mutex per touch event |
| 3 | Bound `layout_cache` (LRU or size cap); store `LineLayout` directly; read-lock on hit | 4 | M | Stops memory growth in long sessions |
| 4 | Make grayscale glyphs use the cached CGContext path (or at least remove dead branch); avoid the `to_vec` in atlas upload for R8 (upload from raster buffer directly) | 3 | M | Halves per-glyph copies |
| 5 | Replace blocking `device.poll(Wait)` on resize with deferred texture destruction / `PollType::Poll` | 6 | M | Removes rotation jank; wasm-safe pattern |
| 6 | Cache CTFont per (FontId, size); hoist `is_emoji` out of the glyph loop | 5 | S | Faster cache-miss layout |
| 7 | Ring-buffer or cap the signpost log; skip `format!` when logging disabled | 7 | S | Stops slow leak |
| 8 | Profile draw-calls-per-vsync during scroll/fling; only force-render when GPUI hasn't scheduled a frame | 9 | S | Possibly fewer redundant draws (needs profiling) |
| 9 | Reuse the initially created surface instead of dropping it | 8 | S | Faster startup only |

## Quick wins

- Gate `input_diag_log` (finding 1) — a few lines, biggest win.
- Early-out + cached selector in `dispatch_pointer_sample` (finding 2).
- Cap the signpost `Vec` (finding 7).
- Hoist `is_emoji` out of the per-glyph loop (finding 5, trivial half).
