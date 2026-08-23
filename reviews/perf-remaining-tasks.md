# Performance review implementation task list

Updated: 2026-08-23

This list consolidates the unfinished work from `reviews/perf-*`. A checked
item has an implementation in the current `main` worktree; partially checked
items still have the explicit follow-up shown beneath them.

## P0

- [x] Remove the legacy `gpu2d::Chart2DElement` readback/re-upload renderer.
- [ ] Retain Vello painter/chart state across parent rebuilds.
  - Migrate remaining transient `VelloChartElement::with_builder` callers in
    gpui-px and short-lived `VelloScenePainter` callers in gpui-audio-kit.
  - Add repaint/re-registration counters and warmed allocation contracts.
- [x] Retain workflow connection and obstacle render snapshots until graph,
  selection, or viewport inputs change.

## P1

- [x] Return shared `Arc<LineLayout>` values from the platform text contract
  and AU/iOS layout caches.
- [ ] Finish glyph bitmap ownership/pooling for AU and iOS.
  - Remove the remaining AU scratch-to-result copy and iOS grayscale Canvas
    allocation without leaving Core Graphics contexts pointing at moved data.
- [x] Replace Android's per-key Java `KeyEvent` with a retained
  `KeyCharacterMap`.
- [x] Make `SolvedTree::as_map()` an allocation-free retained-index view and
  expose retained solver capacity APIs.
- [ ] Replace the builder text-measure trait-object pointer cache key with an
  explicit stable measurement identity/revision contract.
- [x] Bound keybinding search/hint caches and replace full-entry hashing with
  retained allocation identity.
- [ ] Complete pretext's range-backed segmentation and Knuth-Plass result
  ownership work. The common no-split numeric path now moves rather than
  clones strings.
- [ ] Cache complete Python-runtime `TriangleMesh` and `ScalarField` objects,
  and retain all showcase `ElementId` values instead of formatting per render.
  Temporary JSON dirty-domain trees and sorted object-key allocations are
  removed.
- [x] Move process-wide shared `MeshCompute` ownership into gpui-d3rs.
- [x] Coalesce redundant iOS forced-frame requests until the next display-link
  tick.

## P2

- [ ] Complete wasm scheduling measurement.
  - Medium/low immediate tasks now use `MessageChannel`; record browser
    dispatch/frame baselines in the wasm QA harness.
- [ ] Expand allocation/performance contracts for Vello repaint, axes, mesh
  compute, audio knob/spectrum paint, px hover, and workflow drag/Input render.
  Keybinding warmed palette and hint hits are covered.
- [x] Add accessibility-tree begin/end-frame lifecycle cleanup and stale-node
  regression coverage.
- [x] Finish small robustness cleanup implemented in this pass: theme child
  notification, AU stable unknown-key fallback, removed legacy gpu2d comments
  and shader/readback files.

