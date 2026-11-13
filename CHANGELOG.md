# Changelog

## Unreleased

## 2026-11-13

### gpui-spinorama 0.11.2

- Space frequency-plot markers at least 40 logical pixels apart. Curves retain
  their full measurement detail and legends keep their symbols.

### gpui-d3rs 0.11.1

- Render isolated line-chart markers even when there are no connecting segments.

## 0.11.0

### Changed

- GPUI stack (sotf-3rdparties `gpui` 0.2.4) now renders retained frames,
  ported from longbridge/gpui-fast: unchanged views are drawn from last
  frame's output instead of being rendered, laid out, and painted again;
  Taffy layout nodes and text measurements survive between frames; lists
  scroll by replaying rows. Measured steady-state: one-change panel grid
  179us vs 428us from scratch, showcase Table refresh 523us vs 1272us
  (`qa/perf/20261107-retained-frames-paired.json`,
  `crates/gpui-showcase/benches/retained_frames.rs`). Views that read
  state GPUI cannot track (interior-mutable cells, wall-clock time) must
  notify when it changes, as `AnyView::cached` has always required;
  `GPUI_VIEW_RETENTION=0` restores the previous drawing model.
- Aligned crate and Python versions move to the 0.11 series for the new
  rendering stack.


### Added

- Mesh data-model and I/O wave: rectilinear/curvilinear grids, polygon
  cells, composite (multiblock/AMR) datasets, time-series sampling, VTU
  appended and zlib-compressed I/O, STL/OBJ/PLY surface files, and PVD
  collections — with the Python wrapper kept in sync
  (`mesh.read_mesh` / `write_mesh` / `read_pvd`). Delaunay delegates to
  the `delaunator` kernel Sonium's mesher builds on; mesh generation
  stays in Sonium.

### Added

- Three crates for VTK-class 2D/3D mesh visualization, all at the aligned
  0.10.44 version: `gpui-mesh` (dataset model: VTK-style cells, named
  point/cell fields, scalar/vector/tensor/complex fields, bounded surface
  chunking for large objects, time-series support),
  `gpui-dataset-transformation` (plane slice/clip, threshold, isosurface,
  calculators, warps, glyphs, RK2 streamlines, probe,
  vorticity/divergence/Q-criterion diagnostics, legacy VTK + VTU binary +
  XDMF I/O, optional wgpu compute), and `gpui-scene3d` (orbit camera,
  transfer functions, clip planes and transparency, MIP volume
  raycasting, WGSL shaders, 1080p PNG/MP4 movies, and an interactive
  viewer example with a live cutting plane). Demos: PC-fan airflow,
  optimised speaker port, necking bar.
- Python: `gpui_toolkit.mesh` builds structured meshes with named fields
  and runs slice/clip/isosurface/threshold/diagnostics natively through
  `gpui-python-runtime` (`mesh_filters`, `mesh_streamlines`).
- Flow-visualisation wave on the three crates: solid 3D arrow glyphs and
  tensor ellipsoid glyphs, streamtubes/ribbons, animated dashed
  streamlines, contour lines plus banded filled contours, LIC streak
  fields, lambda-2/swirling-strength vortex criteria, time-dependent
  particle advection with trails, a colour-bar/legend with axis triad
  and annotations, and multi-plane plus clip-box clipping in the
  interactive viewer (new `flow_showcase` demo).

### Changed

- Every workspace crate is aligned to version 0.10.44 (previously ranging
  from 0.10.0 to 0.11.0); internal `gpui-*` dependency requirements now
  pin `0.10`. The foreign-parent platform crates move from their 0.11.0
  line back onto the shared 0.10.x line by request.

### Added

- Foreign-parent editor embedding stack (M1-M6): new `gpui-foreign-parent`
  core plus AU/iOS/Android/wasm/Windows/Linux bindings, all at 0.11.0;
  `gpui-au`, `gpui-ios`, and `gpui-android` moved to 0.11.0 with the new
  embedding APIs. Contract: `docs/foreign-parent-editor-embedding.md`.

## 0.10.33 - 2026-10-09

### Accessibility

- Added stable, frame-scoped `ElementId`/`FocusId` mapping with per-window focused-element queries, rerender cleanup, and duplicate-registration diagnostics.
- Added focused state to UI-kit accessibility bridge snapshots and wired core controls to the mapping for reliable automation assertions.

### Performance and platform

- Retained core text and glyph-layout storage across frames, and reduced platform text-system allocation and lock contention on macOS, Windows, and wgpu.
- Routed browser medium- and low-priority dispatch through `MessageChannel` and added an opt-in scheduling-baseline harness.

### Release Readiness

- Added a three-lane public release policy, contributor/security/support
  governance, explicit Rust 1.89 MSRV metadata, locked package verification,
  and CI release-contract enforcement.
- Selected `gpui-design`, `gpui-profiler`, and `gpui-ui-kit-macros` for the
  first crates.io wave. Deferred `gpui-pretext` and `gpui-builder` until their
  registry predecessors are available; GPUI-dependent crates remain source
  beta.
- Added deterministic Metal component capture for a 200-case PR profile,
  eight-shard nightly coverage of all 1,922 registered cases, versioned
  baselines, strict pixel validation, and 17 gallery contact sheets.
- Raised `gpui-python-runtime` line coverage above its 90% release floor with
  protocol, patch, validation, form, chart, and Scene3D regression tests.

### Dependency and Safety Policy

- Removed yanked `spin` 0.9/0.10 and future-incompatible
  `proc-macro-error2`, made yanked crates a hard cargo-deny failure, recorded
  missing vendored Apache-2.0 metadata, and removed stale policy allowances.
- Replaced the optional camera-backed QR debug path with a renderer-only QR
  example so native capture dependencies and permissions remain host-app
  concerns.
- Enforced safe Rust across portable first-party crates and documented the
  remaining reviewed native FFI boundaries.

### Tooling

- Added the unified `gpui-toolkit` CLI (`gpui-toolkit-cli`): component
  catalog queries with batch reads, page/block/theme templates, theme
  token export, `doctor` health checks, `upgrade` migration notes plus
  deprecated-pattern detection, `toolkit.toml` project configuration,
  and a self-describing JSON manifest. Every command honors `--json`
  with typed envelopes and stable `ERR_*` codes.
- Added `gpui-layout-expr`: a compact layout-expression language
  (`V > (Tx"Hi" + B.primary"Save")`) with parser, registry validation,
  project aliases, CLI check/expand, and the compile-time `layout!`
  proc macro in `gpui-ui-kit-macros`.
- Demo gallery: the contact-sheet index is replaced by one featured
  example rendered across every theme and design style, and each
  gallery card links to a seeded random theme/style demo variant via
  the new `?style=` web query.

### Components

- Added 30 UI-kit components: AppShell/TopNav/MobileNav shell family,
  Calendar, DateRangeInput/TimeInput/DateTimeInput, Chat, Markdown/
  Blockquote, FileInput, Pagination, Skeleton, Field/FieldStatus,
  Tokenizer, Lightbox, Carousel, HoverCard, Grid/Center/AspectRatio
  layout primitives, Layer, VisuallyHidden, SelectableCard,
  MetadataList, Citation, Timestamp, Resizable, and List. Each ships
  with theme coverage, behavior-matrix rows, i18n labels, lab stories,
  showcase sections, and Python bindings. Banner, BottomSheet,
  AvatarGroup, StatusDot, and CodeBlock were already covered by
  existing equivalents (Alert, SwipePanel, Avatar, BadgeDot,
  Code::block).
- Extended `DesignSystem` support to Alert/Toast/ButtonSet/Badge/
  Progress/Tabs/Card and to the audio VolumeKnob/LevelMeter/Spectrum
  family; the component-lab design selector (now including macOS,
  Adwaita, Breeze, and Carbon presets) drives every design-aware
  component, including per-cell designs in matrix mode.

## 0.9.7 - 2026-08-07

### Python Surface

- Completed the typed Python surface registry for all 20 first-party consumer crates, with 51 capability dispositions and a green strict parity gate.
- Added native Rust-backed Python commands for builder solving, design/pretext reports, themes, UI conformance, audio controls, scaffolding, and the full current `gpui-d3rs` algorithm and interaction surface.
- Added typed Python declarations and retained GPUI rendering for charts, accessibility, events/effects, resources, platform services, tooling, audio streams, and 3D scenes.

### Charts and Interaction

- Added the retained mesh-plot foundation: validated unstructured triangle
  meshes, scalar fill/contour/isoline modes, BVH picking, deterministic SVG
  export and accessibility summaries, Python mesh frames/patches, and a
  component-lab story. Added explicit macOS Metal backend selection for the
  retained renderer and Python showcase feature path. GPU adapter and
  reference-machine QA remain tracked as partial capability evidence until the
  full release gate is run.

- Fixed Python chart wheel zoom and drag interaction by retaining painted bounds, converting window coordinates to chart-local coordinates, and invalidating the host view when interaction state changes.
- Added native line curves, dash styles, grouped bars, annotations, per-series opacity, secondary axes, nearest-point inspection, legend toggling, brushing, keyboard navigation, and SVG/PNG/CSV export.

### Audio and Tooling

- Added semantic preview/commit behavior to potentiometers, vertical sliders, and volume knobs, including native delta dragging.
- Added non-mutating scaffold previews with the exact generated-file inventory.

### Performance and Capability

- Added retained frame-rate layout solving, windowed TreeView virtualization,
  shared D3 timer scheduling with optional UI-thread dispatch, and direct mesh
  rendering in the Python GPUI adapter.
- Bounded GPU 3D surface CPU readback by pixel area before creating cached
  `RenderImage` values.
- Reused GPU resolve and readback buffers for 3D surface frames, and added
  bounded QR, CSV/DSV, TopoJSON, and pretext preparation entry points with
  cooperative cancellation where the parser/layout phase can observe it.

### Known Limitations

- Full D3.js semantic parity beyond the current Rust API remains partial; pretext bidi/shaping is
  still backend-dependent; the GPUI surface bridge remains CPU-backed, and
  the Python mesh path is CPU-projected GPUI polygons rather than a true GPU
  mesh pipeline. Output formatting plus arbitrary JSON APIs do not yet share
  the bounded resource contract.

### Vendored Dependencies

- Vendored the GPUI crate closure from zed-industries/zed at tag v1.9.0 into
  crates/3rdparties as history-free snapshots (16 crates: `gpui`,
  `gpui_macros`, `gpui_macos`, `gpui_linux`, `collections`, `util`,
  `gpui_shared_string`, `gpui_util`, `util_macros`, `refineable`,
  `derive_refineable`, `scheduler`, `sum_tree`, `http_client`, `media`,
  `perf`), wired via [patch] so no dependency resolves from zed.git.
  Dependency graph: 827 -> 822 packages. Import:
  scripts/import_gpui_upstream.py. Platform evidence: macOS gate green
  (just qa); iOS pass; tvOS pass; Android pass; Linux/Windows deferred to
  platform CI ("Targets that are not available on the current host are
  tracked in `gpui_toolkit::release_qa_matrix()` and proved by platform CI
  or an attached manual/device report" — qa.md).
- 15 crates are script-vendored snapshots; `gpui_macos` was re-vendored
  pristine with a recorded CGS-symbol-removal patch (Mac App Store
  static-analysis rejection risk), and `gpui_wgpu`/`gpui_windows` remain
  hand-maintained.
- Dropped the GPL-3.0 zed crates `zlog`, `ztracing`, and `ztracing_macro`
  from the vendor set; `sum_tree` carries a small recorded patch
  (`ztracing::instrument` -> `tracing::instrument`).
- Import exclusions: `examples/`, `benches/`, and dev-deps on
  `reqwest_client`/`gpui_platform`/`gpui_web`.
- The import is re-runnable (`--skip`, `--check` drift report) via
  `scripts/import_gpui_upstream.py`, and `scripts/qa_zed_source_check.py` is
  wired into `just qa-deps` as a permanent source-origin gate.
- qa-perf remains environmentally blocked on this host: the main rev fails
  its own baseline (A/B evidence in `target/qa/perf/report-main-ab.md`,
  26/57 benchmarks regressed up to +43%), so the baseline is untouched and
  the block was accepted by the owner.
- Known landmine for Linux/Windows builds: `calloop` and `windows-capture`
  resolve from crates.io while zed patches git forks.

## 0.8.5 - 2026-07-09

### Release QA

- Added aggregate release-readiness metadata in `gpui-toolkit`, including
  stability, publish-plan, release-note, release-packaging, dependency-hygiene,
  release-QA, and vendored-patch reports.
- Added a structured dependency-hygiene policy with `deny.toml`, RustSec
  advisory triage, quick-xml risk acceptance for the current internal snapshot,
  and explicit remaining `cargo-deny` release gates.
- Documented crate-by-crate QA status, missing SOTA features, platform gates,
  and internal/public release posture in `docs/qa-20260707.md`.

### Platform and UX

- Added or tightened platform QA artifacts for AU, iOS, Android, tvOS,
  Windows, showcase, scaffolding, visual-regression manifests, and accessibility
  bridge readiness.
- Expanded UI-kit keyboard, accessibility, virtualization, visual-regression,
  and security-surface metadata for release review.

### Visualization and Tooling

- Expanded `gpui-d3rs` checked APIs, D3 parity reports, benchmark coverage, and
  renderer-independent layout helpers.
- Expanded `gpui-px` chart capability, accessibility, interaction,
  visual-regression, annotation, legend, and static SVG export surfaces.
- Added release artifacts for design docs, design tooling handoff, component
  lab visual diffs, Python packaging/schema metadata, and audio-control
  accessibility/automation/visual reports.

### Vendored Dependencies

- Documented active vendored patches and retained changes for Zed platform
  backends, `objc`, `block`, and `zed-font-kit`.
- Patched `block` locally to resolve the current future-incompatibility report
  and cleaned modern Rust warning debt in active Objective-C/font vendored
  dependencies.
