# Code Review: gpui-profiler — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-profiler` (524 LOC total)

## 1. Purpose / role
Allocation-count probe around `stats_alloc` for GPUI hot paths. Files: `lib.rs` (25), `alloc_count.rs` (267), `global.rs` (10), `examples/alloc_probe.rs` (43), `README.md` (159).

Public API: `AllocSnapshot {bytes,count}` (`alloc_count.rs:13`), `now()` (`:78`), `delta_since()` (`:100`), `AllocProbe::new/sample/reset` (`:126-147`), `AllocationBudget::zero/new/contains/assert_contains` (`:37-71`). Without `global-allocator` feature everything returns zeros (`:83-86`).

## 2. SOTA gap analysis (vs tracing/tracing-subscriber, Perfetto, pprof, Tracy)
1. **No spans/events/flamegraph** — 2 cumulative counters only.
2. **No thread-local attribution** — one global (`global.rs:9`) across threads.
3. **No sampling, wall-time, or CPU counters.**
4. **No Chrome-Trace/Perfetto export.**
5. **No dealloc/realloc breakdown** — `from_stats` folds realloc into count (`:90-97`).
6. **No peak/RSS/high-water mark.**
7. **No async-task attribution.**
8. **`sample(label)` ignores `label`** (`:148`) — no named series (trap).

## 3. Performance evaluation
Trivially small, no hotspots. `now()` is an atomic-counter read (`:78-81`) — fine at test granularity, noisy per-frame. `delta_since` re-reads `now()` internally (`:100-106`) — hidden global read. `INSTRUMENTED_SYSTEM` intercepts every alloc when enabled (`global.rs:9-10`) — must stay opt-in (it is). Failure-path `format!` only — fine.

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Record `label` in snapshot or remove the param | S | API honesty |
| 2 | Add `peak_bytes` via max tracking | S | high-water visibility |
| 3 | Add thread-local / per-thread probe feature | M | attribution |
| 4 | Add Chrome-Trace/CSV exporter | M | Perfetto parity |
| 5 | Split `count` into allocs vs reallocs (`:95`) | S | accuracy |

## 5. Verdict
Correct minimal probe; needs labels, peaks, thread attribution, and an exporter to be SOTA-adjacent. No perf risk — it *is* the perf tool.
