# Code Review: gpui-component-lab — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-component-lab` (46 files, ~12.3k LOC)

## 1. Purpose / role
Storybook/Ladle clone for GPUI: prop-driven story registry + interactive lab UI + conformance/visual-regression manifests. Largest: `lab_ui/component_lab.rs` (4260), `lib/validate.rs` (1023), `lib/tests.rs` (724), `lib/consts.rs` (703), `bin/gpui_component_lab.rs` (599), `lib/register.rs` (526).

Public API: `ComponentStory::new()->props()->metadata()->conformance()`, `StoryRegistry::{register,story,stories}` + `StoryRendererRegistry`, `register_{ui_kit,px,audio_kit}_stories` (`register.rs:11,161,425`), `run_lab_app(LabAppConfig)` + `ComponentLab` (`:339,364`), `validate_px_chart_conformance` (`validate.rs:99`), `VisualRegressionManifest::diff_visual_case` (`:331`), `StoryProp/StoryPropValue/ViewportPreset/ThemePreset/MotionPreset`.

## 2. SOTA gap analysis (vs Storybook 8, Ladle, Chromatic)
1. **No URL-routed deep links** — `misc.rs:354 id_fragment` is local-only, no shareable `?story=` links.
2. **No MDX/docs pages**, only `StoryDocument` struct.
3. **No controls auto-generation** (manual `render_prop_editor` at `component_lab.rs:1342`).
4. **macOS-gated capture** (`visual_capture.rs:114`) — no cloud snapshots / CI baseline diffing.
5. **No interaction/play tests** (Storybook `play` fns) — only allocation contracts.
6. **No a11y addon panel** (axe equivalent) despite conformance fields.
7. **No virtualized sidebar** — `component_lab.rs:418` clones all story ids per frame.

## 3. Performance evaluation
Coverage 6% (21/340). `component_lab.rs:2036 render_exported_ui_kit_component_story` 497 lines/fan-out 255/CRAP 5112; `:1475 render_layout_controls` 342 lines/8 loops; `register.rs:161 register_px_stories` fan-out 73 at startup. Per-frame allocs: `:418 stories().map(clone).collect()`, `:512 format!`, `:1349-1420 SharedString::new(clone)` per prop, `:2056-2346 vec![]` menu trees per render. `Mutex::lock().unwrap()` on cached story data (`lab_ui/types.rs:206`, `lab_ui/misc.rs:164,191,219,241`); 28 `unwrap` + 22 `expect` (mostly benign `writeln!` but `allocation_contracts.rs:59,63` in sample path).

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Break `:2036` by story family; snapshot registry instead of per-frame clone | M | removes 5k CRAP |
| 2 | Virtualize story list + cache `SharedString` prop labels | M | scroll perf |
| 3 | Replace `lock().unwrap()` with poison-tolerant recovery + test | S | crash safety |
| 4 | Add URL deep-linking (`?story=&props=`) before more widgets | M | highest Storybook leverage |
| 5 | Test `showcase_section_for_story_id` (risk 1113/cyclo 45) + `render_story_preview:1972` | S | risk cut |

## 5. Verdict
Closest thing to a docs-site in the workspace; needs routing, controls-gen, and CI visual diffing before widget count grows. Perf is per-frame registry churn.
