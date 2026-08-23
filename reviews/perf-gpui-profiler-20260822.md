# Perf review: gpui-profiler

Date: 2026-08-22

## Role and hot paths

`gpui-profiler` (3 source files, ~290 LOC: `src/lib.rs`, `src/alloc_count.rs`,
`src/global.rs`) is the workspace's allocation-counting toolkit. It has no
rendering, no GPU code, and no per-frame work of its own — its "hot path" is
*other* crates' hot paths:

- `AllocProbe::sample` / `AllocSnapshot::now` are called per render and per
  mouse/scroll/resize event in gpui-component-lab
  (`crates/gpui-component-lab/src/lab_ui/component_lab.rs:3685-3743`) and
  d3rs-showcase (`crates/gpui-d3rs/bin/showcase/main/showcase_app.rs:832-854`).
- `AllocationBudget` contract tests exist in 8 crates
  (`crates/*/tests/*allocation_contracts*.rs`).

Feature-gating is done right at the crate level: with `global-allocator` off
(the default), `AllocProbe` is a zero-sized struct
(`src/alloc_count.rs:110-113`) and `sample` returns `Default`
(`src/alloc_count.rs:155-158`) — genuinely free. When on, a counting
`#[global_allocator]` wraps the system allocator via `stats_alloc` 0.1.10
(`src/global.rs:9-10`, workspace `Cargo.toml:87`). All 7 consumers that enable
the feature do so via `[dev-dependencies]` (verified in gpui-px, gpui-ui-kit,
gpui-builder, gpui-audio-kit, gpui-keybinding, gpui-ios, gpui-pretext
Cargo.tomls), so release/production builds never install the counter.

## Findings

1. **[Alloc] The instrumented overlay allocates inside the path it measures.**
   `record_sample` stores `label.to_string()` per event
   (`component_lab.rs:585-588`; `showcase_app.rs:285-288`), and
   `render_alloc_overlay` runs 2–3 `format!` String allocations per render
   (`component_lab.rs:3516-3536`; `showcase_app.rs:693-713`). The render sample
   is taken *after* the overlay is built (`component_lab.rs:3741` → `3743`;
   `showcase_app.rs:852` → `854`), so with counting enabled the "render" delta
   permanently includes the overlay's own allocations — the overlay's red/green
   render signal can never go green. High impact on signal quality, trivial
   cost to fix.

2. **[Alloc] "Zero overhead when disabled" is broken at the call sites.**
   With the feature off the probe API is free, but `record_sample` still
   heap-allocates a `String` on every mouse-move/down/up/scroll/resize
   (`component_lab.rs:587`; `showcase_app.rs:287`) — contradicting README
   claims (`README.md:35-36`, `76-78`). All labels are string literals, so
   `Option<(&'static str, AllocSnapshot)>` removes this entirely. Per-event
   String churn in every non-profiler build of both showcase apps.

3. **[Alloc] Counting builds are not timing-representative.** With
   `global-allocator` on, every allocation/reallocation in every thread pays
   relaxed atomic counter updates (`src/global.rs:9-10`). Fine for QA contracts
   and the red-overlay workflow, but a `--features profiler` binary must not be
   used for wall-clock benchmarking (magnitude needs profiling; on alloc-bound
   loops allocator wrappers like this typically cost >10%). README.md:79-82
   already warns; the campaign should make this an explicit rule.

4. **[GPU]/[Roundtrip] None.** The crate contains no wgpu/GPU code and performs
   no readbacks, offscreen renders, or device polls. Nothing to move to GPU.
   No wasm hazard (no `device.poll`/`pollster`; relaxed atomics are fine on
   single-threaded wasm).

5. **[Alloc] Process-wide counters force a serialization discipline — currently
   held by convention, not enforcement.** Contract tests avoid cross-test
   pollution two ways: a single `#[test]` wrapping all measurements
   (`gpui-pretext/tests/allocation_contracts.rs:61-65`) or a static
   `Mutex` (`gpui-px/tests/mesh_plot_allocation_contracts.rs:19,27`). All 8
   contract files also skip under coverage via `CARGO_LLVM_COV`. Both patterns
   plus the coverage skip must be copied verbatim for every new contract, or
   results will be flaky — deltas include any background thread
   (`README.md:83-85`, `121-123`).

6. **[Alloc] Coverage gap vs the campaign's prime suspects.** No allocation
   contracts cover the known worst offenders from `reviews/20260822-vello.md`:
   d3rs `gpu2d::Chart2DElement` (readback roundtrip) and
   `vello2d::VelloChartElement` (per-element `vello::Renderer`/texture/pipeline
   creation). The tool exists but is not aimed at the code the campaign most
   needs to fix.

7. **[Alloc] Minor: duplicated no-op shim in d3rs-showcase.** Because
   gpui-profiler is an optional dep there (`gpui-d3rs/Cargo.toml:40`),
   showcase_app.rs:18-37 re-implements a local no-op `AllocProbe`/
   `AllocSnapshot` instead of using the crate's always-compiled zero-cost API.
   Drift risk; the crate is tiny and free when the feature is off, so the dep
   could just be non-optional. (The shim does not fix finding 2 — its
   `record_sample` still calls `to_string()`, showcase_app.rs:287.)

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | Store `&'static str` labels instead of `String` in both showcases' `record_sample` | 1, 2 | S | Removes per-event alloc; makes disabled-builds truly free |
| 2 | Sample render *before* building the overlay, or gate the overlay's `format!`s behind the `profiler` feature | 1 | S | Render red/green signal becomes meaningful |
| 3 | Add `AllocationBudget` contracts around d3rs `vello2d` scene build / paint prep and the `gpu2d` readback path | 6 | M | Puts the campaign's worst alloc/roundtrip offenders under executable budgets |
| 4 | Campaign rule + README note: never wall-clock benchmark a `profiler`-feature binary | 3 | S | Prevents distorted measurements campaign-wide |
| 5 | Document the two serialization patterns + `CARGO_LLVM_COV` skip as the required template for new contracts | 5 | S | Prevents flaky contracts as coverage grows |
| 6 | Make gpui-profiler a non-optional dep of d3rs-showcase and delete the shim | 7 | S | One source of truth for the probe API |

Per-crate campaign usage pattern (as established by gpui-pretext/gpui-px):
dev-dependency with `features = ["global-allocator"]`, warm caches/reserve
buffers, `probe.reset()`, run the steady-state op, then
`AllocationBudget::zero(name).assert_contains(probe.sample(name))` — in a
dedicated integration-test binary, serialized, coverage-skipped.

## Quick wins

- `&'static str` labels in `record_sample` (both showcases) — minutes, fixes
  findings 1+2 at the source.
- Reorder render sampling before overlay construction (or feature-gate the
  overlay text) so the render counter can actually read zero.
- Delete the d3rs-showcase no-op shim by making the dep non-optional.
- One-paragraph "how to add an allocation contract" template in README.md
  (patterns already exist in gpui-pretext and gpui-px to copy from).
