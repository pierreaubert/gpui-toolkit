# Bug Review: gpui-ios — 2026-08-25

Scope: full scan of `crates/gpui-ios` — all 62 `.rs` files under `src/` (~9,850 lines incl. unit tests), `Cargo.toml`, `tests/accessibility_allocation_contracts.rs`, `benches/accessibility_diff.rs`, plus `README.md`/`AGENTS.md`/`TUTORIAL.md` and the Swift bridging contract described in the README. The `ios/` platform module is cfg-gated to `target_os = "ios"|"tvos"`, so host verification covers only the portable parts: `cargo test -p gpui-ios` is green (31 lib tests + 1 allocation-contract integration test, 0 failed) on this macOS host; the UIKit/wgpu code was reviewed by reading, not executed. The crate is a platform backend (FFI boundary, wgpu/Metal window, CoreText text system), so the UI-component and GPU-readback categories apply only at the edges.

## Findings

Ranked by severity. No Critical issues found.

## Resolved during follow-up (2026-08-26)

- **Widget timeline JSON:** replaced hand-built `Debug` formatting with `serde_json`; output has stable lowercase kind strings and correctly escapes all string/control-character payloads. Host regression tests parse both ordinary and control-character timelines.
- **Keychain updates:** `SecItemUpdate` now matches the class/server/account identity and supplies only `kSecValueData` as update attributes. The add fallback retains the complete item dictionary, so repeat writes no longer pass immutable query fields as replacement attributes.
- **Hardware key diagnostics:** press logging now flows through the existing `GPUI_IOS_INPUT_DIAG` gate, and the press-specific diagnostic selectors are not queried when that diagnostic is disabled.
- **Diagnostic-only ObjC queries:** every `sendEvent`, gesture-delegate, touch, indirect-scroll, and hardware-press metadata query now checks `input_diag_enabled()` before asking UIKit for values used solely in diagnostic output.
- **Callback reentrancy:** text-input and keyboard-layout callbacks are taken out of their thread-local slots before invocation and restored only when no reentrant registration occurred. Tests cover callbacks safely unregistering themselves.
- **Accessibility action callback reentrancy:** the global callback is taken out of its mutex slot before invocation and restored only when unchanged. Its regression test unregisters it during dispatch without deadlocking and confirms that it is not restored afterward.
- **Touch/input/momentum reentrancy:** callbacks are taken out of the input slot before running; nested input is FIFO-queued, nested UIKit touches are retained and deferred until the gesture-state borrow ends, and recursive frame requests cannot re-enter momentum pumping. The request-frame callback uses the same take/restore discipline.
- **Pencil/hover callback reentrancy:** the global mutex slots now use the same take/restore-with-generation pattern, so application callbacks run without their mutex held. A Pencil regression test verifies self-unregistration without deadlock.
- **Application lifetime retention:** the current code uses GPUI's supported `Application::clone_app_cell()` rather than relying on `Application`'s private layout.
- **Deferred frame flags:** when no request-frame callback has been registered yet, text/forced frame flags are restored instead of being silently consumed.
- **Renderer initialization:** a Metal surface/context/renderer creation failure now returns an error from `IosWindow::new` instead of caching a non-rendering fallback atlas and continuing with a permanently blank window.
- **Document-picker lifetime:** picker calls now use `UIDocumentPickerModeImport`, so UIKit supplies an app-sandbox copy whose returned path remains valid after the delegate callback instead of a short-lived security-scoped open URL.
- **iOS target closure:** restored the missing `pencil::has_pencil_callback` API used by the UIKit touch path; it avoids stylus-property queries when no consumer is registered. Also removed now-unused iOS imports.
- **Momentum cursor position (disproved):** momentum events intentionally retain the release-point position so GPUI routes the whole fling to the same scroll target. Integrating deltas would make hit testing drift across controls; this is not a cursor-motion report.
- **Feature flags and local Finder/editor artifacts (disproved):** empty feature names are the workspace-wide QA compatibility surface; the named backup/.DS_Store paths are not tracked by Git and are already excluded from source artifacts.

Verification: `cargo test -p gpui-ios` (35 unit tests plus allocation contract) and `cargo check -p gpui-ios --target aarch64-apple-ios` pass.

### High

1. **Widget timeline JSON is malformed — `kind` is emitted unquoted via `{:?}`.**
   `src/widget.rs:173-183` (`write_timeline_json`) formats the payload with
   `{{\"id\":{:?},\"kind\":{:?},...}}`. `{:?}` on `WidgetSnapshotKind` prints `Widget` /
   `LiveActivity` with no quotes, so every generated `*.timeline.json` file is invalid JSON
   (`"kind":Widget`) and a strict `JSONSerialization`/`JSONDecoder` consumer on the
   WidgetKit side fails outright. Secondary issue in the same function: using Rust `{:?}`
   as a JSON string escaper (`src/widget.rs:160-168`) is not JSON-correct for control
   characters — Rust Debug emits `\u{7f}`-style escapes, which JSON rejects.
   Fix: quote the kind explicitly (e.g. `\"kind\":\"{:?}\"`, or better a `match` to
   `"widget"`/`"live_activity"`) and hand-escape strings for JSON (or pull in the
   `serde_json` already used elsewhere in the workspace). A test that runs the output
   through `serde_json::from_str` would catch both.

2. **Keychain `write_credentials` cannot update an existing item.**
   `src/ios/keychain.rs:48-58` builds one `attrs` dictionary containing `kSecClass`,
   `kSecAttrServer`, `kSecAttrAccount`, and `kSecValueData`, then passes that same
   dictionary as the *attributesToUpdate* argument of `SecItemUpdate`. Apple's API
   contract forbids the item-class key in the update dictionary (it returns
   `errSecParam` -50), so the `SecItemUpdate` branch fails for any credential that
   already exists — only the first write per server succeeds, and subsequent
   password rotations error out ("updating iOS keychain item failed: -50").
   Additionally the query at `src/ios/keychain.rs:44-46` matches on `(class, server)`
   only, so two accounts on the same server collide: the update applies to whichever
   item matches first, and the second `SecItemAdd` can fail with `errSecDuplicateItem`.
   Fix: give the update a separate dictionary containing only `kSecValueData` (plus
   account if intended), and add `kSecAttrAccount` to the match query. Confidence is
   based on the documented `SecItemUpdate` contract; a simulator round-trip test
   (write → write again → read) would confirm.

### Medium

3. **Unconditional `eprintln!` diagnostics on every hardware-key press.**
   `src/ios/window/handle.rs:29-31` and `src/ios/window/handle.rs:43-45` format and
   write two stderr lines per `presses*Began/Changed/Ended/Cancelled` event,
   unconditionally. Sibling diagnostics in this crate are gated behind
   `GPUI_IOS_INPUT_DIAG` (`register.rs:49-64`); these two were left on. Cost is
   per-keypress string allocation plus stderr I/O in production, and log spam on
   device. Fix: route both through `register::input_diag_log`.

4. **thread_local `RefCell` held across arbitrary app callbacks — reentrancy panic across the FFI boundary.**
   `src/lib.rs:50-60` (`dispatch_text_input`) and `src/lib.rs:68-78`
   (`dispatch_keyboard_layout_change`) invoke the registered app callback while
   holding `borrow_mut()` on the thread-local slot. A callback that re-enters —
   e.g. calls `set_text_input_callback(None)` to unregister itself, or calls
   `set_keyboard_height()` from a layout-change callback (which recursively calls
   `dispatch_keyboard_layout_change`, `src/lib.rs:137-144`) — panics with
   "already mutably borrowed". These dispatchers run inside ObjC-driven
   `extern "C"` paths (`handle_text_input`, keyboard NSNotification blocks), where
   unwinding aborts the process. Fix: take the callback out of the slot (or wrap it
   in `Rc` and clone) before invoking, restoring afterwards — the same take-restore
   pattern this crate already uses for `resize_callback`/`request_frame_callback`.
   The same shape exists with `std::sync::Mutex` (deadlock instead of panic) in
   `src/pencil.rs:95-111` and `src/accessibility.rs:500-506`, where the global
   callback mutex is locked across the app callback.

5. **`handle_touch` holds `input_callback`/`touch_states` `RefCell` borrows across event dispatch and synchronous frame renders.**
   `src/ios/window/ios_window.rs:694-707` borrows both cells mutably for the entire
   gesture state machine, and every `emit(...)` runs arbitrary GPUI/app code; several
   arms also call `request_forced_frame()` (`ios_window.rs:590-602`), which invokes
   GPUI's frame callback — a synchronous render — while those borrows are live.
   `pump_momentum` (`ios_window.rs:1415-1447`) likewise holds
   `momentum_scroller.borrow_mut()` across the input callback. Any re-entrant path
   that touches the same cell (a second `pump_momentum` via
   `gpui_ios_request_current_frame` from app code, a nested touch dispatch, a
   callback that triggers text input which borrows `input_callback` at
   `ios_window.rs:1597`) panics, and across the ObjC boundary that aborts. I did not
   find a concrete in-tree trigger — GPUI's draw path does not dispatch input — so
   this is a fragility finding rather than an observed crash. Fix: clone the callback
   out of the cell per dispatch (an `Rc` bump per touch event is cheap), or wrap
   dispatch in a reentrancy guard that queues nested events.

6. **`retain_application_for_process_lifetime` transmutes `gpui::Application` on a layout assumption guarded only in debug builds.**
   `src/ios/ffi/misc.rs:109-127` transmutes `&Application` to `&Rc<AppCell>`,
   relying on the pinned GPUI revision keeping `Application` a single-field tuple
   struct. The only guard is `debug_assert_eq!` on *size* — equal size does not prove
   equal layout, and release builds (what ships on devices) skip the check entirely.
   A GPUI bump that changes the field set while keeping the size identical silently
   clones/leaks the wrong pointer bytes. It works against the pinned rev today.
   Fix: promote the check to a hard `assert!`, and ideally upstream a sanctioned
   handle (`Application::leak()` or similar) so the transmute can go away.

### Low

7. **Momentum deltas never advance the reported position.**
   `src/momentum.rs:203-208` fills `MomentumDelta.position_x/y` from `last_x/last_y`,
   which are set once in `fling()` (`src/momentum.rs:159-171`) and never updated in
   `step()`. Every synthetic momentum `ScrollWheel` therefore reports the finger's
   release point for the whole fling. That is arguably the right hit-test target
   (scroll the container you flicked), but the field name promises a moving position
   and any consumer using it for cursor-following effects gets stale coordinates.
   Fix: integrate the position in `step()` (`self.last_x += dx; …`) or drop the
   fields from the delta.

8. **Failed renderer init is silently cached as a permanent blank screen.**
   `src/ios/window/ios_window.rs:2056-2074`: if wgpu renderer creation failed in
   `IosWindow::new` (only a `log::error!` at `ios_window.rs:423-434`), the first
   `sprite_atlas()` call caches the dummy `FallbackAtlas` and returns it forever —
   the window stays alive rendering nothing, and the fallback's tile `HashMap`
   (`src/ios/window/types.rs:175-178`, `fallback_atlas.rs:59`) grows unboundedly
   while discarding glyph pixels. Fix: don't cache the fallback (re-check the
   renderer each call), and propagate renderer-init failure as an `Err` from
   `IosWindow::new`/`open_window` so the shell can react.

9. **Document-picker results are not security-scope accessed.**
   `src/ios/document_picker.rs:111-126` converts picked `NSURL`s to plain paths and
   drops the URLs without `startAccessingSecurityScopedResource`. In Open mode the
   returned URLs are security-scoped; files outside the app's sandbox (iCloud Drive,
   external providers) will fail to open for the receiver of the `oneshot`. Fix:
   begin access in the delegate callback and hand the caller a token/scope guard,
   or read the file data in the callback where access is implicit. Confirm on
   device with an iCloud-hosted file.

10. **Frame-request flags can be silently consumed when no callback is registered.**
    `src/ios/ffi/misc.rs:18-39` swaps `TEXT_INPUT_DIRTY` and `forced_frame_pending`
    *before* checking whether `request_frame_callback` exists; with no callback the
    pending forced render is dropped on the floor. Narrow window (between window
    creation and `on_request_frame` registration), but a touch in that interval
    loses its frame. Fix: only clear the flags when the callback is actually taken,
    or re-set them in the `None` branch.

11. **Per-event ObjC attribute queries run even when input diagnostics are off.**
    `src/ios/window/register.rs:77-80` (`sendEvent`), `register.rs:239-242`,
    `register.rs:262`, `register.rs:282`: the `msg_send!`s for event
    type/subtype/modifiers/buttons execute unconditionally; only the
    `format!`+write is gated inside `input_diag_log`. Four ObjC message sends per
    UIEvent is small but free to avoid. Fix: early-return on the env check before
    querying (move the gate out of `input_diag_log` for these call sites).

12. **`Cargo.toml` carries copy-pasted feature flags; stray backup/junk files in the crate.**
    `Cargo.toml:21-28` declares empty features `autoeq`, `gpu-2d`, `gpu-3d`,
    `reqwest`, `showcase`, `spinorama`, `tokio`, `urlencoding` — no
    `cfg(feature = ...)` exists anywhere in the crate (verified by grep).
    `crates/gpui-ios/Cargo.toml~`, `crates/gpui-ios/.DS_Store`, and
    `crates/gpui-ios/src/.DS_Store` are also sitting in the tree. Fix: delete the
    unused features and the junk files (the sibling review of gpui-keybinding
    flagged the identical feature-list copy-paste).

## GPU/CPU data-flow notes

No GPU→CPU→GPU cycles found. `draw()` (`src/ios/window/ios_window.rs:2047-2054`)
hands the `Scene` straight to `gpui_wgpu::WgpuRenderer`; there is no
`device.poll`, `pollster`, or `map_async` anywhere in the crate (grep-verified),
so nothing on the iOS UI thread blocks on GPU completion and no texture data
round-trips through the CPU. The only pseudo-atlas is `FallbackAtlas`
(`src/ios/window/fallback_atlas.rs`), which deliberately discards rasterized
pixel data and never uploads — acceptable as a crash-avoidance stub, subject to
finding 8 (it should never become the permanent atlas). Glyph rasterization
(`src/ios/text_system/ios_text_system_state.rs:305-421`) is CPU-side via
CoreText/font-kit into reusable caller buffers (`rasterize_glyph_into`), which
GPUI's atlas uploads once per glyph — the normal atlas path, not a per-frame
re-upload. Surface reconfiguration on rotation is incremental
(`update_drawable_size`, `ios_window.rs:1794-1801`), not pipeline/surface
re-creation.

## UI/UX consistency

This crate renders no components itself; the UI-facing surface is the input and
accessibility bridge.

- The accessibility bridge is the strong point: snapshot diffing with reusable
  scratch buffers, per-node UIKit property updates, order-change-only array
  rebuilds, and an allocation-free warmed path enforced by
  `tests/accessibility_allocation_contracts.rs`. VoiceOver actions route back
  into a GPUI callback with proper trait mapping
  (`src/ios/window/accessibility.rs`).
- Minor input inconsistency: cursor-movement keys from a hardware keyboard are
  dispatched *twice* — once as ANSI escape sentinels into the global text-input
  callback (`src/ios/window/ios_window.rs:1697-1712`) and again as GPUI key
  events (`ios_window.rs:1715-1727`). Components consuming both channels must
  dedupe themselves; the text path (`handle_text_input`) has the same
  dual-channel design by intent (documented at `ios_window.rs:1577-1584`), but
  the arrow-key duplication is not documented anywhere.
- The hardware keymap (`src/ios/text_input.rs:27-78`) covers letters, digits,
  and common named keys but no keypad block or media keys; unmapped keys become
  `"unknown-xx"` strings that GPUI keybinding specs can never match. Acceptable
  coverage for the current apps, worth a note in the README's Known Limitations.

## Clean bill

- **Momentum/velocity math** (`src/momentum.rs`): weighted-least-squares velocity
  estimator and exponential-decay integrator are numerically guarded (denominator
  fallback, dt clamps, velocity clamping) and well tested; finding 7 aside, no
  bugs found.
- **Text system** (`src/ios/text_system/`): a faithful port of Zed's macOS
  CoreText path — layout cache is keyed correctly over `(text, size, runs)` via a
  borrowed-key trait, caches have explicit caps, UTF-16↔UTF-8 index conversion is
  total, and CF/CT object lifetimes in `apply_features_and_fallbacks` are
  balanced (create-rule releases checked line by line).
- **FFI entry points** (`src/ios/ffi/gpui_mod.rs`): all null-checked, window
  pointers validated against the registry before dereference
  (`registered_window`), and the frame pump is `catch_unwind`-wrapped with a
  global disable on panic — the right failure mode for a display-link callback.
  Window unregister-on-Drop plus ivar clearing (`ios_window.rs:156-184`) closes
  the dangling-pointer hole for post-destruction UIKit callbacks.
- **Platform-view registry** (`src/platform_view/`): snapshot cache invalidation
  is consistent, no lock-order inversion (factories → views only), and host-side
  tests cover create/sort/hit-test/caching.
- **Widget snapshot I/O** (finding 1 aside): path sanitization, validation, and
  the async executor variant are sound.
