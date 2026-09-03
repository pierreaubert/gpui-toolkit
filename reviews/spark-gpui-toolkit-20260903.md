# Code Review: gpui-toolkit (facade) — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-toolkit` (~3.8k LOC)

## 1. Purpose / role
Aggregate facade + release-QA metadata: single-dep re-export (`ui/audio/charts/themes/tooling/platform/ios/all` features) plus compile-time release gates. Files: `vendored_patches.rs` (877), `release_qa.rs` (707), `release_notes.rs` (650), `dependency_hygiene.rs` (458), `publish_plan.rs` (342), `stability.rs` (331), `release_packaging.rs` (324), `lib.rs` (150).

Public API (25 re-exports, `lib.rs:75-150`): `crate_stability_manifest()`, `release_qa_matrix()/platform_capability_matrix()` (`release_qa.rs:96-102`), `dependency_hygiene_report()`, `publish_plan()/entries()`, `release_notes_report()/artifact_report()`, `release_packaging_report()`, `vendored_patch_manifest()/patches()`; feature-gated `pub use gpui_{au,audio_kit,builder,component_lab,d3rs,design,design_tools,ios,keybinding,miniapp,pretext,profiler,px,python_runtime,scaffolder,themes,ui_kit,macros}`. Doctest (`lib.rs:56-64`) asserts `!all_passed()`/`!all_release_ready()` — ships knowingly unreleasable.

## 2. SOTA gap analysis (vs Tauri/Electron app-shell)
1. **No window/shell runtime** — no `BrowserWindow`, tray, menu-bar, multi-window state.
2. **No IPC/command bridge** (Tauri `invoke`, Electron `ipcMain`) — static reports only.
3. **No auto-updater, protocol handlers, single-instance lock, deep-link routing.**
4. **No crash-reporting/telemetry, installer/signer pipeline** — `release_packaging.rs:216` "evidence not recorded", blocking (`:310`).
5. **No plugin sandbox** — re-exports `python_runtime` with no capability permissions model.
6. **No asset bundler/sidecar packaging** — `publish=false` (`Cargo.toml:11`); mobile features need target QA (`lib.rs:17-20`).
7. **No gate enforcement** — `platform_capability_matrix()` separates declared vs executed evidence but nothing gates `cargo publish`; `release_qa.rs:557` only checks completeness.

## 3. Performance evaluation
Trivially cheap (max complexity 50, MI 46–52). `vendored_patches.rs:774 covered.insert(name.clone())` per walk entry — intern `&'static str`. `release_notes.rs:133,185` iterator filters re-run per doctest — cache `OnceLock<Vec>`. `dependency_hygiene.rs:148` / `release_qa.rs:102` string-concat tables rebuilt per CI invocation — pre-allocate. Zero blocking-call risk (only `is_blocking` status predicates). Perf work here is wasted vs shell-runtime gaps.

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Split facade vs `gpui-release-gates` — `all` feature pulls AU+iOS into desktop builds | M | dependency hygiene |
| 2 | Make gates executable: wire `platform-installers` + dry-runs (`release_notes.rs:600-609`) to real publish/installer scripts | M | honest release signal |
| 3 | `#[cfg(feature)]` compile test per aggregate (`ui/audio/charts/themes/tooling/platform/ios/all`) | S | feature-matrix safety |
| 4 | Drop `name.clone()`s; return `&'static` slices from `CRATE_STABILITY_MANIFEST` | S | micro-allocs |
| 5 | Decide publish story: remove `publish=false` + fix `gpui_wgpu` git blocker or document as internal meta-crate | S | roadmap clarity |

## 5. Verdict
Useful aggregator, misleading release story. Either make gates executable or split the facade from QA metadata.
