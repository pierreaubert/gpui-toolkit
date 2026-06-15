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

The instrumented showcase applications render a small in-UI overlay in the
top-right corner. It shows the last render and mouse-move deltas, plus the most
recent sample from any probed event (`mouse-down`, `mouse-up`, `scroll`,
`resize`, or `prop-change` in the component lab). The overlay turns red whenever
any of those samples allocated, giving you immediate visual feedback without
reading logs.

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
