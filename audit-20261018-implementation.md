# Audit implementation and evidence

Tracking issue: http://192.168.1.32:3001/pierre/gpui-toolkit/issues/7

Branch: `perf`. The original audit remains the requirements source. A checked
item requires implementation and verification evidence, not just a planned API.

## Correctness

- [ ] C1: camera-relative clipping, translation invariance and precision.
- [x] C2: coherent prepared-flow field updates and provenance.
- [ ] C3: validity masks across transforms and rendering.
- [ ] C4: alpha/interpolation/wireframe/line/capability/transparency semantics.
- [ ] C5: checked sizes, allocation budgets and bounded decompression.
- [ ] C6: FFI ownership, thread confinement and reentrant lifetime tests.
- [ ] C7: grapheme editing and measurement cache identity.

## CPU, memory and power

- [ ] P1: retained scene preparation, bounded topology extraction and DPI.
- [ ] P2: shared GPU context, resident outputs, asynchronous staging and limits.
- [ ] P3: byte-budgeted caches, error-state retention and trim hooks.
- [ ] P4: shared table rows, viewport virtualization and stable interaction.
- [ ] P5: compact volume acceleration and bounded streaming export.
- [ ] P6: bounded Python control lines and aggregate queue bytes.
- [ ] Demand-driven platform scheduling and animation liveness.
- [ ] Thermal, low-power and memory-pressure integration.
- [ ] Physical-device lifecycle, memory and energy qualification.

## Quality and scene architecture

- [ ] Q1: correct target resolution, fresh required samples and strict comparison;
  collector/comparator fixes are verified, with device-specific provenance and
  full CI qualification still required.
- [ ] CPU/GPU numerical, retention, input, lifetime and fuzz gates.
- [ ] Taffy instrumentation and retained-node experiment with decision evidence.
- [ ] Scene phases A/B: observability, host GPU draw and revisioned retention.
- [ ] Scene phase C: portable resident flow, volume and mesh transformations.
- [ ] Scene phase D: CUDA experiment, interoperability and adoption evidence.
- [ ] Scene phase E: MLX experiment, bounded memory and adoption evidence.
- [ ] Full acceptance matrix, broad QA, private PR and completion review.

## Evidence log

- Initial state: audited commit `f556d489c6b78ff3c41d1b858ad44f76b59d2916`;
  only the audit report was untracked. No implementation item is complete yet.
- Goal turn 1: progress. The implementation scope remains the entire audit.
- Q1: 20 performance-script tests pass. These include a real Cargo metadata
  fixture with separate environment/config target directories, isolated sample
  subprocesses, empty/missing/noise-only comparisons, configuration mismatches,
  malformed timings and explicit benchmark-removal handling. CI uses the same
  collector for base and candidate workspaces. Scene and flow families are
  required. A real Criterion scene run emitted all three fresh records into
  the resolved isolated target; comparing those records with themselves passed
  as a collector/comparator smoke test, not as evidence of a speedup.
- C2: mandatory-Metal tests verify updated velocity and diagnostic fields,
  `None` after update, invalid-length rejection and restoration to rotation.
  `diagnostics` now requires mutable access to serialize input revisions.
- C1 is partially implemented: GPU vertices and clipping planes use the same
  local origin, with translation tests through `1e9`; CPU camera/world precision
  and further geometric cases remain open.
- C3 is partially implemented: threshold honors masks; rendering borrows point
  scalars, excludes invalid cell averages, and removes invalid triangles before
  GPU upload. Other transformations and volume validity remain open.
- C4 is partially implemented: transfer alpha chooses the transparent pass,
  missing samples do not write depth, and empty GPU chunks are omitted. Wireframe,
  interpolation parity, clipping capabilities and compositing remain open.
- C5 is partially implemented: CPU surface/volume frames and GPU target/readback
  have checked dimensions and explicit byte budgets; VTU has bounded input,
  decoded arrays/numeric expansion, descriptor counts and block counts. Header
  overflow/truncation and decompression beyond declared sizes are rejected before
  unbounded expansion. Further device-buffer/import aggregation remains open.
- P1 extraction: fixed-size face keys remove per-face allocations; single-entry
  hash lookup, compact optional faces and canonical ordering are covered by 13
  mesh unit tests. Stack-backed clipping/projection and immediate opaque drawing
  remove the remaining ordinary per-triangle allocations. Retaining geometry and
  host GPU integration are still required.
- Bounded release probe, 48³ grid, 128×128 image:

  | Operation | Audit allocation calls | Current calls | Audit allocated bytes | Current bytes |
  |---|---:|---:|---:|---:|
  | Prepare | 1,245,941 | 77 | 73,642,256 | 45,090,288 |
  | Software frame | 1,298,975 | 80 | 77,760,040 | 45,221,392 |

  Same bounded dataset/allocator methodology as the audit, linked to the perf
  worktree via `/Volumes/home_tmp/tmp/gpui-audit-perf-probe`. These are cumulative
  allocation bytes, not live/RSS memory. Shared-host timing remains too noisy to
  claim a latency or energy improvement. Wireframe and raw measure-cache probes
  still reproduce their original failures; those items remain unchecked.
- P3 partial: five tests execute the actual previously excluded tick-cache
  source, covering churn, oversized labels, shared heap storage and formatter
  invalidation. Browser GPU failure retains one terminal error instead of
  leaking on each query; wasm qualification remains open. Other caches remain.
- P6 partial: 14 Python transport tests pass, including bounded chunked and
  unterminated input, recovery, stderr history byte limits and existing child
  lifecycle/mmap handling. Aggregate inbound/pending queue bytes remain open.
- Core verification after the stack-buffer change: 65 transformation, 13 mesh
  and 34 scene tests passed with `GPUI_REQUIRE_GPU=1`; Metal initialized.
  The GPU example also passes `cargo check --features gpu,png` against the
  fallible target API. `git diff --check` passes.
  Full-workspace formatting already has unrelated baseline differences; changed
  Rust files are formatted individually without rewriting those files.
- Strict Clippy passes for mesh, transformation and scene libraries with GPU
  enabled (`--no-deps -- -D warnings`). The existing large-dataset and pressure
  pipeline integration tests pass. Four small pre-existing lint issues in the
  transformation library were cleaned up to keep that gate enforceable.
- Source refinement to C4: software line rendering already rejects outside
  endpoints. Its remaining bug is discarding a crossing segment instead of
  clipping it to the retained half-space; do not implement a duplicate endpoint
  rejection test and treat that as completion.

## API changes in this work

- `GpuRenderer::create_target` now returns `Result`; the GPU example uses `?`.
- `PreparedFlow::diagnostics` takes `&mut self`; callers retain one input revision.
- `RenderLimits` and explicit limited render methods allow smaller host budgets.
- `VtuReadLimits` and limited text/byte readers allow constrained import budgets.
- `axis::clear_tick_label_cache` invalidates formatter state and handles pressure.
