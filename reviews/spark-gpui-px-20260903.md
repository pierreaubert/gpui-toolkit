# Code Review: gpui-px — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-px` (69 files, ~33.4k LOC)

## 1. Purpose / role
Plotly Express-style chart builders (line/bar/scatter/pie/heatmap/contour/isoline/treemap/boxplot/surface3d + unstructured `MeshPlot`) rendering via `gpui-d3rs` CPU/GPU paths. Largest: `mesh_plot/mesh_plot_chart.rs` (5284), `mesh_plot/interaction.rs` (2093), `mesh_plot/export.rs` (1987), `line/line_chart.rs` (1801), `lib/static_export.rs` (1793).

Public API: `LineChart/line`, `BarChart/bar`, `ScatterChart/scatter`, `PieChart/pie/donut`, `AreaChart`, `HeatmapChart`, `ContourChart`, `IsolineChart`, `Treemap/TreemapNode/TilingMethod`, `BoxPlotChart`, `Surface3DChart`, `MeshPlot/mesh_plot` + `MeshPlotState/MeshPlotView/MeshPlotBackend/MeshRenderMode`, `ColorScale/ColorRange/Colorbar`, `ChartTheme/Annotation/Legend`, `ChartAccessibilitySummary`, `StaticSvgOptions`, `ChartCapabilityReport`.

## 2. SOTA gap analysis (vs Plotly Express, Vega-Lite, ECharts, Observable Plot)
1. **No declarative grammar** (Vega-Lite JSON spec, faceting, repeat/layer/concat).
2. **No faceting / small-multiples operator** — imperative builders only.
3. **No linked brushing / cross-filter** coordination across views.
4. **No big-data path** — no LTTB/min-max downsampling, no WebGL instancing; full CPU geometry per frame.
5. **No statistical transforms** (regression, KDE, auto-binning, aggregation) beyond `box_stats.rs`.
6. **No animation/transition system** (ECharts `setOption` transitions, Plotly `animate`).
7. **No DataFrame/Arrow input**; no streaming-window contract beyond `benches/streaming_prepare.rs`.
8. **SVG-string-only export** (`lib/static_export.rs:276,386,409,498`); no canvas/PNG parity, thin a11y beyond summary structs.

## 3. Performance evaluation
- `mesh_plot_chart.rs:764 build_frame` — 1913 lines/cyclo 161/cogn 545/nesting 9/fan-out 337/MI 0.0/CRAP 26082. Largest liability in workspace.
- Every `build()` is a god-function + untested: `line_chart.rs:1106` (695 lines/cyclo 86), `scatter_chart.rs:605` (532 lines/risk 6240), `bar_chart.rs:388` (460 lines/risk 5062), contour/isoline/heatmap equivalents. Coverage 5% (36/798).
- Clone-heavy retained frame: `mesh_plot_chart.rs:315 mode.clone()`, `:357 plot.clone()`, `:376 state.clone()`, `:389 prepared_planar_frame.clone()`, `:437 last_valid_plot = Some(self.plot.clone())`, `:723,:757 cached/prepared.clone()`; 419 `.clone()` crate-wide.
- Per-frame string/geom allocs: `line_chart.rs:43,65,357,1394,402,441` (`Vec::new`, point-tuple clones, title/annotation clones).
- No vertex-buffer diffing — full re-prepare per interaction; only draw-epoch map (`mesh_plot_chart.rs:54`).

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Split `build_frame` into prepare/project/axes/series/overlay; enforce <150-line gate | L | removes 26k CRAP |
| 2 | Dirty-flag retained mesh: re-project only on camera/data change | M | interaction fps |
| 3 | `Arc<PreparedFrame>` + CoW instead of clones; bench `mesh_plot_frames` as gate | M | allocs → ~0 |
| 4 | LTTB + min-max decimation for line/scatter >10k pts | M | big-data parity |
| 5 | Golden SVG coverage for top-risk `build()`s via `static_export.rs` harness | S | regression safety |

## 5. Verdict
Feature-rich but `build_frame` is the workspace's riskiest function. SOTA = grammar/faceting/brushing/decimation/animation. Perf = split, retain, decimate.
