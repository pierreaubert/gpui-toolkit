# Code Review: gpui-hello-web — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-hello-web` (~79 LOC)

## 1. Purpose / role
Minimal wasm starter proving `gpui_miniapp` web boot; `just wasm-serve-hello` target. Files: `src/main.rs` (64), `tests/wasm_smoke.rs` (15). Binary-only, no `lib.rs`. Native `main` prints usage and returns `FAILURE` (`:52-56`); wasm `imp::start` does `web_init → current_platform().expect → Application::with_platform().run_embedded → open_window(640×560).expect → web_mark_ready → mem::forget(handle)` (`:27-49`). View is one quad + one text line (`HelloWeb: Render`, `:9-25`).

## 2. SOTA gap analysis (vs Vite/Next starters, trunk/dioxus/leptos templates)
1. **Single hardcoded view** — no routing, assets, env config.
2. **No HMR status UI** (relies on external `just` recipe + COOP/COEP headers).
3. **No error boundary** — `expect("web platform")` / `expect("failed to open window")` (`:30,44`) white-screens on WebGPU-less browsers.
4. **No loading/fallback UI** before `web_mark_ready()` (Chrome 113+/Safari 26+ only).
5. **No in-crate e2e** beyond `wasm_smoke.rs:9` platform-construct check.
6. **No PWA manifest/service-worker/offline story.**
7. **No theming/i18n hooks** unlike `run_showcase` (`MiniAppConfig::with_theme/with_i18n`).

## 3. Performance evaluation
Negligible — single static `div`, no paint loops, zero `unwrap`, 3× `expect` on boot path only. `mem::forget(handle)` (`:48`) is an intentional page-lifetime leak, correctly commented — but copy-paste into real apps leaks every handle. Fixed 640×560 window re-layouts on resize without debounce (inherited from miniapp). No per-frame allocs.

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Keep as-is; point real starters at `gpui-showcase`/`gpui-miniapp` | — | scope guard |
| 2 | Replace boot `expect`s with `console_error` + fallback `<div>` for non-WebGPU browsers | S | no white-screen |
| 3 | Comment `mem::forget` as page-lifetime-only | S | prevents leak copy-paste |
| 4 | Add query-param selection + resize-observer example | S | two most-copied needs |
| 5 | Assert `web_mark_ready` observed in smoke test (match `just wasm-visual hello`) | S | e2e parity |

## 5. Verdict
Correct as a spike. SOTA work belongs in miniapp/showcase templates, not here.
