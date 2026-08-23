# Perf review: gpui-miniapp

Date: 2026-08-22

## Role and hot paths

`gpui-miniapp` is a thin application shell for examples/showcases: it picks the
per-OS GPUI platform backend (`current_platform`, `src/misc.rs:11`), registers
global actions (quit/theme/i18n/design-language), builds the menu bar, opens one
window wrapping the caller's view in a `MiniAppShell`, and runs the app
(`src/mini_app.rs:117-305`). It renders nothing itself beyond a two-`div`
wrapper (`src/mini_app_shell.rs:99-119`).

Per-frame surface: exactly one `Render::render` on `MiniAppShell`, producing two
`div`s around the inner `AnyView` (`src/mini_app_shell.rs:107-118`). All other
code runs once at startup or once per menu click (language/theme actions).

**Bottom line: this crate has no meaningful perf surface.** There is no wgpu
usage, no readback, no scene/geometry work, no per-frame data marshalling. The
findings below are all negligible-impact; the report exists to confirm that
audit result, not to drive a fix campaign. The declared `gpu-2d`/`gpu-3d`
features (`Cargo.toml:27-28`) have no code behind them in this crate — they are
demos-facing feature flags only, not an unused GPU path here.

## Findings

1. **[Alloc] Pointless thread_local cache around a no-op computation** —
   `MiniAppShell::content_size` caches `(width, height) -> (width, height)`
   through `CONTENT_SIZE_CACHE: RefCell<Option<ContentSizeCache>>`
   (`src/mini_app_shell.rs:25-27`, body `src/mini_app_shell.rs:60-79`). The
   cached computation is a tuple identity (`src/mini_app_shell.rs:70`), so the
   cache's thread_local + RefCell overhead plausibly exceeds the "work" it
   avoids. Impact: none measurable; it's dead weight, not a cost center. (Test
   coverage for the cache behavior exists at `src/tests.rs:335-353`.)

2. **[Alloc] Trivial per-frame element/AnyView churn** — each render clones the
   inner `AnyView` and constructs two `div`s (`src/mini_app_shell.rs:117`,
   `src/mini_app_shell.rs:107-115`). `AnyView::clone` is Arc-cheap and GPUI
   element construction allocates nothing retained past paint. Impact:
   negligible; noted only for completeness.

3. **[Alloc] Full menu rebuild on every language switch** —
   `build_menus_with_language` (`src/mini_app.rs:308-384`) allocates a fresh
   `Vec<Menu>`, ~30 `MenuItem`s, and `format!("Quit {}", config.app_name)`
   (`src/mini_app.rs:315`) on each `SetLanguage*` action
   (`src/mini_app.rs:186`, `src/mini_app.rs:200`, `src/mini_app.rs:214`,
   `src/mini_app.rs:228`, `src/mini_app.rs:242`). Runs once per user click, and
   only when `with_i18n` is enabled. Impact: negligible (~1 µs scale); a
   language switch is not a hot path.

4. **[Alloc] Repeated `cx.refresh_windows()` after global mutations** — theme,
   design-language, and language actions each call `cx.refresh_windows()`
   (`src/mini_app.rs:162`, `src/mini_app.rs:188`, `src/mini_app.rs:398`,
   `src/mini_app.rs:405`). This repaints all windows, but with a single window
   and user-initiated events this is the correct GPUI idiom, not churn.

5. **[GPU/Roundtrip] None found** — the crate never touches wgpu: no
   `map_async`, `read_texture`, `device.poll`, `pollster`, or offscreen-render
   pattern anywhere in `src/` (grep over the crate confirms; rendering is 100%
   GPUI's own backend). No GPU opportunity exists at this layer: the shell's
   content is plain `div()` layout, already handled by GPUI's Metal/renderer
   backends selected in `misc.rs`.

6. **[Startup] `std::mem::forget(app.run_embedded(launch))` on wasm** —
   `src/mini_app.rs:301` intentionally leaks the application handle for the
   page lifetime. Documented in the comment; one-time and by design, but worth
   knowing it's a permanent (page-lifetime) allocation, not a leak bug.

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | None required — exclude this crate from the perf campaign's active work list | all | — | avoids wasted effort |
| 2 | (Optional cleanup) remove `CONTENT_SIZE_CACHE` and return `(width, height)` directly; keep the tests but drop the cache assertions | 1 | S | code clarity, not speed |

No action is expected to move any measurable performance metric. If the campaign
wants numbers anyway, wrapping a demo in `gpui-profiler` will show allocations
attributed to this crate are dominated by the wrapped view, not the shell.

## Quick wins

- None that matter for performance. The only <1-day item is the cosmetic
  cache removal in finding 1, which is a simplification rather than a speedup.
