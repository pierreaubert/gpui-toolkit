# Perf review: gpui-px

Date: 2026-08-22

## Role and hot paths

`gpui-px` is the Plotly-Express-style charting layer over `gpui-d3rs`. Two distinct
hot-path shapes:

- **Builder charts** (scatter/line/area/bar/pie/treemap/boxplot/heatmap/contour/isoline):
  `build()` validates all arrays, computes scales/domains, flattens paths, renders axis
  tick labels via `render_glyph_text`, and constructs elements (`VelloChartElement` or
  GPUI `canvas`). In immediate-mode GPUI usage the whole `build()` re-runs whenever the
  owning view re-renders (see the showcase: `bin/showcase/showcase_app.rs:430,592` —
  `chart_builder.build()` inside `render_*_demo`).
- **`MeshPlot`** (`src/mesh_plot/mesh_plot_chart.rs`): retained live element;
  `build_frame()` (line 604) re-runs on every `cx.notify()` / `window.refresh()` from
  hover, pan, zoom, brush, click handlers (lines 1586, 1604, 1699, 1704, 1719, 2058,
  2078, …). GPU uploads are revision-gated, but a large block of CPU prelims is not.

Existing perf assets: criterion benches `benches/streaming_prepare.rs` and
`benches/mesh_plot_frames.rs` (registered in `Cargo.toml:65-71`), results in
`qa/perf/baseline.json` (gpui-px records), a documented zero-allocation streaming
contract (`README.md:431-434`), and `gpui-profiler` in dev-deps (`Cargo.toml:62`).
No allocation-count tests outside the streaming contract; no per-frame timing tests
for `build_frame`.

## Findings

1. **[Alloc] MeshPlot `build_frame` redoes O(N+T) CPU work per interaction frame.**
   Every notify runs `self.mesh.validate()` (full finiteness/range/zero-area scan of
   positions and triangles, `mesh_plot_chart.rs:613` + `gpui-d3rs/src/mesh/model.rs:97-149`),
   a fresh `projected: Vec<[f64;2]>` (`mesh_plot_chart.rs:620-626`), `MeshTopology::build`
   (`mesh_plot_chart.rs:638`), and `accessibility_summary()` with `format!` strings
   (`mesh_plot_chart.rs:614`). Additionally `format!("{mode:?}")` allocates a Debug string
   per frame (`mesh_plot_chart.rs:751`). Hover/scroll/drag each call `window.refresh()`,
   so this is per-pointer-event cost on meshes that may have 10⁵–10⁶ triangles.

2. **[Alloc] Retained 2D scene state is unconditionally rebuilt, not updated.**
   `build_retained_scene_state` (`mesh_plot_chart.rs:2623-2732`) is called every frame
   (call site `:995`) and always rebuilds `MeshTopology::build` (`:2643` — the *second*
   topology build in the same frame), `prepare_upload` (`:2644`), a fresh
   `positions_f32` Vec (`:2660-2669`), and `prepare_field` (`:2671`), then overwrites
   `state.upload = Some(upload)` (`:2698`) even when nothing changed. Revision caching
   (`interaction.rs:572-630`) protects contours/BVH/grid but not this upload path.
   Defeats the campaign goal for the crate's flagship retained renderer.

3. **[Roundtrip] Contour/isoline mesh modes fall back to `gpu2d::Chart2DElement` per frame.**
   `mesh_plot_chart.rs:1334-1448`: for any mode other than `Mesh`/`ScalarFill`, a new
   `Chart2DElement` is constructed eagerly inside `catch_unwind` on every `build_frame`,
   re-projecting every triangle/band/isoline CPU-side into draw calls
   (`:1357-1439`), then rendering offscreen → `map_async` readback → `paint_image`
   (`gpui-d3rs/src/gpu2d/mod.rs:12`, `gpu2d/element.rs:266,324`; device init uses
   `pollster::block_on`, `gpu2d/device.rs:36` — also a wasm hazard). This is the
   canonical roundtrip anti-pattern flagged in `reviews/20260822-vello.md`, re-created
   per interaction frame instead of once.

4. **[Roundtrip] `MeshSceneElement` without a registered custom draw does a blocking
   offscreen readback every paint.** When `mesh_custom_draw_supported(OS)` is false
   (`mesh_plot_chart.rs:1299,1316`), `paint` calls `render_offscreen` per frame
   (`gpui-d3rs/src/mesh/gpu/element.rs:100-107`), which does `map_async` +
   `device.poll(PollType::Wait)` (`gpui-d3rs/src/mesh/gpu/offscreen.rs:409-412`) and
   re-uploads via `paint_image`. Synchronous GPU stall per frame on any platform
   lacking the custom-draw registration; would hang under wasm.

5. **[Alloc] Stateless `VelloChartElement` closures rebuild scenes whenever the chart
   element is reconstructed.** `VelloChartElement` carries `state: None,
   scene_size: None` at construction and only skips the builder when size is unchanged
   (`gpui-d3rs/src/vello2d/element.rs:277-330, 494-502`); `Drop` unregisters the custom
   draw (`element.rs:266-272`). All five px closures (`area.rs:311`, `pie.rs:317`,
   `bar/bar_chart.rs:1026`, `boxplot/box_plot_chart.rs:740`, `treemap.rs:530`) build
   fresh kurbo `BezPath`s per invocation (`area.rs:478-499`, `pie.rs:403-430`). Under
   immediate-mode rebuild-per-render (showcase pattern), each frame means: new closure
   scene (Vec allocs), renderer re-resolve, custom-draw register/unregister churn, and
   `shared.scene = scene.clone()` on revision change (`element.rs:520-522`). (Extent of
   per-frame rebuild in real apps depends on how often parents re-render — needs
   profiling with `gpui-profiler`.)

6. **[Alloc] Axis/title glyph SVG is regenerated on every `build()` / `build_frame`.**
   `render_glyph_text` runs `chart_text_layout` (builds an SVG `String` per label,
   `gpui-d3rs/src/text/glyph_text/render.rs:23-29`, `chart.rs:14-60`) for the title and
   every axis tick (`gpui-d3rs/src/axis/render.rs:94-139` — one glyph element per tick,
   ~2 axes × 10–20 ticks). During pan/zoom labels legitimately change, but hover-only
   refreshes regenerate identical SVG strings for every tick. GPUI's `paint_svg` caches
   by `cache_key`, so GPU raster is cached; the churn is CPU string/layout work plus
   `svg.clone()` per canvas closure. Log ticks themselves are already cached
   (`line/misc.rs:72-92`) — the layout step is not.

7. **[Alloc] Per-pick allocations on the hover path.** `pick_2d` allocates
   `Arc::from(plot_id)` and clones ids on every successful pick
   (`mesh_plot/picking.rs:102-110`), and `pick_at` clones the pick twice
   (`mesh_plot/interaction.rs:1021-1023`). Called from `on_mouse_move`
   (`mesh_plot_chart.rs:1607-1616`) — up to per-pointer-event. The spatial index itself
   is properly retained (`interaction.rs:735-767`), so this is small but pure churn.

8. **[GPU] Static builders do all scale/flatten work on CPU at build time with no GPU
   path for large series.** Line/scatter map points via `cached_line_points`
   (`line/line_chart.rs:1379,1404`) and flatten curves to polylines CPU-side
   (`area.rs:284-303`); 1M-point series are then drawn as flattened path segments
   through Vello/GPUI. This is acceptable at interactive sizes but there is no
   LOD/decimation or GPU-side scale transform for large series (needs profiling to
   quantify; `streaming_prepare` bench covers only the prepare step, not render).

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | Gate `build_frame` prelims on revisions: cache `validate()` result, `projected`, and topology by `(geometry_revision, positions ptr)` the same way contours/BVH are cached (`interaction.rs:735-787` pattern) | 1, 2 | M | Removes O(N+T) per pointer event on mesh plots |
| 2 | Make `build_retained_scene_state` skip `MeshTopology::build`/`prepare_upload`/`positions_f32`/`prepare_field` when revisions and Arc ptrs are unchanged; only update `view_transform`/`color` | 2 | S–M | Largest single alloc/CPU win in the crate |
| 3 | Port contour/isoline mesh modes off `gpu2d::Chart2DElement` onto the retained 2D GPU scene (the upload already carries bands/isoline params) or onto `vello2d` | 3 | L | Eliminates per-frame offscreen→readback→re-upload in contour modes; also fixes wasm hazard |
| 4 | In `MeshSceneElement`/`render_offscreen` fallback: cache the last rendered frame keyed by (revision, size, camera) instead of re-rendering + blocking poll every paint; document as native-only | 4 | M | Removes per-frame GPU stall on non-custom-draw platforms |
| 5 | Give `VelloChartElement` persistent scene/backend state across element rebuilds (keyed registry or GPUI element state) so identical charts don't rebuild scenes/re-register custom draws each render | 5 | M | Kills per-frame scene Vec churn and register/unregister cycles for all builder charts |
| 6 | Cache `chart_text_layout` by (text, config) key (hash exists as `cache_key` input); skip layout when key matches previous frame | 6 | S | Removes per-refresh SVG string churn for axes/titles |
| 7 | Return `Arc<str>`-free or pre-interned ids from `pick_2d` (borrow plot_id; store `Arc<str>` once in state) | 7 | S | Small; cheap to land with #1 |
| 8 | Extend `mesh_plot_frames` bench + `qa/perf/baseline.json` with a `build_frame`-after-hover benchmark and a Velio scene-rebuild counter so regressions in 1/2/5 are caught | all | S | Makes the campaign measurable for this crate |

## Quick wins

- #2 guard clause in `build_retained_scene_state` (revision + ptr equality → early
  return after updating `view_transform`/`color`).
- #6 memoize `chart_text_layout` (thread-local `HashMap` keyed by the existing cache
  hash, same style as `line/misc.rs:72-92`'s `LOG_TICK_CACHE`).
- #7 stop allocating `Arc::from(plot_id)` per pick.
- Cache `mesh.validate()` result behind the geometry revision (call once per geometry
  change instead of per frame).
- Drop the per-frame `format!("{mode:?}")` (`mesh_plot_chart.rs:751`) unless a toolbar
  frame is actually being built.
