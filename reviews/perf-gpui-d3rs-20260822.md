# Perf review: gpui-d3rs

Date: 2026-08-22

## Role and hot paths

`gpui-d3rs` (lib `d3rs`) is the D3-inspired charting layer: scales/axes/shapes
(pure CPU compute), plus three GPU painting stacks and one compute stack:

- `gpu2d` (legacy): `Chart2DElement` → `Chart2DRenderer` renders offscreen with
  wgpu, reads pixels back, re-uploads via `window.paint_image` — per paint.
  Used by `gpu2d/shapes/render.rs` (8 call sites), `gpu2d/shapes/lod_scatter.rs:145`,
  the showcase force demo, and gpui-px `mesh_plot_chart.rs:1335`.
- `vello2d` (current): `VelloChartElement`/`VelloScenePainter` → zero-copy
  `WgpuCustomDraw` (vello → offscreen storage texture → composite pipeline into
  the GPUI frame) or `CpuRasterizer` (vello_cpu) fallback → `paint_image`.
- `mesh/gpu`: retained wgpu/Metal custom draws (revision-cached, good), plus
  `MeshCompute` wgpu compute (field reductions, marching triangles) and CPU BVH
  picking (`mesh/bvh.rs`).
- `gpu3d`/`sphere_gallery`: offscreen surface renderers with readback→paint_image.
- `text/glyph_text`: chart labels painted as per-label SVG strings via
  `window.paint_svg` (axis ticks, surface labels, spinorama legends).

Existing perf infra: `benches/` (vello2d scene build + CPU raster, mesh_prep,
path_strings, force, large datasets), `qa/perf/2026-08-17-vello2d-bench.md`
(100k-point scene build 56 ms, CPU raster 516 ms), revision counters in
`mesh/gpu/retained.rs`, `profiler` feature flag (Cargo.toml:20). No allocation-
count tests found.

## Findings

1. **[Roundtrip] gpu2d `Chart2DElement` paints via render→readback→re-upload every
   frame.** Native paint (`gpu2d/element.rs:300-331`) calls `end_frame()`, which
   does `device.poll(PollType::Wait, 5s)` + `recv_timeout` (`renderer/chart2_drenderer.rs:880-885`),
   copies rows out of a mapped staging buffer, then `paint_image` re-uploads the
   same pixels to GPUI's atlas. For an 800×600 chart that is ~1.9 MB down + up
   per paint with a full GPU stall on the UI thread. The module documents the
   pattern itself (`gpu2d/mod.rs:12`). This is the canonical anti-pattern the
   campaign targets; wasm defers the readback but keeps the same data flow
   (`element.rs:258-298`).

2. **[Alloc] gpu2d re-creates every GPU buffer per frame.** `submit_frame`
   calls `create_buffer_init` for each non-empty batch (10 buffers/frame worst
   case: `chart2_drenderer.rs:708-721, 732-745, 756-769, 781-794, 807-820`) plus
   a fresh staging buffer (`:833-838`). No buffer reuse or `write_buffer` into a
   retained buffer. The CPU-side batch Vecs do retain capacity (`primitives/line.rs:61`),
   so the churn is purely on the GPU-resource side.

3. **[Alloc] A new `Chart2DRenderer` = 5 pipelines + font atlas, and elements
   are rebuilt per render.** `Chart2DRenderer::try_new` compiles 5 shader
   modules/pipelines and a 1024² `TextAtlas` (`chart2_drenderer.rs:117-126, 123`).
   Callers construct `Chart2DElement::new(...)` inside `render()` (showcase
   `bin/showcase/showcase_modules/force.rs:78`; gpui-px `mesh_plot_chart.rs:1335`),
   and GPUI rebuilds elements on every re-render — so on native, every repaint
   pays full pipeline compilation per chart. `with_renderer` (`element.rs:93`)
   exists for sharing but no in-tree caller uses it. The device itself is shared
   (`gpu2d/device.rs:24-47`); pipelines/atlas are not.

4. **[Alloc] vello2d: per-element vello `Renderer`, offscreen texture, and
   composite pipeline — no per-device sharing.** Each `WgpuVelloDraw` lazily
   creates its own `vello::Renderer` (`vello2d/wgpu_draw.rs:145-167`), its own
   `Rgba8Unorm` storage texture (`:171-188`), and its own WGSL
   `CompositePipeline` (shader module + pipeline + bind group layout,
   `:189-191, 283-369`). A gallery of N charts = N vello renderers and N
   compiled composite pipelines on the same device. Renderer and pipeline could
   be per-`WgpuContext` shared; only the texture is genuinely per-element.

5. **[Alloc] vello2d re-encodes the vello scene every draw and clones the
   `ChartScene` on every revision change.** `to_vello_scene` rebuilds a fresh
   `vello::Scene` per `draw_wgpu` call (`wgpu_draw.rs:140`, `gpu_scene.rs:10-27`)
   — vello encoding allocates per path. The element clones the whole
   `ChartScene` (Vec of `BezPath` commands) into `SharedScene` whenever the
   revision differs (`vello2d/element.rs:181-184, 520-523`). Mitigated by the
   revision check, but a live data source (audio meters via `VelloScenePainter`)
   bumps the revision every frame, so both costs are per-frame there. The
   comment at `gpu_scene.rs:8-9` calls encoding "cheap" — (needs profiling)
   against the 100k-point scene_build bench (56 ms, qa/perf note above).

6. **[Roundtrip] vello2d CPU fallback: full raster→convert→swap→upload chain,
   well cached but heavy on miss.** `CpuRasterizer::rasterize` renders then
   expands the pixmap via per-pixel `flat_map(...).collect()` into a fresh
   `Vec<u8>` (`vello2d/cpu.rs:53-59`); the element then scans the whole buffer
   for all-zero (`element.rs:212, 544`), swaps R↔B per pixel (`:216-218`,
   `:554-556`), and uploads via `paint_image`. That is three extra full-buffer
   CPU passes plus an allocation per rasterization. The revision+size cache
   (`element.rs:195-206, 531-542`) and `drop_image` discipline make steady-state
   cheap; misses (resize, live data) pay the full chain. vello_cpu's
   `Pixmap.data()` is already RGBA-premultiplied `PremulRgba8` — the collect and
   possibly the swap are avoidable with a direct cast (needs API check).

7. **[GPU] `MeshCompute` creates its own wgpu instance/adapter/device and 4
   compute pipelines per construction — and the production caller constructs it
   per contour computation.** `AdapterCompute::try_new` (`mesh/gpu/compute.rs:206-300`)
   does `pollster::block_on` adapter+device creation (third device in the
   process, after `Gpu2DContext` and gpui_wgpu's). gpui-px calls
   `MeshCompute::try_new()` inside `contour_geometry_with_compute`
   (gpui-px `mesh_plot_chart.rs:3315`) with no caching — device creation is
   tens to hundreds of ms, paid per recompute (needs profiling for exact cost).
   There is no shared/global `MeshCompute`.

8. **[Roundtrip] mesh compute ops: fresh buffers + blocking readback per call.**
   Each op allocates 5-8 buffers via a `create_buffer` closure
   (`compute.rs:513-563, 784-836`) and blocks on `map_async` +
   `device.poll(Wait)` + `recv_timeout(5s)` (`compute.rs:378-391, 644-657,
   908-921`). Results cross gpu→cpu only to be turned into `ChartScene`/
   geometry that goes right back to the GPU for painting — a full roundtrip
   per contour/field update. Also a hard wasm hazard (documented in AGENTS.md).

9. **[Roundtrip] gpu3d / sphere_gallery: own device + synchronous readback per
   rendered frame.** `Surface3DRenderer::new` creates a private device via
   `pollster::block_on` (`gpu3d/renderer/surface3_drenderer.rs:43`); readback is
   `map_async` + `poll(Wait)` (`:639-650`). `Surface3DElement` caches by
   bounds/camera key (`gpu3d/element/surface3_delement.rs:582-601`), so static
   frames are cheap — but camera drag invalidates the key every frame, i.e. a
   blocking readback per drag frame. `SphereGalleryRenderer` has the same own-
   device + sync readback (`sphere_gallery/renderer/sphere_gallery_renderer.rs:35,
   506-517`) and its element re-renders every paint with no cache
   (`sphere_gallery/element.rs:270-290`).

10. **[Alloc] glyph_text builds an SVG string per label per render.** Axis ticks
    and surface labels go through `chart_text_layout` → `format!` SVG document
    (`text/glyph_text/chart.rs:117-128`) plus a `DefaultHasher` cache key
    (`:139-148`); `render_glyph_text` clones the SVG into the canvas closure
    (`text/glyph_text/render.rs:28-29`). `paint_svg` presumably caches
    rasterization by key, but the string build runs per tick per render —
    a 13-tick axis = ~15 SVG strings per view rebuild (axis/render.rs:131-518).
    `paint_glyph_text_at` additionally clones the rasterized pixel buffer per
    paint (`text/glyph_text/paint.rs:75`).

11. **[Alloc] spinorama demo recomputes contours and clones grid data on every
    view render.** `render_contour_2d.rs` re-runs `ContourGenerator` marching
    over the full freq×angle grid and `.clone()`s `spl`, `angles`, and
    `log_freq_values` (`bin/spinorama_demo/app/render_contour_2d.rs:67-102`)
    inside the render method — any GPUI invalidation (hover, resize of an
    unrelated pane) re-does the whole compute.

12. **[Roundtrip] `MeshSceneElement` fallback renders offscreen per paint.**
    Without a registered custom draw, `paint` calls `render_offscreen` and
    `paint_image` every frame (`mesh/gpu/element.rs:100-107`). The CPU fallback
    is a per-frame full rasterization with no revision check (the retained
    revisions in `MeshSceneState` are not consulted on this path).

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | Port remaining gpu2d consumers (px mesh_plot fallback at `mesh_plot_chart.rs:1335`, `gpu2d/shapes/render.rs`, force demo) to vello2d `WgpuCustomDraw`, then deprecate `Chart2DElement` | 1, 2, 3 | M-L | Removes the worst roundtrip + per-frame buffer churn + per-render pipeline compiles |
| 2 | Share one `vello::Renderer` + composite pipeline per `WgpuContext` (registry keyed by device), keep only the offscreen texture per element | 4 | M | N-chart galleries: N→1 renderer/pipeline |
| 3 | Cache `MeshCompute` (global `OnceLock` or reuse `Gpu2DContext`'s device); make px reuse it | 7 | S | Removes per-recompute device creation |
| 4 | Reuse vertex/index/staging buffers across frames in `Chart2DRenderer` if gpu2d must live on | 2 | S | Big alloc win for legacy path until #1 lands |
| 5 | Replace `paint_glyph_text_at`/`paint_chart_text_at` SVG strings with GPUI text or pre-shaped cached labels; avoid per-tick `format!` | 10 | M | Cuts per-render string churn on every axis |
| 6 | vello2d CPU: drop the per-pixel collect (cast pixmap data directly), skip the all-zero scan or fold it into the swap | 6 | S | One alloc + two O(pixel) passes per raster miss |
| 7 | gpu3d/sphere_gallery: reuse the shared device; add camera-drag caching to sphere gallery like surface3d's key cache | 9 | M | Smooth interaction; one less device |
| 8 | Consult `geometry_rev`/`field_rev` in `MeshSceneElement` offscreen fallback to skip unchanged repaints | 12 | S | Steady-state fallback becomes free |
| 9 | Spinorama: memoize contour computation on (data, mode, size) instead of recomputing in `render()` | 11 | S | Demo interactivity |
| 10 | Add allocation-count tests (gpui-profiler) for: one vello2d repaint, one axis render, one mesh compute op | all | S | Regression guard for the campaign |

## Quick wins

- Cache `MeshCompute` behind a `OnceLock` and have gpui-px reuse it (Finding 7) — S.
- Reuse staging/vertex/index buffers in `Chart2DRenderer::submit_frame` (Finding 2) — S.
- Remove the `pixels.iter().all(|&b| b == 0)` full scan and the `flat_map().collect()` in the vello2d CPU path (Finding 6) — S.
- Gate `MeshSceneElement`'s offscreen fallback on revision change (Finding 12) — S.
- Stop re-rasterizing glyph text pixels per paint in `paint_glyph_text_at`; cache `RasterText` by (text, config) like the SVG path's cache key (Finding 10) — S.
