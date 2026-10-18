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
