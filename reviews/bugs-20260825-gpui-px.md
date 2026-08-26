# Bug Review: gpui-px — 2026-08-25

Scope: read-only audit of `crates/gpui-px` (~32k lines under `src/`): all chart builders (line, scatter, bar, area, boxplot, heatmap, contour, isoline, pie/donut, treemap, surface3d, mesh_plot), the interaction layer, static SVG export, color scales/colorbar, and the QA/metadata modules. Findings were verified by reading the code and cross-checking call sites; no code was changed and no tests were run (not required for a read-only review). GPU-heavy geometry/compute internals live in `gpui-d3rs` and are out of scope except where gpui-px drives them.

## Findings

### High

1. **Squarify treemap assigns rectangles to the wrong children (default tiling method)**
   `crates/gpui-px/src/treemap/tile.rs:148-152` sorts child indices by value descending and `tile_squarify` pushes rects in that sorted order (`tile.rs:195-223`), but `compute_treemap` zips the returned rects against `children` in declaration order (`crates/gpui-px/src/treemap/tiling_method.rs:102`). Unless children are already sorted descending by value, every rect is paired with the wrong node: labels, values, colors (`category_index` is derived from the declaration index at `tiling_method.rs:103`), and recursively-computed subtrees land on rectangles sized for a different child. `TilingMethod::Squarify` is the default (`treemap/types.rs:12-13`), and the existing tests only assert `build().is_ok()` or call `tile_squarify` directly (`treemap/tests.rs:66-97,189-203`), so nothing catches the misassignment. A second consequence: `tile_squarify` filters out zero-value children (`tile.rs:137-140`), so `rects.len() < children.len()` and the zip also silently drops the trailing children — a positive-value child can lose its rectangle entirely (it is never recursed into, and the zero-value child that received its rect early-returns at `tiling_method.rs:60-62`).
   *Fix:* have `tile_squarify` return rects keyed back to original child indices (e.g. push `(orig_index, rect)` pairs and re-sort by index, emitting a zero-area placeholder for filtered children) so the zip in `compute_treemap` stays aligned, and add a test that builds a squarify treemap with ascending values and asserts each named node lands on a rect of proportional area.

### Medium

2. **MeshPlot 2D keyboard pan is inverted relative to the shared chart handler**
   `crates/gpui-px/src/mesh_plot/interaction.rs:1176-1179` maps `PanLeft => pan_by_pixels(-24.0, 0.0)` and `PanUp => (0.0, -24.0)`, while the shared handler for all other charts maps `PanLeft => pan_by_pixels(+pan_step, 0.0)` and `PanUp => (0.0, +pan_step)` (`crates/gpui-px/src/interaction/chart_interaction.rs:408-411`). `pan_by_pixels` shifts the x domain by `-(dx/width)*range` and the y domain by `+(dy/height)*range` (`chart_interaction.rs:332-379`), so positive `dx` moves the viewport left — the shared handler matches the platform convention (Left arrow shows data to the left, Up arrow shows data above) and the MeshPlot mapping pans the opposite way on both axes. Mouse-drag panning in MeshPlot uses a different code path and is unaffected, which makes the keyboard path feel doubly broken by contrast.
   *Fix:* flip the signs in `mesh_plot/interaction.rs:1176-1179` to match `chart_interaction.rs:408-411`, and add a keyboard-pan direction test to `MeshPlotState::handle_key_with_permissions` coverage.

3. **Static SVG export emits `NaN` coordinates for log axes with wide-ratio data**
   `resolve_xy_domain` (`crates/gpui-px/src/lib/static_export.rs:1091-1107`) returns the auto domain from `extent_padded_iter` (additive 5% padding) with no positivity clamp for log scales. When the data spans more than ~20× (e.g. `y = [0.001, 1.0]`), the padded minimum goes ≤ 0; `map_scaled` then computes `domain.0.log10()` = NaN (`static_export.rs:1354-1371`) and `draw_axes` computes NaN tick labels (`static_export.rs:1201-1204`), so `line(...).y_scale(Log).to_svg()`, `scatter(...).to_svg()`, and `area(...).to_svg()` produce invalid SVG containing literal `NaN` for legitimate, validation-passing data. The boxplot/heatmap/contour static exports do clamp with `.max(1e-10)` (`static_export.rs:637-646`, `contour/contour_chart.rs:135-144`), and the live line chart pads multiplicatively (`line/line_chart.rs:1306-1311`), so this is an inconsistency unique to the line/scatter/area export path; the existing export tests only use narrow-range log data (`static_export.rs:1632-1647`).
   *Fix:* clamp the resolved log domain in `resolve_xy_domain` (or route through `clamp_log_domain` from `interaction/misc.rs:2-19`) and add an export test with `y = [0.001, 1.0]` on a log axis asserting the SVG contains no `NaN`.

4. **Squarify rows under-fill the rectangle after the first row**
   In `tile_squarify`, the row strip thickness is computed as `(row_sum / total) * area / w` (and the vertical variant at `crates/gpui-px/src/treemap/tile.rs:214`) where `total` is the fixed level total but `w`/`h` shrink as rows are placed (`tile.rs:210-223`). The standard algorithm divides by the *remaining* value, not the level total. Concretely, two equal children in a 2×1 rect produce rects `(0,0,1,1)` and `(1,0,2,0.5)` — the last row occupies only half of the remaining column, leaving dead space that no child ever claims. Visible as unexplained empty bands in squarified treemaps with more than one row.
   *Fix:* track `remaining_value` (subtract `row_sum` each iteration) and use `row_height = row_sum * h / remaining_value` / `row_width = row_sum * w / remaining_value`.

5. **Extra deep copy of draw data on every element render (treemap + boxplot legacy canvas)**
   The canvas prepaint closures deep-clone the prepared draw state: `crates/gpui-px/src/treemap.rs:485` clones `(Vec<RectDrawData>` — each holding a `String` name — plus the whole `BTreeMap` of hover groups), and `crates/gpui-px/src/boxplot/box_plot_chart.rs:530` clones `Vec<BoxDrawData>` including both outlier `Vec<f32>`s per box. gpui's `canvas` is `FnOnce` (`crates/3rdparties/gpui/src/elements/canvas.rs:10-19`), so this is once per element render rather than per animation frame — but the chart element is rebuilt on every re-render, making this a second full copy per render of data the same render pass already built. `box_plot_chart.rs:527` additionally clones `draw_data` for the vello path unconditionally whenever the `vello` feature is compiled in, even when the Legacy renderer was selected.
   *Fix:* wrap the prepared data in `Rc`/`Arc` and clone the handle into the closure (the area chart already does exactly this with `Arc<Vec<Point>>`, `area.rs:299-305,343-345`); gate the `vello_draw_data` clone behind the `renderer_2d == Renderer2D::Vello` check.

### High dispositions

1. **Squarify child association (2026-08-26): Fixed.** The tiler retains declaration-order output while packing non-zero children in value order; zero-value children receive zero-area placeholders so later children cannot be shifted or dropped. Regression verifies a 10/90 declaration-order pair receives proportional rectangles and checks zero-value alignment.
2. **MeshPlot keyboard panning direction (2026-08-26): Fixed.** The planar MeshPlot handler now uses the shared chart interaction signs: Left moves the visible X domain toward lower data values and Up moves the visible Y domain toward higher values. `keyboard_navigation_pans_planar_viewport_in_chart_direction` first zooms inside the source bounds, then verifies both directional transitions through the public permission-aware handler.
3. **Static SVG automatic log domains (2026-08-26): Fixed.** Automatic log domains now use positive multiplicative padding, matching live line charts, rather than additive padding that could cross zero. This is applied to line/scatter exports and the area export's Y domain. `static_export_auto_log_domains_stay_finite` exports all three with `0.001..1.0` data and asserts that the SVG contains neither `NaN` nor infinity.
4. **Squarify later-row underfill (2026-08-26): Fixed.** Each packed row now consumes its proportion of the *remaining* value and rectangle extent, not its proportion of the original total applied to an already-shrunk rectangle. `squarify_fills_the_parent_rectangle_across_multiple_rows` verifies that a 60/30/10 layout covers its complete parent area; the full treemap test group passes (31 tests).
5. **Treemap/boxplot render-time deep copies (2026-08-26): Fixed.** Prepared draw data and treemap paint groups are now held in `Rc` and copied only by handle into canvas prepaint/paint, click, and selected Vello paths. Boxplot no longer constructs a Vello data handle when the legacy renderer is selected. `cargo check -p gpui-px`, treemap tests (31), and boxplot tests (48) pass.

### Low

6. **Unbounded thread-local log-tick cache keyed by exact f64 bit patterns**
   `crates/gpui-px/src/line/misc.rs:72-92`: `LOG_TICK_CACHE: RefCell<HashMap<(u64, u64), Vec<f64>>>` has no eviction. Interactive zoom/pan on a log axis inserts one entry per unique domain forever (entries are tiny, but the map grows without bound for the lifetime of the thread).
   *Fix:* cap the map (e.g. clear when `len()` exceeds a few hundred entries, or use a small LRU); at minimum clear it when the axis scale type changes.

7. **Contour/isoline live builders don't clamp plot dimensions; tiny charts produce negative sizes**
   `crates/gpui-px/src/contour/contour_chart.rs:507-508` and `crates/gpui-px/src/isoline/isoline_chart.rs:503-504` compute `plot_width = layout_width - left_margin` / `plot_height = layout_height - title_height - bottom_margin` with no `.max(0.0)`, unlike bar (`bar/bar_chart.rs:538-545`) and boxplot (`boxplot/box_plot_chart.rs:276-278`). `validate_dimensions` only requires width/height > 0, so e.g. `.size(50.0, 50.0)` with a title validates and then builds scales with negative ranges and `px(negative)` sizes. The static-export path rejects this via `StaticLayout::new` (`lib/static_export.rs:1118-1139`), so live and export disagree.
   *Fix:* clamp with `.max(0.0)` (or return `InvalidDimension` when the plot area collapses) in both builders.

8. **Live scatter/area log auto-domains use additive padding + `1e-10` clamp vs line's multiplicative padding**
   `crates/gpui-px/src/scatter/scatter_chart.rs:777-794` and `crates/gpui-px/src/area.rs:233-250` pad with `extent_padded_iter` (additive) and then clamp the domain minimum at scale construction (`scatter_chart.rs:902` area, `area.rs:400-423`), while the line chart pads multiplicatively (`line/line_chart.rs:1306-1311`). For small positive data the additive pad dips below 0 and clamps to `1e-10`, producing a lopsided log axis with wasted decades.
   *Fix:* use the line chart's multiplicative padding for log scales in scatter/area for consistent axis appearance.

9. **Grouped-bar corner radius honored on the canvas backend but ignored on the vello backend**
   The legacy canvas path rounds each bar with `add_rounded_rect_to_path(..., border_radius)` (`crates/gpui-px/src/bar/bar_chart.rs:1073-1080`), while `bar_chart_scene` emits plain `fill_rect`s (`bar_chart.rs:1130-1140`). The same chart renders with rounded bars on one backend and square bars on the other; the single-series path goes through d3rs `render_bars_selected`, which handles radius consistently.
   *Fix:* emit rounded rects in `bar_chart_scene` (kurbo `RoundedRect`) when `border_radius > 0.0`, and include `border_radius` in the vello cache key at `bar_chart.rs:1025-1029` (it is currently absent, so toggling only the radius reuses a stale scene).

10. **Live colorbar spaces tick labels evenly; SVG colorbar positions them by value**
    `Colorbar::render` stacks tick labels in a `justify_between` column (`crates/gpui-px/src/colorbar.rs:126-130`), which spaces any tick set evenly, while `to_svg` maps each tick value to its true fractional position (`colorbar.rs:198-210`). With custom non-uniform ticks the live and exported colorbars disagree.
    *Fix:* position live tick labels by the same `(range[1] - tick) / (range[1] - range[0])` fraction used by `to_svg`.

11. **O(n·h) front-drain reshape in surface3d grid build**
    `crates/gpui-px/src/surface3d.rs:510-515` does `z.drain(..grid_width)` once per row; each drain memmoves the entire remaining Vec, making grid construction quadratic in the row count.
    *Fix:* iterate `z.chunks(grid_width)` (or walk an index) and build rows without draining.

### Low dispositions

6. **Log-tick cache growth (2026-08-26): Fixed.** The per-thread cache now clears before inserting a 257th distinct domain, capping it at 256 entries while retaining same-domain reuse. `log_tick_cache_has_a_bounded_number_of_domains` exercises 257 distinct valid log domains and verifies that cap.
7. **Contour/isoline negative live plot dimensions (2026-08-26): Fixed.** Both builders now clamp margin-subtracted plot dimensions to zero before scale and element construction. The focused contour and isoline tiny-layout tests verify that a `50×50` titled chart yields `(0, 0)` rather than negative dimensions (2 tests pass).
8. **Scatter/area automatic log padding (2026-08-26): Fixed.** A shared `extent_log_padded_iter` now supplies positive multiplicative padding, matching line charts, for scatter X/Y (including secondary series) and area X/Y/Y0 domains. The helper regression verifies the `0.001..1.0` bounds; scatter (30) and area (23) test groups pass, as does the public SVG finite-output regression.
9. **Grouped-bar Vello corner radius/cache identity (2026-08-26): Fixed.** The Vello scene now emits rounded rectangles when a positive grouped-bar radius is selected, and the radius participates in the scene cache key. The focused Vello regression verifies both curved path geometry and distinct cache keys; the bar test group passes (37 tests).
10. **Live colorbar non-uniform tick positions (2026-08-26): Fixed.** Live labels are now absolutely positioned from the same normalized range fraction as SVG labels, instead of being evenly distributed by flex layout. The shared fraction helper’s non-uniform 100/75/25/0 regression and the colorbar group (4 tests) pass.
11. **Surface3D front-drain reshape (2026-08-26): Fixed.** Row-major Z data is now reshaped through non-overlapping `chunks_exact` slices instead of repeated front drains, eliminating quadratic tail memmoves. The `gpu-3d`-gated row-order regression and Surface3D test group pass with `--features gpu-3d`.

## GPU/CPU data-flow notes

- gpui-px contains no `device.poll`, `pollster`, or `map_async` calls; all wgpu device interaction lives in `gpui-d3rs`. The crate's GPU-facing surface is the vello2d `VelloChartElement::with_builder(...).cache_key(...)` pattern (e.g. `bar/bar_chart.rs:1023-1054`, `boxplot/box_plot_chart.rs:740-788`), which is safe for the wasm deferred-readback model because scene building is pure CPU-side and cached by content hash. The `AGENTS.md` caveat that the gpu3d/gpu-compute paths assume blocking wgpu applies to d3rs, not to anything in this crate — but note the mesh_plot toolbar wires `gpu-3d` scene transforms directly (`mesh_plot/mesh_plot_chart.rs:2528-2546`), so exercising MeshPlot 3D on wasm would hit that d3rs limitation through this crate.
- Vello cache keys hash every f32 of the draw data (e.g. `box_plot_chart.rs:754-766`), which is O(n) per render but correct; finding 9 notes one missing key field (`border_radius` for grouped bars).

## UI/UX consistency

- Finding 2 (inverted MeshPlot keyboard pan) is the main behavioral inconsistency: keyboard panning works one way on every chart except MeshPlot 2D.
- Findings 8, 9, and 10 are cross-chart/backend/live-vs-export visual inconsistencies for log axes, grouped-bar corners, and colorbar ticks respectively.
- Finding 7 makes live contour/isoline accept chart sizes that the static export rejects.
- Positively: color-scale endpoints, CVD screening tests (`color_scale.rs:462-495`), accessibility summaries, and legend/annotation metadata are consistent across chart families.

## Clean bill

Areas checked that look correct:

- **MeshPlot background preparation**: `begin/finish_*_preparation` key-based stale-result rejection (`mesh_plot/interaction.rs:720-904`) is sound; the `unreachable!`/`expect` sites in `mesh_plot_chart.rs` (1013, 1191, 1647, 2594, 2880, 2894) are each guarded by an immediately preceding invariant check.
- **Picking**: `pick_2d`/`pick_3d`/`pick_revolved_3d` (`mesh_plot/picking.rs`, `picking3d.rs`) validate before indexing; the direct index into `field.values` (`picking.rs:93-101`) is only reachable after `field_value_is_pickable` confirmed the index.
- **Streaming caches**: `cached_line_points`/`cached_scatter_points` (`line/line_chart.rs:34-67`, `scatter/scatter_chart.rs:28-65`) gate `Arc::make_mut` reuse on `strong_count == 1` and re-validate finiteness — the allocation-free streaming claim is implemented as documented.
- **Validation plumbing**: `lib/validate.rs`, `ColorRange::resolve` (`color_range.rs:25-65`), and the static-export validators consistently reject empty/NaN/mismatched/non-monotonic/non-positive input and preserve error variants through export (covered by tests at `static_export.rs:1649-1759`).
- **Color scales and colorbar math**: palette interpolation handles empty/single-color palettes (`color_scale.rs:91-102`); `Colorbar::tick_values` guarantees endpoint ticks and drops non-finite values (`colorbar.rs:81-95`); XML escaping is applied to all user strings in SVG output.
- **ChartSize/aspect handling**: invalid aspect ratios and dimensions are ignored rather than producing NaN geometry (`lib/chart_size.rs:75-102`, with tests).
- **Slice/dice/binary tiling**: unlike squarify, these emit one rect per child in declaration order, so the `compute_treemap` zip is aligned; zero totals are rejected before layout (`treemap.rs:113-119`, `treemap/tiling_method.rs:59-62`).
- **Box statistics**: whisker fallback to data min/max when no points fall inside the 1.5×IQR fence is handled in both live and static paths (`boxplot/box_stats.rs`, `static_export.rs:834-839`); `.bins(0)` is rejected up front in both.
- **RefCell usage** in mesh interaction handlers is short-lived, and paint callbacks use `try_borrow` (`mesh_plot_chart.rs:3757`), so no re-entrant borrow panics from the paint path.
