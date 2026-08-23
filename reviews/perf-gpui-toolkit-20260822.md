# Perf review: gpui-toolkit

Date: 2026-08-22

## Role and hot paths

`crates/gpui-toolkit` is a **pure umbrella + release-QA metadata crate**. It has
two halves, neither of which is on any per-frame, per-event, or rendering path:

1. **Re-exports** (`src/lib.rs:113-150`): feature-gated `pub use` of every
   product crate (`gpui-ui-kit`, `gpui-d3rs`, `gpui-px`, `gpui-au`, …). The
   Cargo.toml (`crates/gpui-toolkit/Cargo.toml`) exists mainly to define the
   feature surface (`ui`, `audio`, `charts`, `themes`, `core`, `tooling`,
   `platform`, `all`) used by release QA. Note: despite the name, the Python
   wheel source is **not** here — it lives at
   `crates/gpui-python-runtime/python/gpui_toolkit` (see
   `scripts/build_python_package.py:15`). This crate is not the wheel.
2. **Release metadata** (`stability.rs`, `release_qa.rs`, `release_notes.rs`,
   `release_packaging.rs`, `publish_plan.rs`, `dependency_hygiene.rs`,
   `vendored_patches.rs`; ~3.7 kLOC total): `&'static` tables of gate/evidence
   records (e.g. `CRATE_STABILITY_MANIFEST` at `stability.rs:117`, the
   `VENDORED_PATCHES` table behind `vendored_patches()` at
   `vendored_patches.rs:660`) plus `to_markdown_table()` report builders.

The only allocation-bearing code is the Markdown report builders, and a repo-wide
search shows they are called **only by this crate's own unit tests** (e.g.
`release_qa.rs:594`, `release_qa.rs:693`, `dependency_hygiene.rs:444`,
`publish_plan.rs:333`, `release_packaging.rs:312`) and the lib.rs doctest —
no other crate, script, or binary invokes them. The only external reference to
`gpui_toolkit` (`crates/gpui-python-runtime/bin/showcase/python_ir_showcase.rs`)
is string literals naming Python modules, not this Rust crate.

**There is no meaningful runtime perf surface in this crate.** No GPU work, no
readbacks, no paint/layout/event loops, no hot data marshalling.

## Findings

1. **[Alloc] `push_str(&format!(...))` row building in report generators** —
   every `to_markdown_table()` builds each table row with a temporary `String`
   from `format!` and appends it (e.g. `release_qa.rs:103-114`,
   `release_qa.rs:114`, `vendored_patches.rs:105`, `publish_plan.rs:94`,
   `dependency_hygiene.rs:148`, `release_notes.rs:140`, `release_notes.rs:192`,
   `release_packaging.rs:93`). Impact: negligible — called only in tests,
   producing reports of tens of rows. Cited for completeness, not as actionable
   perf work. Using `write!(markdown, ...)` would remove the temporaries if the
   style ever gets copied into a hot path.

2. **[GPU]/[Roundtrip] None present.** No wgpu/vello/Metal usage, no
   `map_async`/`read_texture`/`device.poll` anywhere in the crate (verified by
   reading `Cargo.toml` — zero non-workspace rendering deps — and grepping the
   sources). The charts feature merely re-enables `gpui-d3rs`/`gpui-px`
   (`src/lib.rs:121-140`); the known gpu2d/vello2d issues live in those crates
   and are covered by their own reviews.

3. **[Build-time, not runtime] Umbrella feature fan-out.** The only real "cost"
   this crate can impose is compile time and transitive feature unification:
   `all` pulls every crate including `gpui-au`/`gpui-ios`, and `charts` enables
   `gpui-d3rs`/`gpui-px` with `default-features = true`
   (`crates/gpui-toolkit/Cargo.toml`), which re-enables the WGPU-backed defaults
   noted in `src/lib.rs:11-12`. Not a runtime perf issue, but downstream crates
   depending on `gpui-toolkit` with broad features will pay for compiling (and
   linking) GPU stacks they may not use. (Needs profiling — a `cargo build
   --timings` comparison — to quantify.)

## Recommendations

| Action | Finding | Effort | Payoff |
|---|---|---|---|
| No runtime perf action required for this crate; redirect campaign effort to `gpui-d3rs` (gpu2d readback, per-element vello renderer) and `gpui-px` | 2 | — | High (elsewhere) |
| Optional: depend on product crates directly instead of `gpui-toolkit` features in binaries, to avoid compiling unneeded GPU/platform stacks | 3 | S | Lower build times (needs profiling) |
| Optional style fix: `write!` into the buffer instead of `push_str(&format!(...))` in `to_markdown_table()` | 1 | S | Cosmetic |

## Quick wins

None worth doing inside this crate. The only <1-day item is the
`push_str(&format!(...))` → `write!` tidy-up (finding 1), and even that is
style, not performance.
