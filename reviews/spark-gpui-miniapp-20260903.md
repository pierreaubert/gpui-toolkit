# Code Review: gpui-miniapp — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-miniapp` (~1.7k LOC)

## 1. Purpose / role
Batteries-included `Application` template for examples/showcases: menu bar, theme toggle (Cmd+T), i18n menu, window sizing. Files: `mini_app.rs` (534), `tests.rs` (507), `mini_app_config.rs` (110), `mini_app_shell.rs` (71), `misc.rs` (148), `lib.rs` (54).

Public API: `MiniApp::run<V,F>` (`mini_app.rs:120`), `MiniAppConfig::new/with_*`, `MiniAppShell`, actions `Quit/ToggleTheme/SetLanguage{English,French,German,Spanish,Japanese}` (`lib.rs:32-43`); private `SetThemeVariant/SetDesignLanguage` (`mini_app.rs:15-83`).

## 2. SOTA gap analysis (vs mini-program runtimes, Electron BrowserWindow shell, Tauri bootstrap)
1. **Single-window only** — `open_window` once (`mini_app.rs:249-266`).
2. **No lifecycle hooks** (on_close/on_reopen/deep-link).
3. **No plugin/extension point** — `build_view` closure only.
4. **No persistence** (bounds, theme, locale not restored).
5. **No error boundary** — window failure is `eprintln!` + `quit()` (`:267-271`).
6. **Thin CLI** — only `--window-min-size` (`:509-527`).
7. **5 hardcoded languages** (`lib.rs:36-42`), no runtime locale loading.
8. **No telemetry/logging integration.**

## 3. Performance evaluation
No hotspots. 7× `config_rc.clone()` per action closure (`mini_app.rs:171,180,188,196,201,206,211,216`) — cheap `Rc` bumps, one shared `Rc` suffices. `MiniAppShell::render` clones `inner` every frame (`mini_app_shell.rs:68`). `refresh_menus` rebuilds full menu + `format!("Quit {}", app_name)` (`:335`) on every toggle (`:173-183`). `title.clone()` per launch (`:254`) trivial; `--window-min-size` error allocs cold (`:509-527`).

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Hoist single `config_rc` clone outside action registrations | S | noise |
| 2 | Share `MiniAppShell.inner` via `Rc`/handle (avoid per-frame clone) | S | frame alloc |
| 3 | Debounce `refresh_menus` — rebuild affected submenu only | S | toggle cost |
| 4 | Add `on_window_close` hook + `run_multi` | M | shell parity |
| 5 | Persist theme/locale/bounds (`dirs` + serde) | M | UX continuity |

## 5. Verdict
Right-sized starter shell. SOTA = multi-window, lifecycle, persistence. No perf risk.
