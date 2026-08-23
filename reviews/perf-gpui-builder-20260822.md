# Perf review: gpui-builder

Date: 2026-08-22

## Role and hot paths

`gpui-builder` is a pure, platform-agnostic constraint-layout solver: it takes a
borrowed `LayoutNode` declaration tree + viewport size + `LayoutPreferences` and
produces a `SolvedNode`/`SolvedTree` with concrete pixel sizes. The hot path is
`solve_tree_into` (`src/solver/solve.rs:99`) invoked on every window resize and
every divider-drag mouse-move, plus `Sizing::Text` measurement through
gpui-pretext (`src/solver/misc.rs:92`). Secondary paths: `LayoutState` →
`LayoutPreferences` snapshotting (`src/state.rs:201`), `ChassisLayout::solve`
(`src/plugin_chassis/chassis_layout.rs:83`), and the showcase binary's per-frame
render (`src/bin/layout_showcase/showcase_view.rs:122`).

The crate is already substantially perf-hardened: pooled child-info scratch
(`solve.rs:215-244`), recycled child-index buffers (`solved/solved_tree.rs:164-188`),
a persistent text-measurement cache (`src/solver/misc.rs:24-52`), an
allocation-contract test requiring 1,000 warmed resize solves with **zero**
allocations (`tests/allocation_contracts.rs:11-60`), and Criterion benches
(`benches/solved_tree.rs`). The findings below are what remains.

## Findings

1. **[GPU] No GPU surface — correctly so.** The crate contains no wgpu/vello code
   at all (grep for `map_async|device.poll|pollster|wgpu|vello` in `src/` returns
   nothing). The solver is scalar float arithmetic over small trees; moving it to
   GPU would be a loss. However, `Cargo.toml:34-35` declares `gpu-2d` and `gpu-3d`
   features that are **not referenced by any `cfg(feature = ...)`** in the crate —
   dead feature flags that mislead consumers about capabilities. No roundtrips
   exist either; there is nothing to fix on goals 1–2 beyond deleting the flags.

2. **[Alloc] Showcase binary re-solves with fresh storage every frame.**
   `src/bin/layout_showcase/showcase_view.rs:218` calls `solve_tree()` (fresh
   `SolvedTree` arena + id `HashMap` + child-index buffers per render,
   `solve.rs:88-92`) inside `Render::render`, which runs on every
   `cx.notify()` from drag mouse-moves (`showcase_view.rs:269-273`). It also
   rebuilds `LayoutPreferences::new` per frame, allocating two `HashMap`s
   (`src/types/layout_preferences.rs:16-27`). The retained zero-alloc path
   (`RetainedLayoutSolver`, `solve.rs:140`) exists but the showcase — the
   reference integration consumers will copy — does not use it. Impact: ~5–10
   small heap allocations per frame at 60–120 Hz during drags; also the tree
   size is tiny here, so this is mainly a pattern problem, not a measured cost.

3. **[Alloc] Text-measure size cache misses on every frame of a resize drag.**
   The size cache is keyed by `(measure_ptr, cross_size.to_bits(),
   line_height.to_bits(), axis)` (`src/solver/misc.rs:96-99`). During a window
   resize, `cross_size` changes continuously, so every frame misses the `sizes`
   map and re-runs `layout()` / `layout_with_lines()` in gpui-pretext
   (`misc.rs:123-143`). The prepared-text layers (`prepared_vertical` /
   `prepared_horizontal`) do survive, so re-preparation is avoided, but full
   line-wrapping layout re-runs per text slot per frame. For a layout with many
   `Sizing::Text` slots this is likely the dominant per-frame cost (needs
   profiling — the existing `text_cache_hit` bench in `benches/solved_tree.rs:185`
   only covers the constant-viewport hit path). Possible mitigation: quantize
   the cross_size key, or cache per-line widths and re-wrap incrementally.
   Note also `measure_ptr` is a raw `&dyn TextMeasure` data pointer
   (`misc.rs:96`); if a measurer is dropped and another allocated at the same
   address, stale entries collide — a correctness-adjacent footgun, not perf.

4. **[Alloc] `SolvedTree::as_map()` allocates a full HashMap on every call.**
   `src/solved/solved_tree.rs:232-245` caches an id→index map in a `OnceLock`,
   but then `.collect()`s a brand-new `HashMap<&str, &SolvedNodeData>` on each
   invocation — O(n) allocation per call despite the "subsequent calls reuse"
   doc. Per-frame consumers doing repeated id lookups allocate every frame.
   Compare `find()` (`solved_tree.rs:201-206`), which is already O(1) and free.

5. **[Alloc] `ChassisLayout::solve` returns a fresh `Vec` per call.**
   `src/plugin_chassis/chassis_layout.rs:83-88` reuses thread-local scratch for
   its working buffers (`chassis_layout.rs:19-45`) but the output
   `SolvedChassis { sections: Vec<SolvedSection> }` (`solved_chassis.rs:7`) is
   allocated fresh every solve. Minor (one Vec per frame), but inconsistent
   with the scratch-buffer discipline the same function already implements.

6. **[Alloc] `LayoutState` snapshot path allocates twice per solve.**
   `LayoutState::preferences()` builds fresh ratio/collapsed `HashMap`s
   (`src/state.rs:201-222`) and `as_preferences()` clones them again
   (`src/state.rs:237-239`). The zero-alloc `as_preferences_ref()` exists
   (`state.rs:245-247`); callers on the frame path should use it. Also
   `LayoutState::set_ratio`/`toggle_collapsed` store `String` ids
   (`state.rs:129-133, 164-167`) — fine at interaction rate.

7. **[Alloc] Priority-collapse passes are O(n²) per container.**
   `allocate_main_axis` rescans all children for the min-priority candidate on
   every collapse iteration (`src/solver/child_info.rs:63-93`, acknowledged in
   the comment); `ChassisLayout::solve` additionally recomputes `min_sum`
   inside the collapse loop (`chassis_layout.rs:98-125`). No allocation, and
   fine at UI sibling counts (n ≤ ~10), but a pathological wide container
   collapses in quadratic time. Similarly `node_count()` re-walks the whole
   declaration tree on every `solve_tree_into` (`src/types/layout_node.rs:71-82`,
   called at `solve.rs:118`) — a full extra O(n) traversal per solve that could
   be cached by the caller. Both are low impact.

8. **[Alloc] Showcase inspector rebuilds Strings per row per frame.**
   `collect_visual_tree_rows` allocates `String` id + `Option<String>` tier per
   node per render (`src/bin/layout_showcase/types.rs:31-39`), plus several
   `format!`/`SharedString::from` per row (`showcase_view.rs:770-784`) and
   `collapsed_tabs()` allocating a Vec per frame (`showcase_view.rs:225`,
   superseded by the allocation-free `collapsed_slots()`, `solved_tree.rs:259`).
   Demo-only code, but again it is the visible integration example.

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | Move showcase to `RetainedLayoutSolver` + hoist `LayoutPreferences` construction out of `render` (rebuild only on drag-state change); switch to `collapsed_slots()` | 2, 8 | S | Removes all steady-state frame allocations from the reference integration; validates the zero-alloc API in a real app |
| 2 | Profile `Sizing::Text` resize drags; if hot, quantize the `cross_size` cache key or cache line-widths separately from wrapped height | 3 | M | Eliminates the main per-frame CPU cost for text-heavy layouts during resize (needs profiling first) |
| 3 | Rework `SolvedTree::as_map` to return an iterator or a borrowed view instead of collecting a new `HashMap` per call (or fix the doc) | 4 | S | O(n) alloc per call removed for lookup-heavy consumers |
| 4 | Add `SolvedChassis::into`-style reuse or a `solve_into(&mut SolvedChassis)` variant | 5 | S | One Vec per frame saved on chassis redraws |
| 5 | Delete the unused `gpu-2d`/`gpu-3d` (and audit `autoeq`/`spinorama`/`reqwest`/`tokio`/`urlencoding`) feature flags | 1 | S | Accurate capability surface; no functional change |
| 6 | Cache `node_count()` on the caller side / accept a capacity hint in `RetainedLayoutSolver::solve` | 7 | S | Removes one O(n) walk per solve |
| 7 | Key `TextMeasureCache` by a caller-supplied measure id instead of a raw pointer | 3 | M | Removes the stale-pointer collision hazard; perf-neutral |

## Quick wins

- Showcase: `RetainedLayoutSolver` + `collapsed_slots()` + prefs rebuilt only on change (findings 2, 8) — half a day.
- `as_map`: stop collecting a fresh map per call or fix the misleading doc comment (finding 4) — <1 hour.
- Delete dead `gpu-2d`/`gpu-3d` feature flags in `Cargo.toml` (finding 1) — minutes.
- `ChassisLayout::solve_into` reusing the output Vec (finding 5) — <1 hour.
- Document `as_preferences_ref()` as the frame-path choice in `state.rs` and the README (finding 6) — minutes.
