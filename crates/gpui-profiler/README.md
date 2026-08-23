# gpui-profiler

Lightweight allocation profiling utilities for GPUI applications.

The main goal is to verify that interactive UI operations — moving, resizing,
clicking, repainting — do not allocate heap memory after the initial app build.

## Usage

Add the crate as a dependency and enable the `global-allocator` feature when
you want to measure allocations:

```toml
[dependencies]
gpui-profiler = { path = "../gpui-profiler" }

[features]
profiler = ["gpui-profiler/global-allocator"]
```

Wrap the work you want to measure with `AllocProbe`:

```rust
use gpui_profiler::AllocProbe;

let mut probe = AllocProbe::new();
// ... work ...
probe.sample("after-update");
```

When the `global-allocator` feature is enabled, `sample` returns the allocation
delta since the last sample or reset. It does **not** print to stderr; callers
surface the delta however they want.

When the feature is disabled, the API is still available but reports zeros, so
instrumented code can be left in place with no overhead.

## Safe enable/disable pattern

Keep `AllocProbe` instrumentation in the code you care about, but expose the
counting allocator through an opt-in feature on the final binary:

```toml
[dependencies]
gpui-profiler = { path = "../gpui-profiler" }

[features]
profiler = ["gpui-profiler/global-allocator"]
```

Then run ordinary builds without the feature, and profiling builds with it:

```bash
cargo run -p your-app
cargo run -p your-app --features profiler
```

Only the profiling build installs the counting global allocator. This matters
because Rust allows a binary to define only one `#[global_allocator]`; do not
combine `gpui-profiler/global-allocator` with other allocator/profiler crates
such as `dhat` in the same binary.

The crate includes a runnable minimal example:

```bash
cargo run -p gpui-profiler --example alloc_probe
cargo run -p gpui-profiler --example alloc_probe --features global-allocator
```

The first command keeps the probes compiled in but reports zero allocations.
The second command enables counting and should show allocations for the vector
growth section.

## Overhead expectations

- Without `global-allocator`, `AllocSnapshot::now`, `AllocProbe::reset`, and
  `AllocProbe::sample` return zero/default snapshots and do not install a
  global allocator.
- With `global-allocator`, every allocation and reallocation performs relaxed
  atomic counter updates. This is useful for QA and interactive profiling, but
  it should not be enabled in release hot paths unless you intentionally want
  diagnostic overhead.
- Samples are event-level approximations. Other threads can allocate between a
  probe reset and sample, so treat deltas as a signal for regressions rather
  than an exact frame budget.

The instrumented showcase applications render a small in-UI overlay in the
top-right corner. It shows the last render and mouse-move deltas, plus the most
recent sample from any probed event (`mouse-down`, `mouse-up`, `scroll`,
`resize`, or `prop-change` in the component lab). The overlay turns red whenever
any of those samples allocated, giving you immediate visual feedback without
reading logs.

> Do not use a binary built with `global-allocator` or a `profiler` feature for wall-clock benchmarking. The global allocator records every allocation with atomic updates, so timing results include diagnostic overhead. Use an ordinary release build for latency or throughput benchmarks.

## Integration with gpui-component-lab

The component lab is already instrumented. Run it with the `profiler` feature
to see allocation reports for render and mouse-move events:

```bash
cargo run --bin gpui-component-lab --features profiler
```

## Integration with d3rs-showcase

The d3rs showcase is also instrumented. Run it with the `profiler` feature to
see allocation reports for render and mouse-move events:

```bash
cargo run --bin d3rs-showcase --features profiler
```

Any non-zero allocation count during those events is a signal that the hot
path is still allocating.

## Implementation notes

- `gpui-profiler` installs a counting `GlobalAlloc` wrapper when the
  `global-allocator` feature is enabled.
- Only one crate in a binary can define `#[global_allocator]`, so this feature
  must not be combined with other global allocators such as `dhat`.
- The counters are atomics, so allocation counting is thread-safe but not
  synchronized with `sample`; treat the numbers as approximate event-level
  totals rather than exact frame budgets.

## Allocation-contract test template

Allocation counters are process-wide. Contract tests must serialize their measured section and skip under coverage, where instrumentation and parallel work make deltas unreliable. Use either one dedicated integration test containing every allocation assertion, or protect each measured test with the same static `Mutex`.

```rust
if std::env::var_os("CARGO_LLVM_COV").is_some() {
    return;
}

static ALLOCATION_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
let _lock = ALLOCATION_TEST_LOCK.lock().expect("allocation test lock");
```

Warm caches and reserve buffers before `probe.reset()`; only then sample the steady-state operation. Keep the lock for the full reset → operation → assertion interval.

## Allocation budgets

Use `AllocationBudget` to turn a warmed-up hot-path expectation into an
executable contract:

```rust
use gpui_profiler::{AllocProbe, AllocationBudget};

let mut probe = AllocProbe::new();
// Warm caches and reserve reusable buffers first.
probe.reset();
// Run the steady-state operation.
AllocationBudget::zero("meter-update").assert_contains(probe.sample("meter-update"));
```

For operations that intentionally grow bounded state, use
`AllocationBudget::new(name, max_count, max_bytes)`. Budgets must document the
input size and warm-up assumptions in the owning crate's test.
