# Changelog

## Unreleased

## 0.9.7 - 2026-08-07

### Python Surface

- Completed the typed Python surface registry for all 20 first-party consumer crates, with 51 capability dispositions and a green strict parity gate.
- Added native Rust-backed Python commands for builder solving, design/pretext reports, themes, UI conformance, audio controls, scaffolding, and the full current `gpui-d3rs` algorithm and interaction surface.
- Added typed Python declarations and retained GPUI rendering for charts, accessibility, events/effects, resources, platform services, tooling, audio streams, and 3D scenes.

### Charts and Interaction

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

- D3 interaction/animation/GPU parity remains partial; pretext bidi/shaping is
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
