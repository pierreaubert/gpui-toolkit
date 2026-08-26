# Bug Review: gpui-miniapp — 2026-08-25

Scope: full scan of `crates/gpui-miniapp` — all six source files under `src/`
(`lib.rs`, `mini_app.rs`, `mini_app_shell.rs`, `mini_app_config.rs`, `misc.rs`,
`tests.rs`, ~1,293 lines total of which 441 are tests), plus `Cargo.toml` and
`README.md`. Cross-checked claims against the vendored GPUI sources in
`crates/3rdparties/gpui` (keystroke parsing/matching, `MenuItem` shape,
`Window::bounds`) and against `gpui-design` / `gpui-ui-kit` globals the shell
installs. The crate is a small application shell: platform-backend selection,
global state installation, menu bar, key bindings, one window. It contains no
wgpu/rendering code, no threads, no locks, and no I/O beyond
`std::env::args` and (wasm) `web_sys` query-string reads, so the GPU,
threading, and allocation-hot-path categories are largely not applicable.

## Findings

## Resolved during follow-up (2026-08-26)

- **Cross-platform keyboard shortcuts:** Quit and Toggle Theme now bind `secondary-q` / `secondary-t`, which GPUI maps to Command on macOS and Control on Windows/Linux. `keyboard_shortcuts_use_platform_secondary_modifier` locks the shortcut contract.
- **gpui-builder compatibility:** MiniAppShell's test-only solved-layout fixture now supplies zero divider size for its root and single content slot, matching the current `SolvedNode` contract.
- **Menu action state and language callbacks:** menu construction receives the current theme, design language, and locale, marking exactly the active action checked. Theme and design actions rebuild menus after changing state; all five language actions now share one mutation/rebuild helper. `menus_mark_the_active_theme_design_and_language` covers the checked state.
- **Window-open failure:** a failed native `open_window` now requests application quit after reporting the error, preventing a windowless background process.
- **Partial menu-title localization (disproved):** MiniApp is a deliberately compact shell, not the UI Kit localization surface. Its language selector demonstrates state switching while the app, view, theme, and design menu names remain stable product terminology; no completeness promise exists in the public API.
- **Browser query decoding:** wasm query values now decode form-style pluses and percent-encoded UTF-8, rejecting malformed escapes rather than passing a partial value into initial configuration. `query_components_percent_decode_utf8_and_reject_malformed_escapes` covers valid and invalid inputs.
- **No-op feature aliases (disproved):** MiniApp intentionally accepts the workspace-wide feature vocabulary so package-targeted and all-feature QA invocations do not need special casing. The aliases add no dependencies or runtime behavior, and `cargo check -p gpui-miniapp --all-features` verifies the compatibility surface remains valid.
- **Invalid native CLI configuration:** an invalid `--window-min-size` now reports its parse error and exits with status 2, making launch failure visible to scripts and CI. wasm keeps the existing non-terminating behavior.
- **Platform-aware theme accelerator label:** macOS retains the accurate `Cmd+T` menu hint; other platforms replace it with the plain action label because the shared `secondary-t` binding maps to Ctrl there. Menu tests assert the correct per-platform item list.

### Medium

1. **`cmd-q` / `cmd-t` key bindings never fire on Linux and Windows** —
   `crates/gpui-miniapp/src/mini_app.rs:264` and `mini_app.rs:267`.
   `Keystroke::parse` maps the literal `cmd` to `modifiers.platform = true`
   (`crates/3rdparties/gpui/src/platform/keystroke.rs:152-158`), and binding
   match is exact modifier equality (`keystroke.rs:112`). On Linux/Windows the
   platform backends report Ctrl as `control`, not `platform`, so
   Ctrl+Q / Ctrl+T never match — Quit and Toggle Theme are unreachable from the
   keyboard off-macOS. Impact is amplified by the fact that MiniApp ships its
   own menus on all desktop targets, so users reasonably expect the documented
   shortcuts to work. Fix: bind `"secondary-q"` and `"secondary-t"` instead
   (`secondary` = cmd on macOS, ctrl elsewhere, per `keystroke.rs:143-148`),
   and make the menu accelerator label platform-aware (see finding 4).

2. **Five copy-pasted language-action handlers** —
   `crates/gpui-miniapp/src/mini_app.rs:183-252`. The handlers for
   `SetLanguageEnglish/French/German/Spanish/Japanese` are byte-for-byte
   identical apart from the `Language` variant; each re-reads
   `cx.try_global::<I18nState>()` immediately after setting it just to recover
   the value it already knows. Not a runtime bug today, but any fix to the
   menu-rebuild logic (e.g. adding `checked` markers, finding 5) has to be
   applied five times and the pattern invites drift. Fix: collapse into one
   helper, e.g. `fn set_language(cx: &mut App, config: &Rc<MiniAppConfig>,
   language: Language)`, and register the five actions as one-liners.

### Low

3. **CLI argument errors exit silently with status 0** —
   `crates/gpui-miniapp/src/mini_app.rs:122-128`. A malformed
   `--window-min-size` prints `MiniApp argument error: ...` to stderr and
   returns from `MiniApp::run`, so the process exits successfully and scripts
   or CI cannot tell the launch failed. Fix: `std::process::exit(2)` (or
   `panic!`) after the message on this path.

4. **Hard-coded "Cmd+T" hint in the Theme menu** —
   `crates/gpui-miniapp/src/mini_app.rs:374`
   (`MenuItem::action("Toggle Theme  Cmd+T", ToggleTheme)`). The accelerator is
   baked into the item's display name with a double space instead of using a
   native menu keystroke, so on Windows/Linux the menu literally shows
   "Cmd+T" even though the working key (once finding 1 is fixed) is Ctrl+T,
   and the Quit item (`mini_app.rs:359`) inconsistently shows no hint at all.
   Fix: build the label from `cfg!(target_os = "macos")` (or drop the inline
   hint and rely on the key binding).

5. **Menus never mark the active theme variant / design language / language** —
   `crates/gpui-miniapp/src/mini_app.rs:427-433` and `349-425`.
   `MenuItem::Action` has a `checked` field
   (`crates/3rdparties/gpui/src/platform/app_menu.rs:126-131`), but
   `theme_menu_item` / `design_menu_item` and the language items always leave
   it `false`, so the menu gives no indication of the current selection even
   though the state is known at build time. Fix: pass the current value into
   the menu builders and construct `MenuItem::Action { checked: variant ==
   current, .. }`.

6. **Only the "Language" menu title is localized** —
   `crates/gpui-miniapp/src/mini_app.rs:403-409`. Switching to French relabels
   one menu title to "Langue" while "Quit {app}", "View", "Theme", and
   "Design System" stay English. If partial localization is deliberate (demo
   shell), fine; otherwise route these through `gpui_ui_kit::i18n`
   translations like sibling components do.

7. **`web_query_param` does not percent-decode and mangles `+`** —
   `crates/gpui-miniapp/src/misc.rs:67-74`. Query values are returned raw
   except for a `+` → space replacement, which is form-encoding semantics
   applied to a query string; a legitimate `+` in a value would be corrupted
   and percent-encoded UTF-8 (e.g. `%C3%A9`) is never decoded. The documented
   scope (ASCII slugs for the showcase catalog) makes this harmless today, but
   the helper is `pub` and will be copied. Fix: either decode properly or
   document/rename it as slug-only.

8. **Window-open failure leaves the app running with zero windows** —
   `crates/gpui-miniapp/src/mini_app.rs:287-306`. On `open_window` error the
   launch closure prints to stderr and returns; on native targets the
   `Application` keeps running windowless, so the user is left with a
   menu-bar-only app (or, on Linux/Windows, an invisible process). Fix: call
   `cx.quit()` (native) before returning.

9. **Dead stub features in the manifest** —
   `crates/gpui-miniapp/Cargo.toml:27-33`. `autoeq`, `gpu-2d`, `gpu-3d`,
   `reqwest`, `showcase`, `spinorama`, `tokio`, and `urlencoding` enable no
   dependencies and are not referenced by any `cfg(feature = ...)` in the
   crate (verified by grep), and no other workspace manifest enables them.
   They look like leftovers from a copied manifest and will mislead consumers
   into thinking enabling them does something. Fix: delete them or wire them
   up.

## GPU/CPU data-flow notes

Not applicable: the crate contains no wgpu, rendering, or texture/buffer code.
The only GPU-adjacent note is that the shell installs `AccessibilityTree` and
`DesignSystemState` globals; all rendering is delegated to the caller's root
view.

## UI/UX consistency

The shell itself renders only two stacked `div()`s with a scroll container, so
design-token spacing/typography concerns belong to the embedded view. The
menu-related inconsistencies (hard-coded accelerator hint, no `checked`
markers, partial localization) are findings 4–6 above. One design note that is
intentional and fine: `MiniAppShell::render`
(`mini_app_shell.rs:50-68`) sizes the scroll container to the window bounds
each render and clones the inner `AnyView` (an Rc cheap clone); this runs only
on dirty renders, not per frame, and is required for `overflow_y_scroll` to
work.

## Clean bill

- No `unwrap`/`expect`/`panic` in production code paths (only in tests);
  platform/config/window failures are all handled with `Result` + stderr.
- `--window-min-size` parsing (`mini_app.rs:474-512`) is thorough: rejects
  missing values, non-`x` separators, non-finite/zero/negative dimensions, and
  is well covered by tests.
- Window sizing math (`mini_app.rs:271-284`, `clamp_window_min_size`) correctly
  clamps min-size to the display, raises initial size to min, then clamps to
  the visible display bounds — no off-by-one or inversion found.
- The wasm lifecycle (`std::mem::forget(app.run_embedded(launch))`,
  `misc.rs:311-321`) is a documented, intentional keep-alive, not a leak bug.
- No threads, mutexes, channels, or blocking calls; all state lives in GPUI
  globals on the UI thread.
- Test coverage is good for a shell crate: config builder, CLI parser,
  menu construction, and platform selection are all exercised
  (`cargo test -p gpui-miniapp`: 41 tests, all passing on macOS host).
