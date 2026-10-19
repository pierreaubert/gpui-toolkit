# Audit implementation and evidence

Tracking issue: http://192.168.1.32:3001/pierre/gpui-toolkit/issues/7

Branch: `perf`. The original audit remains the requirements source. A checked
item requires implementation and verification evidence, not just a planned API.

## Correctness

- [x] C1: camera-relative clipping, translation invariance and precision.
  Validated local equations, robust coefficient rescaling, explicit precision
  limits and CPU fallback are covered alongside the camera/frustum regressions.
- [x] C2: coherent prepared-flow field updates and provenance.
- [ ] C3: validity masks across transforms and rendering.
- [ ] C4: alpha/interpolation/wireframe/line/capability/transparency semantics.
- [ ] C5: checked sizes, allocation budgets and bounded decompression.
- [ ] C6: FFI ownership, thread confinement and reentrant lifetime tests.
- [ ] C7: grapheme editing and measurement cache identity.

## CPU, memory and power

- [ ] P1: retained scene preparation, bounded topology extraction and DPI.
  Scene-owned retention, DPI-aware presentation and verified canonical-grid
  boundary traversal are implemented; general topology extraction and aggregate
  preparation budgets remain open.
- [ ] P2: shared GPU context, resident outputs, asynchronous staging and limits.
  Shared-device host encoding and resident GPUI presentation are implemented;
  staging/resource budgets, other renderers and device qualification remain.
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
- `CellToPointPlan` and `CellToPointLimits` expose bounded snapshot preparation;
  `TransformError::ResourceLimit` adds a preparation-budget rejection variant.
- `gpui_mesh::try_for_each_surface_chunk` visits source-owned chunks from a
  `ValidatedDataSet`; public scene `prepare_chunks` validates storage before use.
- Surface uniform ABI **4** keeps the 192-byte block and wireframe ULP slot.
  Physical viewport dimensions occupy bytes 184/188; point-cell hosts use
  `scene_uniform_block_for_target`. Older builders leave that viewport zero.

## Retained scene and host GPU checkpoint

- `Scene::prepare` retains immutable geometry across camera and material changes.
  Copy-on-write changes to positions, topology, selected scalar values or masks
  invalidate it. Scene clones share one cache; duplicate/reordered actors reuse
  prepared geometry. Unrelated fields are not retained by that cache.
- `Scene::clear_preparation`, `preparation_stats` and `PreparedScene::byte_len`
  expose cache release and counters. Revision keys pin source allocations for
  safe identity checks without retaining derived CPU geometry. A weak-reference
  test verifies that clearing the cache actually releases those derived buffers.
- The software frame loop borrows prepared chunks and cached line indices.
  The same bounded release probe now reports:

  | Operation, 48³ grid | Initial audit calls | Previous checkpoint calls | Current calls | Current allocated bytes |
  |---|---:|---:|---:|---:|
  | Explicit preparation | 1,245,941 | 77 | 77 | 45,090,288 |
  | Warm 128×128 software frame | 1,298,975 | 80 | 2 | 131,072 |

  The two frame allocations are RGBA and depth buffers. This is cumulative
  allocation traffic for warmed rendering, not live/RSS memory: one-time
  preparation and retained geometry still cost memory. Grids 16³ and 32³ also
  produce two allocations and 131,072 bytes per warmed frame. Timings are not
  used to claim latency or energy gains on this shared host.
- `SceneElement::id` opts into window-owned presentation state. Identified views
  reuse a single unchanged image, rasterize at physical display dimensions, and
  invalidate on camera, data, material, clipping, background or size changes.
  The showcase and interactive viewer now use stable IDs.
- With `gpui-gpu`, identified views on wgpu hosts render on the existing host
  device, encode into the host command encoder and composite a resident texture.
  There is no pixel readback or second device. Camera/material updates preserve
  geometry buffers, and unchanged scene frames reuse the offscreen target.
  The view unregisters its custom draw when its window-owned state is dropped.
- GPU view upload releases derived CPU geometry. Native Metal tests verify zero
  retained preparation bytes afterwards, with no rebuild for material changes.
  This does not eliminate canonical source arrays or initial preparation peaks.
- Uniform copies are ordered inside the host encoder. LUT changes create new
  material resources, preserving earlier encoded passes. Tests encode two
  cameras and two material revisions before one host submission and check both
  images, rather than testing only separate submissions.
- Offscreen color is premultiplied for host compositing. Native export converts
  it to straight RGBA, skipping opaque pixels. Tests check alpha, host pixels
  outside the visible rectangle and clipped texture coordinates. An unused
  transfer function no longer overrides flat-color alpha or software color.
- GPU wireframe, line cells and more than six clip planes now return explicit
  unsupported-feature errors. GPUI uses software for those scenes. Implementing
  the missing features and remaining CPU/GPU parity issues is still required.
- Verification: 48 scene library tests pass with `GPUI_REQUIRE_GPU=1` on Metal;
  65 transformation and 13 mesh tests pass. GPU-enabled scene examples and the
  software-only GPUI library compile. Strict Clippy passes for the three core
  libraries including the GPUI GPU path. The scene library compiles cleanly for
  wasm with `gpui-gpu`, nightly and the repository's build-std configuration.
  Browser runtime/device behavior and native non-wgpu presentation are not
  qualified by those compile and offscreen tests.
- Probe correction: the wireframe comparison now uses a valid scalar field.
  Comparing an entirely masked surface could not establish a wireframe bug.
  The corrected probe still reports identical images; the feature remains open.

Additional API changes:

- Construct scenes with `Scene::new()` or `Scene::default()`; scene-owned cache
  state makes external field-only struct literals incompatible.
- `GpuRenderer::from_shared_device`, `encode` and `encode_to_views` support host
  presentation; `sync` updates materials and reports geometry replacement.
  Geometry/field revisions currently replace the whole uploaded scene; finer
  per-actor and field-buffer updates remain open.
- Standalone `new_async` takes an explicit power preference. Native `new` now
  defaults to low power. Blocking initialization and `read_rgba` are native-only.
- `GpuTarget::color_view` exposes the premultiplied sampleable texture. Caller
  attachments use `GpuAttachments` and retain caller-owned submission ordering.

Goal turn 2: progress. The full audit scope is still open, including Taffy
experiments, CUDA/MLX adoption experiments, remaining correctness findings,
other platform hot paths and physical mobile memory/energy qualification.

## Camera, clipping and raster checkpoint

- Camera targets and world-space eye positions now use `f64`. Orientation comes
  from orbit angles without subtracting rounded world positions. View matrices
  subtract the origin before the float conversion. Fitted fractional targets,
  panning and projection are tested through origins of `1e12 + 0.125`.
- Software surface vertices remain local throughout clipping and projection.
  User planes use the same local origin, and polygons are clipped against all
  six homogeneous camera planes before perspective division. Near/far-outside
  triangles leave the background unchanged; crossing geometry is compared with
  an independently constructed clipped mesh.
- Two actors with distinct origins and oblique planes produce identical images
  after translation within each backend. Metal/software coverage differs by at
  most four pixels per color at 96×64; color interpolation is evaluated separately.
  Axis-plane coverage also agrees at all tested origins. Existing scene3d has no
  picking implementation; its new analytic ray/projection agreement tests do not
  claim to qualify the separate d3rs picking implementation.
- Volume rays are precomputed analytically in local coordinates. Orthographic
  rays are parallel, and both projections respect near/far limits. Changing
  orthographic eye distance preserves the image. Translated-volume coverage is
  unchanged through `1e12 + 0.125`, with mean byte error no greater than one.
- `Camera::fitted` now encloses the bounds' sphere with a field-of-view margin.
  The old `1.2 * diagonal` radius could crop a cube corner. Validated
  `fitted_for_aspect` also fits portrait and landscape viewports. Tests project
  every corner and reject invalid aspect/bounds.
- Invalid cameras fail before presentation retention or frame/staging work.
  Narrow valid frustums can dolly without a reversed-clamp panic. GPU render,
  encode and caller-view methods now return `Result`; examples and host drawing
  handle it. Uniform staging checks the device buffer limit. Equal GPU sort
  distances have a deterministic actor-order tie break.
- Clipped lines retain their crossing segment instead of being dropped. Frustum
  clipping bounds raster iteration by viewport size. A very long segment is
  tested in the clipping helper without running an unbounded pixel loop.
  Line composition visits each pixel once per segment, preserves straight alpha,
  and writes opaque depth; reversing opaque actor order preserves the image.
- Eight-bit subpixel edge equations and the top-left rule remove coverage changes
  caused by triangulation and prevent double blending on shared edges. Transparent
  quads have identical images under either diagonal and no alpha seam. Surface
  and line source-over composition preserves destination alpha. A constant planar
  translucent triangle agrees with Metal's straight RGBA within two byte levels.
  Volume background alpha, varying-field interpolation, wireframe and general
  intersecting transparency still need the remaining C4 work.
- Ordinary unclipped triangles now project and rasterize directly. Conservative
  bounds classification skips unnecessary per-triangle clipping checks, while
  boundary cases use the full clipper. The first corrected version showed a
  28–34% larger-mesh CPU cost increase in a paired diagnostic. The direct triangle
  path removed that observed regression:

  | Fixed camera, warm 128×128 frame | `9d50f5bf` median ms | Current median ms |
  |---|---:|---:|
  | 16³ | 0.3695 | 0.2975 |
  | 32³ | 0.9820 | 0.9160 |
  | 48³ | 1.8230 | 1.8140 |

  Four process medians per version, ordered ABBA twice after process warmup;
  each process uses the audit's one-warmup/five-sample recipe. Both probes use
  the old fixed camera radius, so the fitting correction cannot appear as a CPU
  gain. All process records report median counts of two allocations and
  131,072 allocated bytes. The
  [raw evidence](qa/perf/20261018-scene-clipping-paired.json) includes CPU/OS/Rust
  identity, source hashes, sample arrays and methodology. Shared-host variation
  prevents interpreting these as energy or full-application latency evidence.
- Verification: 67 scene library tests pass with required Metal; strict scene
  Clippy passes. The actual native viewer test passes at 2× DPI, driving orbit,
  pan, zoom, reset and clipping controls while checking state and rendered-image
  changes. Required-Metal CI now runs that test and fails instead of skipping
  when its renderer is unavailable. Existing pressure-pipeline and large-dataset
  integrations pass, all scene examples compile, and the wasm library compiles.
- C1 remains unchecked pending extreme plane-coefficient contracts: world plane
  normals are still cast to shader floats, and nonfinite/overflowing coefficients
  need explicit validation or robust rescaling. The tested coordinate fixes do
  not prove correctness for those inputs. The full objective remains open.

Goal turn 3: progress. No broad completion or physical power/memory claim.

## Plane coefficient and presentation checkpoint

- Clipping now uses `dot(local_position, normal) - offset` in every path.
  Normals are rescaled in `f64`, local offsets use compensated products/sums,
  and tangential origin components never cross the float boundary. Opposite-sign
  terms cancel before an intermediate sum can overflow. Nonfinite inputs and
  offsets beyond `f64` range return typed errors.
- Tests rescale normals by `1e-300`/`1e300`, move the plane point by `1e12` along
  a tangent, and give its unused axis a `1e250` coordinate. Metal and software
  images remain identical to their respective controls; volume clipping also
  preserves its image. Zero normals retain all points. Sufficiently distant
  planes use finite constant half-space coefficients instead of infinities.
- Required coefficients that underflow or become subnormal shader floats return
  `UnsupportedPrecision`. This includes coefficients hardware may flush to zero.
  The GPUI presentation decision selects and retains an actual software image,
  then switches back to a custom draw when the coefficients become representable.
  A coplanar fixture proves that ignoring the small coefficient would change
  the selected half-surface. Invalid coefficient updates cannot partially change
  another actor's material or the uploaded scene background.
- Shader uniform ABI **version 2** packs six plane equations into one vector each.
  The block is **192 bytes**, down from 288; GPU buffers, ordered staging copies
  and the WGSL layout use the same exported size constant. Plane equations are
  prepared on upload/clip changes and reused during camera frames.
- GPU compatibility previously walked every cell type on each GPUI paint. It is
  now cached by source revision. Camera and material-only updates reuse topology
  verdicts and shared origin metadata; tests check 24 camera updates without a
  second topology scan. Unsupported-precision verdicts are retained as well.
- Verification: **78** scene library tests pass with required Metal, including
  actual shader output and the window's presentation-selection path. Native
  2×-DPI viewer interactions, pressure integration, strict Clippy, all scene
  examples, software-only GPUI and clean wasm compilation pass. The bounded
  release probe still reports median **2 allocations / 131,072 bytes** for warm
  frames at 16³, 32³ and 48³. No power or mobile runtime qualification is claimed.
- C1 is checked against its camera/coordinate/half-space requirements. The
  separate platform qualification, data-size, material/interpolation and numeric
  transformation requirements remain open.

API migration for custom GPU hosts:

- `ClipPlane::to_uniform` returns `Result<[f32; 4], SceneError>` containing
  `[normal.xyz, offset]`; it no longer returns an origin/normal pair.
- Public scene uniform builders return `Result` and use the 192-byte ABI.
  Hosts must use the matching `SURFACE_WGSL`, `SCENE_UNIFORM_VERSION` and
  `SCENE_UNIFORM_SIZE`; the plane count is at byte 176.

Goal turn 4: progress. The full audit implementation remains active.

## Transfer storage, material validation and volume alpha checkpoint

- Built-in palettes now borrow static immutable stops. Custom palettes sort once
  and share `Arc` storage across scene/material snapshots; changing a clone's
  domain does not change its source. Opacity classification uses cached stop
  metadata instead of walking the palette on each query.
- `TransferFunction::validate` rejects nonfinite/reversed domains, invalid log
  domains, zero bands and invalid stop positions/RGBA. Direct `map`/`lut_bytes`
  calls return transparent results for malformed settings instead of a lookup
  panic. Active render settings fail before geometry preparation or GPU upload.
  Unused transfer/flat-color settings do not invalidate a retained frame.
- `TransferFunction::prepare` returns a borrowed, allocation-free sampler. Surface
  and volume rendering prepare domain constants once, including stable wide
  linear ranges and close/wide logarithmic ranges. Banded log mapping applies
  its quantization as well. The public convenience lookup remains available.
- The raw-f32 GPU path now rejects collapsed/overflowing/subnormal transfer
  domains explicitly. GPUI retains a software image for those domains, preserves
  that decision through camera updates, and returns to GPU presentation when the
  domain becomes representable. Rejected GPU material updates leave previously
  uploaded materials and background intact; tests render before and after the
  rejection to verify the transaction. This does not solve raw f64 field-to-f32
  precision or CPU/GPU interpolation parity; C4 remains open.
- Volume source-over composition now includes background alpha and exports
  straight RGBA. Transparent samples preserve a translucent background, and
  translucent volume samples over transparent/translucent backgrounds retain
  their alpha and expected color mixtures.
- Verification: **88** scene library tests pass with required Metal. The native
  2x-DPI viewer and pressure integration pass; all scene examples, software-only
  GPUI and wasm compile. Strict Clippy passes with default features and with
  `gpui-gpu`. The bounded release probe retains **2 allocations / 131,072 bytes**
  per warmed 128x128 frame on 16³, 32³ and 48³ datasets.
- The [allocation evidence](qa/perf/20261019-transfer-allocation.json) includes
  probe source, source hashes and five-sample arrays. Built-in construction,
  custom palette cloning and 10,000 prepared lookups each report zero allocation
  traffic in every recorded sample. Custom palette construction, retained source
  memory, whole-application latency and energy are outside that measurement.

Additional API: `PreparedTransfer` and `TransferFunction::prepare` are public;
render entry points can return `SceneError::InvalidMaterial` for invalid active
RGBA, opacity or transfer settings. No broad completion claim is made.

## Raw measurement identity checkpoint

- Raw `MeasureCache` lookups now select the provider's semantic key/revision
  before returning any width, grapheme or prefix metric. Unkeyed providers start
  fresh on each raw request, so recycled stack addresses cannot reuse old data.
  The original bounded reproducer now returns **3 and 6**, rather than 3 and 3.
- `MeasureCache::scope` returns a borrowed `MeasurementSession` whose methods
  accept only text. This enforces one provider during a solve while allowing
  cached lookups for default, unkeyed providers. New unkeyed sessions invalidate
  prior metrics; stable semantic keys may reuse across sessions. Interior metric
  changes still require a new `cache_key`, as documented by `TextMeasure`.
- Ordinary pretext preparation now uses that scoped path. Existing cache-hit
  allocation tests and benchmarks declare their immutable provider's semantic
  key explicitly. A new allocation contract covers 1,000 warmed width, grapheme
  and prefix requests through an unkeyed session, with zero allocation traffic.
- Regression tests cover different temporary providers, unkeyed in-place changes
  between raw requests, all three metric kinds, reuse within one borrow, a fresh
  session at the same address, and semantic revision changes inside a session.
- Verification: 166 pretext library tests and its selected integration/corpus/
  property/allocation tests pass; 134 builder library tests and its selected
  integrations pass. Strict pretext Clippy, benchmark compilation and wasm
  compilation pass. Raw unkeyed lookups intentionally remeasure; callers needing
  repeated caching should use a session or a truthful stable semantic key.
- C7 remains open for grapheme editing/native text contracts. P3 remains open for
  O(1) recency, full payload byte accounting, oversized-entry admission and the
  other toolkit caches. Identity correctness does not complete those requirements.

Cache benchmark smoke operations, release allocation contracts and both public
documentation examples also pass. Smoke execution is not a latency comparison.

## Measurement cache retention and CPU checkpoint

- Clock eviction replaces the linear recency-queue scan on every hit. A hit sets
  one second-chance bit. Admission/eviction can walk the queue; this is approximate
  recency, rather than the former strict LRU policy. Eviction and admission reuse
  known hash lookups, and width-only hits no longer clone stored grapheme arrays.
- Byte admission now charges shared keys and metric arrays, Arc headers/padding,
  actual hashbrown allocation size and recency-queue capacity. Over-budget entries
  can be evicted, including the last entry. Zero limits disable retention.
  `MeasureCacheStats` exposes payload/container charges, retained high-water,
  entries and activity. `trim` releases cache-owned allocations; `clear` preserves
  reusable container capacity and counters.
- `get_segment_metrics` now returns an owned, shared-array snapshot. This allows
  oversized results to be returned without a reference pinning them in the cache.
  Caller-owned snapshots/arrays, allocator bookkeeping and transient table
  growth/shrink working memory are outside the retained-byte limit. This is not
  a total-process or peak-allocation budget.
- Each of the two thread-local scratch buffers retains at most 64 KiB. Oversized
  working buffers are dropped after the operation. Scratch is rented out of TLS
  while calling a measurer, so nested preparation does not panic on a live TLS
  borrow. A pressure trim generation prevents active operations from returning
  their old scratch after a trim request. Public current-thread scratch byte and
  trim hooks expose that separate retained storage.
- Tests cover byte accounting for arrays and containers, 1,000 churn iterations,
  oversized results, zero/tiny budgets, shared-array release, second-chance
  retention, nested measurement and scratch trimming during an active callback.
  The serial allocation contract checks cold container/array charges against
  actual allocator request bytes and keeps zero-allocation keyed/scoped hits.
- Paired bounded diagnostic against `8851de7f`:

  | Work per sample | Baseline median ms | Current median ms |
  |---|---:|---:|
  | 10,000 randomized hits, 128 entries | 2.2221 | 0.0855 |
  | 10,000 randomized hits, 4,096 entries | 39.3141 | 0.1391 |
  | 10,000 admissions rotating 512 strings through eight entries | 0.4658 | 0.4723 |
  | 100 full preparations of the Unicode paragraph | 2.4593 | 2.1024 |

  Four process medians per version, ABBA twice; one warmup and five samples per
  process. Entry counts and width sums are checked outside timing. Random hit
  selection uses the high bits of a 64-bit LCG; repeating a low-bit permutation
  would make this a misleading LRU benchmark. Both hit variants allocate zero.
  Admission calls are 10,007 to 10,006, with 404,188 to 404,540 allocated bytes;
  preparation calls stay 43,400, with 6,077,000 to 6,125,000 bytes (about +0.8%).
  Admission time is about +1.4% in this run; shared-host variation prevents an
  equivalence claim. These are cache/text workloads, not full-frame or energy
  acceptance. The [raw paired evidence](qa/perf/20261019-pretext-clock-paired.json)
  includes all samples, identical probe source, hashes and methodology.
- The first clock implementation showed +27% admission time; reducing redundant
  hash lookups removed most of that observed cost before this checkpoint. The
  retained-byte budget is enforced after operations and can require cold-path
  shrinking; its allocator working-set peak still needs the separate C5 work.
- Verification: 170 pretext library tests plus its integration/corpus/property/
  allocation suites pass. Builder tests, strict Clippy, populated benchmark
  smoke, release allocation contracts, public docs and wasm compilation also
  pass for this checkpoint. P3 remains open for the other caches, pressure
  wiring and application/device qualification; C7 editing work is still open.

API migration: `MeasureCache::get_segment_metrics` and the session equivalent
return `SegmentMetrics` by value. Shared arrays remain cheap to clone. A zero
`with_budgets` entry limit now explicitly disables retention. New public APIs:
`MeasureCacheStats`, `MeasureCache::stats`/`trim`, current-thread scratch byte/
trim hooks and `MAX_RETAINED_METRIC_SCRATCH_BYTES`.

## Validity, empty results and retained volume inputs checkpoint

- `Field::is_valid` now defines the common sample predicate: an existing,
  unmasked entry whose components are all finite. `FieldData::entry_is_finite`
  supplies the shape-independent component test. Missing mask slots fail closed;
  dataset validation still reports malformed lengths and rejects infinities.
- Newly computed fields use one validity completion path. Unsupported/nonfinite
  outputs become masked NaNs, so finite placeholders cannot establish validity.
  Fully valid results avoid a mask allocation. Point calculators share the source
  mask when it remains exact; a new computed failure creates a new mask without
  changing the source. Magnitudes use a cheap ordinary squared-norm path with
  `hypot` fallback for overflow/underflow, covering representable values around
  `1e±200`. Magnitudes preserve source units; complex phase records radians.
- Interpolation requires every nonzero contributor to be valid, without
  renormalizing away missing contributors. Exact zero weights are skipped before
  multiplication and validity checks, so a valid endpoint beside a masked NaN
  stays valid. Scalar/vector/tensor/complex interpolation shares the same rule
  and writes output directly without an intermediate scalar buffer.
- Volume probing, volume rendering and controlling-field isosurface extraction
  require all four tetrahedral nodes to be valid. A missing region cannot create
  a finite probe or isosurface from placeholders. Surface probing returns `None`
  at a missing nearest node; it does not substitute a distant valid sample.
  Nonfinite queries/computed probes return `None`; nonfinite iso levels error.
  Auxiliary interpolated fields retain their own nonzero-support validity rule.
- Empty filter results are valid datasets when cells and attached fields are
  also empty. Empty selections can be filtered/probed again, and both CPU and
  Metal scene rendering produce only the requested background. Bounds remain
  `None`; geometry-dependent operations may still return explicit empty-input
  errors. Empty positions with nonempty cells continue to fail validation.
- `VolumeRenderer` shares immutable positions and selected scalar storage instead
  of cloning the dataset and copying scalars. This removes one eight-byte scalar
  copy per source point and stops retaining unrelated fields. Storage-identity
  and weak-reference tests prove sharing and release. Valid support is resolved
  when building the tetrahedral index; wholly missing input renders background
  without ray marching. Compact grid storage, budgets and field-only updates
  remain P5/C5 work.
- Regression evidence covers finite masked placeholders, masked NaNs, every
  component shape, short masks, zero interpolation weights, representable tiny/
  huge norms, computed overflow, mask sharing/invalidation, partial volume support,
  all-invalid volumes, missing nearest probes, empty pipelines and CPU/Metal empty
  presentation. Core library tests: 66 transformation, 14 mesh, 91 scene tests,
  with Metal required. The three public validity integration tests and existing
  large-dataset/pressure integrations pass. Further checks are logged below.
- C3 stays unchecked: point/cell conversion, CPU/GPU diagnostics, warp/glyph/trace
  and other field-consuming paths still need contract adoption and qualification.
  No whole-platform validity, total-memory or energy completion is claimed.

API addition: `Field::is_valid` and `FieldData::entry_is_finite`. Dataset validation
now accepts a consistently empty result. Calculator outputs expose validity masks
and correct derived units; unrepresentable derived magnitudes are missing rather
than valid infinities. The audit report remains the original requirements source.

Validity checkpoint verification completed: strict Clippy for all three core
libraries passes, the native 2x-DPI viewer passes, all scene examples compile,
and the final core wasm closure compiles. Rustdoc targets complete with zero
runnable examples; this is not additional behavioral coverage. The bounded
release probe still reports two allocations / 131,072 bytes for warmed 128x128
software frames on 16³, 32³ and 48³ inputs; preparation allocation counts remain
60/71/77. The original mask and measure-cache reproducers report zero retained
threshold cells and widths 3/6. Wireframe remains unchanged and open under C4.

The next C3 work is conversion and CPU/GPU stencil diagnostics, followed by the
remaining consumers, time sampling and import/export validity boundaries. The
full audit objective, including Taffy and CUDA/MLX experiments and physical-device
qualification, remains active.

## Isosurface scalar-unit and extreme interpolation checkpoint

- A newly added metamorphic test failed on committed `c5c590d1`: multiplying
  scalar values and the iso level by `1e-200` moved the same extracted point from
  x=0.25 to x=0.5. The kernel compared a scalar difference with absolute machine
  epsilon and substituted a midpoint, making geometry depend on chosen units.
- Interpolation now treats only an exactly zero span as equal. Finite nonzero
  spans retain their ratio, including subnormal values; overflowing differences
  between finite endpoints use a half-scaled ratio. Position interpolation uses
  a finite weighted sum across opposite signs and anchors same-sign interpolation
  at the nearer endpoint, preserving exact endpoint coordinates.
- The regression checks unchanged geometry and controlling scalar values for
  scale factors `1e-200`, a representable subnormal and `f64::MAX`. Separate
  coordinates at ±`f64::MAX` remain finite, and an endpoint at -`1e-200` is retained
  beside a coordinate at -`f64::MAX`. These tests use four-point datasets and do
  not request large allocations. The public validity suite now has four tests.
- This closes this specific extractor precision bug, not C4 as a whole. Raw f32
  scalar preparation, cross-backend interpolation, wireframe/line paths and
  general transparency remain open. Conversions/diagnostics and the remaining
  C3 consumers remain the next validity work.

Extractor verification completed: 64 default-feature / 66 GPU-feature library
tests and all four public validity tests pass. The required-Metal pressure
pipeline, strict core Clippy and the wasm transformation library also pass.

## Validated point/cell conversion and shared decomposition checkpoint

- Point/cell conversion now handles scalar, vector, tensor and complex data
  componentwise, preserves labels and units, replaces the requested output name,
  excludes masked contributors, and emits masked NaNs when support is absent.
  Cell-to-point conversion uses positive representable geometric measures and
  the highest supported topological dimension at each point. Concave planar
  polygon weights use signed vector area; degenerate cells do not contribute.
  Finite averages survive overflowing intermediate sums/products through a rare
  bounded convex fallback. Measures that cannot be represented remain excluded;
  this does not close general extreme-coordinate requirements under C4.
- Dataset validation rejects association/storage mismatches and duplicate field
  names within one association. The same name across point/cell data is valid.
  Ordinary schemas validate without temporary name storage; larger schemas use
  borrowed sorted names. Public immutable `ValidatedDataSet` views allow a caller
  to pay full input validation once across multiple conversions. Rust's borrow
  rules prevent mutation while the view is reused; a compile-fail example tests
  that property. A dimension summary computed during validation selects compact
  eight-byte per-point weight scratch for uniform meshes. Mixed-dimensional
  meshes retain dimension-aware weights.
- One allocation-free `gpui_mesh::tetrahedra` iterator replaces seven duplicated
  allocating decomposition implementations across transforms and volume rendering.
  Tests verify cell counts, mapped indices, malformed arity and reference volumes;
  10,000 hexahedron decompositions make zero allocations. Conversion allocation
  tests bound calls and bytes as grid sizes increase, including wholly missing
  input. These are bounded fixtures, not proofs of aggregate memory limits.
- `qa/perf/20261019-conversion-paired.json` records identical release probe source,
  baseline `e812e685`, candidate source hashes, raw samples and separate allocation
  measurements. CPU instrumentation is disabled; ABBA is repeated twice, each
  process has a warmup and five samples, and outputs are checked outside timing.
  At 48³, cold cell-to-point scalar/vector medians improve 16.0%/11.6%; allocation
  calls fall from 207,656 to 7 and allocated bytes from 34.27/39.58 MB to 2.65/6.19
  MB. The batch includes one full validation and all four scalar/vector direction
  conversions in both variants: median CPU time falls 19.6%, calls 415,326 to 26,
  and bytes 80.50 MB to 15.49 MB. These are allocated bytes, not peak RSS.
- Point-to-cell is a measured regression: cold scalar/vector medians are
  2.98x/2.30x baseline because the old implementation skipped input validation and
  overflow protection. Reusing a prevalidated view reduces ratios to 1.31x/1.15x.
  This cost is explicit; callers doing multiple operations should retain the
  view rather than repeatedly validate unchanged input. No universal speedup,
  mobile budget or energy acceptance is claimed from these shared-host timings.
- Public validity integration coverage now includes all field shapes, partial and
  wholly missing input, isolated points, geometric weighting, mixed dimensions in
  both orders, concave polygon orientation, scale changes, finite extreme means,
  output replacement, schema errors and validation-view reuse. C3 stays unchecked:
  CPU/GPU stencil diagnostics and other field consumers still require adoption.

Strict core Clippy and the full core wasm library closure pass for this checkpoint.
The original audit remains the requirements source. TokenSave's served main graph
is stale relative to this worktree, so worktree source was checked directly. Its
absolute-path reader also rejected the unindexed worktree; a sanitized tool-gap
report can be filed at https://github.com/aovestdipaperino/tokensave.

Final conversion verification: 66 transformation, 15 mesh and 91 scene library
tests pass with Metal required, as do ten public validity tests, allocation
contracts and the pressure/large-dataset integrations. Two positive rustdoc
examples and the compile-fail mutation example pass. The lockfile adds only the
two native test allocator dependency edges; no production dependency is added.

## CPU/Metal flow validity and combined CPU diagnostics checkpoint

- CPU and both GPU flow paths now require all four velocity nodes of a
  tetrahedral stencil to be valid. Unsupported points produce masked NaNs,
  including isolated points, wholly missing input and degenerate cells. Computed
  nonfinite results and nonfinite accumulated support cannot establish validity.
  GPU velocity packing carries a validity flag and substitutes zero storage for
  masked samples so NaNs cannot contaminate supported neighbors.
- Cold CPU/GPU flow entry points validate input schemas before indexing. Cold
  GPU diagnostic outputs replace existing names and carry the same labels/units
  as CPU outputs. Unsupported raw GPU Jacobians contain NaNs. Unrepresentable
  f32 velocity/local-position input errors explicitly instead of silently
  overflowing; callers can use CPU diagnostics. This does not resolve C4's
  scale-dependent determinant cutoff or general CPU/GPU precision differences.
- Prepared velocity updates retain their prepared validity mask. Invalid length,
  unmasked NaN/infinity and unrepresentable f32 updates fail before modifying the
  GPU buffer or retained input revision. Masked NaN updates are supported. Tests
  confirm a failed update leaves subsequent `diagnostics(None)` coherent.
- New CPU `flow_diagnostics` computes vorticity, Q and divergence from one
  Jacobian pass. `vorticity_magnitude` computes directly and preserves a user
  field named `__vorticity_tmp`; it also replaces an existing requested output
  correctly and uses the robust magnitude helper. Fixed output units retain the
  existing assumption that coordinates and velocity use matching length units
  and seconds; general dimensional metadata remains outside this checkpoint.
- Three public flow regression tests include analytic rotation, finite masked
  placeholders, masked NaNs, partial stencil support, isolated points, wholly
  missing fields, degenerate geometry, output replacement, preserved user fields,
  malformed input and rejected-update rollback. Metal is required in verification.
- `qa/perf/20261019-flow-combined.json` compares two APIs in the same candidate
  build: a three-operation chain and the combined call. Four allocator-disabled
  release processes alternate case order; separate allocation builds record
  calls/bytes. At 48³ the combined median is 12.55 ms versus 34.89 ms (64.0% less
  wall time), with 98 versus 267 calls and 144.11 MB versus 414.64 MB allocated.
  Analytic outputs are checked outside timing. This is not a comparison against
  the previous commit, a total CPU-time measurement, peak RSS or energy evidence.
  The large allocation totals expose remaining materialization of tetrahedra
  and gradients in the CPU algorithm; bounded scratch remains necessary.
- Strict transformation all-target Clippy passes after fixing audit test
  assertions for the current lint set. The core wasm closure passes. Default
  CPU tests and required-Metal transform library/public tests pass; full core
  verification follows before committing. C3 remains unchecked because other
  field consumers, time sampling and IO boundaries are still open.

Flow checkpoint full-core verification completed: 66 transformation, 15 mesh and
91 scene library tests, thirteen public validity/flow tests, allocation contracts
and pressure/large-dataset integrations pass with Metal required.

## Streaming CPU Jacobians checkpoint

- CPU flow now streams one tetrahedral stencil at a time in deterministic cell
  order. It no longer materializes tetrahedra, gradients, worker results or
  scoped worker threads. The required per-point accumulator remains 80 bytes per
  point; returned fields and their Arc conversions add output-proportional
  allocations. This bounds scratch by point count, not by tetrahedron count.
- Each stencil reuses one adjugate for all three velocity components rather than
  repeatedly solving by Cramer's rule. Its volume-weighted Jacobian is computed
  once before contributing to its four nodes. Ordinary final normalization
  reuses a normal reciprocal; extreme reciprocals retain division. This changes
  floating-point rounding, so bit identity with the old algorithm is not claimed.
  Existing numerical scale limits remain C4 work.
- Independent tests check a non-axis-aligned affine field under orientation
  reversal, translation and rescaling, plus unequal adjacent volumes with
  different gradients. The latter verifies that Jacobians are averaged before
  the nonlinear Q diagnostic. A fixed four-point allocation contract holds from
  one to 20,000 repeated cells: no cell-count-dependent temporary allocation.
- `qa/perf/20261019-flow-streaming-paired.json` compares preserved release binaries
  from `63350761` with the new source. Baseline source hashes were verified before
  editing; probe source is identical. Allocator-disabled ABBA runs are repeated
  twice, with separate allocation runs and analytic checks outside timing. At
  48³, combined diagnostics improve from 12.52 to 9.22 ms (26.3% less wall time),
  with 18 versus 98 calls and 17.70 MB versus 144.11 MB allocated (87.7% less).
  The separate-operation chain improves from 34.89 to 24.85 ms. Worker aggregate
  CPU time, energy, peak RSS and mobile thermal behavior are not measured.
- Required-Metal full-core verification passes: 66 transformation, 15 mesh and
  91 scene library tests, fifteen public validity/flow tests, allocation contracts
  and pressure/large-dataset integrations. Transformation all-target Clippy and
  the core wasm closure pass. C3/P1/P2 remain unchecked: other validity consumers,
  GPU adjacency/staging budgets and the broader retention work are still open.

## Warp/glyph validity, topology and coupled eigensolver checkpoint

- Vector/scalar warps validate input before indexing and require finite scale.
  Missing samples retain their original coordinates and source masks. Zero-scale
  and wholly missing warps share source position storage; scalar zero displacement
  also avoids normal extraction. Computed nonfinite positions/normals error rather
  than returning an invalid dataset. FMA retains finite vector results when an
  intermediate product would overflow before cancellation; normal magnitudes use
  the robust helper. Surface extraction/normal scratch remains a C5/P1 budget item.
- Line, solid-arrow and tensor glyphs validate input and skip masked samples,
  including finite placeholders. Parameters must be finite and output point-index
  ranges are checked before append. Unrepresentable glyph geometry is omitted.
  Derived magnitude units follow the source. `GlyphKind::Line` now centers its two
  endpoints at ±`scale*v/2`; the old positive endpoint was `scale*v`, producing an
  asymmetric line. Tiny/huge representable vector norms survive unit rescaling.
- Solid arrows were not closed despite the old test name: the base lacked a cap
  and the shaft/head transition had missing triangles. Corrected arrows have 34
  triangles and the same 19 vertices, including a base cap and complete annulus.
  Independent edge counts require two opposite uses of every edge; signed volume
  and Euler characteristic verify outward closed geometry for both scale signs.
  Tensor sphere indices also faced inward; they now face outward. Tensor radius
  scale uses absolute magnitude so negative scale preserves outward winding.
- A new eigenvalue test failed before the fix on `[[2,1],[1,2]]`: the old Jacobi
  angle returned zero rotation for equal diagonals and reported `[2,2]` instead of
  `[1,3]`. The solver now uses `atan2(2*a[p][q], a[q][q]-a[p][p])/2`, normalized by
  matrix magnitude before iteration. Residual/orthonormality tests cover known
  coupled 2D and fully 3D spectra at scales 1, `1e-200`, `1e200`. Public regressions
  verify rotated tensor axes and lambda-2 for coupled strain (1, previously 5).
  General nonsymmetric eigen/swirling numerical qualification remains C4 work.
- Glyph generation streams in point order with fixed local arrays. It removes
  per-glyph result vectors, per-cell small vectors and scoped workers. Solid-arrow
  radial directions use fixed circle constants. Only output-proportional vectors
  grow; serial allocation contracts cap calls/bytes from 16 to 256 samples. Those
  bounds are regression fixtures, not an aggregate import/render memory budget.
- `qa/perf/20261019-glyph-streaming-paired.json` records allocator-disabled release
  ABBA runs twice and separate allocation builds. Baseline `e812e685` glyph code
  exactly matches pre-change `ba23b340`; source hashes and identical probe source
  are included. At 4,096 unmasked samples, line/solid/tensor medians improve
  48.5%/72.6%/79.3%. Calls fall from 12,338/118,918/200,850 to 36/56/66; allocated
  bytes fall from 2.15/21.56/35.20 MB to 1.24/13.98/18.22 MB. Solid output includes
  the extra cap triangles. Counts, magnitudes and the old/new closure difference
  are checked outside timing. This measures host wall time and allocated bytes,
  without peak RSS, aggregate worker CPU time, energy or mobile qualification.
- Final required-Metal core checks pass: 69 transformation, 15 mesh and 91 scene
  library tests; 21 public validity/flow/geometry tests; allocation contracts and
  pressure/large-dataset integrations. Transformation all-target Clippy and the
  full core wasm closure pass. C3/C4 remain unchecked: tracing, particles, LIC,
  FTLE, analysis/statistics, time and IO consumers, general interpolation and
  transparency semantics still require work. No whole-platform completion claim.

## Tracing, temporal support and compact cell lookup checkpoint

- Vector/adaptive/tensor traces, particles, streaks, LIC and FTLE now share
  strict four-node stencil support. Missing finite placeholders and masked NaNs
  cannot produce velocities/directions. Wholly missing fields yield empty paths
  or missing diagnostics; zero velocity remains a valid stationary sample.
  Unsupported initial particles/releases are omitted. Existing particle trails
  freeze when support is lost; streak particles are removed. Computed nonfinite
  FTLE/LIC/age values become masked NaNs rather than valid zero/infinity.
- Pathline integration previously started at the destination frame time. With
  `v(t)=t`, a half-second step from x=1 produced x=1.375 instead of x=1.125.
  Streak releases also advanced immediately and placed a new release at x=1.125
  instead of its seed x=1. Both bugs are reproduced in the baseline probe and
  corrected by analytic integration/release tests. RK2 now checks midpoint and
  endpoint support; frame timestamps and position updates use FMA where useful.
  Streak age carries seconds; unrepresentable ages are missing independently of
  otherwise valid particle geometry.
- `TimeSeries::new` validates every dataset, timestamps, point counts and shared
  topology. New `TimeSeries::validate` supports checking directly mutated public
  state. Time samplers validate once before retaining immutable borrows. Extreme
  finite time spans use a half-scaled blend fraction, and temporal velocities use
  a bounded convex blend. Nonfinite options/seeds and unrepresentable frame times
  error before dispatch. Direct `TimeSeries::sample`/`locate` APIs and field schema
  interpolation still need separate work; this is not complete time-series QA.
- Normalized tracing supports huge and subnormal finite vectors without overflow
  or a unit-dependent `1e-300` speed cutoff. Stencil weights must be finite, are
  clamped within the containment tolerance and normalized before interpolation.
  Convex fallback preserves finite extreme averages and subnormal constants.
  Stagnant representable positions stop normalized paths instead of emitting
  repeated zero-length segments. Absolute determinant cutoffs and general extreme
  coordinate precision remain C4 work.
- A compact cell BVH replaces retained tetrahedron arrays in both vector and
  tensor locators. It stores original cell indices plus tree nodes, enumerating
  a cell's fixed tetrahedral decomposition only when queried. Counts are computed
  before allocating index arrays; a fixed query stack needs no allocation. Masks
  are borrowed and wholly missing input retains no index. A 4,096-tetrahedron
  fixture proves the retained index is smaller than the old tetrahedron array;
  a separate cold-construction allocation budget covers total allocated bytes.
  Small models have fixed tree overhead; aggregate resource budgets remain C5.
- Ordered linear-reference tests cover masked/unmasked regions and many boundary
  queries. Overlapping cells preserve the original lowest-cell-index priority
  after spatial partitioning. Bounds expand conservatively for simultaneous
  negative barycentric tolerances; adversarial near-boundary weights are checked.
  Highly overlapping cells can still require linear traversal. This index does
  not yet replace the separate probe/volume acceleration implementations.
- `Cells::lines` and packed particle-cell buffers eliminate per-segment vectors.
  Shared checked point ranges protect trace/glyph output indices before append.
  Streak assembly keeps chronological insertion order and removes dead particles,
  avoiding a temporary per-seed list and redundant sorting. Output/frame retention
  and large trail copying still require the P5/C5 streaming/budget work.
- `qa/perf/20261019-tracing-paired.json` records fresh allocator-disabled release
  ABBA runs twice versus `b3441be0`, with separate allocation builds, identical
  probe source, raw records, source hashes and baseline/candidate timing-bug
  observations. Cold validation and BVH construction are included. On the 8³
  fixture, streamlines/particles/LIC/FTLE median wall times improve 67.2%/45.1%/
  89.7%/86.7%; allocated bytes fall from 124.3/175.0/57.8/107.1 KB to
  54.3/62.6/13.3/19.4 KB. The 4³ FTLE fixture is a measured 2.8% regression;
  no universal speedup is claimed. Unsupported FTLE boundary values differ
  intentionally: baseline zero/valid, candidate masked NaNs. These are bounded
  host diagnostics, not peak RSS, energy or physical-device acceptance results.
- Required-Metal full core verification passes: 72 transformation, 15 mesh and
  91 scene library tests; 27 public validity/flow/geometry/tracing tests; allocation
  and pressure/large-dataset integrations. Mesh/transform all-target Clippy and
  the full core wasm closure pass. Two positive rustdoc examples and the borrow
  compile-fail example pass. C3/C4/P5 stay unchecked: time-field sampling,
  statistics/analysis, decimation/other geometry consumers, IO boundaries, general
  interpolation, transparency, memory budgets and device qualification remain.

## Immutable time sampling and playback checkpoint

- `TimeSeries::sample` now matches point/cell fields by name for scalar, vector,
  tensor and complex data. Shared fields require compatible shapes/units and
  intersect validity masks. Missing computed entries are masked NaNs; unchanged
  arrays may share storage with unspecified raw values at masked entries.
  One-sided fields, labels, order and dataset ID follow the nearest step (ties
  choose the later step). Endpoints/clamped queries share the selected source.
  Same-name incompatible fields error instead of silently selecting a shape.
- `TimeAxis` validates timestamp state once and supports allocation-free O(log S)
  lookups. `TimeSeriesSampler` borrows immutable state; `PreparedTimeSeries` owns
  it through an Arc, with compact per-adjacent-step field plans. Dataset checks,
  name matching, payload equality, mask-subset checks and ordinary-range numeric
  certification run during preparation. Equal independent arrays share the
  earlier payload throughout an interval. Subset masks share the stricter Arc;
  incomparable masks are intersected per output, avoiding retained N-entry masks
  for every interval. Prepared clones allocate nothing and release payloads when
  the last owner/sample drops. Three compile-fail rustdoc cases protect immutable
  validated state; three positive examples also pass.
- Ordinary interpolation anchors to the nearer endpoint and branches once per
  field, permitting vectorization. An independent endpoint regression failed on
  the first fast kernel: interpolation from `1e150` to zero at `1-EPSILON/2`
  produced `1.8170968e134` instead of `1.1102230e134`. Endpoint anchoring fixes
  that error. Extreme/subnormal inputs use bounded scaled convex interpolation;
  tests cover overflowing finite time spans and exact subnormal values. The first
  fully defensive per-component kernel also caused about a 2x dense retained
  sampling regression; preparation-time certification removes that hot-path cost.
- `Player` uses prepared state and the same complete sampling semantics. Finite
  controls, signed elapsed time/speed, reverse/multiple wraps, non-looping stops,
  single-step playback, huge finite ranges and error-state atomicity have public
  regressions. `seek`/`advance` report changes so hosts can suppress redundant
  redraws. Particle/FTLE sampling retains a validated axis and rejects changing
  velocity units. Host scheduling adoption and actual power measurement remain
  open. Movie generation still collects all frames; streaming/budgets remain P5/C5.
- Intentional API changes: `TimeSeries::locate`, `Player::new`, `Player::current`,
  `Player::seek` and `Player::advance` are fallible; seek/advance return a change
  flag. All repository callers/examples are updated and compiled. A raw series'
  one-off sample validates the axis and selected datasets; prepared sampling
  validates all input states/adjacent schemas once. Keep the prepared sampler for
  repeated work; public mutable raw state cannot safely skip validation scans.
- `qa/perf/20261019-time-sampling-paired.json` contains identical release probe
  source, source hashes, raw allocator-disabled ABBA runs twice, separate allocator
  builds, and explicit preparation measurements versus `08e8dd02`. At 48³:

  | Case | Previous API median | Retained median | Previous allocated bytes | Retained allocated bytes |
  |---|---:|---:|---:|---:|
  | Fixed geometry, one changing field | 813.6 µs | 39.9 µs | 46,477,008 | 1,770,256 |
  | Deforming geometry, one changing field | 825.1 µs | 172.8 µs | 46,477,008 | 7,078,688 |
  | Every field changes | 992.3 µs | 990.4 µs | 46,477,008 | 46,477,008 |

  Fixed/deforming player medians improve 84.1%/37.9% on a separate equivalent
  scalar/vector fixture; dense player time regresses 2.3%. Preparation takes
  1.70/1.94/3.89 ms respectively and allocates 760 bytes beyond the moved input
  metadata for this two-step fixture. The one-off validated sampling API regresses
  2.10x/2.55x/4.89x: correctness and preparation scans are included there, while the
  baseline omitted validation. This is an explicit cold-cost tradeoff, not a
  universal speedup. Repeated callers use retained state. Measurements are bounded
  host wall time and cumulative allocations; no CPU-time, peak RSS, energy, mobile
  or whole-frame acceptance claim follows from them.
- Required-Metal native checks pass: 72 transformation, 15 mesh and 91 scene
  library tests, 40 public validity/flow/geometry/tracing/time tests, allocation
  contracts and large-dataset/pressure integrations. Allocation contracts cover
  10,000 axis lookups and constant-size shared results, plus bounds for one
  changing field and intersected masks at 64/4,096 points. Core wasm checking,
  strict core library Clippy, mesh/transform all-target Clippy, playback-test
  Clippy with `--no-deps`, and all scene examples with `gpui-gpu,png,mp4` pass.
  Broader playback-test Clippy without `--no-deps` hits existing UI-kit lints in
  input/radio/Markdown code; those unrelated files are unchanged. C3/C4/P5 remain
  unchecked: statistics/analysis, decimation/other geometry consumers, IO,
  general transparency/interpolation, aggregate budgets and device qualification
  still require implementation and verification.

## Scalar analysis, bounded expressions and adaptive probing checkpoint

- Statistics and histograms now exclude missing point/cell values, including
  finite placeholders. The first public regression failed before the fix:
  `[1,99,3,NaN]` with support `[true,false,true,false]` reported a count of three
  instead of two. Invalid raw datasets now error rather than silently treating
  malformed field/mask state as a population. Wholly missing populations error.
- Statistics use one supported-value buffer and linear-time median selection,
  replacing a full sort and repeated vector growth. Even counts use the arithmetic
  midpoint of the two middle values. Centered/scaled Neumaier sums preserve finite
  means/deviations over `[-MAX,MAX]`, cancellation with small residuals, subnormal
  data and small spreads around a large mean. In particular, `[1e16,1e16+2]` has
  population deviation 1 even though its exact mean is not representable. Missing
  populations allocate no value-sized scratch; inputs remain unchanged.
- Histograms use two streaming passes and O(bins) storage, without computing a
  median or retaining field-sized scratch. Bin assignment divides before scaling
  by count, avoiding underflowed widths; overflowing finite ranges use half-scaled
  fractions, and convex bin centers remain finite. Maximum values enter the final
  bin; constants enter the first bin. Bin counts outside `1..=1_048_576` error
  before allocation, bounding count/result arrays to at most 24 MiB together.
- Calculator expressions bind fields/functions once into bounded postfix
  instructions, with one reusable evaluation stack. Field search, recursive AST
  evaluation and argument-vector allocation no longer run per point. Missing
  dependencies and nonfinite intermediate results stay missing through min/max,
  multiplication by zero and subsequent operators. Derived entries become masked
  NaNs; an existing output field is replaced. Empty data still checks field names,
  types and arity. Scientific notation, Unicode identifiers and conventional
  right-associative powers/unary precedence are covered. Source, nesting and
  instruction limits (64 KiB/128/4096) bound parser storage/recursion independently;
  a 4095-instruction flat expression succeeds and the next oversized one errors.
- Scalar probing shares tracing's strict stencil support and finite convex
  interpolation. Small batches walk original cells without an index; larger
  batches use the compact BVH. Both preserve original-cell priority, including
  overlapping cells with differing masks. Linear lookup is checked against an
  independent ordered reference across boundaries. Surface nearest-point ordering
  now survives squared-distance overflow/underflow instead of selecting a wrong
  node; it still returns missing when the nearest node is unsupported.
- Resampling streams query positions and scalar results, removes the copied point
  array and intermediate Option arrays, and validates source state once across
  fields. It preserves labels/units. Unsupported samples retain the requested
  finite/NaN fill with a false support bit; infinite fill errors. These semantics
  intentionally correct the former valid-placeholder output. General geometric
  determinant cutoffs and volume acceleration unification remain C4/P5 work.
- `qa/perf/20261019-analysis-paired.json` records identical release probes,
  baseline `ffcf4d52`, source/binary hashes, allocator-disabled ABBA runs twice and
  separate allocation builds, with correctness checks outside timing. Cold
  validation, expression compilation and index construction are included:

  | Case | Baseline median | Current median | Baseline allocated bytes | Current allocated bytes |
  |---|---:|---:|---:|---:|
  | Calculator, 48³ points | 14.180 ms | 4.129 ms | 23,168,790 | 1,771,502 |
  | Statistics, 48³ points | 1.177 ms | 0.929 ms | 1,933,312 | 884,736 |
  | Histogram, 48³ points / 64 bins | 1.229 ms | 0.596 ms | 1,934,848 | 1,536 |
  | Probe, 24³ source / 64 queries | 9.996 ms | 4.880 ms | 2,098,176 | 1,024 |
  | Probe, 24³ source / 256 queries | 39.530 ms | 8.885 ms | 2,101,248 | 166,904 |
  | Resample, 8³ source to 9³ / two fields | 6.346 ms | 0.770 ms | 303,624 | 141,024 |

  Calculator calls fall from 663,606 to 27; histogram calls from 19 to 2. The first
  prototype built an index for every query and took about 9.2 ms for a cold single
  query versus 0.35 ms previously. An eight-query cutoff still regressed 32-query
  time 69%; preserved historical ABBA records show why it was rejected. A cutoff
  of 64 removes that measured regression. Single near/far query medians are now
  48/114 µs versus 346/474 µs, with only the 16-byte output allocation. The cutoff
  is a measured host heuristic, not a universal/mobile crossover guarantee. These
  are wall-time/cumulative-allocation diagnostics, without aggregate CPU-time,
  peak RSS, thermal, power or whole-frame/device acceptance claims.
- Required-Metal native core verification passes: 72 transformation, 15 mesh and
  91 scene library tests, 50 public validity/flow/geometry/tracing/time/analysis
  regressions, allocation contracts and large-dataset/pressure integrations.
  Contracts cover fixed calculator allocation counts, O(bins) histogram storage,
  supported-value statistics scratch, missing-population errors, bounded BVH
  scratch and index-free empty/single queries. Strict mesh/transform all-target
  Clippy and the full core wasm closure pass. C3/C4/C5/P5 remain unchecked:
  decimation/append and other geometry consumers, IO, general material semantics,
  aggregate budgets, streaming and physical-device qualification still need work.

## Packed geometry and coherent append checkpoint

- Decimation now averages scalar/vector/tensor/complex point fields over supported
  members only, emits masked NaNs for clusters with no field support, and subsets
  every cell-field shape with its mask. Vertex cells survive; cells with repeated
  remapped nodes are dropped. Field metadata is preserved. Source datasets/options
  are checked before processing. The former i64-saturating grid key merged distinct
  large coordinates; keys now preserve distinct representable coordinates when
  quotients exceed exact integer precision or overflow. Ordinary grid cubes retain
  zero-origin floor semantics, including negative coordinates and signed zero.
- Ordinary cluster means use compensated sums. Overflowing clusters use a bounded
  exact integer accumulator, retaining tiny residuals after cancellation instead
  of returning infinity or losing them through global scaling. Its two 34-limb
  signs cover finite f64 values summed up to a 64-bit usize count, and rounding
  uses one u128 division. Accumulators live per component on the stack; a lazily
  built O(points+clusters) membership table is reused across fields. No large
  accumulator array is retained per cluster. Independent Python Fraction checks
  match all 720 generated finite-bit-pattern cases, including constant extremes,
  mixed signs, cancellation and subnormal ties. The checked-in oracle fixture and
  portable `qa/numeric/generate_exact_mean_fixture.py` reproduce those expectations.
- Append validates both datasets, combined sizes/indices and all shared field
  shapes/units before copying geometry. It concatenates source masks; absent sides
  receive masked NaNs instead of valid zeros. Empty-side schemas and payload/mask
  sharing are covered. Matching uses borrowed names, an aligned-schema fast path
  and indexed lookup for large reordered schemas. Geometry is concatenated into
  packed arrays without per-cell vectors. Internal PVD callers use the same API.
- Threshold and sphere selection share packed cell output and compute first-use
  point order once, replacing per-cell vectors and per-field sorting. Identity
  selections share source buffers; empty selections skip a source-sized remap.
  Shape/mask subsetting is common with decimation. Threshold rejects NaN/reversed
  bounds while accepting ordered infinite bounds. Sphere selection directly tests
  robust distances, preserving user fields named `__sphere_distance`; it no longer
  stages a colliding implementation field. Radius/center checks and inclusive
  boundary behavior cover zero, subnormal and extreme finite ranges.
- All ten new public filter test functions fail on unchanged baseline production
  source `2ddc33de`, including invalid output field lengths, lost masks, saturated
  clustering, infinite means, inconsistent units and the sphere-name collision.
  All pass on the candidate. Existing glyph/warp geometry regressions are retained
  unchanged; a test-file name collision was caught and corrected before the full
  suite, with new cases placed in `geometry_filter_validity.rs`.
- `qa/perf/20261019-geometry-packed-paired.json` contains identical release probe
  source, source/binary hashes, allocator-disabled ABBA runs twice and separate
  allocation builds. Cold validation/planning is included. At 24³ points:

  | Scalar/vector case | Baseline median | Current median | Baseline calls / bytes | Current calls / bytes |
  |---|---:|---:|---:|---:|
  | Merging decimation | 969 µs | 515 µs | 36,570 / 2,272,855 | 47 / 750,734 |
  | Identity decimation | 1,321 µs | 738 µs | 36,582 / 7,778,739 | 20 / 1,411,323 |
  | Append both inputs | 550 µs | 249 µs | 24,400 / 9,331,758 | 23 / 6,650,046 |
  | Append empty side | 273 µs | 66 µs | 12,230 / 4,666,535 | 7 / 496 |
  | Select all cells | 732 µs | 252 µs | 12,283 / 5,294,830 | 30 / 2,897,646 |
  | Sphere includes all | 1,024 µs | 278 µs | 12,308 / 5,979,571 | 34 / 2,911,531 |

  All-shape append/threshold/sphere cases also improve (38.8%/70.9%/74.8%).
  Tensor/complex decimation is covered by correctness/allocation checks rather
  than an invalid baseline performance comparison. Timed data has no missing
  values/overflow; exact fallback cost is not qualified by those timing results.
  These are bounded wall-time/cumulative-allocation diagnostics, without aggregate
  CPU time, peak RSS, energy, thermal or physical-device acceptance claims.
- Required-Metal full core verification passes: 75 transformation, 15 mesh and
  91 scene library tests; 60 public validity/flow/geometry/tracing/time/analysis
  regressions; allocation and large-dataset/pressure integrations. Allocation
  contracts cover all four field shapes, packed geometry, empty selection,
  compatibility errors before output copying, and many overflowing clusters
  without per-cluster heap accumulators. Strict mesh/transform all-target Clippy
  and the full wasm core closure pass. The rational fixture regenerates exactly.
- C3/C4/C5/P5 remain open. Smoothing/affine correctness, remaining field/IO
  consumers, general rendering semantics, aggregate memory budgets, streaming
  and device qualification still need work. The exact helper currently serves
  clustering. `qa/numeric/20261019-open-mean-reductions.json` records further C4
  defects for the next reduction pass: statistics on `[MAX,-MAX,1e-300]` returns
  zero instead of `1e-300/3`, and point-to-cell conversion on `[1e16,3,-1e16]`
  returns `4/3` instead of 1. Passing mask tests does not qualify numeric precision
  across all consumers; those findings are explicitly unresolved.

## Cancellation precision and CPU cost checkpoint

- Statistics now retains the mean of `[MAX,-MAX,1e-300]` as `1e-300/3`;
  point-to-cell conversion retains `[1e16,3,-1e16]` as 1. Mixed-sign means use
  exact accumulation and one correctly rounded division. Centered/scaled
  compensated variance keeps its independent mean so small spreads around large
  values survive rounding of the reported raw mean. Ordinary same-sign statistics
  and conversion retain their floating paths; this is not a promise of correctly
  rounded means for all same-sign populations.
- Clustering also handles cancellation whose compensated sum stays finite but
  loses a residual: `[1e308,1,1e-300,-1e308,-1]` retains `1e-300/5`, for every
  field shape. Sign tracking is per component/cluster. Geometry uses the
  zero-origin cube invariant to avoid unnecessary mixed-sign accumulation; the
  negative nonzero quotient underflow case now maps to floor -1 instead of zero.
  This prevents merging opposite-sign tiny coordinates at extreme cell widths.
- Exact accumulation uses bounded adaptive storage: certified exact floating
  prefixes, checked signed-magnitude u128 sums, then the existing 34-limb-per-sign
  fallback. Promotion retains the original contributor count without replaying
  that helper's input or allocating scratch. Small cell reductions construct
  wide storage only on failed checked promotion. Dense point-to-cell fields can
  share a conservative floating or signed-i64 range certificate; a certified
  integer cell traverses its contributors once. Power-of-two counts avoid integer
  division where exact scaling is safe. Sparse selections inspect referenced
  values rather than allocating field-sized exact accumulators.
- The Python Fraction oracle now contains 1,125 finite-bit-pattern cases. It
  covers ordinary/full precision ranges, quantized values, cancellation,
  overflow, subnormal ties and either side of the u128 range boundary. It
  regenerates byte-for-byte. Wide, adaptive, small and eligible shared-integer
  paths match its expected bits. Public statistics and conversion tests exercise
  mixed-sign cases in both orders and fixed/variable cell types. Additional
  public tests cover all four field shapes, masks, metadata and sparse/dense
  support. All five new test functions fail on unchanged `7dd24e2c` production
  source and pass on the candidate.
- `qa/perf/20261019-reduction-precision-paired.json` records 64 cases with
  identical release source/locks, source/binary hashes, allocator-disabled ABBA
  twice and separate allocation builds. It measures both wall time and actual
  aggregate process CPU using checked `CLOCK_PROCESS_CPUTIME_ID` calls in the
  benchmark only. Input construction, output checks/destruction are excluded;
  raw validation, numerical certificates and fallback scratch are included.
  Timed output checks use a tolerance on conditioned bounded data; the separate
  independent oracle supplies precision acceptance. At 24³ points:

  | Case | Baseline CPU | Current CPU | Baseline allocated bytes | Current allocated bytes |
  |---|---:|---:|---:|---:|
  | Positive scalar conversion, raw | 94.5 µs | 100.5 µs | 195,288 | 195,288 |
  | Mixed quantized scalar conversion, validated | 41 µs | 59.5 µs | 195,288 | 195,288 |
  | Mixed quantized vector conversion, validated | 59 µs | 103.5 µs | 584,632 | 584,632 |
  | Mixed full-precision vector conversion, validated | 59.5 µs | 329 µs | 584,632 | 584,632 |
  | Mixed quantized clustering | 501.5 µs | 540 µs | 665,326 | 610,030 |
  | Mixed full-precision clustering | 489.5 µs | 716.5 µs | 665,326 | 796,662 |

  These are material CPU regressions, not performance acceptance. The first
  streaming prototype measured about 25× baseline CPU for the full-precision
  vector case; the shared integer certificate and single cell traversal reduce
  that to 5.5× in the paired run. Exploratory superseded records are retained and
  labelled without unavailable intermediate source hashes. At 48³ points,
  mixed full-precision statistics uses 765 µs CPU versus 512.5 µs and retains the
  same single 884,736-byte scratch allocation. Cancellation-sensitive clustering
  can build the shared membership table, increasing cumulative allocated bytes;
  certified quantized fields omit correction/sign buffers. CPU/memory work
  remains open: revise certificate reuse and component traversal, then qualify
  whole-frame/device workloads before adoption claims. CPU time does not measure
  energy; these shared-host diagnostics qualify neither mobile peak RSS nor
  thermal/power behavior.
- Required-Metal native core verification passes: 75 transformation, 15 mesh and
  91 scene library tests, 65 public validity/flow/geometry/tracing/time/analysis
  regressions, allocation contracts and large-dataset/pressure integrations.
  Strict mesh/transformation all-target Clippy and the core wasm closure pass.
  Allocation contracts cover sparse exact conversion without source-sized
  scratch, one statistics value buffer and many wide cancellation clusters
  without per-cluster heap accumulators.
- `qa/numeric/20261019-reduction-precision-status.json` preserves the two resolved
  prior diagnostics and a newly executed remaining defect: three equal-weight
  vertex cells with `[1e16,3,-1e16]` still convert to `4/3` instead of 1 at their
  shared point. Weighted cell-to-point accumulation and interpolation remain C4
  work. The earlier open-diagnostic artifact is historical evidence, not the
  current status. C3/C4/C5 and all previously open platform, rendering, budget,
  streaming, Taffy, CUDA/MLX and physical-device acceptance work stay unchecked.

## Machine-integer reduction optimization checkpoint

- Point-to-cell conversion chooses its arithmetic strategy once per collection.
  Scalar output consumes the single-component row buffer without another copy.
  Empty and wholly unsupported selections avoid unnecessary certification work.
  Direct division is selected outside the loop when certified vector sums have
  arbitrary contributor counts; complete power-of-two cells retain exact
  reciprocal multiplication. A regression checks masked and mixed-size cells
  against the correctly rounded bits of -5/3, which differs from multiplying
  -5 by the rounded reciprocal of 3.
- The signed-i64 certificate now carries an optional exact inverse binary unit.
  Where that inverse is a normal finite power of two, multiplying each supported
  value by it produces an exact integer below 2^63. Safe float-to-integer
  conversion replaces per-value bit decoding. Tiny-unit cases retain general
  decoding, including signed zeros. Division/scaling decisions are shared across
  the components of a row. General u128 rational rounding stays out of that hot
  path. No unsafe code, dependencies, approximation mode or relaxed rounding
  guarantee was introduced.
- Clustering uses the same certificate and accumulates directly into one
  machine-integer row per cluster. Consuming conversion to float rows can reuse
  that allocation; this path needs neither correction/sign arrays nor the
  membership table. Unsupported values are excluded before integer decoding,
  and groups with no supported values retain masked NaNs. Wide/adaptive fallback
  remains for values or contributor counts outside the certificate.
- The independent oracle expands to 1,305 cases, preserving the earlier 1,125
  rows byte-for-byte. New cases cover normal inverse-unit boundaries, signed
  zeros, tiny/general decoding, extreme exponents and either side of the signed
  machine-word prefix bound. It regenerates exactly. Private checks also compare
  every eligible floating certificate with the rational result. Public tests
  check all four component shapes across fixed/variable small cells and
  clustering, plus masks/empty support in the new integer path. An allocation
  regression on identical 4,096-point geometry fails on unchanged `fd9eaefc`
  production source and passes here: ordinary machine-integer cancellation uses
  no more calls/bytes than overflowing means with the same output topology.
- `qa/perf/20261019-reduction-machine-integer-paired.json` records the final
  three-way comparison: correct baseline `fd9eaefc`, candidate and historical
  inexact `7dd24e2c`. Identical release source/locks, production/test/binary hashes,
  64 cases, allocator-disabled ABCCBA twice and separate allocation builds are
  preserved. No agent-owned builds/checks ran during measurement. CPU is actual
  aggregate process CPU; fixture construction, output checks and destruction
  remain outside timing. At 24³ points:

  | Case | Correct baseline CPU | Current CPU | Historical inexact CPU | Current calls / cumulative bytes |
  |---|---:|---:|---:|---:|
  | Positive scalar conversion, validated | 51 µs | 44 µs | 41 µs | 5 / 195,288 |
  | Positive vector conversion, validated | 73.5 µs | 68 µs | 65 µs | 5 / 584,632 |
  | Mixed quantized vector conversion, validated | 106.5 µs | 105 µs | 62.5 µs | 5 / 584,632 |
  | Mixed full-precision scalar conversion, validated | 120 µs | 68 µs | 42.5 µs | 5 / 195,288 |
  | Mixed full-precision vector conversion, validated | 336 µs | 117 µs | 62 µs | 5 / 584,632 |
  | Mixed full-precision clustering | 728.5 µs | 527.5 µs | 524 µs | 40 / 610,030 |

  The full-precision vector case uses 65% less CPU than the correct baseline;
  clustering uses 28% less. Clustering calls fall from 46 to 40 and cumulative
  bytes from 796,662 to 610,030. Remaining regressions are retained: separated-sign
  vector conversion is 106 versus 101.5 µs CPU, separated-sign clustering 578
  versus 546.5 µs, and the 8³ positive clustering case 24.5 versus 21.5 µs. This
  shared-host diagnostic does not establish controlled-device thresholds. It
  also does not qualify non-power-of-two cell CPU costs. Further ordinary-loop
  specialization and certificate reuse remain performance work; the current
  full-precision vector cost is still 1.89× the historical inexact path, and
  quantized vector cost 1.68×. A batched floating-certificate prototype showed
  no clear benefit and was removed. CPU time is not energy, and cumulative
  allocation is not peak/live/RSS or mobile qualification.
- Required-Metal native verification passes: 75 transformation, 15 mesh and 91
  scene library tests, 68 public regressions, allocation contracts and
  large-dataset/pressure integrations. Strict mesh/transformation all-target
  Clippy and the core wasm closure pass. `git diff --check` passes.
  `qa/numeric/20261019-reduction-machine-integer-status.json` re-executes the
  current public probes: statistics and point-to-cell residuals remain correct;
  weighted cell-to-point still returns 4/3 instead of 1. C3/C4/C5 and every
  previously open platform, rendering, memory-budget, streaming, scheduling,
  Taffy, CUDA/MLX and physical-device requirement remain open.

## Exact representable cell-weight reduction checkpoint

- Cell-to-point conversion no longer loses the residual of `[1e16,3,-1e16]`
  in three equal-weight cells: it returns 1 instead of 4/3. Unequal-weight
  cancellation also retains residuals, including `[MAX,1e-300,-MAX]` with
  weights `[1,2,1]`, whose mean is `1e-300/2`. Means are correctly rounded with
  respect to the computed positive finite f64 geometric measures. The geometric
  measure formulas are unchanged: degenerate/nonrepresentable measures still
  do not contribute. Improving those formulas and their representable range is
  separate C4 work; this checkpoint does not qualify exact physical geometry.
- `cell_mean.rs` owns selection and accumulation. Common geometric weights stay
  inline; excluded cells need only a lazily allocated bitset. Unequal weights
  are retained once in a dense array. A global certificate permits floating
  products/sums only when every product, numerator prefix and denominator prefix
  is exact and representable. Complete power-of-two divisors use an exact inverse;
  other vector divisors use direct division, preserving cases such as 5/3.
- If floating accumulation cannot be certified, common weights cancel before
  multiplication. Signed 64/128-bit field certificates bound every prefix.
  Unequal weights can instead use certified signed 64/128-bit products and
  unsigned weight sums, encoding each source cell once before scattering.
  Native integer division handles suitable ratios without a u128 divide. The
  numerator/denominator are rounded only after exact accumulation; no relaxed
  precision mode or unsafe production code was added.
- Wider ranges use compact point/cell incidence, with u32 cell IDs where they
  fit and usize otherwise. Count storage becomes offsets in place; no per-point
  vectors or per-point heap accumulators are created. The adaptive weighted mean
  starts with u128 magnitudes and promotes only on failed checked accumulation.
  Its 67-limb positive/negative/denominator storage covers finite f64 products
  in units of `2^-2148` plus a 64-bit contributor count. Remainder division handles
  ties, subnormals and saturated integer denominators without overflowing shifts.
  Accumulators are used sequentially on the stack. A single point with common
  weights needs no incident-cell index.
- Highest supported cell dimension is selected independently at each point;
  lower dimensions are excluded before exact scatter/indexed accumulation.
  Scalar/vector/tensor/complex masks and metadata survive. Unsupported points
  retain masked NaNs. Repeated polygon node incidences retain their multiplicity;
  field association and output-name replacement semantics are unchanged.
- `qa/numeric/generate_weighted_mean_fixture.py` regenerates all 590 independent
  Python Fraction expectations byte-for-byte. Private checks cover both input
  orders, forced wide accumulation and every eligible floating/64/128-bit
  certificate. Public star meshes use analytic one-axis line lengths and compare
  every central mean bit-for-bit with that oracle. Fixed fixture counts and pair
  shape checks prevent empty/truncated fixtures from silently passing. Additional
  public tests cover all four shapes, masks/units, dimension priority in both
  orders and vector rounding with repeated polygon incidences. Three of the four
  public test functions fail on unchanged `19d2c942` production; the existing
  dimension-priority behavior already passes there.
- Native allocation contracts execute with the coverage skip guard absent.
  With 63 and 6,000 incident vertex cells, all four shapes retain the exact mean
  within 16 calls / 8,192 cumulative bytes, including a range needing wide
  accumulation. With 32/256 separate three-line groups, wide unequal-weight
  residuals remain correct under output/incidence-proportional budgets; a heap
  accumulator per point would exceed those budgets. Ordinary equal-weight grid
  conversion retains its existing allocation budget.
- `qa/perf/20261019-weighted-conversion-paired.json` preserves identical release
  source/locks, source/test/binary hashes, 78 cases, allocator-disabled ABBA twice
  and separate allocation builds. Cold geometric measures, certificates and all
  scratch are timed; input construction, output checks and destruction are not.
  Validated views exclude source validation only. Baseline `19d2c942` is inexact;
  conditioned timing checks use tolerance 1e-12, while the independent oracle
  supplies precision acceptance. At 24³ points:

  | Validated case | Inexact baseline CPU | Current CPU | Baseline allocated bytes | Current allocated bytes |
  |---|---:|---:|---:|---:|
  | Uniform positive vector | 444 µs | 442 µs | 774,760 | 774,760 |
  | Uniform mixed quantized vector | 443.5 µs | 448.5 µs | 774,760 | 774,760 |
  | Uniform full-precision vector | 437.5 µs | 480.5 µs | 774,760 | 774,768 |
  | Stretched mixed quantized vector | 442.5 µs | 469 µs | 774,760 | 872,096 |
  | Stretched full-precision vector | 449.5 µs | 673.5 µs | 774,760 | 982,696 |

  These correctness costs remain performance work. Unequal full-precision
  vector conversion uses about 50% more CPU and 27% more cumulative allocation.
  A 4,096-cell / one-point full-precision vector fan uses 50 versus 18.5 µs CPU
  and 800 versus 672 bytes. Common quantized grids remain close to the former
  CPU cost; a certified floating path removed the first prototype's roughly 75%
  unequal-weight quantized-vector regression. Full-precision weights still add
  certificates/count preparation, and arbitrary source geometries/wide fallback
  CPU are not qualified by these timings. Next CPU work should retain immutable
  geometric weights and certificates across field conversions/revisions rather
  than rebuilding them per call. This requires explicit ownership/invalidation
  evidence. Shared-host CPU time is not energy; cumulative allocation is not
  peak/live/RSS, and these measurements do not qualify mobile/thermal behavior.
- Required-Metal full core verification passes: 76 transformation, 15 mesh and
  91 scene library tests, 72 public regressions, allocation contracts and
  large-dataset/pressure integrations. Strict mesh/transformation all-target
  Clippy, changed-file formatting and the core wasm closure pass.
  `qa/numeric/20261019-weighted-mean-status.json` re-executes the three earlier
  public diagnostics: statistics, point-to-cell and weighted cell-to-point now
  all match their expected values. Historical open-diagnostic artifacts remain
  unchanged. C3/C4/C5 and all previously open platform, rendering, budget,
  streaming, scheduling, Taffy, CUDA/MLX and physical-device requirements remain
  open; this checkpoint is not full performance or audit acceptance.

## Retained cell-to-point preparation checkpoint

- `CellToPointPlan<'a>` borrows one immutable validated dataset and prepares
  geometric weights once. Positions, topology, cell values and masks cannot
  change through that borrowed dataset while the plan is live. A compile-fail
  doctest checks this; mutation of independent dataset clones still uses normal
  copy-on-write ownership. Rebuild the plan after changing the source. No global
  cache, content fingerprint, revision collision or source-buffer copy is used.
  Per-field masks overlay the geometric weights without changing per-point
  dimension priority or correctly rounded weighted means.
- `convert_field` starts from the original snapshot; `convert_fields` preserves
  multiple outputs with one dataset metadata clone. Every source cell name is
  resolved before conversion/cache changes, and repeated output names follow
  request order. Scalar flattening reuses the computed buffer. The raw and
  validated legacy entry points remain available. `CellToPointLimits` defaults
  to an eight-MiB simultaneous geometry payload limit and no output cache.
  Bitmap-to-dense promotion is checked before allocating both payloads. The new
  `TransformError::ResourceLimit { resource, required, limit }` variant exposes
  rejection details; downstream exhaustive matches must handle that addition.
- Callers can explicitly enable a byte-limited, single latest-source-field
  cache. Numeric and validity buffers are shared with outputs, including aliases;
  changing returned buffers through copy-on-write cannot corrupt the entry.
  A miss releases the old cache reference before computing. Oversized results
  are returned without retention. `clear_output_cache` releases the plan's
  reference; returned datasets can still own those buffers. Weak-reference tests
  prove cache-only numeric payloads are released on an oversized miss and clear.
  Drop the plan to release its geometry. Limits exclude borrowed source,
  returned outputs, metadata, allocator overhead and temporary conversion scratch;
  they are not total-memory, peak-RSS or process-pressure guarantees.
- Both Python mesh request paths now coalesce consecutive existing
  `cell_to_point` operations automatically, using one validation/preparation and
  ordered batch conversion. There is no request-format change. Every other
  operation ends the group, so geometry changes rebuild preparation. Malformed
  field syntax runs sequentially to preserve error precedence. A geometry-budget
  rejection also falls back to the existing sequential path to preserve valid
  pipelines; that fallback retains the old temporary-memory behavior and is not
  an aggregate pipeline memory cap. The Python path keeps output caching disabled
  and releases the plan after each group; cross-request/scene retention is still
  separate work.
- The 590 independent weighted Fraction cases pass bit-for-bit through both raw
  and prepared conversion in both cell orders. Six public plan tests cover all
  field shapes, masks/metadata, mixed dimensions, aliases/replacements, budgets,
  source changes after dropping plans, copy-on-write, cache release and name
  preflight. The single serial allocation contract verifies zero-allocation
  common-weight preparation, bitmap/dense allocation peaks and cache hits at
  64/4,096 points across all four shapes. A hit stays within 16 allocations and
  8,192 cumulative bytes regardless of point count. Coverage's skip guard was
  absent. Three added bridge tests cover grouped equivalence, both request entry
  points, errors, geometry changes and forced budget fallback. The bridge's old
  24-triangle arrow assertion failed on unchanged `300a2e0c`; it now checks the
  existing closed 34-triangle arrow contract. All nine bridge tests pass.
- `qa/perf/20261019-prepared-conversion-paired.json` compares correct committed
  `300a2e0c` with the candidate using identical release probe source/locks,
  separate targets, allocator-disabled ABBA twice and separate allocation builds.
  The artifact retains source/test/binary hashes, all raw blocks, 60 baseline
  cases and 216 candidate cases. CPU uses aggregate process CPU time. Warm
  preparation and cache hits explicitly exclude initialization; cold preparation
  and batch cases include it. Output checks compare every component bit and
  mask. At 24 cubed points:

  | Case | Correct baseline CPU | Candidate CPU | Candidate cumulative bytes | Additional retained payload |
  |---|---:|---:|---:|---:|
  | Uniform positive vector, legacy cold | 427.1 µs | 434.9 µs | 774,952 | 0 |
  | Uniform positive vector, prepared | 427.1 µs | 126.2 µs | 774,952 | 0 |
  | Uniform positive vector, primed cache hit | 427.1 µs | 0.117 µs | 792 | 331,776 |
  | Unequal full-precision vector, legacy cold | 623.9 µs | 643.9 µs | 982,888 | 0 |
  | Unequal full-precision vector, prepared | 623.9 µs | 334.3 µs | 885,552 | 97,336 |
  | Unequal full-precision vector, primed cache hit | 623.9 µs | 0.141 µs | 792 | 429,112 |
  | Uniform positive scalar/vector/scalar batch | 1,401.1 µs | 663.0 µs | 1,438,584 | 0 after return |
  | Unequal full-precision scalar/vector/scalar batch | 1,763.3 µs | 1,043.5 µs | 1,867,720 | 0 after return |

  Prepared unequal-vector calls eliminate one allocation and 97,336 cumulative
  bytes by retaining those weights. The unequal batch reduces 26 calls /
  2,063,928 bytes to 18 / 1,867,720. Single cache hits need three metadata
  allocations, not new point arrays. Costs remain visible: legacy cold single
  calls have median candidate/baseline CPU ratio 1.020, worst 1.041; creating a
  fresh plan for one conversion has median ratio 1.024, worst 1.044. Preparation
  is intended for reuse, and these cold regressions remain optimization work.
  The measurements cover core batches used by the bridge, not JSON parsing,
  serialization or end-to-end Python latency. Shared-host thermal/affinity state
  was uncontrolled. CPU savings are not energy measurements, and cumulative
  allocation is not live/peak/RSS or mobile acceptance. Certificates/incidence
  still rebuild per uncached field; broad revisioned scene retention remains open.
- A clean candidate rebuild passes required-Metal core verification: 76
  transformation, 15 mesh and 91 scene library tests, 78 public regressions,
  allocation contracts and large-dataset/pressure integrations. Both public
  doctests (including compile-fail), strict mesh/transformation/Python all-target
  Clippy, changed-file formatting and the core wasm closure pass. Baseline test
  comparison exposed shared-target stale bridge artifacts; affected task-local
  packages were cleaned, candidate checks rebuilt, and release measurements use
  separate baseline/candidate targets. Frozen `audit-20261018.md` is unchanged.
  C3/C4/C5, retained scene/GPU work, all remaining platform, memory-pressure,
  scheduling, text/FFI, Taffy, CUDA/MLX and physical-device requirements remain
  open. This checkpoint does not complete the audit implementation.

## Perspective-correct surface interpolation checkpoint

- Software triangles now carry scalar and smooth normal attributes through user
  and homogeneous frustum clipping. Raster coverage/depth keeps the existing
  subpixel/top-left contract; fragment attributes use perspective-correct
  barycentric weights before transfer sampling and two-sided lighting. Winding
  reversal reorders the attributes with the projected vertices. Constant scalar
  and normal triangles retain a pre-shaded pixel loop. Variable triangles
  precompute constant colors/lighting and reciprocal clip weights once, and
  equal clip weights use affine interpolation without a per-fragment reciprocal.
  f64 attribute sums prevent finite f32 scalar averages from overflowing.
- Software and WGSL define a zero interpolated normal as ambient light, avoiding
  dependence on `normalize(0)`. Private regressions cover cancelling normals,
  reciprocal clip weights beyond f32 range, equivalent clip-w ratios from
  subnormal through very large finite floats, and finite maximum f32 scalars.
  These kernel checks do not qualify every extreme camera/geometry on a device.
- `tests/surface_interpolation.rs` independently intersects f64 pixel rays with
  world triangles and derives scalar values, area-weighted smooth normals,
  lighting and straight-alpha composition. It exercises perspective/orthographic
  projections, varying triangle depth, near/user clipping, flat color with smooth
  normals, point-field/alpha gradients, and origins `0` / `1e12 + 0.125`.
  Sixteen software fixture renders stay within 1 byte level for opaque content
  and 2 for transparency; eight Metal renders use a 3-level bound for GPU
  interpolation/LUT/unorm rounding. Each fixture requires more than 100 interior
  samples and a nonconstant color range. Coverage boundaries are deliberately
  excluded from this attribute oracle and retain the separate coverage tests.
  Unchanged `b3d438c4` fails the new oracle: one pixel returns 104 instead of the
  independent expected 78. The raw failure is preserved with the paired evidence.
- Transparent triangle storage now shares `RenderLimits.max_frame_bytes` with
  RGBA/depth buffers. Growth preflights simultaneous old/new payloads before
  fallible reservation, and stable tie order uses an in-place sort without a
  second sort buffer. Tests check exact growth-peak boundaries, rejection without
  changing capacity, an opaque frame with no remaining scratch allowance, and
  admission of the corresponding transparent triangle with a fitting allowance.
  A frame whose pixels fit can now return a resource-limit error when its
  transparent scratch does not fit; hosts can choose a larger explicit budget.
  Retained geometry, scene-order metadata and allocator overhead remain outside
  this bound. It is not a total process-memory or physical mobile-RSS guarantee.
- `qa/perf/20261019-surface-interpolation-initial.json` retains the first correct
  prototype's costs. `qa/perf/20261019-surface-interpolation-paired.json` records
  the final implementation against unchanged `b3d438c4`: identical release source
  and locks, separate targets, allocator-disabled ABBA twice, separate allocation
  builds, 21 cases, source/test/binary hashes, raw samples and actual aggregate
  process CPU. Geometry is prepared once outside timing; warm frames include
  clipping/projection, shading, frame allocation and transparent sorting/scratch.
  Baseline images have incorrect constant triangle shading, so these are
  correctness-cost measurements, not equivalent-output speedup claims.

  | Warm frame | Inexact baseline CPU | Corrected CPU | Baseline / corrected allocated bytes |
  |---|---:|---:|---:|
  | Constant-normal flat plane, 512² | 84.0 µs | 83.5 µs | 2,097,152 / 2,097,152 |
  | Scalar-gradient plane, 512² | 84.5 µs | 276.0 µs | 2,097,152 / 2,097,152 |
  | 48³ grid, flat color / smooth normals, 128² | 1,596.0 µs | 1,734.5 µs | 131,072 / 131,072 |
  | 48³ grid, scalar gradient, 128² | 1,675.0 µs | 2,081.0 µs | 131,072 / 131,072 |
  | 48³ grid, translucent gradient, 128² | 3,029.5 µs | 3,421.5 µs | 2,739,280 / 3,801,152 |
  | 24³ grid, scalar gradient, 512² | 1,503.0 µs | 2,820.0 µs | 2,097,152 / 2,097,152 |

  Constant shading retains its small pixel loop and ordinary opaque frames
  allocate only RGBA/depth. The first prototype's 48³ flat-color CPU ratio was
  1.199; specialization reduced it to 1.087 in the final paired run. Compact
  projected vertices reduce that prototype's translucent allocation from
  4,587,584 to 3,801,152 bytes. Costs remain: actual smooth lighting/transfer
  evaluation is more expensive, and corrected translucent attributes still use
  about 39% more newly allocated bytes than the old path (15 versus 18 calls).
  Compact transparent commands, retained preparation and faster exact transfer
  evaluation remain optimization work. Allocation bytes include positive
  reallocation growth; they are not full resized-buffer traffic, peak/live/RSS
  or energy. Shared-host affinity/thermal state was uncontrolled. No physical
  phone or whole-application power claim follows from these timings.
- Required-Metal scene verification passes: 93 library tests, the new two-test
  fragment integration, existing public/pressure integrations, and the actual
  native viewer interaction test at 2× DPI. Software-only verification passes
  71 library tests plus the independent interpolation test. That configuration
  exposed a pre-existing missing test-only `effective_color` helper; its cfg now
  matches the presentation tests, and an unused GPU-only fixture is gated to its
  consumers. Strict Clippy passes for production scene code and the new
  integration target with `--no-deps`; the broader all-target gate still fails
  on existing example/unit-test and UI-kit dependency lints. These are not
  suppressed or reported as a broad pass. Changed-file formatting, diff checks
  and the final GPU/GPUI wasm library closure pass.
- C4 remains open: wireframe rendering, GPU line/point parity, arbitrary
  intersecting-transparency composition, nonlinear/banded transfer parity and
  remaining f64-to-f32 geometry/scalar limits still require work. Other open
  correctness/platform, aggregate memory, scheduling, text/FFI, Taffy, CUDA/MLX,
  device and final-acceptance requirements remain unchanged. The original audit
  is frozen; this checkpoint is implementation progress, not full completion.

## Wireframe topology, clipping and resident GPU checkpoint

- Requested wireframes now draw unique black mesh edges in software and the
  shared GPU renderer. Edge width is one physical target pixel, and edge alpha
  follows the selected scalar transfer/actor opacity. User/frustum clipping
  preserves the original edge segments and interpolates scalar attributes;
  display clipping does not invent a cut outline. Software uses perspective
  interpolation for varying edge alpha. Ordinary line cells reuse the bounded
  projected-line loop and retain their existing behavior.
- Preparation canonicalizes/sorts/deduplicates edge pairs. Multi-chunk actors
  additionally assign each global edge to one chunk, preventing double blending
  on partition boundaries; single-chunk preparation avoids that ownership table.
  Masked triangles cannot produce edges. A quad split into one-triangle chunks
  still produces five total edges, under either winding; masking one triangle
  leaves three. Compaction retains only the unique edge payload. Multi-chunk
  ownership-table working memory and aggregate preparation bounds remain P1/C5
  work; this is not a bounded extraction implementation.
- The portable GPU path uses `LineList`, the resident position/normal/scalar
  buffers and one extra edge index buffer per nonempty wire chunk. It does not
  require the optional polygon-line feature, duplicate vertex buffers or CPU
  pixel readback. Edge passes blend without writing depth. Camera/material
  updates preserve the exact edge-buffer handle across 12 draws and one geometry
  preparation. Changing the wireframe preparation option still rebuilds/uploads
  the revision. GPUI now accepts wireframes on this GPU route; compatibility and
  origin metadata survive 24 camera updates with one topology scan for both
  plain and wire actors. Every new GPU attribute/index upload checks individual
  device buffer limits, u32 index counts and byte sums before buffer creation.
  Aggregate GPU residency/staging budgets remain open.
- A fixed normalized-depth offset was rejected: it allowed hidden wires through
  a foreground surface only `1e-5` world units away. Software and WGSL now move
  overlay depth forward by at most four representable f32 values and leave the
  depth buffer intact. Visibility uses that stated tolerance. The saved initial
  compiled libraries fail the final thin-occluder oracle on both CPU and GPU
  (321 / 317 differing bytes); current rendering matches the occluder-only image
  exactly in both actor orders, opaque/translucent modes and both projections.
  `qa/numeric/20261019-wireframe-depth-occlusion.json` preserves linked-library,
  binary, source and fixture hashes, the exact fixture and raw rejected results.
- Surface uniform ABI **3** retains size/alignment and clipping offsets. Slot
  `scalar_range.w` (byte 76) is now the f32 representation of the ULP count,
  filled by the standard builders. Custom hosts using `SURFACE_WGSL` must use
  that matching layout. The wire fragment entry `fs_wire` uses the same material
  bindings/vertex layout, a line-list index stream, alpha blending and no depth
  writes. The version and slot are covered by the uniform test.
- `tests/wireframe.rs` exercises both renderers against independently projected
  edge locations, single-edge alpha composition and exact opaque occlusion.
  It checks large origins, user clipping, missing-field masks, near-plane
  crossings, every near-clipped segment at three interior locations, and varying
  perspective-correct alpha after clipping. Edge coverage permits 1.5 pixels of
  backend raster-rule difference; alpha checks away from joins allow three byte
  levels for sample-position/LUT/unorm differences. The fixture does not claim
  identical endpoint/junction coverage across backends. Unchanged `47299cd5`
  fails with zero visible edge pixels; the exact old fixture/result is embedded
  in the performance evidence. Required-GPU scene verification passes 97 library
  tests and all public/pressure integrations, including both wireframe and
  existing surface interpolation tests. Software-only verification passes 73
  library tests and both render integrations; the native 2×-DPI viewer passes.
- `qa/perf/20261019-wireframe-paired.json` preserves identical release probes and
  exact dependency locks, separate targets, allocator-disabled ABBA twice,
  separate allocation builds, all raw samples and source/test/binary hashes.
  Cold preparation uses a fresh scene/cache. Warm software checks one retained
  geometry build and exact within-version pixels. GPU submission measures the
  public encode/submit API with resident buffers; GPU completion, readback and
  validation are outside timing. Wire-disabled CPU/GPU image hashes must match
  the baseline exactly. Initial fixed-offset prototype measurements remain in
  `qa/perf/20261019-wireframe-initial.json` and are not acceptance evidence for
  the corrected occlusion behavior. At 48 cubed points, 128 square pixels:

  | Case | Baseline CPU | Current CPU | Current calls / allocated bytes | Baseline / current derived payload |
  |---|---:|---:|---:|---:|
  | Cold preparation, wire disabled | 26.522 ms | 26.370 ms | 87 / 45,092,649 | 689,264 / 689,264 |
  | Cold preparation, wire enabled | 26.543 ms | 27.734 ms | 89 / 45,728,841 | 1,325,456 / 1,007,360 |
  | Opaque software, wire disabled | 2.082 ms | 2.094 ms | 2 / 131,072 | 689,264 / 689,264 |
  | Opaque software, wire requested | 2.097 ms | 5.191 ms | 2 / 131,072 | 1,325,456 / 1,007,360 |
  | Translucent software, wire requested | 3.443 ms | 6.389 ms | 15 / 3,801,152 | 1,325,456 / 1,007,360 |

  Baseline software never draws the requested edges, so enabled timings record
  the cost of the added work, not an equivalent-output regression gate. Unique
  edges halve the old edge-index payload, saving 318,096 bytes here; cold wire
  preparation uses about 4.5% more CPU. Cumulative allocated bytes do not fall
  during compaction, and its transient old/new storage overlap is not measured
  as peak memory. Warm software wiring adds no allocation. The candidate's GPU
  opaque submissions use 35.5 µs CPU without wires and 37.0 µs with them, with
  60 / 9,258 versus 64 / 9,766 allocation calls/bytes. GPU wire submissions keep
  those 64 calls / 9,766 bytes at 8³, 24³ and 48³ while uploaded geometry grows
  from 22,400 to 1,007,360 bytes; the edge buffer is resident, not rebuilt per
  frame. This is CPU submission cost, not GPU execution latency/throughput or
  energy. Baseline GPU rejects wireframes, so its wire-disabled submissions are
  only an overhead reference. Driver initialization/first use, cross-chunk
  performance, total live/RSS/VRAM and real-device power remain unqualified.
- Strict Clippy passes for production scene code and both independent render
  integrations with `--no-deps`. Changed-file formatting, diff checks and the
  GPU/GPUI wasm library closure pass. Previously recorded broad Clippy failures
  remain open. C4 remains unchecked for standalone line/point behavior and
  material validity, general intersecting transparency, nonlinear/banded
  transfer parity and remaining f64-to-f32 limits. All other open platform,
  aggregate-memory, scheduling, text/FFI, Taffy, CUDA/MLX, physical-device and
  completion requirements remain in scope. The original audit is unchanged.

## Standalone line/point correctness and resident GPU checkpoint

- Line cells now interpolate selected point scalars and preserve selected cell
  values per primitive. Vertex cells draw one physical target pixel. Both are
  unlit and honor validity masks, transfer alpha, actor opacity, user/frustum
  clipping and opaque depth. An invalid endpoint omits its entire line; invalid
  cells/points are omitted. Lines draw before point markers within an actor.
  Surface depth ties now use less-or-equal, matching the GPU comparison and
  making the last coincident opaque primitive visible. General intersecting
  transparency and ordering across primitive kinds remain open C4 work.
- Software reads canonical f64 positions/scalars, carries attributes through
  clipping, and uses perspective-correct line interpolation. An independent
  pinhole oracle caught a 19-byte-level color error when attributes were sampled
  at coverage-step positions. Interpolation/depth now use the segment projection
  of each covered pixel center. Constant color and depth skip that calculation;
  unclipped segments skip redundant endpoint interpolation/clamping. Geometry
  coverage still permits backend endpoint/junction raster-rule differences.
  The perspective oracle allows six color levels for subpixel/LUT/unorm effects,
  checks three interior positions after near-plane clipping, adds user clipping,
  and deliberately interpolates raw values outside the transfer range before
  mapping. Clamping endpoints first would fail that oracle.
- Preparation retains compact cell IDs and borrows the canonical connectivity,
  positions, scalar arrays and masks. IDs use u32 where representable and a wide
  path for larger usize cell indices; the boundary is tested without constructing
  billions of cells. Pure line/point datasets skip surface extraction and cell
  averaging. In mixed datasets their cell values cannot pollute surface averages.
  Existing surface-to-surface cell averaging remains unresolved. At ordinary cell
  counts each valid line/point retains four ID bytes. This replaces eight bytes
  per old line; point selection adds storage where points were previously absent.
  Empty selection metadata adds 72 bytes to each prepared geometry relative to
  the old single line vector, including surface-only actors; this is excluded
  from derived-payload counters. Compaction's transient overlap, infallible
  selection-vector growth and aggregate preparation limits remain C5/P1 work.
- GPU LineList/PointList buffers contain interleaved local position plus raw
  scalar (16 bytes/vertex), two vertices/line or one/point, without normals or
  index buffers. Per-kind opaque/transparent pipeline pairs initialize lazily;
  surface-only uploads initialize neither pair. Each packed upload is bounded by
  device buffer bytes, the u32 draw count and the default one-million-primitive
  chunk cap. Temporary packing is bounded per chunk; byte sums are checked before
  buffer creation. Aggregate staging/residency budgets remain open. Tests check
  cap arithmetic, exactly 48 uploaded bytes for a line plus a point, stable
  buffer handles over 12 camera/material updates and one geometry preparation,
  and zero uploaded primitive bytes after a mask revision removes both cells.
- The point vertex entry snaps the projected position to a pixel center and uses
  half-open physical viewport bounds. Uniform ABI **4** retains size 192, count
  offset 176 and wireframe ULP offset 76; physical width/height are aligned at
  offsets 184/188. `scene_uniform_block_for_target` fills those dimensions and
  rejects zero sizes. Custom point hosts must adopt that builder/layout; older
  builders intentionally leave viewport zero and therefore suppress points.
- GPU primitive packing rejects nonfinite f32 conversions, underflow-to-zero
  and subnormal results. Transfer-domain precision checks remain in force. GPUI
  retains the compatibility verdict across camera/material changes, selects an
  actual retained software image for unsupported precision and recovers the
  custom GPU route on a representable field revision. Tests cover 24 camera
  updates per verdict with one compatibility scan, revision invalidation, an
  f64 scalar domain separated by 2^-40 and raw values of 1e300. This does not
  qualify arbitrary f64 geometry/scalar ranges for GPU rendering or solve all
  software geometric precision limits; explicit software projection/clipping
  precision failures remain possible.
- `tests/primitives.rs` runs the same public oracles on software and required
  Metal: selected associations at shared nodes, finite masked placeholders,
  large translated origins, one-pixel points, exact half-open viewport edges,
  clipping, unlit gradients, half alpha, near/far rejection, the pinhole test,
  depth ties and exact opaque occlusion in either actor order. In an isolated
  archive of unchanged `171abbfc`, all three tests fail for substantive reasons:
  software lines keep their flat color, software points disappear, and GPU
  upload rejects line cells. The same fixture passes on the candidate.
- `qa/perf/20261019-primitives-paired.json` archives the exact release probe,
  dependency lock, paired runner, raw samples, source/binary/test hashes and
  baseline/candidate oracle results. Baseline/candidate have separate targets;
  allocator-disabled ABBA twice supplies four fresh processes/version with five
  samples/case. Allocation builds are separate, and no task-owned builds/tests
  run during timing. Preparation uses fresh scene caches; warm software retains
  one geometry build. GPU upload timings reuse prepared geometry and initialized
  pipelines; resident submissions exclude completion/readback, which wait and
  check exact within-version pixels outside timing. Initial measurements before
  the constant-value/unclipped fast paths remain in
  `qa/perf/20261019-primitives-initial.json`. All candidate image hashes match
  between those initial and final probes. At 65,536 cells, 128-square frames:

  | Operation | Baseline process CPU | Current process CPU | Baseline / current allocated bytes | Baseline / current derived payload |
  |---|---:|---:|---:|---:|
  | Cold plain-line preparation | 0.454 ms | 0.388 ms | 526,552 / 264,480 | 524,288 / 262,144 |
  | Cold point-colored-line preparation | 0.483 ms | 0.505 ms | 526,649 / 264,577 | 524,288 / 262,144 |
  | Cold cell-colored-line preparation | 0.602 ms | 0.426 ms | 3,672,377 / 264,577 | 524,288 / 262,144 |
  | Cold point preparation | 0.302 ms | 0.408 ms | 2,361 / 264,577 | 0 / 262,144 |
  | Warm flat software lines | 14.398 ms | 15.367 ms | 131,072 / 131,072 | 524,288 / 262,144 |
  | Warm point-colored software lines | 14.418 ms | 66.931 ms | 131,072 / 131,072 | 524,288 / 262,144 |
  | Warm cell-colored software lines | 14.441 ms | 15.735 ms | 131,072 / 131,072 | 524,288 / 262,144 |
  | Warm software points | 0.003 ms | 0.778 ms | 131,072 / 131,072 | 0 / 262,144 |

  Baseline coloring/point cases omit required work and are correctness-cost
  references, not equivalent-output performance gates. Flat software line pixels
  match exactly at all three sizes. Their initial 46% CPU regression falls to
  6.7% at the largest case; 256/8,192-line cases are about 6% faster/0.3% slower.
  **The remaining large flat-line CPU regression stays open.** Warm software uses
  two frame allocations and no per-primitive allocation; retained selection bytes
  halve. Cell-line cold allocation bytes fall about 93% by avoiding unused
  surface expansion. Point preparation now pays for valid point selection.
  The unchanged surface control has exact CPU/GPU pixels and the same draw
  allocation counts; small timing differences are not an improvement claim.
- Candidate GPU uploads use 2,097,152 geometry bytes for 65,536 lines or
  1,048,576 for points; warm upload CPU is roughly 0.41–0.71 ms, with 55 allocation
  calls and 2,105,495 / 1,056,919 allocated bytes. Resident line/point submissions
  take about 37–44 microseconds of process CPU at that size, retaining 61/64
  allocation calls and 9,620 bytes per submission across 256/8,192/65,536 cells.
  Those fixed host/driver allocations remain optimization work. GPU line support
  removes the forced software route for representable GPUI scenes, but upload
  packing duplicates shared line endpoints and increases residency relative to
  CPU ID metadata. First use, GPU execution, overdraw, total RSS/VRAM, mobile
  pressure and measured energy remain unqualified; submission CPU is not power.
- Final required-GPU verification passes 103 scene library tests and all public
  scene/pressure integrations, including existing surface and wireframe oracles.
  Software-only verification passes 75 library tests and all three rendering
  integrations. The native viewer passes at 2x DPI. Strict Clippy passes the
  production library and three rendering integration targets with `--no-deps`;
  previously recorded broad all-target lint failures remain open. Changed-file
  formatting/diff checks and the GPU/GPUI wasm library closure pass.
- C3/C4 remain unchecked for other validity consumers, surface cell ownership,
  nonlinear/banded transfer parity, arbitrary transparency and remaining f64
  limits. Aggregate memory, scheduling, FFI/text, other platform/device gates,
  Taffy, CUDA/MLX experiments and full acceptance remain in scope. The original
  audit is unchanged; this checkpoint does not complete the implementation goal.

## Source-owned surface cell fields checkpoint

- Previous goal turn: progress (`d4f651eb`). This turn fixes the remaining
  surface cell-to-point averaging error: each emitted triangle retains its
  source cell's converted scalar, and a masked cell's entire surface triangles
  are omitted. Missing data does not change the input mesh's topology or create
  new interior cut faces; callers selecting a different domain must filter the
  dataset. Point-colored/uncolored semantics remain unchanged.
- `gpui_mesh::try_for_each_surface_chunk` visits bounded chunks with borrowed
  per-triangle usize source-cell IDs, using an immutable `ValidatedDataSet` view.
  Callback errors stop traversal. Local index counts are capped to fit u32.
  The cell renderer additionally caps chunks at the existing one-million-triangle
  default. It consumes each chunk directly, avoiding the full boundary-triangle
  array and three full-source-point f64 averaging arrays for cell fields.
  **Volume face discovery still builds a dataset-wide map**; total extraction,
  source-size and GPU residency budgets remain open P1/C5 work.
- Volume face-map entries store a one-based source cell plus local face index,
  then reconstruct only surviving boundary polygons. The optional owner keeps
  its niche; an entry is no larger than the old polygon entry on this target.
  Canonical face ordering, winding and shared-face culling are preserved. Tests
  cover exact owner identities for adjacent hexes, direct triangle/quad/polygon
  cells mixed with lines/vertices, tetrahedra/wedges/pyramids/hexahedra, chunk
  sizes 1/3/7/unlimited, deterministic order and callback cancellation. Core
  surface helpers retain their existing public signatures; the visitor is an
  additive API.
- Scene preparation preserves ordinary shared vertices for their first value.
  Only disagreements in converted scalar bits append position/normal/scalar
  attributes; identical cell values reuse geometry. Smooth geometric normals
  are computed from the valid triangles before splitting, then copied unchanged
  to duplicates. A two-hex test keeps 12 vertices for equal values and 16 for
  different values, with exact corresponding normals. This does not resolve
  existing normal changes across different chunk partitions or all f64 surface
  precision limits. Public `prepare_chunks` now validates source geometry/field
  storage before gathering; scene-owned preparation still validates once per
  new geometry revision. Invalid cell-value/mask lengths return mesh errors.
- Cell splits can produce multiple local representations of one global wire
  edge. Cell-field preparation deduplicates those edges within and across chunks;
  the first emitted valid owner supplies a shared edge's transfer alpha. Counts
  remain five for a split two-triangle quad and three with one masked cell, at
  chunk budgets 1/2/100. A translucent rendered seam composes once. That explicit
  ownership rule does not solve general intersecting-transparency composition.
- `tests/cell_fields.rs` compares against separately constructed flat-colored
  cell actors: exact full images, shared red/blue nodes, half opacity, clipping,
  origins 0 and 1e12+0.125, volume-front owner colors and unique wire alpha. Its
  independent mask case removes a center cell whose every node also belongs to
  valid neighbors; the expected hole remains empty. All three public tests fail
  on isolated unchanged `d4f651eb`: 8,192 differing software bytes / 8,128 GPU
  bytes for cell-color bleeding, and an opaque blue center instead of a hole.
  The same fixture passes on the candidate. No screenshot tolerance masks those
  failures; image comparisons are exact within each backend.
- `qa/perf/20261019-cell-fields-paired.json` contains exact release source, lock,
  runner, raw allocator-disabled ABBA-twice samples, separate allocation builds,
  source/test/binary hashes and baseline/candidate oracle output. It covers
  8/24/48-cubed grids, 128-square frames, point colors, uniform/varying/missing
  cell colors, and a 24-cubed varying-cell wire case. Point and uniform-cell
  CPU/GPU pixels must match the baseline exactly. Cache preparation is fresh for
  cold cases; warm frames/uploads/submissions retain one preparation. Readback,
  validation and GPU completion are outside submission timing. At 48 cubed:

  | Operation | Baseline process CPU | Current process CPU | Baseline / current Rust allocated bytes | Baseline / current geometry payload |
  |---|---:|---:|---:|---:|
  | Cold point-color preparation | 26.340 ms | 25.931 ms | 45,092,721 / 45,092,721 | 689,264 / 689,264 |
  | Cold uniform-cell preparation | 27.004 ms | 25.103 ms | 47,746,929 / 45,168,081 | 689,264 / 689,264 |
  | Cold varying-cell preparation | 26.805 ms | 25.593 ms | 47,746,929 / 45,965,285 | 689,264 / 931,408 |
  | Cold missing-cell preparation | 26.766 ms | 25.158 ms | 47,746,929 / 45,168,081 | 689,264 / 684,752 |
  | Warm uniform-cell software | 1.926 ms | 1.931 ms | 131,072 / 131,072 | 689,264 / 689,264 |
  | Warm varying-cell software | 2.099 ms | 1.928 ms | 131,072 / 131,072 | 689,264 / 931,408 |

  Uniform-cell preparation saves about 7% CPU and 2,578,848 cumulative allocated
  bytes with the same output/payload. Cold allocation calls increase 90 to 102
  (121 with varying cells) because the streamed chunk arrays grow incrementally
  and discontinuities need a split map. Varying cells reduce preparation CPU
  about 4.5% here, but change output semantics and increase retained/uploaded
  payload about 35%; this is a correctness/memory tradeoff. Their GPU upload CPU
  rises 121.5 to 142 microseconds, while resident submissions use about 35
  microseconds / 60 Rust allocations / 9,944 bytes in both versions. These are
  CPU submission figures, not GPU latency/throughput or power measurements.
- Costs stay explicit: at 8 cubed, cold point-color preparation uses about 5%
  more CPU, uniform cells about 1.3% more and varying cells about 9.1% more.
  At 24 cubed, varying-cell wires add about 2.3% cold CPU and 197,416 allocated
  bytes (payload 241,280 to 297,952). The point/uniform unchanged-output controls
  retain the same draw allocation counts/payload; small timing differences do
  not establish a platform-wide improvement. Warm frames add no per-triangle
  allocation. Counters include positive realloc growth and exclude peak/live
  memory, GPU/native allocations, metadata and vector capacity overhead.
- An extra specialization storing polygons for unowned callers, plus conditional
  snapshot bookkeeping, did not improve the small point-color control in its
  paired run. It was removed; raw results remain in
  `qa/perf/20261019-cell-fields-unowned-experiment.json`, explicitly rejected as
  final acceptance evidence. All 52 candidate image hashes agree between that
  experiment and the retained simpler implementation. The small cold CPU cost
  remains open, alongside the earlier large flat software-line regression.
- Verification passes 18 mesh / 74 transformation / 106 required-GPU scene
  library tests and their public integrations, including pressure and all four
  render oracle targets. Software-only passes 78 scene library tests and all
  four render integrations. The native viewer passes at 2x DPI; mesh doctests,
  targeted strict production/integration Clippy, changed-file formatting/diff
  checks and the GPU/GPUI wasm library closure pass. Broad all-target lint
  failures previously recorded remain open. The final source/test hashes match
  the retained paired measurements exactly.
- C3/C4 remain unchecked for other validity consumers, finite f64 values outside
  the surface f32 representation, nonlinear/banded GPU transfer parity and
  arbitrary transparency. Full topology/aggregate budgets, scheduling/power,
  text/FFI, other platforms/devices, Taffy, CUDA/MLX and acceptance remain in
  scope. The original audit stays frozen; this is further implementation
  progress, not completion of the full goal.

## Verified structured-grid boundary traversal checkpoint

- Previous goal turn: progress (`6d90238a`). Canonical x-fastest hexahedral
  connectivity now bypasses the dataset-wide face map. No grid dimensions or
  trusted constructor metadata are stored by the model, so eligibility is
  inferred from the first cell and point count, then **every cell type, offset
  and node index is checked** against the complete grid. Products/counts are
  checked, dimensions must be at least two, and point IDs must fit u32. Reordered
  cells, a rotated last cell, edited connectivity, extra unused positions,
  malformed offsets/types or inconsistent dimensions reject the fast path.
  Rejected/mixed/unstructured topology retains the general face-map algorithm.
- After that O(cell_count) proof, three stack cursors enumerate the XY, XZ and
  YZ boundary planes in increasing minimum-node order. A three-way merge uses
  XY/XZ/YZ tie order, equivalent to the existing sorted polygon-node keys.
  Actual polygons are reconstructed from their original owner and face table;
  source-cell IDs, face order, diagonal choice and winding remain exact.
  Face emission is O(boundary_face_count) with constant auxiliary cursor storage;
  it neither scans every grid point nor constructs/sorts a global face list.
  Arbitrary rectilinear/curvilinear coordinates are supported because eligibility
  depends entirely on connectivity. Positions, fields, source ownership and
  public APIs/ABI remain unchanged.
- This is a topology-specific bound, not full P1/C5 completion. Traditional
  `boundary_triangles` still retains its output vector, point preparation still
  uses surface/chunk arrays, cell-wire preparation keeps edge ownership state,
  and general volume topology still uses the full face map. Aggregate source,
  output, staging and GPU budgets remain open. The input grid constructors still
  use unchecked dimension products/casts and duplicate temporary cell/reference
  arrays; source inspection also confirms no cells are emitted when any grid
  dimension is one. Constructor bounds and lower-dimensional behavior need
  separate correction/qualification.
- New tests compare exact ordered `(source cell, triangle)` sequences against
  the general algorithm for all 125 dimension triples from 2 through 6, plus
  long thin grids and negative spacing/large origins. They verify rectilinear
  and warped-coordinate eligibility, edited/reordered fallback, invalid metadata,
  and immediate callback cancellation. Cursor arithmetic reaches the last u32
  point at conceptual 2^32-point shapes without large allocations. These are
  arithmetic tests, not claims that such datasets fit device memory.
- `tests/structured_boundary_memory.rs` adds an isolated native allocation guard:
  a 48-cubed canonical grid, prevalidated outside the probe, 4,096-triangle
  chunks, all callback geometry black-boxed, exact triangle count, and fewer
  than four MiB cumulative Rust allocations during traversal. It does not bound
  peak/live memory or include the source/validation. The identical release test
  fails unchanged `6d90238a` with **44,285,984 allocated bytes** and passes the
  candidate. The crate's existing profiler dev dependency supplies the allocator;
  no production dependency was added. Criterion's three existing scene cases
  now unwrap operation results, so returned errors fail the benchmark rather
  than appearing as fast successful work. The bench target passes compilation.
- `qa/perf/20261019-structured-boundary-paired.json` records the exact release
  source/lock/runner, source/test/binary hashes, raw allocator-disabled ABBA-twice
  timing/process-CPU samples and separate allocation builds. Baseline/candidate
  targets are isolated. One warmup plus five samples/case, four fresh timing
  processes/version, with no task-owned build/check/test work during timing.
  Every cross-version image/ordered-output hash and retained payload length
  matches exactly across **72 cases**: point/uniform/varying/missing fields at
  8/24/48 cubed, a last-cell-edited fallback control, a 24-cubed wire case, and
  boundary-vector/owned-chunk traversal at 8/24/48/96 cubed. The visitor hashes
  source owners and ordered global node IDs, not just triangle counts.

  | Operation | Baseline process CPU | Current process CPU | Baseline / current cumulative Rust allocated bytes |
  |---|---:|---:|---:|
  | Cold point-colored scene, 48 cubed | 26.388 ms | 1.694 ms | 45,092,721 / 1,445,765 |
  | Cold uniform-cell scene, 48 cubed | 25.533 ms | 2.026 ms | 45,168,081 / 1,521,125 |
  | Cold varying-cell scene, 48 cubed | 25.917 ms | 2.505 ms | 45,965,285 / 2,318,329 |
  | Cold missing-cell scene, 48 cubed | 25.488 ms | 1.990 ms | 45,168,081 / 1,521,125 |
  | Cold rejected-grid scene, 48 cubed | 26.389 ms | 26.442 ms | 45,092,721 / 45,092,721 |
  | Ordered boundary vector, 96 cubed | 235.651 ms | 2.287 ms | 348,127,236 / 1,572,864 |
  | Owned-chunk visit, 96 cubed | 236.990 ms | 7.117 ms | 348,827,704 / 2,273,332 |

  Cold scene cases include validation and preparation; source construction is
  excluded, cache state is fresh. The raw boundary-vector case includes its
  connectivity proof and output allocation. Visitor validation occurs once
  outside timing, while the connectivity proof, chunk remapping and callback
  hash are timed; it retains only count/hash. Its chunk arrays are bounded by
  the 4,096-triangle budget, not by the dataset's volume-cell count. The boundary
  vector retains 1,299,600 payload bytes at 96 cubed in both versions; capacity
  and allocator overhead are excluded from that length-based payload figure.
- At 48 cubed, point-scene cold CPU falls about 94% and cumulative allocation
  bytes about 97%; cell-scene cold CPU falls 90–92%. The prior small canonical
  grid cold regression is superseded: at 8 cubed point preparation is 115.5 to
  31.5 microseconds, with 230,025 to 41,693 allocated bytes. The edited-grid
  controls remain within about 0.5% CPU across these sizes and keep the old
  allocation costs. Warm software/draw allocations and GPU geometry payloads
  remain equal; small observed warm timing differences are not an improvement
  claim. There is no new per-frame work. The earlier software-line CPU regression
  remains open because this optimization concerns volume boundary extraction.
- GPU upload/submission timing retains the previous method: initialized device,
  retained CPU geometry and GPU buffers, completion/readback/validation outside
  submission timing. These numbers do not establish GPU execution throughput,
  mobile peak memory, whole-application performance or measured power savings.
  Allocation counters include positive realloc growth, exclude native/GPU
  allocations, and are not live/RSS measurements. No physical-device energy
  qualification is claimed.
- Verification passes 25 mesh / 74 transformation / 106 required-GPU scene
  library tests and public integrations, including all four rendering oracles;
  software-only passes 78 scene library tests and all four rendering integrations.
  Native 2x-DPI viewer, mesh doctests, strict mesh test Clippy, targeted scene
  production/integration Clippy, changed-file formatting/diff checks and GPU/GPUI
  wasm library closure pass. Scene bench compilation passes. A post-measurement
  unit-only checked-conversion assertion was corrected for Clippy; its old source
  and old/new hashes are archived, and the production prefix matches the measured
  code exactly. No production timing was repeated for that test-only change.
- P1/C5 remain unchecked for general topology, constructor/import and aggregate
  budgets; C3/C4, scheduling/power, other renderers/caches, text/FFI, platform and
  device matrices, Taffy, CUDA/MLX and final acceptance remain in scope. The
  original audit is frozen and the full implementation goal remains active.

## Checked grid construction and singleton topology checkpoint

- Previous goal turn: progress (`be2a4f55`). Grid constructors now check dimension
  products, u32 point addressing, connectivity/offset counts and per-array
  address space before reserving geometry. Nonfinite affine origins/spacing,
  overflowing affine endpoints, nonfinite rectilinear axes and nonfinite explicit
  positions return errors. Curvilinear position counts must match the dimensions.
  New typed mesh errors distinguish resource limits, failed vector reservations
  and mismatched grid position counts. Existing factory signatures remain.
- Public `GridBuildLimits { max_working_bytes }` and the three additive
  `*_grid_with_limits` factories let hosts set an explicit construction budget.
  Existing factories use **256 MiB**. This is a behavior change: formerly accepted
  large grids can now return a resource error. Python specs use that default;
  untrusted requests cannot choose an unlimited override. Constrained hosts should
  lower it to their grid share of the aggregate application budget.
- The plan bounds simultaneous **array element storage**, including Vec-to-Arc
  conversion overlap and curvilinear input capacity. Positions convert before
  cell arrays are built. Allocator/Arc headers, rounding, identifiers, borrowed
  axes and later-added fields are excluded. It is neither an RSS bound nor an
  allocation-success guarantee: vector reservations are fallible, but final Arc
  allocations retain standard Rust infallible behavior. Source/import, output,
  staging, GPU and total-application budgets remain open.
- Cell types, offsets and connectivity are built directly with checked capacities,
  removing the full temporary cell tuples and reference arrays. Three active axes
  keep exactly the previous VTK hexahedra and x-fastest order. Two/one/zero active
  axes now produce quads/lines/a single vertex; previously every singleton axis
  suppressed all cells. Quad corners follow increasing active-axis order.
  Coincident/nonmonotonic finite coordinates remain permitted; this is topology
  construction, not a nondegenerate geometric Jacobian guarantee.
- The baseline-compatible public oracle covers all active-axis permutations,
  exact 3D/2D connectivity, finite coordinates and impossible dimensions. All
  three tests fail substantively on isolated unchanged `be2a4f55` (missing plane
  cells, accepted NaNs and capacity overflow) and pass on the candidate. Additional
  tests exercise exact budget boundaries, oversized curvilinear capacity,
  arithmetic/address headroom, default/explicit limits and wrong position counts.
  An isolated native allocation test verifies rejected requests allocate no
  grid-sized arrays. Python decoding checks enormous dimensions and singleton
  topology. CPU and required-Metal render tests compare seven singleton shapes
  against independently constructed quad/line/vertex cells with exact full images
  and nonempty foreground.
- `qa/perf/20261019-grid-build-paired.json` archives the release probe, identical
  lock, runner, source/test/binary hashes, raw samples and baseline/candidate
  oracle results. Separate timing/allocation and baseline/candidate targets are
  used. Allocator-disabled ABBA twice gives four fresh timing processes/version,
  one warmup and five samples/case, with no task-owned builds/tests during timing.
  Coordinate input setup is excluded; curvilinear input consumption/conversion is
  timed, its pre-staged clone is not. Validation, ordered exact-bit geometry hashes
  and destruction are outside timing. All nine equivalent-work 3D cases at
  16/48/96 cubed match output hashes, cell counts and array payloads exactly.

  | Constructor, 96 cubed | Baseline process CPU | Current process CPU | Baseline / current cumulative allocated bytes |
  |---|---:|---:|---:|
  | Affine | 11.258 ms | 3.282 ms | 177,216,351 / 112,772,183 |
  | Rectilinear | 11.021 ms | 3.273 ms | 177,216,351 / 112,772,183 |
  | Curvilinear | 9.545 ms | 2.196 ms | 155,982,687 / 91,538,519 |

  CPU falls 70–77% and cumulative allocated bytes 36–41% at that size; allocation
  calls fall 49 to 9 (curvilinear 48 to 8). Final array payload stays 56,386,047
  bytes. At 16/48 cubed, all equivalent-work constructor cases also improve CPU
  and allocation bytes. These allocation counters include positive realloc growth
  and are not peak/live memory measurements.
- Separate allocator-disabled macOS `time -l` runs perform one 96-cubed build per
  fresh process, ABBA twice for each constructor, and verify the same exact output
  hash. Median **maximum process RSS is 146,538,496 to 115,015,680 bytes** for all
  three constructors, about 21.5% lower. RSS includes runtime/allocator, source
  coordinate generation and validation/hash; curvilinear RSS runs retain no
  template clone. It differs from the planned element-workspace budget and says
  nothing about VRAM or mobile device memory. Thermal/affinity are uncontrolled;
  power savings and whole-application behavior are unmeasured.
- Six lower-dimensional measurements are correctness-cost references, not speedup
  gates: a 64-square plane now builds 3,969 quads (roughly 14.5–15 microseconds,
  versus 9–10 for zero cells), and a 4,096-point line builds 4,095 lines (12–15.5
  microseconds versus 9). Plane payload grows 98,312 to 197,537 bytes; line payload
  to 167,927. Their increased work/memory must not be hidden as a regression-free
  optimization. No new per-frame work is introduced.
- Verification passes 26 mesh / 74 transformation / 106 required-GPU scene
  library tests and public integrations, plus 78 software-only scene library
  tests. The new rendering integration passes both CPU and required Metal.
  All 223 Rust Python-runtime tests pass with a supported Python interpreter.
  The initial system Python 3.9 run failed its cross-language import; the package
  declares Python >=3.10, and scoped PATH selection of Homebrew Python resolves
  that environment failure. Seven mesh doctests, strict mesh-test Clippy, targeted
  scene production/grid-integration Clippy and the GPU/GPUI wasm closure pass.
  Previously recorded broad all-target lint failures remain open. Final measured
  production/test/lock hashes match the current files exactly.
- C5/P1 remain unchecked for general topology, imports and aggregate residency.
  C3/C4, other caches/renderers, demand scheduling/power, text/FFI, full platform
  and physical-device coverage, Taffy, CUDA/MLX experiments and final acceptance
  remain in scope. The original audit stays frozen and the full implementation
  goal remains active.

## General volume face-workspace checkpoint

- Previous checkpoint: progress (`3ecd939d`). General volume extraction now
  collects fixed-size face records into one fallibly reserved array, sorts keys
  in place and emits only keys with exactly one owner. The production global hash
  map and separate surviving-face vector are removed. Source cell IDs, sorted
  polygon-key order, owning winding, fan diagonals and the rule that all faces
  with two or more owners are culled remain unchanged. Canonical hexahedral grids
  retain the constant-space cursor route. The previous map implementation remains
  under `cfg(test)` as an independent ordered-output oracle.
- Additive `SurfaceExtractionLimits { max_face_bytes }`,
  `try_for_each_surface_chunk_with_limits` and `surface_chunks_with_limits`
  preflight the conservative face-table record count, element bytes and address
  space before reserving or invoking any callbacks. Degenerate faces can reduce
  actual record count but do not reduce this preflight allowance. Only face-record
  element storage is covered: allocator overhead, sorting stack, source arrays,
  chunk vectors/remapping, callbacks' retained output and GPU residency are not.
  Face reservation failures return typed allocation errors; other chunk/output
  vectors retain their prior infallible allocation behavior.
- Scene point/cell/uncolored preparation uses the **64 MiB** default face-record
  allowance. `prepare_chunks_with_limits` provides a standalone explicit override.
  Default retained scene preparation can now return a mesh resource-limit error
  for large unstructured/noncanonical meshes. A host supplying its own allowance
  must also budget the other live arrays; this is not an aggregate scene bound.
  Legacy mesh convenience/callback APIs retain unlimited face-workspace behavior
  for signature compatibility and can panic on a failed face reservation. Other
  transformation/export consumers of those legacy APIs still need migration.
- `surface_chunks` now feeds a shared chunk builder directly from extraction,
  removing the full intermediate triangle vector. Collected surface chunks do
  not allocate source-cell IDs. The builder reuses its remapping table across
  chunk boundaries, caps local indices to u32 and preserves first-use vertex
  ordering. It still retains all chunks for convenience callers, including scene
  point preparation; callback streaming remains available to consuming hosts.
- Tests check exact mixed direct/volume triangle order, owners and winding,
  budgets at the exact internal record-byte boundary, rejection before direct
  surface callbacks, zero workspace for canonical/surface-only paths, callback
  cancellation and three-owner nonmanifold culling. Scene tests verify explicit
  limits for uncolored, point and cell associations. Existing 125 canonical-grid
  comparisons and edited/reordered fallback cases compare against the old map.
  An isolated native 48-cubed noncanonical visitor guard with 4,096-triangle
  chunks requires fewer than 32 MiB cumulative Rust allocations, excluding source
  construction/validation. The identical release fixture fails unchanged
  `3ecd939d` at 44,285,984 bytes and passes the candidate. This is allocation
  traffic, not a peak/live-memory guarantee.
- `qa/perf/20261019-general-boundary-paired.json` records exact release
  probe/lock, runner, raw allocator-disabled ABBA-twice samples, separate
  allocation builds, source/test/binary hashes and the memory oracle. Four fresh
  timing processes/version, one warmup and five samples/case; no task-owned
  builds/tests overlap measurement. All **72 cases** preserve cross-version
  image/ordered-output hashes and retained payload lengths exactly. The general
  grids rotate the last hex to defeat canonical detection while preserving face
  sets. This is an exercised general algorithm, not a representative survey of
  every arbitrary tetrahedral mesh distribution. The old map stays the separate
  unit oracle for mixed cell families.

  | Operation | Baseline process CPU | Current process CPU | Baseline / current cumulative allocated bytes |
  |---|---:|---:|---:|
  | Cold general point scene, 48 cubed | 27.071 ms | 20.334 ms | 45,092,721 / 21,192,757 |
  | Cold general uniform-cell scene, 48 cubed | 26.104 ms | 20.555 ms | 45,168,081 / 21,455,141 |
  | Cold general varying-cell scene, 48 cubed | 26.674 ms | 21.100 ms | 45,965,285 / 22,252,345 |
  | Cold general missing-cell scene, 48 cubed | 26.026 ms | 20.511 ms | 45,168,081 / 21,455,141 |
  | General owned-chunk visit, 48 cubed | 26.426 ms | 20.062 ms | 44,285,984 / 20,573,044 |
  | General boundary vector, 96 cubed | 241.178 ms | 173.398 ms | 348,127,236 / 166,188,864 |
  | General owned-chunk visit, 96 cubed | 246.397 ms | 178.366 ms | 348,827,704 / 166,889,332 |

  Cold preparation uses a fresh Scene/cache and includes validation; source
  construction is excluded. Owned visitors prevalidate outside timing and hash
  exact owners/global IDs inside timing, retaining only count/hash. Boundary
  vectors check exact ordered IDs outside timing. Face detection/preflight,
  collection, in-place sorting and culling remain timed. At 48 cubed, general
  cold CPU falls 21–25% and allocation bytes 52–53%; 8/24-cubed general cases also
  improve both. Varying-cell wire preparation at 24 cubed falls 3.692 to 3.029 ms
  and 6,575,633 to 3,373,949 allocated bytes.
- The unchanged canonical point control saves allocation bytes but has a small
  cold CPU cost: 29 to 29.5 microseconds at 8 cubed, 336.5 to 342 at 24 cubed,
  and 1.696 to 1.719 ms at 48 cubed (about 1.3–1.7%). At 48 cubed its allocation
  traffic drops 1,445,765 to 1,258,741 bytes, with one extra allocation call.
  Dynamic chunk-vector growth replaces the old exact-sized index reservation;
  capacities and allocator rounding are excluded from retained payload figures.
  This cost remains explicit, alongside the earlier large flat-line CPU cost.
  Warm frame/draw allocations and CPU/GPU geometry payloads remain unchanged;
  small warm timing changes do not establish a per-frame improvement.
- Separate allocator-disabled macOS `time -l` runs perform one general 96-cubed
  visitor per fresh process, ABBA twice, with identical owner/node hash. Median
  maximum process RSS falls **425,746,432 to 284,475,392 bytes**, about 33.2%.
  These runs include source construction, validation and callback hashing; they
  use the legacy unlimited visitor deliberately to qualify the larger workload.
  That 96-cubed general face array exceeds the new 64 MiB scene default and would
  require an explicit allowance in a limited API. RSS is not the face-record
  element budget, VRAM or mobile memory. No power/energy, GPU execution time,
  whole-application or physical-device improvement is claimed.
- The first measurement attempt was rejected because the oracle test rebuilt
  the allocation executable without its GPU feature, yielding only 40 cases.
  Rebuilding the intended feature set and rerunning produces 72 cases in every
  timing/allocation process. An output-path-only correction is archived with the
  exact executed runner; the preceding structured-boundary artifact is restored
  byte-for-byte from Git. No compiled source, probe, binary, sample or measurement
  logic changed for that destination correction.
- Verification passes 27 mesh / 74 transformation / 106 required-GPU scene
  library tests and public integrations, plus 78 software-only scene library
  tests and software integrations. Mesh doctests, strict mesh-test Clippy,
  targeted scene production/workspace-test Clippy, native viewer compilation,
  all 223 Rust Python-runtime tests with supported Python, and the wasm GPU/GPUI
  library closure pass. The pre-existing broad all-target lint
  failures remain open. Final source/test/lock hashes match the measured files;
  the original audit and earlier evidence artifacts remain unchanged.
- P1/C5 remain unchecked for legacy consumer migration, retained-scene host
  allowance configuration, chunk/output/source/import and aggregate CPU/GPU
  budgets. This bounds one workspace by returning structured errors; it does not
  stream arbitrarily large unstructured topology through a tiny allowance.
  C3/C4, remaining renderer/cache work, scheduling/power, text/FFI, other
  platforms/devices, Taffy, CUDA/MLX experiments and full acceptance remain in
  scope. The full implementation goal remains active.

## Host-owned scene allowances and bounded GPU upload checkpoint

- Previous goal turn: progress (`295ae7ad`). `Scene::surface_limits` and
  `with_surface_limits` now pass the host allowance through retained preparation,
  software rendering and GPU upload. Limits govern new face-workspace allocation;
  already prepared geometry needs no face workspace and stays reusable under a
  lower allowance. A changed source or cleared cache must satisfy the new limit.
  Scene clones still share preparation; a rejected build preserves the previous
  usable snapshot. No budget change forces a geometry rebuild by itself.
- `GpuUploadLimits`, stored on `Scene::gpu_upload_limits` (and available through
  `with_gpu_upload_limits`), defaults to **256 MiB requested GPU resources** and
  **4 MiB temporary primitive-vertex element bytes per packing chunk**. Upload
  preflight counts all visible actors' position/normal/scalar/index/edge buffers,
  packed line/point vertices, 256 RGBA8 LUT texels and the 192-byte uniform.
  CPU-shared geometry is counted once per actual uploaded actor because GPU
  upload currently duplicates those buffers. Buffer/index arithmetic and material
  buffer/LUT width device limits are checked before new actor GPU resources.
  Empty triangle chunks retain the prior skipped-upload behavior.
- Transactional `GpuRenderer::sync` includes the old scene's owned resource
  lengths while preparing its replacement. It rejects before GPU replacement
  allocation when old plus new cannot fit, preserving the old scene on failure.
  Hosts can explicitly drop the old scene and upload afresh when simultaneous
  replacement is impossible. An unchanged scene is checked against its current
  allowance without repacking/reuploading; lowering the packing allowance to zero
  is safe while geometry remains resident. Authoritative resource accounting is
  private, so changing the legacy public `geometry_bytes` diagnostic cannot bypass
  admission. `GpuScene::resource_bytes()` exposes the requested-resource total.
- This is a scoped upload allowance, not a physical VRAM or process-memory cap.
  It excludes allocator/driver overhead, targets/pipelines, canonical/prepared CPU
  arrays, wgpu internal staging, other uploaded scenes and resources retained
  externally by pending command buffers. Material/frame updates have separate
  working sets. The primitive allowance governs requested temporary vector
  elements; allocator rounding is outside its scope. Source/output/cache and
  in-flight CPU/GPU aggregate accounting remain open. CPU preparation can still
  allocate before GPU preflight; this change does not make CPU output bounded.
- Primitive buffer sizing now intersects device/draw limits with the host packing
  allowance. At 1,048,576 cells, the former one-million-cell cap produced two
  buffers, with largest temporary line/point vectors requesting 32,000,000 /
  16,000,000 bytes. The 4 MiB default produces eight line buffers or four point
  buffers. Final geometry bytes stay equal. This reduces packing peak storage
  while increasing draw/buffer bookkeeping; a host can choose a larger allowance
  when profiling justifies the memory cost.
- GPUI presentation keys now include resource policies without changing geometry
  identity. Failed resource creation/resize/sync/encode retains one failure key,
  drops failed owned resources and clears derived CPU preparation where needed.
  Camera/background ticks matching the failure no longer recreate pipelines or
  repack geometry. Changed source/material/size/policy/device/format permits a
  retry. `Scene::retry_gpu_resources()` explicitly permits one more attempt after
  pressure/transient-failure recovery; the host then requests a repaint. Successful
  geometry remains reusable. This avoids an indefinite transient-failure verdict
  without adding a polling timer or busy retry loop.
- Native headless GPUI tests exercise 32 camera/background ticks with a denied
  upload: one actual resource-initialization attempt, no retained derived CPU
  bytes, explicit one-shot retry under the still-failing policy, success after
  raising the allowance, dropped resources on failed sync, 32 more camera ticks
  without another initialization, and recovery. The test uses actual wgpu draws;
  it does not assert measured power savings. Custom-draw failures still suppress
  painting under the existing host contract; surfacing a detailed presentation
  error to the product UI remains open. Public upload/sync return typed errors.
- Tests also cover overflow/exact byte boundaries, independent material device
  limits, GPU duplication of CPU-shared actor geometry, primitive packing at the
  minimum line stride, rejected requests before lazy primitive pipeline creation,
  successful exact-budget upload, unchanged resident data under a zero packing
  allowance, old-plus-new rejection with exact preservation of old rendered
  pixels, and successful transactional retry. Public render tests compare exact
  point/line/surface images with 32-byte packing versus the default. Retained
  preparation tests cover cold rejection, warm reuse, changed-source rejection,
  cache clearing and externally held snapshot validity.
- `qa/perf/20261019-upload-budget-paired.json` archives release source/lock,
  runner, raw CPU/wall/allocation/RSS samples and source/test/binary hashes.
  Isolated baseline/candidate and timing/allocation targets; allocator-disabled
  ABBA twice gives four fresh timing processes/version, one warmup and five
  samples/case, with separate allocation runs and no task-owned builds/tests
  during timing. All **24 cases** preserve cross-version image hashes and GPU
  geometry payloads exactly. An initialized Metal renderer and prepared source
  precede timing. Upload is CPU buffer/LUT creation/upload work; rendering,
  readback, completion and validation are outside that clock. Sync batches 1,000
  unchanged calls per sample, normalized per call; submit clocks CPU encode and
  queue submission, with completion/readback outside. Line/point cases share
  source points and constant colors, deliberately avoiding ambiguous tied-color
  output; repeated overdraw is not a GPU throughput benchmark. Canonical/general
  48-cubed surface controls retain their prepared geometry.

  | Operation | Baseline process CPU | Current process CPU | Baseline / current allocated bytes |
  |---|---:|---:|---:|
  | Upload 1,048,576 lines | 9.298 ms | 9.479 ms | 33,563,719 / 33,569,151 |
  | Upload 1,048,576 points | 5.441 ms | 5.161 ms | 16,786,503 / 16,788,263 |
  | Upload canonical 48-cubed surface | 127.5 µs | 125.5 µs | 10,997 / 10,997 |
  | Upload general 48-cubed surface | 126.0 µs | 137.5 µs | 10,997 / 10,997 |

  Large-line upload CPU increases about 1.9%, and general-surface upload about
  9.1%; these costs remain open for profiling. The initial run also observed a
  general-control cost, about 16%, so no broad CPU speedup is claimed. Large-line
  allocation calls rise 65 to 118 and point calls 65 to 83 because packing creates
  more buffers. Total allocated element traffic is nearly unchanged; the goal is
  smaller simultaneous packing storage, not less cumulative vertex work. Sync
  stays allocation-free (about 24 ns/call in this simple retained one-actor case).
  Resident-submit results are mixed: large lines measure 67 to 56 microseconds
  of CPU, but allocation calls/bytes rise 61/9,234 to 63/10,722; the general
  surface control measures 33.5 to 36.5 microseconds, with 60/9,282 to 63/9,354.
  These counters include Rust-side driver work and do not establish unchanged
  frame allocation cost. Small point submits also show extra calls. Shader
  execution, first use, realistic overdraw and measured energy are not qualified
  by these CPU submission timings.
- Separate allocator-disabled macOS `time -l` runs perform one large upload per
  fresh process, ABBA twice per primitive family. Median maximum process RSS is
  **121,274,368 to 92,110,848 bytes** for lines (about 24.0% lower) and
  **78,528,512 to 66,928,640 bytes** for points (about 14.8% lower). These include
  source/CPU preparation, renderer initialization, render/readback and validation.
  RSS is neither the requested-resource allowance nor physical GPU/mobile memory.
  Thermal/affinity are uncontrolled; no whole-app or energy improvement is claimed.
  The earlier diagnostic remains in `20261019-upload-budget-initial.json`, before
  explicit retry/material-device checks. Its baseline-chunk wording is corrected
  in metadata; the original executed runner is preserved. All final candidate
  images/payloads match that diagnostic exactly, and final measured source/test
  hashes match the files now retained.
- Verification passes 27 mesh / 74 transformation library tests and integrations
  in the broader check, 110 required-GPU scene library tests and all scene
  integrations, 79 software-only scene library tests and integrations, and 223
  Rust Python-runtime tests with supported Python. Targeted strict scene
  production/integration Clippy, doctests, native viewer compilation and the wasm
  GPU/GPUI closure pass. Broad all-target lint failures recorded earlier remain
  open. The original audit and prior checkpoint evidence are unchanged.
- P1/C5/P3 remain unchecked for complete CPU source/output/cache budgets, all
  import/legacy consumers, in-flight submissions and shared device totals. The
  new cold-upload costs, earlier software-line regression, C3/C4 validity and
  representation parity, demand scheduling/thermal/power, text/FFI, other
  renderers/platforms/physical devices, Taffy, CUDA/MLX experiments and full
  acceptance remain in scope. The full implementation goal remains active.
- Review also confirms a remaining C4 camera-precision gap: `Camera::validate`
  checks projection at unit aspect and a view relative to its own target, while
  `encode_to_views` forms actor-relative MVPs and writes them without a final
  finite-coefficient check. Extreme valid finite targets/scales can therefore
  need stronger backend checks. This is unqualified, not completion evidence;
  add a public reproduce-and-recover oracle before the precision fix. Encoding
  error caching must account for camera-dependent failures when those checks
  are introduced, while allocation-failure caching can stay camera-independent.

## Checked GPU camera matrices and fallback recovery checkpoint

- Previous goal turn: progress (`4d85b18b`). The public camera oracle reproduces
  two independent failures on unchanged source: a finite valid target makes the
  actor-relative GPU matrix contain nonfinite coefficients, and a viewport aspect
  makes an otherwise valid orthographic projection collapse horizontally. The
  renderer previously returned success and encoded those uniforms. Both tests
  fail on the exact baseline and pass on the candidate. A tiny valid perspective
  field of view that overflows only at the actual viewport aspect is also covered.
- `Camera::try_view_projection_relative_to` is an additive checked matrix API.
  GPU encoding creates a frame-local projection and trigonometric basis once,
  using the actual positive aspect ratio without the legacy raw helper's minimum
  clamp. It checks required projection terms and final actor-relative matrices
  against a conservative normal-or-zero f32 coefficient profile. Nonfinite,
  collapsed projection or subnormal required/uploaded coefficients return
  `UnsupportedPrecision`; invalid camera/aspect parameters return camera errors.
  Raw matrix helpers keep their original behavior. The profile is explicit and
  does not promise exact f64 geometry or a bound on float rounding error.
- Every actor matrix is checked before GPU staging allocation or encoder commands.
  A per-actor 64-byte CPU matrix scratch field avoids repeating the projection/
  basis work while filling uniforms. The uniform ABI remains 192 bytes; geometry
  buffers/payloads do not change. Matrix scratch is CPU metadata excluded from the
  requested GPU resource allowance. Opaque actors skip unused distance work;
  translucent center sorting uses the local eye offset before world-coordinate
  cancellation. Finite accepted view translations keep f64 squares in range;
  a scaled path preserves very small local distances. General intersecting
  transparency remains approximate.
- The coefficient classifier uses an integer reduction suitable for vectorization.
  Tests compare it against standard-library float classification across both
  signs, every exponent and representative mantissa boundaries at multiple matrix
  positions. Normal viewport/origin matrices remain bit-identical to raw helpers
  across 36 angle/aspect/origin combinations. Tests also cover actual tiny aspects
  instead of the old clamp, conservative subnormal rejection, a large target whose
  world-space eye loses its orbit offset, and underflow-safe local sort lengths.
- GPUI retains a scene/material precision verdict independently of camera precision.
  It checks the current camera against cached actor origins, selects the existing
  software image route for unsupported GPU coefficients, retains static fallback
  images and recovers the GPU route when the camera becomes representable. A bad
  camera no longer poisons the cached topology/material verdict. Encoding failures
  include the camera in their retry key; allocation/resize/sync failures remain
  camera-independent. The existing render key supplies that camera identity,
  avoiding another stored Camera copy in the failure metadata.
- Unclipped scenes now also need origin metadata for camera checks. The first
  applicable source revision scans positions and retains 24 bytes of origin
  elements per visible actor plus Arc/collection metadata. Subsequent camera
  updates reuse it. Position Arc identity, pinned by the prior input, lets scalar,
  mask or topology updates reuse the same bounds independently of full geometry
  identity; changed positions rebuild them. Tests assert Arc reuse across field
  and camera updates and invalidation after copy-on-write position edits. These
  are metadata tests, not whole-GPUI timing or total-memory measurements.
- `tests/camera_precision.rs` owns a real shared wgpu device/queue. Rejection must
  preserve the prior target's exact pixels, a valid earlier pass in the same
  submitted encoder, and that pass when a later actor in a different uploaded
  scene has an invalid matrix. A valid render after rejection recovers unchanged.
  Existing public surface/point/line/wire/cell render oracles stay green. Actual
  `ElementState` tests check image retention and GPU recovery without a source
  change; metadata tests verify no extra geometry preparation or origin rebuild
  over repeated bad-camera updates.
- A third public GPU fixture exposes a defect shared by the old software and
  GPU paths: orthographic near `1e-38`, far `1e36` makes the f32 near-depth bias
  zero. Both baseline backends draw a point behind the near plane (foreground
  counts **1/1**, expected **0/0**). Promoting already rounded matrix coefficients
  to f64 was insufficient. The candidate rejects this GPU projection and uses a
  true f64 primitive projection in software when required f32 terms are lost or
  a primitive matrix becomes nonfinite. Ordinary primitive frames retain their
  original f32 coefficients, promoted once per actor instead of per vertex.
  `tests/camera_reference.rs` independently checks hidden/exact-near/front points
  and a line crossing the near plane. Its identical source fails on baseline and
  passes on candidate. The three required-GPU tests likewise fail on baseline
  and pass on candidate; raw results and exact sources are archived below.
- Software surface/wire homogeneous clipping still uses f32 coordinates. A
  projection requiring the new reference path returns `UnsupportedPrecision`
  before scene preparation or frame allocation when visible surface/volume cells
  exist. The public reference fixture checks this explicit limit and no geometry
  build. This is not full f64 surface support: actor-relative surface matrices,
  attributes, normals, scalar representation and arbitrary transparency still
  need qualification. Camera parameter validation also retains its existing
  unit-aspect f32 checks; this change does not accept every mathematically valid
  f64 camera. GPU fallback applies to the whole scene; per-actor f64 culling or
  rescaling is not implemented.
- Final measurement evidence is
  [`qa/perf/20261019-camera-precision-paired.json`](qa/perf/20261019-camera-precision-paired.json).
  It retains exact probe/runner/lock sources, binary/source/test hashes, all raw
  samples, both public oracle results, and fresh-process RSS. Allocator-disabled
  release ABBA twice gives four fresh processes/version per GPU/software phase,
  one warmup plus five samples/case. Separate allocation processes measure calls
  and positive allocated-byte growth. There are 42 GPU cases (upload, unchanged
  sync and resident CPU submission for primitive sizes, canonical/general 48-cube
  controls and 1/16/128 opaque/translucent actors) plus four software cases in
  separate processes that exit before GPU initialization. Prepared geometry and
  renderer initialization precede timing; GPU completion/readback are outside
  submission timing. All **46** cross-version image hashes and payload lengths
  match exactly. The final candidate also matches the initial diagnostic's 42
  shared cases. Initial evidence remains in `20261019-camera-precision-initial.json`;
  it predates the coefficient-reduction/local-length optimizations and the shared
  near-plane reference correction and is not the final acceptance measurement.

  | Operation/case | Baseline CPU | Candidate CPU | Change |
  |---|---:|---:|---:|
  | Submit, 256 lines | 32.5 µs | 32.0 µs | -1.5% |
  | Submit, 1,048,576 lines | 91.5 µs | 60.0 µs | -34.4% |
  | Submit, general 48-cube surface | 31.0 µs | 31.5 µs | +1.6% |
  | Submit, 128 opaque actors | 180.5 µs | 175.0 µs | -3.0% |
  | Submit, 128 half-alpha actors | 188.0 µs | 178.0 µs | -5.3% |
  | Upload, general 48-cube surface | 108.0 µs | 118.0 µs | +9.3% |
  | Upload, 256 lines | 30.5 µs | 33.0 µs | +8.2% |
  | Software, 256 lines | 21.0 µs | 22.0 µs | +4.8% |
  | Software, 65,536 lines | 5.334 ms | 5.458 ms | +2.3% |
  | Software, 65,536 points | 539 µs | 482 µs | -10.6% |

- These are scoped diagnostics, not an across-platform speedup. The large-line
  scene has extreme overdraw and completion is excluded from submission CPU;
  its result establishes no GPU throughput improvement. Small timings have
  process-clock granularity and uncontrolled thermal/affinity. Software line
  regression and general upload costs remain visible follow-ups. Unchanged sync
  stays allocation-free (roughly 21–22 ns for one actor, 1.1 µs for 128). Every
  resident-submit and software allocation count/byte result matches baseline.
  Upload calls match, while metadata allocation grows 256 bytes for the small
  one-actor Vec capacity, 1,024 bytes for 16 actors and 8,192 bytes for 128 actors;
  the element cost is 64 bytes/actor. Retained GPUI origins add a separate
  24 bytes/visible actor plus metadata and a first-revision position scan, which
  the standalone GPU probe does not time. No total-memory saving is claimed.
- Separate allocator-disabled `time -l` uploads (ABBA twice/kind) measure maximum
  process RSS **92,200,960 to 92,217,344 bytes** for large lines and
  **66,936,832 to 66,936,832 bytes** for large points. Those include preparation,
  GPU initialization, rendering/readback and validation. RSS is not physical VRAM,
  mobile memory or the resource allowance. No measured energy, GPU execution,
  realistic first-use, whole-GPUI or physical-device power claim is made.
- Verification passes 27 mesh / 74 transformation / **116 required-GPU scene**
  library tests and the public integrations, **82 software-only scene** library
  tests and integrations, and **223 Python-runtime Rust** library tests with a
  supported Homebrew Python. Targeted strict scene production/camera integration
  Clippy, doctests, native viewer compilation and the wasm GPU/GPUI library closure
  pass. Existing broad all-target lint failures remain open. Final source/test
  hashes match the measured files; the frozen audit and previous checkpoint
  evidence are unchanged.
- This checkpoint advances C4 and retained camera handling; it does not close C4,
  C3/C5, platform/FFI/text work, aggregate budgets, earlier performance regressions,
  scheduling/thermal/power, Taffy adoption experiments, CUDA/MLX experiments or full
  acceptance. The full implementation goal remains active.
