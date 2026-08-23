# Perf review: gpui-scaffolder

Date: 2026-08-22

## Role and hot paths

`gpui-scaffolder` (`crates/gpui-scaffolder`) is a build-time codegen CLI, not a
runtime crate. `src/main.rs:28-49` parses clap args and calls
`scaffold_app` once; `src/lib.rs:52-199` (`scaffold_app`) creates ~6
directories and writes ~21 template files (Cargo.toml, Justfile, Rust sources,
Xcode/Swift, Gradle/Android files) via one-shot `fs::write` calls
(`src/lib.rs:263-265`). The remainder of `lib.rs` (~1400 lines) is pure
`format!`-based template builders plus unit tests. There is no event loop, no
per-frame paint/layout, no GPU interaction, no data marshalling — the entire
program does a few dozen small allocations and ~27 filesystem syscalls, then
exits.

`Cargo.toml` has no benches, no criterion, no runtime deps beyond `anyhow` +
`clap` (features `gpu-2d`/`gpu-3d`/`tokio`/etc. are empty feature stubs for
scaffold variants, not code). No TODO/FIXME in the crate, no references in
`qa/perf` or `docs/`.

## Findings

This crate has **no meaningful perf surface**. It should be excluded from the
perf campaign's GPU/roundtrip/alloc optimization goals. Recorded for
completeness:

1. [Alloc] One-shot template allocation — `scaffold_app` builds each of ~21
   files as a fresh `String` via `format!` (e.g. `cargo_toml` at
   `src/lib.rs:450-507`, `justfile` at `src/lib.rs:529+`) and writes them with
   `fs::write` (`src/lib.rs:263-265`). Total allocation volume is on the order
   of tens of KB, executed exactly once per CLI invocation. Impact: none.

2. [Alloc] `preview_scaffold` clones `ScaffoldOptions` to force `dry_run`
   (`src/lib.rs:205`) and collects ~21 `PathBuf`s into a `Vec`
   (`src/lib.rs:214-247`). Runs once per preview call. Impact: none.

3. [GPU/Roundtrip] No GPU usage, no wgpu dependency, no readback/poll code
   anywhere in the crate (verified by reading all of `src/`). N/A.

## Recommendations

| Action | Finding | Effort | Payoff |
|---|---|---|---|
| Exclude `gpui-scaffolder` from the perf campaign scope; do not add benches or alloc profiling here | 1–3 | S | Avoids wasted effort |
| (Optional, hygiene only) If `preview_scaffold` is ever called in a loop by a GUI, pass `dry_run: true` by constructing options directly instead of cloning — but no such caller exists today | 2 | S | Negligible |

## Quick wins

None. There is nothing landable here that would measurably change any
performance metric; the correct outcome of this review is a one-line exclusion
note in the campaign tracker.
