# Code Review: gpui-showcase — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-showcase` (55 files, ~7.6k LOC + wasm glue)

## 1. Purpose / role
Runnable gallery of ~45 `gpui-ui-kit` sections + release/visual-capture manifests; thin binary over `Showcase` model. Largest: `showcase.rs` (1162), `release_artifacts.rs` (560), `sections/render_form.rs` (604), `sections/render_layout.rs` (549), `sections/render_thinking_orb.rs` (401). Intentionally tiny public API: `Showcase::new/update/handle_key_down/render/section_header` (`showcase.rs:288,600,701,794,827`), `run_showcase()` (`lib.rs:16`, `MiniAppConfig` 1200×900), CLI `--release-artifacts|--visual-manifest|--window-min-size` (`main.rs:12-37`).

## 2. SOTA gap analysis (vs Storybook/Chromatic docs sites)
1. **No global search/command-palette** (only `render_command_palette.rs:49` demo, unwired).
2. **No deep-linkable routes** — `cached_navigation_id` (`:22`) + 45-branch `render_section_content:600` resets on restart.
3. **No prop knobs** — `render_form.rs:5` (599 lines/fan-out 121) is static markup.
4. **No visual-diff CI gate** — `release_artifacts.rs:379` hand-rolled JSON, manifests never asserted in-app.
5. **No i18n/theme matrix preview** (single theme at a time).
6. **Weak keyboard nav** despite `handle_key_down:701` (cyclo 30).
7. **Mobile shells (`android/ios/tvos/`) duplicate entry points** with no shared nav state.

## 3. Performance evaluation
Coverage 2% (4/198). `render_form.rs:5` 599 lines/fan-out 121, `render_layout.rs:4` 418 lines/fan-out 124, `render_progress.rs:4` fan-out 146 — hundreds of elements built unconditionally; `showcase.rs:600` 45 branches + `:827` nesting 7/4 loops. Per-frame strings: `showcase.rs:30-38 format!("nav-{section:?}")` + collect per nav item, `render_form.rs:151`, `render_thinking_orb.rs:175,177,230,262,294,323,360` per frame. `render_qr.rs:56,67 .expect()` panics whole gallery on QR failure; `showcase.rs:733,777 chars().next().unwrap()` panics on empty key.

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Lazy-render only active `ShowcaseSection`; split `render_form/render_layout` by widget | M | startup + frame cost |
| 2 | Precompute nav IDs once in `new` instead of per-frame `format!+collect` | S | nav allocs → 0 |
| 3 | Replace `expect/unwrap` in render/key paths with fallback UI | S | crash safety |
| 4 | Replace hand JSON (`release_artifacts.rs:130-161,379`) with `serde_json` | S | correctness |
| 5 | Add interaction tests for `showcase_group.rs:135`, `showcase.rs:600/794/701` | S | 2% coverage is the risk |

## 5. Verdict
Demo app, not a library — keep it thin. Highest leverage is lazy sections + routing/search so it can stand in for Storybook until component-lab matures.
