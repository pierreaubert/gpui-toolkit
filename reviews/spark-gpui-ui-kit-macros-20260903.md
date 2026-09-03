# Code Review: gpui-ui-kit-macros — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-ui-kit-macros` (4 files, ~1.5k LOC)

## 1. Purpose / role
Proc-macro helpers for `gpui-ui-kit`: `ComponentTheme`, `ComponentBuilder`, `FormField` derives generating `Default` + `From<&Theme>` and builder setters. Files: `lib.rs` (50), `derive.rs` (~950), `builder_field.rs` (485), `misc.rs` (31), `tests/compile.rs`.

Public API: only 3 exports (`lib.rs:37-50`). Attributes: `#[theme(default=.., from=.., from_expr=..)]`, `#[theme_path]`, `#[gpui_path]`, `#[field(required/optional/into/default=/builder=/skip)]`.

## 2. SOTA gap analysis (vs MUI styled()/cva, shadcn variants, SwiftUI @ViewBuilder)
1. **No variant-prop macro.** No `cva()`-style variant/size codegen — each component hand-writes `*Variant/*Size` enums.
2. **No prop-table/doc generation** from `#[field]` metadata for showcase/Storybook parity.
3. **No a11y lint.** Cannot enforce `aria_label/aria_role` fields at compile time.
4. **No i18n-key exhaustiveness check** — missing language fails at test-time, not macro-time.
5. **No deprecation/rename migration helper** (`#[deprecated_variant]` → codemod).
6. **No memoization hints** (e.g. `#[memo(eq)]` for render caching).
7. **Thin UI-test coverage** — only `tests/compile.rs`, no `trybuild` snapshots for bad inputs (error UX itself is good: `derive.rs:9-16,26-48` span-accurate errors).

## 3. Performance evaluation
Compile-time only; zero runtime cost by design. No complexity outliers. `derive.rs:11` `.expect()` is an unreachable internal invariant; `builder_field.rs:307-310,321,323,337,339` `expect/unwrap` are inside `#[cfg(test)]` helpers only. Real cost is downstream: `ComponentTheme` expands per-component `From<&Theme>` cloning every color field, amplifying monomorphized code in `table.rs`-scale `fan_out:112` builds. No expansion-size benchmark exists.

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Add `#[variant]` macro generating cva-style variant/size matchers | M | kills boilerplate enums |
| 2 | Emit prop-docs JSON from `#[field]` for component-lab/showcase | S | Storybook parity |
| 3 | Compile-time check that themed structs cover all `Theme` fields | S | catches drift |
| 4 | Add `trybuild` UI tests for bad `#[theme]`/`#[field]` inputs | S | prevents regressions |
| 5 | Publish `cargo expand` snapshots so theme-derive bloat is visible pre-merge | S | guards hot `render()` sizes |

## 5. Verdict
Small, well-scoped crate. SOTA work is all additive codegen (variants, docs, lints). No runtime perf action needed beyond watching expansion bloat downstream.
