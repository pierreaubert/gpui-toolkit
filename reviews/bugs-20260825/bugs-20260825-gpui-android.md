# Bug Review: gpui-android — 2026-08-25

Scope: full scan of the `gpui-android` crate — all 12 Rust sources under
`crates/gpui-android/src/` (~7,700 lines: `lib.rs`, `momentum.rs`,
`accessibility.rs`, `platform_view.rs`, and `android/{mod,platform,window,jni,
dispatcher,keyboard,display,platform_view}.rs`) plus the Java glue
(`GpuiActivity.java`, `GpuiFileProvider.java`) and `Cargo.toml`. I also ran two
builds as evidence: host `cargo test -p gpui-android` (passes: 5 tests + 1
doctest, because the entire `android` module is `cfg(target_os = "android")`)
and `cargo check -p gpui-android --target aarch64-linux-android` with the
pinned NDK 27.2.12479018 toolchain from the `justfile` — which **fails with 18
compile errors**. Focus areas: correctness, threading/deadlock, JNI lifetime
safety, per-frame allocation, and GPU surface lifecycle. Rendering itself is
delegated to `gpui_wgpu`, which is out of scope here.

## Findings

### Critical

1. **The crate does not compile for its only real target.**
   `cargo check -p gpui-android --target aarch64-linux-android` (same
   CC/CXX/AR env as `just showcase-android-check`) fails with 18 errors:
   - `crates/gpui-android/src/android/jni.rs:613` — `Instant::now()` used but
     only `Duration` is imported (`jni.rs:41-48`).
   - `crates/gpui-android/src/android/window.rs:794` — `AndroidWindow::
     needs_frame_pump` reads `self.momentum`, but `momentum` is a field of
     `AndroidPlatformWindow` (`window.rs:1205`), not `AndroidWindow` (E0609).
   - `crates/gpui-android/src/android/window.rs:2029,2050,2181` —
     `AndroidPlatformWindow::update_ime_position` reads `self.state`, which
     does not exist on that struct (E0609; `self.window.state` intended).
   - `crates/gpui-android/src/android/window.rs:2186-2245` —
     `update_cursor_anchor_info` is written against `jni::JNIEnv`, which no
     longer exists in the locked `jni 0.22.4` (`Env`/`EnvUnowned` replaced it);
     every `call_method`/`new_object`/`exception_clear` in that function
     fails (E0599), and `GlobalRef` now needs a lifetime parameter
     (`window.rs:1216-1217`, `jni.rs:235`, E0107).
   - `crates/gpui-android/src/android/window.rs:2290` — the dead
     `let data = None;` branch in `FallbackAtlas::get_or_insert_with` fails
     type inference (E0282).

   Impact: `gpui-showcase/android/Cargo.toml:30` and
   `gpui-miniapp/Cargo.toml:63` both depend on this crate, so
   `just android-check` and the `qa-android-emulator` evidence claimed in
   `reviews/20260808-qa.md` cannot currently be produced. The breakage looks
   like a `jni` 0.21→0.22 upgrade plus recent edits (IME anchor caching,
   `needs_frame_pump`) that were never compiled for the target — the host
   build/tests pass because `src/android/` is entirely `cfg`'d out off-Android.

   Fix: port the stale JNI sites to the 0.22 API (`Env`, `GlobalRef<'static>`
   or reborrow per call), import `Instant`, move `needs_frame_pump` onto
   `AndroidPlatformWindow` (or pass the `Arc<Mutex<MomentumState>>` into
   `AndroidWindow`), change `self.state` → `self.window.state`, and delete the
   dead `None` branch in the fallback atlas. Then add
   `cargo check -p gpui-android --target aarch64-linux-android` (NDK env from
   the justfile) to the host-side QA gate so target-only code cannot rot
   invisibly again.

### High

2. **`PlatformDispatcher::dispatch_on_main_thread` (the trait impl GPUI
   actually uses) never sets `main_thread_wake_pending`.**
   `crates/gpui-android/src/android/dispatcher.rs:537-546` pushes the task and
   pokes the wake pipe, but unlike the inherent method at
   `dispatcher.rs:255-266` it omits `main_thread_wake_pending.store(true, ...)`.
   The event loop relies on `take_main_thread_wake()`
   (`jni.rs:851-854`, comment at `dispatcher.rs:150-153`) to pump a frame after
   the looper callback drained a task without yielding `PollEvent::Wake`.
   GPUI's `ForegroundExecutor` dispatches through `Arc<dyn PlatformDispatcher>`,
   i.e. the trait impl — so foreground task results (the standard
   background→foreground UI update path) can run without any subsequent
   `request_frame()`, leaving the UI stale until an unrelated input/system
   event arrives. Fix: set the flag in the trait impl, or make both paths share
   one enqueue helper.

3. **`PlatformDispatcher::dispatch_after` (trait impl) never wakes the
   looper.** `dispatcher.rs:548-561` inserts the delayed task but skips the
   `wake_pipe()` that the inherent `dispatch_after` performs at
   `dispatcher.rs:297-298` precisely so "a blocking looper [can] recompute its
   timeout". When the loop is blocked in `poll_events(None)` (idle, no delayed
   tasks known at poll time — `jni.rs:603-618`), a timer scheduled from another
   thread will overshoot its deadline until some unrelated event wakes the
   loop. Fix: wake the pipe in the trait impl too (same shared helper).

4. **Momentum fling never animates once the finger lifts.**
   In `run_event_loop` (`crates/gpui-android/src/android/jni.rs:603-618`),
   `needs_frame_pump` only shortens the poll timeout to 8 ms; the render gate
   `should_pump_frame = force_frame_after_poll || platform_event_woke_frame ||
   take_main_thread_wake()` (`jni.rs:851-854`) never includes it, and
   `PollEvent::Timeout` explicitly does not set `platform_event_woke_frame`
   (`jni.rs:627`). After the final touch event there is no event and no
   dispatcher wake, so `win.request_frame()` is never called again and
   `MomentumScroller::step()` (invoked only inside the request-frame callback,
   `window.rs:1510-1570`) never runs — the fling freezes on its first frame
   while the loop busy-wakes at 125 Hz doing nothing. Fix: include
   `needs_frame_pump` in `should_pump_frame`.
   Related stale-state bug in the same path: `MomentumScroller::step()`
   (`crates/gpui-android/src/momentum.rs:173-209`) never updates
   `last_x/last_y` (set only by `fling()`, `momentum.rs:159-171`), so every
   momentum `ScrollWheelEvent` reports the fling-*start* position forever
   (`MomentumDelta.position_x/y`, used at `window.rs:1514-1515`). Either
   integrate the position in `step()` or drop the fields.

5. **Blocking GPU call held under the window state lock, contradicting the
   in-code comment.** `AndroidWindow::handle_resize`
   (`crates/gpui-android/src/android/window.rs:679-687`) says
   "update_drawable_size calls device.poll(Wait) which can take time — take the
   renderer out to avoid holding the state lock", but the take/update/put-back
   all happen inside the same `state` guard (dropped only at line 690).
   Meanwhile `bounds()`, `scale_factor()` etc. all need that lock, and the
   jni.rs lifecycle design (comment at `jni.rs:63-76`) exists precisely because
   blocking on this lock from the native thread risks an InputDispatcher ANR.
   Fix: actually drop the guard before `update_drawable_size` (compute
   `new_w/new_h`, release, then lock again to swap the renderer back).

### Medium

6. **A panic in `renderer.draw()` permanently loses the renderer.**
   `AndroidWindow::draw` (`window.rs:767-780`) takes the renderer out of the
   state, draws unlocked, and puts it back. If `renderer.draw(scene)` panics
   (wgpu validation error, OOM), the put-back never runs and every subsequent
   frame, resize, and `term_window` sees `renderer: None` — the window is
   silently dead until process restart. Any concurrent `draw`/`has_surface`/
   `sprite_atlas` during the draw also observes `None` and silently drops work
   (frames) or fabricates a fresh `FallbackAtlas` (`window.rs:1989-1993`).
   Fix: restore-via-guard (a small drop guard that re-inserts the renderer on
   unwind), and skip the `FallbackAtlas` substitution when a real atlas existed.

7. **`set_active` silently drops transitions under lock contention.**
   `window.rs:910-943` uses `try_lock()`; if the state lock is busy (e.g. held
   by a resize — see #5), neither `state.is_active` nor the
   active-status callback is updated, and the change is never retried.
   The atomic `active` flag was flipped, so `is_active()` and the
   callback-visible state can diverge permanently. Fix: queue a pending
   transition instead of dropping it.

8. **Hidden platform views keep swallowing touches.**
   `PlatformViewHandle::set_visible(false)` only forwards to the view
   (`crates/gpui-android/src/platform_view.rs:189-191`); the registry's bounds
   map is only cleaned on dispose (`platform_view.rs:305-311`). `hit_test`
   (`platform_view.rs:325-337`) therefore keeps reporting hits for invisible
   views, and `process_input_events` (`jni.rs:431-439`) returns
   `InputStatus::Unhandled` for those touches — creating invisible dead zones
   where GPUI receives nothing. Fix: track visibility in the registry (or
   remove bounds on hide) and skip hidden views in `hit_test`.

9. **IME `DeleteSurrounding` mixes UTF-16 counts with GPUI text ranges.**
   Java's `deleteSurroundingText` delivers UTF-16 code-unit counts
   (`GpuiActivity.java:317-320` → `jni.rs:1441-1451`), which are applied
   directly to `selection.range.start/end` (`window.rs:1460-1465`). GPUI
   input-handler ranges are UTF-8 byte offsets, so any non-ASCII text before
   the caret makes the deletion range wrong. Confirm against
   `PlatformInputHandler::selected_text_range`'s documented unit; if it is
   UTF-8, convert via the surrounding text rather than raw arithmetic.

10. **Accessibility path is O(n²) with a JNI round-trip per node query and per
    tree update.** Every `TreeUpdate` triggers a `gpuiAccessibilityChanged`
    JNI call (`crates/gpui-android/src/accessibility.rs:82-86`); each TalkBack
    `createAccessibilityNodeInfo` then JNI-calls `nativeAccessibilitySnapshot`,
    which locks the global state and serializes the *entire* tree to JSON
    (`accessibility.rs:28-58`), after which the Java provider linearly scans
    the node array per node (`GpuiActivity.java:358-390`, `findNode`/
    `findParent`). A full screen exploration is O(nodes²) with a full-tree
    snapshot per node. Fix: coalesce change notifications to one per frame,
    and cache the parsed snapshot + an id→node index on the Java side,
    invalidated by the change callback.

11. **`Platform` trait answers disagree with the platform's own state.**
    `keyboard_layout()` hardcodes `"en-US"`
    (`crates/gpui-android/src/android/platform.rs:1661-1665`) instead of using
    the JNI-backed `query_keyboard_layout_id_via_jni()` (`platform.rs:1075`),
    and `window_appearance()` unconditionally returns `WindowAppearance::Dark`
    (`platform.rs:1457-1459`) while windows default to `Light`
    (`window.rs:523`) and the real value is only pushed via `ConfigChanged`
    (`jni.rs:760-774`). GPUI consumers reading appearance through the trait
    get `Dark` on a light-mode device until a config change happens. Fix:
    cache the live appearance in platform state (updated from `ConfigChanged`)
    and return it; build the layout object from the JNI query.

### Low

12. `AndroidTouchState::upsert` silently overwrites slot 0 when all 8 slots
    are full (`window.rs:267`) — a 9th simultaneous touch corrupts tracking of
    the first. Log-and-drop would be safer.

13. Window and display IDs are derived from raw `ANativeWindow` pointers
    (`window.rs:512`, `display.rs:76`); after surface teardown the allocator
    can hand back the same address, colliding with the previous ID in
    `WindowList`. A monotonic counter would be stable.

14. `Drop for AndroidPlatform` only removes the `PLATFORM_CALLBACKS` entry
    when dropped on the owner thread (`platform.rs:1342-1350`); otherwise the
    thread-local entry leaks (bounded by platform-instance count, so minor).

15. `GpuiFileProvider.FILES` (`GpuiFileProvider.java:32-48`) is only pruned by
    `delete()`, which external apps never call — one retained `File` per
    `open_with_system` for the process lifetime.

16. The IME shadow buffer `GpuiInputView.editable`
    (`GpuiActivity.java:260,292-321`) accumulates committed text forever and
    is never resynced with the Rust-side document, so IME composing-region
    offsets diverge over long sessions; the accessibility provider's
    `virtualIds`/`nodeIds` maps (`GpuiActivity.java:333-335`) also grow
    unboundedly as GPUI node IDs rotate.

17. Platform-view creation params are serialized as `k=v|k=v`
    (`crates/gpui-android/src/android/platform_view.rs:68-73`) with no
    escaping — values containing `=` or `|` are corrupted for the Java side.

18. `VelocityTracker.count` is incremented and reset but never read
    (`momentum.rs:23,48,82`) — dead state; the window-size logic lives in the
    fixed array + age filter.

19. Several `std::sync::Mutex::lock().unwrap()` sites on the input path
    (`src/lib.rs:121,127`, `platform_view.rs:250,262,267,278,300,310,326,343`)
    turn one poisoned mutex into a permanent panic on all subsequent input
    events; `parking_lot` (used elsewhere in the crate) or `.unwrap_or_else(
    |e| e.into_inner())` would avoid that.

## GPU/CPU data-flow notes

The crate hands all rasterization to `gpui_wgpu::WgpuRenderer`; I found **no
GPU→CPU readback** in this crate itself. Two GPU-lifecycle issues instead:

- **Full renderer rebuild on every surface cycle** (`window.rs:587-601`,
  `init_window`): on each background→foreground transition the whole
  `WgpuRenderer` — pipelines, bind groups, and the glyph/sprite atlas — is
  destroyed and recreated, forcing every glyph to be re-rasterized on the CPU
  and re-uploaded to the GPU after every pause. wgpu only requires recreating
  the `Surface` (and reconfiguring it) when the `ANativeWindow` changes; the
  `Device`, pipelines and atlas textures survive. Splitting surface
  re-creation from renderer construction in `gpui_wgpu` would keep all atlas
  data resident on the GPU across surface loss.
- **Blocking present/resize on the UI thread**: `draw()` intentionally blocks
  in `get_current_texture()`/Mailbox present for frame pacing (comment at
  `window.rs:761-766`, `jni.rs:621-625`), and resize blocks in
  `device.poll(Wait)` (see finding #5). Acceptable as a pacing strategy, but
  the lock must not be held across it.

## UI/UX consistency

Not a component crate, so design-token questions don't apply. Behavioral
consistency issues worth noting: appearance reported as always-Dark via the
`Platform` trait while windows track a real value (finding #11); keyboard
layout always `"en-US"` despite the JNI query existing (finding #11);
`AKEYCODE_BACK` is mapped to `"escape"` (`keyboard.rs:300`), so Android's back
button reaches GPUI as an Escape keystroke rather than a navigation event —
deliberate-looking, but it means apps cannot distinguish back from a hardware
Escape key.

## Resolved during follow-up

- Fixed the target-only compilation failure: imported `Instant`; updated JNI retained-object types and IME calls for `jni` 0.22; corrected IME window-state ownership; placed shared momentum state on `AndroidWindow`; and gave the fallback-atlas dead branch a concrete type. The repository’s `just showcase-android-check` now succeeds for the aarch64 Android showcase with the full feature matrix. This check is already enforced by `.github/workflows/ci.yml`, so no separate QA-gate addition was needed.
- Fixed `PlatformDispatcher` parity: the trait `dispatch_on_main_thread` path now sets `main_thread_wake_pending`, and its delayed-task path wakes the looper after scheduling. This restores foreground-task frame requests and timely timer deadlines.
- Fixed momentum frame delivery: active momentum now enters the event loop’s render gate as well as its polling-timeout calculation. `MomentumScroller::step` updates the reported pointer position after each displacement; added `momentum_position_advances_with_each_step`. Verified with `cargo test -p gpui-android momentum` (4 passed), `just showcase-android-check`, and `git diff --check`.
- Fixed Android window lifecycle locking: resize now removes the renderer, drops `WindowState`, performs the potentially blocking drawable-size update, then restores the renderer. Unlocked drawing uses a `RendererRestore` drop guard, so a panic cannot leave the renderer permanently absent. `set_active` now synchronizes callback-visible state with the atomic state instead of dropping a transition on temporary lock contention; callbacks still run unlocked. Verified with `just showcase-android-check` and `git diff --check`.
- Fixed invisible platform-view touch interception. The registry now records visibility alongside bounds, `PlatformViewHandle::set_visible` updates it, and only visible views count or match hit tests. Added `hidden_views_do_not_intercept_touches`. Verified with `cargo test -p gpui-android platform_view`, `just showcase-android-check`, and `git diff --check`.
- Fixed `Platform` state answers: `keyboard_layout()` now builds from the JNI-backed layout query (with its existing `en-US` fallback), and `window_appearance()` derives Light/Dark from the primary window’s state updated by `ConfigChanged`, defaulting to Light before a window exists. Verified with `just showcase-android-check` and `git diff --check`.
- Fixed touch overflow and recycled native-pointer identities. `AndroidTouchState` now logs and drops contacts beyond its eight-slot capacity rather than corrupting slot zero; Android windows and displays use monotonic per-process IDs instead of raw native pointers (or dimensions). Added touch-capacity and same-size identity regressions. Verified with `cargo test -p gpui-android android_touch_state`, `cargo test -p gpui-android unique_ids`, `just showcase-android-check`, and `git diff --check`.
- Fixed off-thread platform-drop callback cleanup while preserving thread-local non-`Send` callback ownership. `Drop` now queues registry removal through the foreground dispatcher when necessary; added `off_thread_drop_defers_callback_registry_cleanup_to_owner`. Verified with `cargo test -p gpui-android off_thread_drop_defers_callback_registry_cleanup_to_owner`, `just showcase-android-check`, and `git diff --check`.
- Finding #9 was not a bug. Android's `deleteSurroundingText` reports UTF-16 code units, and GPUI documents both `InputHandler::selected_text_range` and `replace_text_in_range` as UTF-16 ranges. The existing arithmetic therefore preserves the correct unit, including around non-ASCII text and surrogate pairs; no conversion is appropriate.
- Fixed accessibility snapshot scaling (#10). Rust now prunes accessibility nodes disconnected by an incremental tree update; Java coalesces invalidations to one Android animation frame, caches the parsed snapshot until that invalidation, and builds node/parent indexes once per snapshot. Node queries now avoid repeated JNI/JSON work and use O(1) indexed lookups instead of linear scans. The snapshot refresh also prunes virtual-node ID mappings for disappeared nodes, resolving the accessibility-map half of #16. Added `snapshot_drops_nodes_disconnected_by_an_incremental_update`; verified with `cargo test -p gpui-android accessibility` (2 passed), `just showcase-android-check`, `:app:compileDebugJavaWithJavac`, `cargo fmt --check`, and `git diff --check`.
- Fixed file-provider registry retention (#15). Each `open_with_system` content URI is now a ten-minute lease. The provider removes its registration on the main-thread timer and rejects/removes it on any later lookup, while preserving temporary read access long enough for the receiving app to open the file. Verified by compiling the actual Java source set with `:app:compileDebugJavaWithJavac` and `git diff --check`.
- Fixed the remaining #16 IME shadow-buffer retention. The Java `Editable` is not the authority for GPUI edit ranges—Rust uses the active input handler's selected/marked UTF-16 ranges—so the claimed direct range corruption was not reproducible. It did retain stale context across focused controls and grow without bound, however. Installing a new input handler now clears it, and every IME edit retains at most 4,096 UTF-16 code units. Together with the virtual-ID pruning recorded under #10, this resolves #16. Verified with `just showcase-android-check`, `:app:compileDebugJavaWithJavac`, `cargo fmt --check`, and `git diff --check`.
- Fixed lossy platform-view creation parameters (#17). The existing ordinary `key=value|…` representation is retained for compatibility, but entries are now sorted and `\\`, `=`, and `|` are escaped with backslash so arbitrary keys and values round-trip. The Java-factory decoding contract is documented on `AndroidPlatformViewFactory`; added a target-side regression test for separator and backslash encoding. Verified production code with `just showcase-android-check`, `cargo fmt --check`, and `git diff --check`.
- Removed unused `VelocityTracker.count` (#18). Sample-ring occupancy is already represented by the slots themselves and the age filter, and the counter had no read path. Verified behavior with `cargo test -p gpui-android momentum` (4 passed), plus `cargo fmt --check` and `git diff --check`.
- Fixed poisonable input-path mutexes (#19). The global IME queue and platform-view registry now use the crate's existing non-poisoning `parking_lot::Mutex`; all associated `.lock().unwrap()` calls are gone. A panic while mutating one of these small registries can no longer make subsequent keyboard or platform-view input panic. Verified with `cargo test -p gpui-android platform_view` (1 passed), `just showcase-android-check`, `cargo fmt --check`, a zero-result mutex-unwrap search over the affected files, and `git diff --check`.

## Clean bill

- Dispatcher pipe/ALooper mechanics (`dispatcher.rs:419-449`) correctly drain
  the non-blocking pipe and re-leak the `Arc` each callback; fds are
  non-blocking and closed on drop.
- Thermal polling is properly throttled to 1 Hz and gated behind API-29 error
  handling (`platform.rs:1235-1268`).
- Clipboard and credential JNI paths are careful: null checks on every
  intermediate object, exceptions cleared, AES-GCM with a fresh randomized IV
  per write (`GpuiActivity.java:191-228`).
- Emoji-font CBDT detection reads only the 12-byte header + table directory,
  avoiding a multi-MB copy just to choose a load strategy
  (`platform.rs:490-504`).
- Momentum math (exponential decay + displacement integration,
  `momentum.rs:173-209`) and the velocity weighted least-squares are correct
  and unit-tested.
- Key-code tables and meta-state bitmasks in `keyboard.rs` match
  `<android/keycodes.h>` / `<android/input.h>` on the values I spot-checked.

## Follow-up regression evidence

- Host-runnable lifecycle regressions now cover renderer restoration after unwinding, callback-visible active-state synchronization, and both dispatcher wake paths. `cargo test -p gpui-android` passed 11 library tests plus one doctest; `just showcase-android-check` also passed for `aarch64-linux-android`.
- Java JVM regressions now exercise FileProvider lease expiry/cleanup through the registry used by `GpuiFileProvider`, plus the IME shadow-buffer trim policy used by `GpuiInputView`. `:app:testDebugUnitTest` passes with the installed Android SDK.
