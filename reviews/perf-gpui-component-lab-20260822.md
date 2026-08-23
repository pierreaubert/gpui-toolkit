# Perf review: gpui-component-lab

Date: 2026-08-22

## Role and hot paths

`gpui-component-lab` is an interactive storybook/designer app (lib: story/prop/registry
data model under `src/lib/`; UI: `src/lab_ui/`, driven by `src/bin/gpui_component_lab.rs`).
It renders prop-driven previews of `gpui-ui-kit`, `gpui-audio-kit` and `gpui-px` chart
stories, a responsive preview matrix (viewports × themes), a live-reload watcher for
`*.story.json` + design-token JSON, and an optional `visual-capture` headless screenshot
lane for CI. Hot paths:

- `Render for ComponentLab` (`src/lab_ui/component_lab.rs:3543`) — per-frame: child-entity
  state sync + alloc sampling.
- `LabPreviewArea` → `render_preview_area` / `render_single_preview` / `render_matrix`
  (`component_lab.rs:1490`, `1498`, `1575`) → `render_story_preview` (`1644`) → per-story
  builders, incl. chart data generation and `gpui-px` chart construction per render.
- Matrix mode multiplies every story build by `viewports × themes` cells
  (`lib/responsive_preview_matrix.rs:12-24`).
- Live-reload poll loop, 750 ms timer (`component_lab.rs:450-465`) →
  `reload_live_preview_state` (`src/lib/types.rs:50`) on the app context.
- Interaction handlers: `set_prop` (`component_lab.rs:575`), `sync_layout_state` (`666`),
  mouse/scroll alloc samplers (`3685-3716`).
- `visual-capture` screenshot lane (`src/lab_ui/visual_capture.rs:94-223`).

The crate already does several things right: persistent child entities with dirty-guards
(`component_lab.rs:3590-3675`), memoized data caches (`lab_ui/misc.rs:160-253`,
`lab_ui/types.rs:98-211`), a cached `DesignSystem` map (`misc.rs:344-363`), and an
integrated `gpui-profiler::AllocProbe` overlay (`component_lab.rs:3486-3540`) which is a
zero-sized no-op unless the `profiler` feature is on (`gpui-profiler/src/alloc_count.rs:105-160`).

## Findings

1. [Alloc] Mesh/story fixtures are rebuilt from scratch on every preview render.
   `render_mesh_plot_story` (`component_lab.rs:3002-3238`) calls `mesh_plot_large_mesh`
   (`225-261`: 128×128 grid → 16 641 positions + 32 768 triangles) and
   `mesh_plot_large_field` (`263-282`) fresh per render; every other `mesh_plot_*`
   fixture (`101-223`) likewise allocates new `Vec`s per call. In matrix mode each cell
   re-enters `render_story_preview` (`1582-1638`), so the large-mesh story builds
   ~12 copies per `LabPreviewArea` render. Each rebuild likely forces d3rs to re-derive
   topology/contours and re-upload GPU buffers (needs profiling — dedupe by mesh id may
   exist in `gpui-px`, not verified here). Impact: high for mesh stories.

2. [Roundtrip] Contour/isoline mesh stories use the legacy `gpu2d::Chart2DElement`
   offscreen-render→readback→`paint_image` path. `px.mesh_plot.filled_contours`,
   `.isolines`, `.combined` (`component_lab.rs:3053-3094`) select
   `MeshRenderMode::FilledContours/Isolines/FillAndIsolines`, which in `gpui-px`
   (`crates/gpui-px/src/mesh_plot/mesh_plot_chart.rs:1334-1448`) falls back to
   `d3rs::gpu2d::Chart2DElement::new` — the canonical roundtrip anti-pattern flagged in
   `reviews/20260822-vello.md`. Component-lab enables it transitively (`gpu-2d` is a
   default feature of `gpui-px`; `Cargo.toml:39` does not set `default-features = false`).
   In matrix mode this multiplies to one offscreen surface + readback per cell. Impact:
   high when those stories are visible; fix belongs in gpui-px/d3rs but component-lab is
   the crate that exercises it most.

3. [Alloc] Data caches lock a global `Mutex` and deep-clone the whole `Vec` per call.
   `scatter_story_data` (`misc.rs:160-168`), `scalar_field_data` (`184-192`),
   `boxplot_story_data` (`212-220`), `spectrum_magnitudes` (`231-239`),
   `line_story_data`/`area_story_data` (`types.rs:98-110`, `158-170`) and
   `bar_story_data` (`types.rs:203-211`) all return `.clone()`d vectors. These run once
   per chart-story render and once per matrix cell. Impact: low–medium (vectors are
   ≤ ~1k elements), but trivially fixable with `Arc<[f64]>`.

4. [Alloc] Per-frame work in `Render for ComponentLab` even when nothing changed:
   `live_status.clone()` (`3594`), `story.id.clone()` (`3628`, `3647`),
   `viewport.id.clone()` / `theme.id.clone()` / `motion.id.clone()` (`3648-3650`) for the
   dirty-guard comparisons; `record_sample("render")` (`3743`) does `label.to_string()`
   (`587`) per render, and the root `on_mouse_move`/`on_mouse_down`/`on_scroll` handlers
   (`3685-3716`) each do an `entity.update` + `record_sample` per event even with the
   profiler feature off (the sample is a no-op, but the `String` label alloc and entity
   update are not). Impact: low per frame, but it is unconditional churn in the hottest
   path; the string allocs are avoidable with `&'static str` labels / `Cow`.

5. [Alloc] `rebuild_derived_state` (`component_lab.rs:443-448`) rebuilds
   `sidebar_labels` for **all** stories — a `format!` + `String` clone per story
   (`build_sidebar_labels`, `425-441`) — on every story selection (`561-573`), reload
   (`699-712`) and live-reload apply (`490-513`), although labels only change when
   documents reload. Impact: low (per-click, not per-frame), easy to split.

6. [Alloc] `sync_layout_state` (`666-678`) serializes layout state to a `serde_json::json!`
   tree on every layout-control change (slider drags call `set_layout_*` continuously,
   `606-659`). Per drag-step JSON tree + `"Unsaved changes"` `SharedString`. Impact:
   low–medium during slider drags.

7. [GPU/startup] `build_ui_showcase_entities` (`component_lab.rs:3470-3482`, called at
   `355`) eagerly creates an `Entity<Showcase>` for every ui-kit showcase story (~40
   sections) at startup regardless of which story is selected. Visual-capture mode
   already limits this to one (`351-354`). Impact: startup latency + resident memory;
   lazy creation on first selection would remove most of it.

8. [Roundtrip] The `visual-capture` lane is an intentional gpu→cpu readback
   (`cx.capture_screenshot`, `visual_capture.rs:179`) plus PNG encode + FNV checksum over
   all pixels (`323-331`). This is inherent to screenshot CI and runs offline, one
   `HeadlessAppContext` shared across cases (`145`) — acceptable; only note: per-case
   window create/draw/release/remove (`166-194`) could reuse one window and swap the
   story to cut capture-suite wall time (needs profiling).

9. [Live reload] `poll_live_preview` (`component_lab.rs:456-488`) runs
   `reload_live_preview_state` synchronously on the app context every 750 ms: directory
   scan + `metadata()` per file (`src/lib/latest.rs:6-34`) and, on change, full JSON
   re-parse of every story document (`story_document.rs:45-63`) plus token validation
   (`lib/types.rs:60-70`). Small dirs make this cheap, but it is blocking I/O on the UI
   thread; moving the scan/parse into the spawned async block (only the `apply` needs
   `update`) removes jank risk on large story dirs.

10. [GPU] Dead feature flags: `gpu-2d = []` and `gpu-3d = []` (`Cargo.toml:15-16`) are
    empty and do not propagate to `gpui-px`/`gpui-d3rs`, while the hard dependency
    (`Cargo.toml:39`) always enables `gpui-px/gpu-3d` + `gpu-metal`. Misleading knobs;
    either wire them through (`gpui-px/gpu-2d` etc.) or delete.

No criterion benches or allocation-count tests exist for this crate (`lab_ui/tests.rs`
and `lib/tests.rs` are conformance-only); `qa/perf/` has no component-lab entry.

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | Memoize mesh/story fixtures (OnceLock per story id, or store built `TriangleMesh`/`ScalarField` in the story document) | 1 | M | High — removes 16k-vertex rebuild × cells per render |
| 2 | Route mesh contour/isoline modes off `gpu2d::Chart2DElement` (vello2d or retained `MeshSceneElement`); coordinate with gpui-px/d3rs | 2 | L | High — removes offscreen readback per visible cell |
| 3 | Return `Arc<[f64]>` / `&'static` from data caches instead of cloning | 3 | S | Medium |
| 4 | `&'static str` alloc-sample labels; skip `entity.update` when profiler feature is off (`#[cfg]` the handlers) | 4 | S | Low–medium, removes per-frame/per-event churn |
| 5 | Only rebuild `sidebar_labels` on document reload, not on story select | 5 | S | Low |
| 6 | Debounce `sync_layout_state` JSON serialization (write on release/save) | 6 | S | Low–medium during drags |
| 7 | Lazy-create `Showcase` entities on first selection | 7 | S | Medium at startup |
| 8 | Move live-reload file scan/parse off the app context into the async task | 9 | S | Robustness/jank |
| 9 | Wire or delete the empty `gpu-2d`/`gpu-3d` features | 10 | S | Hygiene |
| 10 | Add an alloc-count regression test (render + prop-change) using `AllocProbe` under the `profiler` feature | all | M | Prevents regressions; infra already exists |

## Quick wins

- Finding 3: swap cache clones for `Arc`/`&'static` (`misc.rs`, `lab_ui/types.rs`).
- Finding 4: `&'static str` labels + `cfg`-gate the mouse/scroll sampler handlers.
- Finding 5: move `build_sidebar_labels` out of `rebuild_derived_state`.
- Finding 10: delete or propagate the empty `gpu-2d`/`gpu-3d` feature stubs.
