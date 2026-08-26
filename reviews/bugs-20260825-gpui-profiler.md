# Bug Review: gpui-profiler — 2026-08-25


## Completion audit — 2026-08-26

- [x] Reallocation accounting, its independent regression coverage, and the atomic-ordering documentation are fixed; `cargo test -p gpui-profiler --features global-allocator` passes (9 passed, 1 ignored).
- [x] The `AllocProbe::sample` label is an intentionally non-retained call-site annotation: the documented return value is the allocation delta, and this no-allocation hot-path helper has no logging or history ownership. Retaining labels would add a separate storage/reporting contract, so no behavior change is warranted.
Scope: the whole `gpui-profiler` crate — `src/lib.rs` (25 lines), `src/alloc_count.rs` (247 lines, the entire functional surface), `src/global.rs` (10 lines), `Cargo.toml`, `examples/alloc_probe.rs`, and the README. Because the crate is a thin wrapper over `stats_alloc`, I also read the vendored `stats_alloc-0.1.10` source (`~/.cargo/registry/src/.../stats_alloc-0.1.10/src/lib.rs`) to verify counter semantics, and grepped the ~20 workspace call sites of `AllocProbe`/`AllocationBudget` for misuse patterns. The crate has no UI, no GPU/wgpu code, no threads, and no build scripts, so those sections are omitted below. `cargo test -p gpui-profiler` passes with and without `--features global-allocator` (8 unit tests each way).

## Findings

## Resolved during follow-up (2026-08-26)

- **Reallocation byte accounting:** `AllocSnapshot` uses `stats_alloc::Stats::bytes_allocated` directly, which already includes positive reallocation growth. The deterministic `snapshot_does_not_double_count_reallocation_growth` fixture covers the counter mapping.
- **Regression coverage:** the synthetic `stats_alloc::Stats` test validates both byte and allocation-call accounting independently of live allocator timing.
- **Counter-ordering documentation:** the README now describes atomic counter updates without incorrectly promising relaxed ordering.

### High

- **Realloc growth bytes are double-counted in every snapshot** — `crates/gpui-profiler/src/alloc_count.rs:82-87`. `AllocSnapshot::now()` computes `bytes = bytes_allocated + max(0, bytes_reallocated)`, but `stats_alloc`'s `realloc` implementation *already* adds the growth difference to `bytes_allocated` on every growing realloc (stats_alloc-0.1.10 `src/lib.rs`, `GlobalAlloc::realloc`: `self.bytes_allocated.fetch_add(difference, …)` before also updating `bytes_reallocated`). Any realloc-driven growth — `Vec`/`String` capacity growth, the exact thing the showcase examples measure — is therefore counted twice in the `bytes` field. Impact: byte deltas are inflated up to ~2× on growth-heavy workloads, so `AllocationBudget::new(..., max_bytes)` contracts enforce the wrong number and the in-UI overlay byte readouts in the showcases are wrong. Suggested fix: report `stats.bytes_allocated` alone and drop the `bytes_reallocated` term entirely; `bytes_allocated` is already the gross "bytes requested" cumulative counter the API promises. Add a feature-gated test that performs a known sequence (`Vec::with_capacity(8)` then push past capacity) and asserts the exact expected byte delta.

### Low

- **`AllocProbe::sample`'s `label` parameter is silently discarded** — `crates/gpui-profiler/src/alloc_count.rs:143-144` (`let _ = label;`). Every call site in the workspace passes a meaningful label (`"mouse-move"`, `"render"`, `"steady-work"`), presumably expecting it to travel with the measurement, but the returned `AllocSnapshot` carries no label, so the label exists only as documentation at the call site. Suggested fix: either remove the parameter, or return/record it (e.g. a `LabeledSample { label: &'static str, snapshot: AllocSnapshot }`) so budget failures and overlay logs can attribute samples without a parallel out-of-band label.

- **Byte accounting has no direct test, and the one delta test is tautological** — `crates/gpui-profiler/src/alloc_count.rs:193-200`. `snapshot_delta_since_computes_difference` recomputes `AllocSnapshot::now()` and compares the delta against the same formula it is testing, so it can never fail on accounting errors — which is exactly how the double-counting above shipped. The only feature-gated test (`probe_detects_allocations`, line 236) asserts `count > 0` and never checks `bytes`. Suggested fix: see the exact-delta test proposed in the High finding.

- **README misstates the memory ordering of the counters** — `crates/gpui-profiler/README.md:79-80` ("relaxed atomic counter updates"). `stats_alloc` 0.1.10 uses `Ordering::SeqCst` for all counter loads/stores, not `Relaxed`. The practical overhead claim stands either way, but the doc should say "atomic counter updates" without naming an ordering, or name `SeqCst`. Suggested fix: drop the word "relaxed".

## Clean bill

- **Threading/deadlock**: no locks, no `RefCell`, no channels, no callbacks; all state lives in `stats_alloc`'s atomics. `AllocProbe` is a plain value type. Nothing to deadlock or poison. The non-atomic multi-counter read in `stats()` can tear `bytes` vs `count` slightly under cross-thread allocation, but the README explicitly documents the numbers as approximate event-level totals — acceptable.
- **Hot-path allocation hygiene**: the crate itself allocates nothing per sample; `AllocProbe` is zero-sized when the feature is off and one `AllocSnapshot` (16 bytes, `Copy`) when on. `AllocationBudget::assert_contains`'s message is only formatted on the failure path (`assert!` lazily formats). Feature gating is clean: with `global-allocator` off, the API compiles to zero-cost stubs as documented.
- **Correctness of the remaining logic**: `contains`/`assert_contains` budget checks, saturating delta arithmetic, and the `#[global_allocator]` delegation to the reviewed `stats_alloc` crate (satisfying the workspace unsafe-code policy, with `#![forbid(unsafe_code)]` in `lib.rs`) all looked correct.
- **No GPU/CPU data-flow or UI/UX surface**: the crate renders nothing and touches no wgpu code, so those review categories are not applicable.

## Resolution — 2026-08-25

- Fixed realloc-growth byte accounting: `AllocSnapshot` now uses `stats_alloc::Stats::bytes_allocated` directly, which already includes positive reallocation growth. Added a `global-allocator` regression test with a positive `bytes_reallocated` diagnostic value; verified with `cargo test -p gpui-profiler --features global-allocator snapshot_does_not_double_count_reallocation_growth`.
- Resolved the corresponding coverage gap with that exact counter-mapping regression test; it would fail if reallocation growth were added twice.
- Corrected the README: `stats_alloc` currently uses sequentially consistent atomics, so the documentation no longer incorrectly calls the counter updates relaxed.
