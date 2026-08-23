# Perf review: gpui-ui-kit

Date: 2026-08-22

## Role and hot paths

Reusable component library on the vendored GPUI fork: every component builds
`div()` trees plus `canvas(...)` custom-paint closures. There is no direct GPU
access in the crate — all rasterization is delegated to GPUI scene primitives
(`paint_quad`, `paint_path`, text). The real hot paths are therefore:

- **Workflow canvas** (`src/workflow/`): `WorkflowCanvas::render` rebuilds all
  render data on every `cx.notify()`; mouse-move during node/connection/box
  drags notifies every event (`workflow/canvas/workflow_canvas.rs:540,546,552,558`).
  Paint flattens and tessellates every connection bezier each frame.
- **QR components** (`src/qr/`): per-module `paint_quad` loops; animated variant
  re-renders at ~30 fps via a timer (`qr/animated_qr_code.rs:131-147`).
- **Input** (`src/input.rs`): keystroke handlers clone the text buffer per key;
  editing render splits text with O(n) char-index scans per piece.
- **Accessibility tree** (`src/accessibility.rs`): every component render calls
  `global_mut::<AccessibilityTree>()` (`accessibility.rs:977`).

`src/audio/` is orphaned dead code (not declared in `src/lib.rs`) — excluded.
No `map_async` / `read_texture` / `pollster` / `device.poll` / `paint_image`
usage exists in the crate: zero GPU readback surface.

## Findings

1. **[Alloc] Workflow render clones the entire graph every frame.** `render()`
   does `Arc::new(self.state.graph.clone())` (`workflow/canvas/workflow_canvas.rs:993`)
   plus two rebuilt `Vec`s of connection/port and obstacle data (`:944-989`).
   `WorkflowNodeData` owns a `String` title and a `serde_json::Value` user_data
   (`workflow/state/workflow_node_data.rs:12,23`), so the clone walks arbitrary
   JSON per node. During a node drag this fires per mouse-move event. Impact:
   high; scales with graph size × event rate.

2. **[Alloc/GPU] Per-paint bezier flattening + CPU tessellation per connection.**
   `draw_connection` calls `connection_path_avoiding` (`workflow/canvas/draw.rs:40-41`),
   which allocates 1–3 fresh `Vec<Position>` per connection per paint
   (`workflow/bezier.rs:16,141,180,188`), then builds a `PathBuilder` stroke
   (`draw.rs:47-64`) that tessellates via lyon on the CPU every paint
   (`3rdparties/gpui/src/path_builder.rs:7-12`). Nothing is cached across
   frames even when endpoints/viewport are unchanged. Obstacle filtering is
   O(connections × nodes) per paint (`workflow_canvas.rs:1053-1061`, thread-local
   buffer already avoids the Vec alloc but not the scan). GPU opportunity: draw
   connections as shader-evaluated curves (vello-style, cf. d3rs `vello2d`)
   instead of CPU flatten + tessellate.

3. **[Alloc] Hit-testing re-flattens every connection curve per query.**
   `hit_test_with_viewport` iterates all connections and calls
   `connection_path(from, to, 2.0)` — full de Casteljau recursion + `Vec` alloc —
   per connection (`workflow/hit_test.rs:92-96,252`). Runs on every mouse-down,
   mouse-up, right-click, and double-click. A bounding-box pre-reject per
   connection would skip nearly all flattening.

4. **[GPU/Alloc] QR paints one `PaintQuad` per dark module, per paint.**
   `paint_qr_full_from_colors` loops modules² and calls `window.paint_quad`
   per dark cell (`qr/paint.rs:35-53`); a version-40 QR is 177×177 with up to
   ~15k dark modules → ~15k `scene.insert_primitive` calls per frame
   (gpui `window.rs:3747-3765` path). `AnimatedQrCode` repeats the same loop
   with per-module viewport clipping at 30 fps (`qr/animated_qr_code.rs:254-290`)
   and clones the full color `Vec` every render (`:227`, static path `:187`).
   GPU opportunity: rasterize the matrix once into a small bitmap at encode
   time, upload once, paint a single scaled image quad — scaling moves to the
   GPU and per-frame scene churn drops from O(modules²) to O(1).

5. **[Alloc] AnimatedQrCode 30 fps timer drives full re-render + clone.**
   The spawn loop calls `cx.notify()` every 33 ms unconditionally
   (`qr/animated_qr_code.rs:131-147`); each render clones `self.colors`
   (`:227`). Cheap fix: share `Arc<[QrColor]>` into the canvas closure and
   precompute the visible row/col window instead of iterating the full matrix.

6. **[Alloc] Input editing path allocates per keystroke and per render.**
   Every edit arm clones the whole text buffer to fire `on_text_change`
   (`src/input.rs:599,613,683,766,829`); the editing render does
   `display_text.to_string()` plus `char_range_to_string` pieces, each an O(n)
   `char_indices().nth()` scan (`input.rs:364-379,1290-1305`), and password
   mode runs `"•".repeat(n)` twice per render (`input.rs:1096,1261`).
   UTF16↔char conversions are O(n) scans per IME callback (`input.rs:348-362`).
   Note the `EditState` core itself already has a zero-allocation contract test
   (`tests/allocation_contracts.rs`, bench `benches/edit_state.rs`) — the churn
   is in the render/callback shell around it. Cursor/caret math uses a
   hardcoded `char_width = 8.0` (`input.rs:491,971,991`) with no text
   measurement caching — a correctness limitation as much as a perf one.

7. **[Alloc] Accessibility registration mutates a global per component render.**
   `register_accessible` → `global_mut::<AccessibilityTree>()` per component
   per render (`accessibility.rs:975-978`); `register` clones the `ElementId`
   twice per node (`accessibility.rs:922-928`). The tree is documented as
   "rebuilt each render frame" (`accessibility.rs:901`) but the crate never
   clears it — hosts that don't call `clear()` accumulate stale nodes.
   Whether `global_mut` fans out re-renders through `observe_global`
   subscribers is (needs profiling).

8. **[Roundtrip] None found.** No GPU readback, offscreen-render→image→paint,
   or synchronous device polling anywhere in the crate (grep over `src/`,
   `audio/` excluded). The crate inherits GPUI's scene model, which is fine.

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | Don't clone the graph per render: keep render data as `Arc<WorkflowGraph>` swapped only on structural change / viewport change; derive connection/obstacle vecs lazily | 1 | M | Highest — kills per-mousemove O(graph) clones |
| 2 | Rasterize QR matrix to a cached bitmap once; paint one image quad (static + animated via texture offset) | 4, 5 | M | O(modules²)→O(1) primitives/frame; moves scaling to GPU |
| 3 | Cache flattened connection polylines keyed on (from, to, zoom, tolerance); reuse a scratch `Vec` in hit testing with AABB pre-reject | 2, 3 | M | Removes per-frame flatten+tessellation and per-click allocs |
| 4 | Share `Arc<[QrColor]>` into animated QR canvas closures; skip off-viewport rows/cols by index math | 5 | S | Removes 30 fps clone + most clip iterations |
| 5 | Input: pass `&str`/SharedString to `on_text_change` instead of fresh `String`; split-by-byte-offset precomputed in one pass; cache bullet mask | 6 | S | Per-keystroke allocs drop sharply |
| 6 | Add lifecycle/clear discipline (or dirty-diff) for `AccessibilityTree`; extend `allocation_contracts.rs` to Input render + workflow drag paths | 7 | S–M | Bounds memory; prevents regressions |
| 7 | Longer term: move workflow connection rendering to GPU curve evaluation (vello2d-style) | 2 | L | Removes lyon CPU tessellation entirely |

## Quick wins

- Replace `Arc::new(self.state.graph.clone())` with an `Arc<WorkflowGraph>`
  stored on the canvas and only replaced on mutation (finding 1) — the render
  closures already take `Arc::clone(&graph)`.
- `AnimatedQrCode`: hold `Arc<[QrColor]>` and clone the `Arc`, not the `Vec`
  (`animated_qr_code.rs:187,227`).
- Hit-test: early AABB reject around the from/to bounding box before calling
  `connection_path` (`hit_test.rs:92-96`).
- Input password masking: build the bullet string once per value change, not
  twice per render (`input.rs:1096,1261`).
- QR static path: skip the double loop bounds when `modules == 0` is already
  handled; batch row runs of adjacent dark modules into single wider quads —
  one-line change that typically halves primitive count (paint.rs:35-53).
