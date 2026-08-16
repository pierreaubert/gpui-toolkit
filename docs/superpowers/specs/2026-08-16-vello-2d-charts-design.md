# Vello Backend for d3rs/px 2D Charts — Design

Date: 2026-08-16
Status: Approved (design), pending implementation plan

## Goal

Render all 2D charts in `gpui-d3rs` and `gpui-px` through [vello](https://github.com/linebender/vello)
(GPU compute-centric 2D renderer), replacing the CPU `paint_path`/`paint_quad`
rasterization and superseding the `gpu-2d` readback path, with zero GPU→CPU
round-trips on supported backends.

## Background and evidence

- Today, classic 2D charts build `gpui::Path` geometry in `gpui::canvas()`
  callbacks (`gpui-px/src/line/line_chart.rs`, `scatter/scatter_chart.rs`,
  `bar/bar_chart.rs`, `area.rs`, `pie.rs`, `boxplot/box_plot_chart.rs`,
  `treemap.rs`; d3rs surfaces via `surface/render/surface_element.rs`).
- The `gpu-2d` feature renders to a wgpu texture and reads pixels back every
  frame (`gpui-d3rs/src/gpu2d/renderer/chart2_drenderer.rs:802`), then paints
  via `window.paint_image` — a full GPU→CPU→GPU round-trip per frame.
- The vendored GPUI fork already has a zero-copy custom-draw seam, built for
  MeshPlot: `crates/3rdparties/gpui/src/custom_draw.rs` (registry),
  `gpui::Window::paint_custom` (`window.rs:4155`), and
  `crates/3rdparties/gpui_wgpu/src/custom.rs` (`WgpuCustomDraw` trait:
  `draw_wgpu(&self, ctx: &WgpuContext, encoder, target, target_size, bounds,
  scale_factor)`). `WgpuContext` (`gpui_wgpu/src/wgpu_context.rs:9`) publicly
  exposes the shared `instance`/`adapter`/`Arc<Device>`/`Arc<Queue>`.
  MeshPlot proves the seam (`gpui-d3rs/src/mesh/gpu/wgpu_backend.rs:285`).
- The workspace pins wgpu from Zed's git fork (`Cargo.toml:126`, rev
  `357a0c56e0070480ad9daea5d2eaa83150b79e88`), which reports version **29.0.3**
  in `Cargo.lock`.
- vello **0.10.0** (crates.io, 2026-08-14) pins crates-io `wgpu = "29.0.3"` and
  `naga = "29.0.3"`.
- Verified fork delta: Zed's fork is upstream v29.0.3 plus exactly one additive
  internal patch ("Add XCB display handle support to EGL backend",
  `wgpu-hal/src/gles/egl.rs`). No public API changes. Merge base is the v29.0.3
  tag itself.
  (https://github.com/gfx-rs/wgpu/compare/v29.0.3...zed-industries:wgpu:357a0c56)

## Architecture

Three layers, cleanly separated. Only the scene layer is visible to chart code.

### 1. Scene layer — `gpui-d3rs/src/vello2d/scene.rs`

`ChartSceneBuilder`: chart primitives (polyline, marker, rect, polygon,
gradient fill) are emitted into a `vello::Scene` as kurbo geometry + peniko
brushes. No GPU types in this layer — it compiles and unit-tests without wgpu.

### 2. Element layer — `gpui-d3rs/src/vello2d/element.rs`

`VelloChartElement`, a custom `gpui::Element` modeled on `Chart2DElement`
(`gpu2d/element.rs:147`) and MeshPlot's element. In `paint()` it registers its
scene with the custom-draw registry and calls `window.paint_custom(id, bounds)`.

### 3. Backend layer — two implementations, runtime-selected

- `WgpuVelloDraw` (`vello2d/wgpu_draw.rs`): implements `WgpuCustomDraw`; holds a
  `vello::Renderer` created once on the shared `WgpuContext` device/queue.
  Per frame: render the scene to an offscreen texture (transparent base, sized
  `bounds × scale_factor`), then composite into the frame target with an
  alpha-blend blit. Offscreen-then-blit because the GPUI target view lacks
  `STORAGE_BINDING`, which vello's fine pass requires; one blit is cheap and
  respects the bounds/scissor contract documented in `custom.rs`.
- `CpuVelloDraw` (`vello2d/cpu_draw.rs`): `vello_cpu` sparse-strips rasterizer
  into a pixmap → `paint_image`. Used on the Metal renderer (`gpui_macos`) and
  anywhere the wgpu hook is absent. Also the golden-image oracle for QA.

Selection: `VelloBackend::Auto | Wgpu | Cpu`, default `Auto` — probe whether the
wgpu custom-draw dispatch is available, else CPU. No `cfg!` guessing: wgpu also
runs on macOS (gpui-au) and on wasm (gpui_web → `WgpuRenderer`).

## Dependency unification (step 0)

vello 0.10.0 requires crates-io wgpu 29.0.3; the workspace uses Zed's fork,
which is version-identical (29.0.3) and API-identical (delta verified above).

- Add `[patch.crates-io]` entries for `wgpu`, `wgpu-core`, `wgpu-hal`,
  `wgpu-types`, and `naga` pointing at the Zed fork rev `357a0c56`. This unifies
  all wgpu types to one crate instance, so vello can render using GPUI's
  `Device`/`Queue`/`TextureView` directly.
- Validation: `cargo check -p gpui-d3rs --features vello` compiles, and a
  trivial vello scene renders through `WgpuCustomDraw` in a demo.

**Fallback if unification fails** (e.g. MSRV/edition conflict — vello 0.10
needs Rust 1.88, edition 2024): vendor `vello`, `vello_encoding`, and
`vello_shaders` (~10k lines total) into `crates/3rdparties/` with their wgpu
dep redirected at the fork — the same vendoring pattern already used for GPUI.
The zero-copy architecture is unchanged by this fallback.

## Chart migration path

Feature-gated and incremental; no flag-day rewrite.

- `gpui-d3rs` gains a `vello` feature (optional `vello` + `vello_cpu` deps).
- Each chart type gets an internal raster-backend toggle, defaulting to today's
  `paint_path`; vello is enabled per chart as it is ported.
- Port order: scatter first (biggest win, simplest geometry), then line, area,
  bar, then boxplot/treemap/pie.
- `gpui-px` inherits the toggle through its d3rs dependency; px public APIs are
  unchanged.
- The old `gpu-2d` readback path stays for one release for A/B perf comparison,
  then is deprecated (vello supersedes it).

## Error handling

- `vello::Renderer::new` failure or device loss → log once, demote that element
  to the CPU backend for the session. Never panic in `paint()`.
- Per-frame render error → log, skip the frame, degrade gracefully.
- Oversized scenes: vello's `bump_estimate` feature in debug builds warns when a
  scene approaches vello's buffer limits; d3rs's existing downsampling knobs
  remain the first line of defense for very large series.
- wasm: vello supports WebGPU, but the vendored renderer has known
  `cfg(not(wasm))` gaps (see
  `docs/superpowers/specs/2026-08-15-wasm-browser-target-design.md`). Custom-draw
  dispatch on wasm is verified during step 0; the CPU backend is the escape
  hatch.

## Testing

- **Golden images**: `vello_cpu` as deterministic oracle — render each ported
  chart's scene via GPU and CPU, pixel-diff with tolerance, wired into
  `qa/visual`.
- **wasm**: extend `just wasm-visual` baselines with a vello-backed chart
  section.
- **Perf**: `qa/perf` benchmark — 100k/1M-point scatter, `paint_path` vs
  vello-GPU vs vello-CPU; frame-time table committed with results.
- **Feature matrix**: CI checks `gpui-d3rs` with and without `vello`, plus
  `just wasm-check`.

## Explicitly out of scope (YAGNI)

- 3D/surface/MeshPlot paths (already GPU-native).
- vello text rendering (GPUI's text system stays).
- Changes to `gpui-ui-kit` or other crates.
- The `gpu-2d` readback-based vello variant (Option B) — dominated by the CPU
  fallback, not built.
