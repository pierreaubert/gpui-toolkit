# Bug Review: gpui-au — 2026-08-25

Scope: full scan of `crates/gpui-au` — all of `src/` (14 Rust files, ~2,900 lines incl. the `text_system/` and `window/` submodules), the C header `include/gpui_au.h`, `Cargo.toml`, README/AGENTS.md, plus targeted cross-checks against the vendored GPUI (`crates/3rdparties/gpui`) and `gpui_wgpu` where gpui-au's behavior depends on their contracts (keystroke semantics, `on_request_frame` registration, blocking wgpu init). `cargo check -p gpui-au` passes clean. The crate is a thin macOS AUv3 platform backend: FFI entry points driven by Swift, an `AuWindow` wrapping a host NSView + CAMetalLayer + wgpu, a GCD dispatcher, and a CoreText text system cloned from gpui-ios.

## Findings

### High

1. **Shifted/option-modified keybindings can never match — `Keystroke.key` built from modifier-affected `characters`.**
   `crates/gpui-au/src/ffi.rs:388-411` (`key_event`): for unnamed keys, `key` is set from NSEvent's `characters`, which already has Shift/Option applied (shift-a → `"A"`, option-s → `"ß"`). GPUI's contract (`crates/3rdparties/gpui/src/platform/keystroke.rs:22-27`) is that `key` is "the character printed on the key" (unshifted, ASCII-equivalent) and the typed character goes in `key_char`. Keymap entries like `cmd-shift-p` parse to `key: "p"` + shift modifier, so with this code they never match — any binding whose key is a shifted letter is dead.
   **Fix:** add a `characters_ignoring_modifiers` C-string parameter to `gpui_au_key_down`/`gpui_au_key_up` (Swift: `event.charactersIgnoringModifiers`), use it (lowercased) for `key`, and keep `characters` for `key_char`.

2. **Modifiers are never tracked or forwarded for mouse/scroll input.**
   `crates/gpui-au/src/window/au_window.rs:65` declares `modifiers: Cell<Modifiers>`, but nothing ever writes it (only `Cell::new(Modifiers::default())` at lines 105/145/529), so `PlatformWindow::modifiers()` (`au_window.rs:382-384`) always returns empty. Additionally all mouse/scroll FFI entry points (`ffi.rs:260-342`) hardcode `modifiers: gpui::Modifiers::default()` and the FFI signatures (and `include/gpui_au.h:25-41`) carry no modifier-flags parameter at all. Shift-click range selection, ctrl-click context menus, and alt-modified drags silently misbehave, and any GPUI code querying current modifiers gets wrong state.
   **Fix:** add a `modifier_flags: u32` parameter to the mouse/scroll FFI functions, map it with the existing `modifiers_from_ns_event`, store it via `self.modifiers.set(...)` in `dispatch_input`, and pass it in each event.

### Medium

3. **FFI frame path can clobber a re-registered `request_frame_callback`.**
   `crates/gpui-au/src/ffi.rs:237-243`: after invoking the taken callback, it restores it with an unconditional `borrow_mut().replace(cb)`. The crate's own `AuWindow::request_frame` (`au_window.rs:197-205`) and `handle_resize` (`au_window.rs:244-251`) use the guarded `if slot.is_none()` restore precisely so a callback re-registered during the call isn't lost. Today GPUI registers `on_request_frame` exactly once (`crates/3rdparties/gpui/src/window.rs:1541`), so this is latent — but the inconsistency is a landmine if GPUI (or a future toolkit layer) re-registers during frame processing. Note `AuWindow::request_frame` is currently `#[allow(dead_code)]` because the FFI path duplicates it.
   **Fix:** have `gpui_au_request_frame` call `window.request_frame()` instead of open-coding the take/invoke/restore, or adopt the same `is_none()`-guarded restore.

4. **Global `AU_WINDOW` mutex held across arbitrary GPUI callbacks.**
   `crates/gpui-au/src/window/au_window.rs:34-45` (`with_au_window`): the `std::sync::MutexGuard` is held while `f(&AuWindow)` runs, and `f` is frequently an entire GPUI input dispatch or frame render (`ffi.rs:344-348`, `ffi.rs:237-243`). `std::sync::Mutex` is non-reentrant, so any future re-entry into `with_au_window` from within one of those callbacks (e.g. GPUI synchronously triggering a host call that routes back through FFI) deadlocks the host DAW's main thread. The guard buys nothing beyond what the main-thread assertion already provides, since the pointer is only dereferenced on the main thread anyway.
   **Fix:** lock, copy the raw pointer out, drop the guard, then invoke `f` on the dereferenced pointer (still under the main-thread assertion).

5. **`clone_application_cell` transmutes `gpui::Application` to `Rc<AppCell>` guarded only by a `debug_assert`.**
   `crates/gpui-au/src/ffi.rs:106-120`: the lifetime workaround is documented and the rationale is sound, but the layout check is `debug_assert_eq!` — release builds get no protection if the vendored GPUI's `Application` grows a field before the `Rc<AppCell>`, at which point the transmute clones/drops a garbage `Rc` (UB in the host DAW process).
   **Fix:** make the size check a compile-time assertion (e.g. a `const _: () = assert!(size_of::...)`), and/or add a proper `app.cell()` accessor to the vendored gpui so the transmute can go away.

### Low

6. **Redundant double-zeroing of the glyph bitmap.**
   `crates/gpui-au/src/text_system/au_text_system_state.rs:322` does `bitmap.resize(needed, 0)` and then lines 331-332 repeat the same `resize` plus `bitmap.fill(0)`. The first resize is dead code, and when the caller buffer is freshly grown the `fill(0)` re-zeroes bytes `resize` just zeroed (fill is still needed to clear stale pixels on buffer reuse). Per-glyph, so worth the one-line cleanup: drop line 322, keep a single `resize` + `fill`.

7. **`is_active()` hardwired `true`, `is_hovered()` hardwired `false`, hover/active callbacks never fired.**
   `crates/gpui-au/src/window/au_window.rs:409-417`: GPUI throttles frame rate for inactive windows (`crates/3rdparties/gpui/src/window.rs:1561`) and uses hover state for cursor/scroll behavior. In an AU the view is often occluded or the host window unfocused, so the plugin UI renders at full rate and never reports hover transitions. Fix: add FFI entry points for host focus/hover changes and fire `active_status_callback`/`hover_status_callback`.

8. **`AuDisplay::main` doesn't handle a nil `NSScreen.mainScreen` and derives identity from mutable state.**
   `crates/gpui-au/src/display.rs:18-31`: if the host has no main screen (headless/background sessions), `msg_send!` on null yields zeroed bounds/scale, producing `DisplayId(0)` and a uuid of `au-screen-0-0-0`. The uuid also silently changes if the host display's resolution or scale changes, which can confuse GPUI's per-display state. Fix: null-check `screen` and fall back to a constant id/uuid; derive the uuid from something stable (or a constant — an AU view is pinned to one host screen anyway).

9. **`delete_backward` deletes a single UTF-16 code unit, not a character.**
   `crates/gpui-au/src/window/au_window.rs:290-301`: for an empty selection it deletes `start-1..start`. If the character before the caret is a surrogate pair (emoji, many CJK extension chars), the range start lands mid-character; whether GPUI's `replace_text_in_range` tolerates that needs confirmation against the vendored input handler — the gpui-mac backend deletes by grapheme. Confirm and, if needed, step back one full code point (2 UTF-16 units for surrogates).

10. **Per-call NSString allocations in appearance queries.**
    `crates/gpui-au/src/platform.rs:150` and `crates/gpui-au/src/window/au_window.rs:364` build `NSString` via `stringWithUTF8String:` just to compare against the appearance name, on every `window_appearance()`/`appearance()` call. Trivial cost; comparing `name.UTF8String` against the C string directly avoids it.

No other correctness issues found: the C header matches the Rust FFI signatures one-to-one, keychain CF retain/release pairs are balanced, the GCD trampoline converts the runnable pointer exactly once, and FFI null-context handling is tested.

## GPU/CPU data-flow notes

- gpui-au itself performs **no GPU→CPU readbacks** and no per-frame buffer/texture creation: frames flow `Scene → WgpuRenderer::draw → CAMetalLayer` and stay on the GPU; the fallback atlas is a shared `OnceLock` (`au_window.rs:29`) and glyph rasterization reuses caller buffers via `rasterize_glyph_into`. Good.
- **One-time blocking wgpu init on the host UI thread:** `AuWindow::new` (`au_window.rs:168`) calls `WgpuRenderer::new` synchronously, which inside the vendored `gpui_wgpu` does `pollster::block_on(request_adapter)` / `block_on(request_device)` (`crates/3rdparties/gpui_wgpu/src/wgpu_context.rs:39,46`). That stalls the DAW's main thread for tens-to-hundreds of ms when the plugin view opens. If that becomes visible, defer renderer creation to a background thread and render via the existing `FallbackAtlas` path until ready.
- `draw()` holds the `renderer` parking_lot mutex for the whole render+present (`au_window.rs:475-480`); harmless today (all callers are main-thread) but it means a concurrent `sprite_atlas()`/`gpu_specs()` from another thread would block for a full frame.
- The vendored renderer contains a `map_async` + `PollType::Wait` readback (`gpui_wgpu/src/wgpu_renderer.rs:2288-2291`) used by its screenshot path; gpui-au never calls it, so no per-frame GPU→CPU→GPU cycle exists in this crate.

## UI/UX consistency

The crate's only in-tree UI is the demo `AuRootView` (`ffi.rs:49-79`), which hardcodes colors (`rgb(0x1a1a2e)`, `rgb(0x3366ff)`) instead of gpui-design tokens and has no focus/keyboard/ARIA handling. It is clearly a bring-up smoke-test view (real plugin UIs are injected by external crates through `AuContext::new`), so this is informational rather than a defect — but if it ships as the default view, it should either use theme tokens or be gated behind a feature. Keyboard UX is otherwise governed by findings 1 and 2 above, which are the real user-facing gaps.

## Resolved during follow-up

- Fixed modified keybinding matching (#1). `gpui_au_key_down` and `gpui_au_key_up` now accept the host's `charactersIgnoringModifiers` string and use its lowercase value as the GPUI binding key, while preserving modifier-affected `characters` as `key_char`. The public C header was updated accordingly. Added the Shift+P regression test; verified with `cargo test -p gpui-au` (26 passed), `cargo check -p gpui-au`, `cargo fmt --check`, and `git diff --check`.
- Fixed mouse and scroll modifier propagation (#2). All pointer/scroll FFI functions now accept NSEvent modifier flags, the C header mirrors those signatures, event payloads map those flags through the existing AppKit conversion, and `AuWindow::dispatch_input` persists pointer, scroll, and key modifier state for `PlatformWindow::modifiers()`. Verified with `cargo test -p gpui-au` (26 passed), `cargo check -p gpui-au`, `cargo fmt --check`, and `git diff --check`.
- Fixed request-frame callback replacement (#3). The FFI entry point now delegates to `AuWindow::request_frame`, the sole implementation that restores a callback only when no replacement was registered during invocation. Removed the obsolete dead-code allowance. Verified with `cargo test -p gpui-au` (26 passed), `cargo check -p gpui-au`, `cargo fmt --check`, and `git diff --check`.
- Fixed the global-window callback deadlock (#4). `with_au_window` now copies the registered pointer while holding `AU_WINDOW` and drops the mutex before invoking GPUI/host code. To keep that raw handoff valid, the existing main-thread assertion is now active in all non-test builds and covers registration, unregistration, and dereference. Verified with `cargo test -p gpui-au` (26 passed), `cargo check -p gpui-au`, `cargo fmt --check`, and `git diff --check`.
- Removed the layout-sensitive `Application` transmute (#5). Vendored GPUI now explicitly exposes `Application::clone_app_cell`; AU and iOS use that accessor instead of assuming a one-field `Application` representation. This removes release-build undefined behavior if the wrapper layout changes. Verified with `cargo test -p gpui-au` (26 passed), `cargo check -p gpui-au`, `cargo check -p gpui-ios`, `cargo fmt --check`, and `git diff --check`.
- Finding #6 was already fixed in the current source. `rasterize_glyph_into` resizes the caller-reused bitmap to the exact glyph length and immediately calls `bitmap.fill(0)` before creating the CoreGraphics context, so no pixels remain when a smaller glyph follows a larger one. No code change was required.
- Fixed AU active/hover state reporting (#7). Added host-facing `gpui_au_set_active` and `gpui_au_set_hovered` FFI/header functions; `AuWindow` persists those states, exposes them through `PlatformWindow`, and invokes transition callbacks with re-registration-safe ownership. Added `host_focus_and_hover_transitions_update_state_once`; verified with `cargo test -p gpui-au` (27 passed), `cargo check -p gpui-au`, `cargo fmt --check`, and `git diff --check`.
- Fixed display null handling and identity (#8). `AuDisplay` now avoids messaging a null `NSScreen`, yields empty bounds in that fallback, and uses a fixed AU-primary display ID/UUID rather than deriving identity from mutable geometry. Added `null_screen_has_stable_identity_and_empty_bounds`; verified with `cargo test -p gpui-au` (27 passed), `cargo check -p gpui-au`, `cargo fmt --check`, and `git diff --check`.
- Fixed UTF-16 surrogate splitting on Backspace (#9). With an empty selection AU now requests the preceding text and retreats by the last Unicode scalar's `len_utf16()` rather than blindly one code unit. Added `backspace_moves_over_a_complete_utf16_code_point`; verified with `cargo test -p gpui-au` (28 passed), `cargo check -p gpui-au`, `cargo fmt --check`, and `git diff --check`.
- Fixed per-call appearance comparison allocations (#10). Both AU appearance paths now share `is_dark_aqua_appearance_name`, which compares the existing Objective-C name's `UTF8String` bytes with the static Dark Aqua name rather than creating a temporary `NSString`. Added exact-name matching coverage; verified with `cargo test -p gpui-au` (29 passed), `cargo fmt --check`, and `git diff --check`.

## Clean bill

- FFI surface: null-safe on every entry point (tested in `ffi.rs:525-542`); header `include/gpui_au.h` matches Rust signatures.
- `dispatcher.rs`: GCD trampoline is correct; `dispatch_after` nanosecond overflow saturates (tested).
- `keychain.rs`: SecItem calls and CF ownership (create/get rules) are balanced and correct.
- `text_system/string_index_converter.rs`: UTF-8↔UTF-16 advance/rewind logic is correct (tested).
- Layout/glyph caches are bounded (clear-at-cap); `rasterize_glyph_into` genuinely reuses the caller allocation (tested).
- `safety_report.rs` honestly marks the NSView-ownership and global-pointer boundaries as `HostValidationRequired` rather than claiming them audited.
