# Code Review: gpui-d3rs — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-d3rs` (361 files, ~89k LOC)

## 1. Purpose / role
Rust port of d3-array/scale/shape/geo/force/hierarchy/contour + GPUI renderers (`gpu2d`, `gpu3d`, `vello2d`, `mesh`, text) with Observable golden tests. Largest: `geo/path/clip.rs` (2235), `force/mod.rs` (2084), `geo/path/geo_path.rs` (1805), `mesh/gpu/compute.rs` (1688), `gpu2d/shapes/render.rs` (1348), `gpu3d/element/surface3_delement.rs` (1342).

Public API: `scale::{Linear,Log,Band}`, `shape::{Arc,Pie,Line,Area,Stack,CurveType}`, `array::{ticks,bin,median,quantile}`, `color/D3Color`, `interpolate`, `axis`, `contour`, `geo::{Orthographic,Equirectangular,ConicEqualArea}`, `force::Simulation`, `hierarchy/quadtree/Delaunay/chord/sankey/mesh`, `gpu2d/gpu3d/vello2d`, `fetch::{csv,json}`, `format/time/timer/transition/zoom/brush/selection/drag/ease/dispatch/random/tile/polygon/lod`.

## 2. SOTA gap analysis (vs D3.js v7 + Vello)
1. **Hierarchy incomplete** — pack/partition are stubs vs d3-hierarchy.
2. **Curves emit `L`, not native `C/S`** — larger paths, worse Vello AA.
3. **Missing `scale.nice()` parity.**
4. **Projection gaps** — stereographic ~10% clip-boundary mismatch at 142°.
5. **Force is O(n²)** without Barnes-Hut; non-deterministic init, no convergence fast-path.
6. **No Vello scene-graph reuse** (`vello2d/wgpu_draw.rs:351,536 create_buffer` per draw; `mesh/gpu/renderer3d.rs:406`, `wgpu_backend.rs:78` same).
7. **Missing transition/timer parity** for chart animation (`timer/mod.rs:8` is a shim).
8. **`feature_parity.rs` (1095 lines) tracks gaps but has no CI version pin** vs D3 7.9.0 beyond golden JSON.

## 3. Performance evaluation
- `gpu3d/element/surface3_delement.rs:743 paint` 599 lines/cyclo 48/fan-out 113/CRAP 2352, untested.
- GPU compute branchy + untested: `mesh/gpu/compute.rs:464 marching_segments_indexed` (341 lines/cyclo 20), `:821 band_triangles` (325 lines/9 loops).
- Quadtree clones: `quad_tree.rs:112,357,384 item.clone()`, `:798-805 data()` allocates + clones per point; `:507` alloc per removal.
- Force hot math: `force/mod.rs:613,760,1170 sqrt()` in O(n²) loop; `per_link_strength/bias` realloc'd (`:1142-1143`).
- 399 `.clone()`, 446 `unwrap/expect` crate-wide; NaN-unsafe sorts (`array/bin.rs:206`, `statistics.rs:186,245`, `ticks.rs:232` `partial_cmp().unwrap()`); blocking `device.poll`/`pollster` must not run on wasm.

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Split `paint` (`surface3_delement.rs:743`) into camera/cull/tessellate/submit + cache tests | M | removes top CRAP |
| 2 | Zero-copy GPU: persistent buffers + `queue.write_buffer` diffing; remove per-frame `create_buffer` | M | frame allocs → ~0 |
| 3 | Quadtree: `data()` → `extend_into(&mut Vec)` + borrowed `find_all`; index handles vs clone | S | insert/query speedup |
| 4 | Barnes-Hut or squared-distance early-out in `force/mod.rs`; 1k/10k-node benches | M | O(n²)→O(n log n) |
| 5 | `total_cmp` / NaN-filtering `try_*` (follow `Delaunay::try_new` at `delaunay/mod.rs:438`) | S | panic safety |

## 5. Verdict
Strongest parity engine in the workspace; renderers and force layout are the perf frontier. SOTA = curves/hierarchy/nice/projections/transitions. Perf = persistent GPU buffers + Barnes-Hut + clone-free spatial index.
