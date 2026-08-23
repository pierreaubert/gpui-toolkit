# Performance review implementation task list

Updated: 2026-08-23

This consolidates unfinished work from `reviews/perf-*`. Checked items are
implemented in the current `main` worktree.

## P0

- [x] Remove the legacy `gpu2d::Chart2DElement` readback/re-upload renderer.
- [x] Retain Vello painter/chart state across parent rebuilds.
  - GPUI element state now owns backend registration and cached scenes.
  - Remaining gpui-px and gpui-audio-kit transient callers use stable IDs and
    retained painters; registration/repaint counters cover the lifecycle.
- [x] Retain workflow connection and obstacle snapshots until graph,
  selection, or viewport inputs change.

## P1

- [x] Return shared `Arc<LineLayout>` values from the platform text contract
  and AU/iOS layout caches.
- [x] Finish AU/iOS glyph bitmap ownership.
  - Window-owned upload storage is reused through `rasterize_glyph_into`.
  - AU and iOS render directly into caller storage; CGContext pointers cannot
    outlive the call or point at a subsequently moved/resized Vec.
- [x] Replace Android's per-key Java `KeyEvent` with a retained
  `KeyCharacterMap`.
- [x] Make `SolvedTree::as_map()` an allocation-free retained-index view and
  expose retained solver capacity APIs.
- [x] Replace builder's trait-object pointer cache key with an explicit
  `TextMeasure::cache_key()` identity/revision contract.
- [x] Bound keybinding search/hint caches and replace full-entry hashing with
  retained allocation identity.
- [x] Complete pretext range-backed segmentation and Knuth-Plass result
  ownership; remove per-grapheme cache entries and fold grapheme budgeting
  into analysis.
- [x] Cache complete Python-runtime `TriangleMesh` and `ScalarField` objects
  and replace formatted showcase element IDs with allocation-free stable IDs.
- [x] Stream dirty-domain fingerprints without temporary JSON strings,
  object-key vectors, or sorted-key allocations.
- [x] Move process-wide shared `MeshCompute` ownership into gpui-d3rs.
- [x] Coalesce redundant iOS forced-frame requests until the next display-link
  tick.

## P2

- [x] Record the wasm browser scheduling baseline.
  - Medium/low immediate tasks use `MessageChannel` and `just wasm-check`
    passes.
  - The opt-in hello-web harness at
    `qa/perf/wasm-scheduling-baseline.js` records reproducible dispatch/frame
    statistics and environment metadata.
  - Firefox/macOS results are recorded in
    `qa/perf/wasm-scheduling-baseline.md`: `MessageChannel` median 0 ms and p95
    0.02 ms versus `setTimeout(0)` median 4.58 ms and p95 5.04 ms; frame median
    16.66 ms and dispatch-to-frame median 16.46 ms.
- [x] Expand allocation/performance contracts.
  - Vello registration/repaint and scene keys.
  - Axis/chart glyph cache and shared MeshCompute access.
  - Audio knob drag and spectrum updates.
  - MeshPlot hover/navigation/field replacement.
  - UI-kit Input editing and workflow node-drag updates.
- [x] Add accessibility-tree begin/end-frame cleanup and stale-node regression
  coverage.
- [x] Finish robustness cleanup: theme child notification, AU stable unknown
  key fallback, and removal of legacy gpu2d comments/shader/readback files.
