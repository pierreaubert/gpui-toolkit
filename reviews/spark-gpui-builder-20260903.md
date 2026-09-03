# Code Review: gpui-builder — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-builder` (74 files, ~12.5k LOC)

## 1. Purpose / role
Generic priority-collapse / auto-axis / display-tier layout solver for GPUI + audio-plugin chassis, with retained solver, inspector, validation, snapshots. Largest: `solver/solve.rs` (1701), `bin/layout_showcase/showcase_view.rs` (1071), `solved/tests.rs` (791), `solved/solved_tree.rs` (530), `compat.rs` (466), `state.rs` (463).

Public API: `solve/solve_with_cache/solve_tree/solve_tree_into/solve_tree_with_cache`, `RetainedLayoutSolver`, `TextMeasureCache`, `LayoutNode/ContainerNode/SlotNode/Sizing/Axis/DisplayTier/LayoutPreferences`, `SolvedTree/SolvedNode/SolvedTreeMap/CollapsedSlot`, `validate_layout/LayoutValidationReport`, `inspect_layout/inspect_solved`, `LayoutState/LayoutAction`, `LayoutSnapshot/solve_snapshot_matrix`, `LayoutStory/Catalog`, `ChassisLayout/SolvedChassis`, `BenchmarkReport`, `accessibility_tree_from_solved`.

## 2. SOTA gap analysis (vs Yoga/Taffy, Auto Layout, Cassowary)
1. **No general linear-inequality solver** (Cassowary `stay/edit` constraints) — axis-priority collapse only.
2. **No CSS Grid 2-D** (explicit tracks, `minmax()`, placement, subgrid).
3. **No flex wrap + fragmentation parity**; `child_info.rs:17 allocate_main_axis`, `:130 distribute_remaining` are single-axis.
4. **No intrinsic sizing** (`min/max-content`, baseline alignment); `solver/misc.rs:129 compute_text_size` is ad-hoc.
5. **No dirty-bit incremental layout**; `solve.rs:173 RetainedLayoutSolver::solve` clones cache (`:185`).
6. **No RTL / vertical-writing first-class axis.**
7. **No animation-interpolable layout values.**
8. **Bespoke debug tooling** (`layout_debug_report`, `visual_regression.rs`) vs Yoga/Taffy devtools.

## 3. Performance evaluation
- Showcase `render` god-function: `showcase_view.rs:253` 208 lines/cyclo 18/fan-out 65/MI 20.8/CRAP 342, untested.
- Solver core branchy + untested: `solve.rs:315 solve_tree_container` (159 lines/cyclo 16/CRAP 272), `solve.rs:542 solve_container` (142 lines/cyclo 16), `validate.rs:166 validate_sizing` (113 lines/cyclo 15). Coverage ~4% (12/340 fns).
- Per-solve allocs: `solve.rs:210 HashMap<&str,NodeIndex>` rebuilt per solve; `Vec::new()` at `:289,308,404,428,455,520,538,641,662`; pool exists (`:221 CHILD_INFO_POOL`) but not pervasive.
- Untested shared util: `util.rs:6 format_number` fan-in 73/risk 666; `compute_text_size` risk 301.

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Move `showcase_view.rs:253 render`/`:465 render_content` to example-only helpers | S | slims shipped lib |
| 2 | Split `solve_tree_container`/`solve_container` into resolve/collapse/assign passes + tests | M | removes top CRAP |
| 3 | Make solver truly retained: reuse HashMap + child buffers via `solve_tree_into*` (`:103,114`); add allocation contracts | M | per-solve allocs → ~0 |
| 4 | Memoize `compute_text_size` behind `TextMeasureCache` with eviction bound | S | text-layout speedup |
| 5 | Cover `format_number`, `distribute_remaining`, `allocate_main_axis` | S | cheapest risk cut |

## 5. Verdict
Good domain solver, not yet a general flex/grid replacement. SOTA = constraints/grid/wrap/incremental. Perf = retain buffers + memoize measure.
