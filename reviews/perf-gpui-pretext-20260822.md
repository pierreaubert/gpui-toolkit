# Perf review: gpui-pretext

Date: 2026-08-22

## Role and hot paths

`gpui-pretext` is a pure-CPU text measurement + line-breaking library (port of
chenglou/pretext). Two phases:

- **Prepare** (`prepare`/`prepare_with_segments`, `src/layout/prepare_options.rs:127`,
  `src/layout/types.rs:106` `measure_analysis`): whitespace normalization →
  `analyze_text` segmentation (`src/analysis/analysis_profile.rs:216`) → per-segment /
  per-grapheme width measurement via caller-supplied `TextMeasure`
  (`src/measurement.rs:30`). Expensive; dominated by `measure_width` calls and
  segmentation string churn.
- **Layout** (`layout`, `layout_with_lines`, `layout_optimal`,
  `src/layout.rs:25-169`): arithmetic-only greedy walk (`src/line_break/walk.rs:243`)
  or Knuth-Plass DP (`src/line_break/types.rs:537`). Runs per resize.

Real callers: `gpui-builder` `Sizing::Text` sizing
(`crates/gpui-builder/src/solver/misc.rs:92-150`) which already caches
`PreparedText` and final sizes across frames in `TextMeasureCache`; the python IR
showcase `text.prepare_layout` command
(`crates/gpui-python-runtime/bin/showcase/python_ir_showcase.rs:11583-11650`)
which is one-shot per JSON command, not per-frame. So the steady-state hot path
is **layout on cached prepared text**; prepare only runs on text change.

**GPU:** not applicable. The crate has no rendering; measurement is delegated to
the caller's `TextMeasure` (CPU shaping in practice) and line breaking is a
sequential DP. **Roundtrips:** none — no GPU API is touched.

Existing perf assets: criterion bench `benches/layout_temporaries.rs`, zero-alloc
integration contracts `tests/allocation_contracts.rs` (warmed cache hits must be
alloc-free), prior passes in `docs/superpowers/specs/2026-06-20-performance3-top3-design.md`
and `docs/superpowers/plans/2026-06-15-performance-pass-1-4.md` (their thread-local
scratch/Cow items are already landed). No pretext baseline in `qa/perf/`.

## Findings

1. **[Alloc] Segmentation pipeline clones every segment string in
   `split_hyphenated_numeric_runs`** — `src/analysis/split.rs:118` and
   `:128` (`texts.push(split_text.clone())` / `texts.push(text.clone())`) deep-copy
   every segment even when no split occurs. The sibling passes in
   `src/analysis/merge.rs` already move with `std::mem::take`
   (`merge.rs:23,31`). One full copy of all segment text per prepare — pure waste.

2. **[Alloc] Per-grapheme HashMap entries during breakable-word measurement** —
   `MeasureCache::get_grapheme_widths` measures each grapheme via
   `get_width(&seg[start..end], …)` (`src/measurement.rs:166`), and every such call
   inserts a separate `Arc<str>`-keyed entry (`measurement.rs:100-108`). An
   n-grapheme breakable word costs n measure calls + n hash inserts + n Arc
   allocations, inside a cache that is **dropped at the end of prepare**
   (`MeasureCache::new()` at `src/layout/types.rs:112`), so the entries never pay
   off across prepares. The per-segment `Arc<[f64]>` already holds the result;
   the per-grapheme map entries are redundant for the common single-use case.
   (Impact on overall prepare time depends on caller's `measure_width` cost —
   needs profiling with a real GPUI-backed measurer.)

3. **[Alloc] `Vec` double-copy when exporting scratch to `Arc<[f64]>`** —
   `Arc::from(widths.clone())` at `src/measurement.rs:168` and
   `Arc::from(scratch.clone())` at `src/measurement.rs:221` clone the scratch Vec
   and then copy again into the Arc allocation. `Arc::from(scratch.as_slice())`
   followed by `scratch.clear()` does one copy instead of two. Two allocations
   and two copies per breakable segment become one of each.

4. **[Alloc] Knuth-Plass result path clones scratch and re-collects per chunk** —
   `knuth_plass_chunk` fills thread-local `KP_BREAKS_SCRATCH` then returns
   `Some(breaks.clone())` (`src/line_break/types.rs:698`), defeating the scratch
   reuse; multi-chunk optimal layout then re-collects into `global_breaks`
   (`src/line_break/walk.rs:681-684`) per chunk. Two `Vec` allocations per
   chunk per `layout_optimal` call that could be zero (write into a caller
   buffer / remap in place). Note the expensive part — `build_kp_items` — *is*
   properly cached on `PreparedCore.kp_item_cache`
   (`src/line_break/types.rs:275-287, 527-529`), so this only bites on width
   changes (every resize drag frame with `Optimal` strategy).

5. **[Alloc] `layout_with_lines` allocates a fresh `Vec<LayoutLine>` and one
   owned `Cow` per line on every call** — `src/layout.rs:56` (`Vec::new()`, no
   capacity hint — `count_prepared_lines` could pre-size it) and
   `build_line_text_cow` (`src/layout/misc.rs:140-173`). The Cow path is already
   well tuned (borrow for single-segment lines at `misc.rs:149-152`; `mem::take`
   for long lines at `misc.rs:169`), but short multi-segment lines still hit
   `scratch.clone()` (`misc.rs:171`) → one String alloc per line per resize
   frame. Callers that only need widths/counts (e.g. gpui-builder horizontal
   sizing, `crates/gpui-builder/src/solver/misc.rs:140-142`, which materializes
   full line text just to fold `max(width)`) should use `walk_line_ranges`
   (`src/layout/walk.rs:11`) — no text materialization at all.

6. **[Alloc] `MeasureCache::get_segment_metrics` double-hashes on hit** —
   `contains_key` then `get` (`src/measurement.rs:97,110`) hashes the segment
   string twice per hit; the miss path also hashes a third time in `insert`.
   `if let Some(m) = self.cache.get(seg) { return m; }` is one hash per hit.
   Covered by the zero-alloc contract but not by a CPU benchmark.

7. **[Alloc] Bounded prepare does a full extra grapheme pass over the input** —
   `analyze_with_budget` counts graphemes with
   `text.graphemes(true).take(max+1).count()` (`src/layout/prepare_options.rs:97-100`)
   before `analyze_text` re-segments everything. Up to 4M graphemes scanned twice
   on valid input. Only on the budget variants (python IR showcase path), not the
   plain `prepare`.

8. **[Alloc] Text stored 2–3× per prepared block** — `TextAnalysis` owns
   `normalized: String` + `texts: Vec<String>` (`src/analysis/text_analysis.rs:6-8`),
   then `measure_analysis` copies each segment again into
   `PreparedTextWithSegments.segments` (`src/layout/types.rs:186`,
   `text.to_string()`). Peak prepare memory ≈ 3× text size. Segments could be
   `Box<str>` slices/ranges into one owned normalized buffer. Also minor:
   `merged_segmentation.rs:44` `insert_str(0, …)` is an O(n) memmove per CJK
   sticky-carry, and `split.rs:13` collects `char_indices()` into a Vec per call
   (per CJK segment pair, via `merged_segmentation.rs:39`).

9. **[Alloc] Bidi path: single-entry cache with O(n) key compare + full
   re-derive per segment set** — `compute_segment_levels`
   (`src/bidi/mod.rs:147-178`) caches by whole normalized text (`mod.rs:82`);
   on hit it still clones the per-char levels Vec (`mod.rs:153`) and rebuilds
   `seg_levels` by binary search per segment. Only runs on
   `prepare_with_segments`, once per prepare — low priority.

Non-findings (checked, already fine): layout-phase `to_line_break_data` is fully
borrowed (`src/layout/to.rs:8-24`); greedy walk and simple count path allocate
nothing (`src/line_break/count.rs:21-98`); KP active-node scratch is thread-local
(`src/line_break/types.rs:28-37`); `PreparedLineBreakData::slice` borrows the
whole-range chunk list (`src/line_break/types.rs:88-97`); merge passes move
strings instead of cloning (`src/analysis/merge.rs:23`).

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | Replace `.clone()` pushes with `std::mem::take` in `split_hyphenated_numeric_runs` | 1 | S | Removes a full segment-text copy per prepare |
| 2 | `Arc::from(slice)` instead of `Arc::from(vec.clone())` in measurement | 3 | S | Halves allocs/copies for breakable segments |
| 3 | Single-hash `get` in `get_segment_metrics` | 6 | S | 2× fewer hashes on prepare hot loop |
| 4 | Switch gpui-builder horizontal sizing to `walk_line_ranges` | 5 | S | No line-text String on builder cache miss |
| 5 | Have `knuth_plass_chunk` write breaks into a caller `&mut Vec` (or remap in place) | 4 | M | Zero-alloc KP result path on resize drags |
| 6 | Pre-size `lines` in `layout_with_lines` via `count_prepared_lines` or `Vec::with_capacity` | 5 | S | Fewer Vec reallocs per layout |
| 7 | Store segments as ranges into one normalized buffer | 8 | M | ~2–3× peak-memory cut on large texts |
| 8 | Skip per-grapheme HashMap inserts; keep per-segment `Arc<[f64]>` only | 2 | M | Fewer allocs on CJK/long-word prepare (needs profiling) |
| 9 | Fold the budget grapheme count into analysis | 7 | M | Removes one full-text pass on bounded path |

## Quick wins

- Finding 1: two-line change in `src/analysis/split.rs` (`mem::take` like
  `merge.rs:23`); covered by existing layout tests.
- Finding 3: one-line change each at `src/measurement.rs:168` and `:221`.
- Finding 6: rewrite `get_segment_metrics` as get-then-insert
  (`src/measurement.rs:96-111`); allocation contract tests already guard regressions.
- Finding 4: caller-side swap to `walk_line_ranges` in
  `crates/gpui-builder/src/solver/misc.rs:140-142`.
- Add a criterion case for `prepare` on a CJK/long-word corpus to quantify
  findings 2/8 before touching them (`benches/layout_temporaries.rs` currently
  benches only cache hits and tiny layouts).
