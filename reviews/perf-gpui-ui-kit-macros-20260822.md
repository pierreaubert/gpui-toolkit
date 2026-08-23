# Perf review: gpui-ui-kit-macros

Date: 2026-08-22

## Role and hot paths

`gpui-ui-kit-macros` is a pure proc-macro crate (`Cargo.toml:24-25`,
`proc-macro = true`). It exports three derive macros — `ComponentTheme`,
`ComponentBuilder`, `FormField` (`src/lib.rs:37-50`) — that expand into
`Default`/`From<&Theme>` impls and fluent builder setters for
`gpui-ui-kit` components.

**It has no runtime code path at all.** There is no per-frame paint, no
event handling, no data marshalling, no GPU interaction. The generated
code (plain struct literals and field copies, `src/derive.rs:552-580`,
`src/builder_field.rs:174-222`) is allocation-free and compiles down to
the same machine code as hand-written boilerplate. All three campaign
goals (GPU offload, roundtrip elimination, allocation reduction) are
**out of scope for this crate**: there is nothing to move to the GPU,
nothing reads anything back, and the macro itself only runs on the
compile server.

The only legitimate perf dimension is **compile time**: how much the
crate costs to build (its `syn` dependency tree) and how much macro
expansion costs when compiling `gpui-ui-kit` (~34 files use these
derives; 69 references counted in `crates/gpui-ui-kit/src`).

## Findings

1. **[Alloc/compile-time] `syn` compiled with `full` feature for a
   derive-only use case** — workspace `Cargo.toml:210`:
   `syn = { version = "2", features = ["full", "parsing", "extra-traits"] }`.
   This crate only ever parses `DeriveInput`, field `Meta` lists, and
   standalone `syn::Expr`/`syn::Type`/`syn::Path` from string literals
   (`src/derive.rs:212,262,273,470,522`; `src/builder_field.rs:40,98`;
   `src/misc.rs:3-17`). None of that needs `full` (complete item/file
   parsing) or `extra-traits` (Debug/Eq impls on all syntax nodes).
   Impact: `full` roughly doubles syn's own compile time; since every
   proc-macro crate in the workspace shares this dependency spec, the
   feature unification inflates all of them. The win is in workspace
   build time, not macro expansion time. **Verified caveat:** `cargo
   tree -e features -i syn@2.0.119` shows `syn feature "full"` is also
   requested by `bindgen v0.71.1`, `cbindgen v0.28.0`, and the vendored
   `derive_refineable` proc-macro
   (`crates/3rdparties/derive_refineable`) — feature unification means
   dropping `full` from this crate (or the workspace spec) alone will
   **not** remove it from the build graph. Any win requires touching
   `derive_refineable` too, which is vendored upstream GPUI code.

2. **[Alloc/compile-time] Per-expansion `syn::parse_str` on expression
   strings is negligible** — each `from_expr`/`default_expr` string
   literal is re-parsed as a `syn::Expr` at expansion time
   (`src/derive.rs:470-479,521-531`). This is a handful of tiny parses
   per derive invocation; with ~34 consumer sites the total cost is in
   the microsecond-to-millisecond range per clean build. Not worth
   changing; listed only for completeness.

3. **[compile-time] Expansion output is small and does not grow
   downstream compile time meaningfully** — `ComponentTheme` emits four
   small impls (`src/derive.rs:552-580`), `ComponentBuilder` emits one
   `new` plus one setter per field (`src/derive.rs:649-659`). No
   generics explosion, no heavy trait machinery in generated code. No
   action needed.

4. **No GPU / roundtrip findings** — the crate contains no wgpu, no
   rendering, no I/O. Confirmed by reading all four source files
   (`src/lib.rs`, `src/derive.rs`, `src/builder_field.rs`, `src/misc.rs`)
   and `Cargo.toml:27-30` (deps are only proc-macro2/quote/syn).

5. **No existing perf infrastructure for this crate** — no TODO/FIXME
   in the crate, no criterion benches, no allocation-count tests. Tests
   (`src/derive.rs:664-879`, `src/builder_field.rs:225-339`,
   `tests/compile.rs`) are correctness-only, which is appropriate.

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | No action on syn features: `cargo tree -e features -i syn@2.0.119` confirms `full` is independently forced by bindgen/cbindgen/derive_refineable, so per-crate narrowing here changes nothing in the unified build | 1 | — | — |
| 2 | Do nothing. This crate is not a perf lever for the campaign; the generated code is allocation-free by construction. | 1–5 | — | — |

## Quick wins

- None. The one plausible compile-time lever (narrowing syn features)
  was checked and is neutralized by feature unification with
  bindgen/cbindgen/`derive_refineable`. The crate's runtime footprint
  is zero by construction.
