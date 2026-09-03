# Code Review: gpui-scaffolder — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-scaffolder` (~2.2k LOC)

## 1. Purpose / role
`create-next-app` for GPUI+iOS/Android: single-purpose generator emitting desktop+iOS+Android app. Files: `lib.rs` (2071), `main.rs` (50). Pins `GPUI_VERSION 0.2.2`, `GPUI_ZED_TAG v1.9.0` (`lib.rs:8-9`); Java payloads via `include_str!` (`:15-17`).

Public API: `ScaffoldOptions {name,output_dir,force,dry_run}` (`:20`), `ScaffoldedApp {app_dir,package_name,title}` (`:28`), `ScaffoldPreview {app,files}` (`:37`), `scaffold_app()` (`:55`), `preview_scaffold()` (`:206`), CLI `--force/--dry-run`.

## 2. SOTA gap analysis (vs create-next-app, cargo-generate, create-tauri-app)
1. **One hardcoded template** — no registry, `--template`, or remote templates.
2. **No interactive prompts** (name/version/author/license).
3. **No conditional features** (iOS/Android always emitted, no `--no-mobile`).
4. **No version resolution** — pin is a const, no update check.
5. **No git init / post-install hooks** (`cargo build`, `xcodegen`).
6. **No migration path** — `gpui-scaffold.toml` (`:121-124`) written but never read for upgrades.
7. **Divergent dry-run** — `dry_run` returns `ScaffoldedApp` only; file list via separate `preview_scaffold` (can diverge).
8. **No example gallery / success reporting.**

## 3. Performance evaluation
Single-file CLI, no graph hotspots. ~20 sequential `fs::write` (`:109-192`) — fine one-shot but syscall-heavy. `write_file` is non-atomic (`:295-297`) — crash leaves half-written project (contrast `gpui-design-tools` atomic+fsync). 6× `relative_path()` ancestor walks (`:99-107`). `include_str!` of two Java files (`:15-17`) bloats every binary incl. `--help`. `replace_directory` lists twice (`:253`, `:269-275`) + file-by-file unlink (`:259`).

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Atomic temp+rename in `write_file` (copy design-tools pattern) | S | crash safety |
| 2 | Compute `relative_path` once, join thereafter | S | micro-cost |
| 3 | Gate Java payloads behind `mobile` feature / lazy include | S | binary size |
| 4 | Add `--template` + `--no-ios/--no-android` before template count grows | M | flexibility |
| 5 | `scaffold_app` consumes `preview_scaffold` plan so dry-run ≡ apply (`:55` vs `:206`) | S | correctness |

## 5. Verdict
Does one job; needs templates/flags/atomicity before it becomes the default starter. No runtime perf concern.
