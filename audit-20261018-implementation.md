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
  Scene-owned retention and DPI-aware presentation are implemented; bounded
  topology extraction and aggregate preparation budgets remain open.
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
