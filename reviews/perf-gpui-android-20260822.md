# Perf review: gpui-android

Date: 2026-08-22

## Role and hot paths

`gpui-android` is the Android platform backend: an `ALooper`-driven event loop
(`android/jni.rs:533` `run_event_loop`), the `Platform`/`PlatformWindow`
implementations (`android/platform.rs`, `android/window.rs`), a wgpu renderer
delegated to upstream `gpui_wgpu::WgpuRenderer`, and text via
`gpui_wgpu::CosmicTextSystem` seeded with Roboto/Noto fonts.

Hot paths, in cost order:

1. **Main loop iteration** (`jni.rs:552-822`): non-blocking `poll_events` +
   `platform.tick()` + `flush_main_thread_tasks()` + `win.request_frame()` +
   a 500 µs sleep. Runs continuously while the app is foregrounded.
2. **Frame production**: `AndroidWindow::draw` → `WgpuRenderer::draw`
   (`window.rs:744`, `wgpu_renderer.rs:1066`). GPUI renders are demand-driven
   (`invalidator.is_dirty()`, gpui `window.rs:1526`), so `request_frame` per
   loop iteration is cheap when idle — the loop itself is not.
3. **Input**: touch → gesture state machine → coalesced `ScrollWheel` /
   momentum pump (`window.rs:1603-1817`); keys → JNI unicode lookup +
   keystroke construction (`jni.rs:238`, `keyboard.rs:322`); IME events via a
   global `Mutex<VecDeque>` drained every frame (`lib.rs:116-128`).
4. **Startup**: eager load of ~25 system fonts + emoji (`platform.rs:536-641`).

## Findings

1. **[Alloc/CPU] Busy-spin main loop with per-iteration JNI thermal query** —
   `run_event_loop` polls with `Duration::ZERO` and sleeps only 500 µs per
   iteration (`jni.rs:583`, `jni.rs:821`), so the loop free-runs at up to
   ~2000 iterations/s while active. Each iteration calls `platform.tick()`
   (`jni.rs:575`) → `check_thermal_state()` (`platform.rs:1266-1270`) →
   `query_thermal_status_via_jni()` (`platform.rs:1120-1166`), which does
   `env.new_string("power")` + `getSystemService` + `getCurrentThermalStatus`
   with **no throttling** — thousands of JNI roundtrips per second while
   idle. The comment at `jni.rs:580-582` claims "the GPU present call …
   provides natural frame pacing", but with `PresentMode::Mailbox`
   (`window.rs:1156`) acquire is typically non-blocking, and the 500 µs sleep
   contradicts "No sleep". Impact: battery drain, CPU wakeups, GC pressure in
   the JVM. Fix: block on the looper when nothing is pending (or use a real
   vsync/Choreographer signal), and throttle the thermal poll to ~1 Hz.

2. **[Alloc] Eager, duplicated font loading at startup** — `platform.rs:537-641`
   `std::fs::read`s ~25 files under `/system/fonts/` (plus a ~10 MB
   `NotoColorEmoji.ttf`, either system or bundled via
   `load_asset_bytes` `platform.rs:478-496` which does `bytes.to_vec()`) into
   `Cow::Owned` and hands them to `text_system.add_fonts`
   (`platform.rs:635`). `NotoSansCJK-Regular.ttc` alone is ~15–20 MB; the
   whole set is held resident in fontdb for the process lifetime even if the
   app only renders ASCII. Impact: multi-hundred-ms startup I/O and tens of MB
   permanent RSS (needs profiling on-device). Fix: register font *paths* with
   fontdb (mmap) or load Noto/CJK/emoji lazily on first fallback miss.

3. **[Roundtrip] Synchronous `device.poll(Wait)` on resize** —
   `AndroidWindow::handle_resize` → `WgpuRenderer::update_drawable_size`
   blocks the main thread on a full GPU wait (`wgpu_renderer.rs:952-957`)
   before reconfiguring the surface. Fires on rotation and on every
   `APP_CMD_WINDOW_RESIZED`. Only resize-time, but it is a full pipeline stall
   on the UI thread; the window code at least takes the renderer out of the
   state lock first (`window.rs:672-680`). Fix: defer texture destruction or
   use `PollType::Poll`/submission-index waits.

4. **[Alloc] Per-keystroke allocation cluster** — each hardware key event
   allocates: a Java `KeyEvent` object via JNI for the unicode lookup
   (`jni.rs:238-248`), `key.to_string()` in `android_keycode_to_key`
   (`keyboard.rs:313`), up to two more `String`s for `key_char`
   (`keyboard.rs:332-341`), and `c.to_string()` for the text-input side
   channel (`window.rs:1840`). ~5–6 small heap allocs + one JNI object per
   keypress. Fix: return `&'static str` from the keycode table (they are all
   literals), and cache the `KeyCharacterMap` result or use the NDK
   `AKeyEvent` if the SDK level allows.

5. **[Alloc/JNI] `update_ime_position` is 8 JNI calls + 2 Java allocations per
   call** — `window.rs:1999-2111` re-acquires `InputMethodManager`, builds a
   fresh `CursorAnchorInfo$Builder`, and re-fetches `getWindow()/getDecorView()`
   every time. GPUI calls this on every caret/scroll change while the IME is
   visible, i.e. per keystroke during composition. Fix: cache global refs to
   IMM and the decor view once, and early-out when bounds are unchanged.

6. **[Alloc] Per-frame IME drain allocates an empty `Vec`** —
   `drain_ime_events()` (`lib.rs:126-128`) does `drain(..).collect()` on every
   frame from the request-frame callback (`window.rs:1419`), at up to 120 Hz,
   even when the queue is empty. Trivial fix: check `is_empty()` first or swap
   out a reusable `VecDeque`.

7. **[Event volume] Uncoalesced `MouseMove` during scroll drags** — scroll
   deltas are correctly coalesced to one `ScrollWheel` per frame
   (`window.rs:97-105`, `1450-1487`), but every raw `ACTION_MOVE` *also*
   dispatches a `MouseMove` into GPUI (`window.rs:1717-1722`) at touchscreen
   rate (~120–240 Hz), each triggering GPUI hover hit-testing/layout
   invalidation. Fix: coalesce or suppress MouseMove while the gesture is
   `Scrolling` (needs profiling to size the win).

8. **[Alloc] `FallbackAtlas` burns rasterization work and grows unboundedly** —
   while no surface exists, `get_or_insert_with` runs the (CPU swash) glyph
   build closure, then **discards the pixels** and stores a fake tile
   (`window.rs:2141-2178`). `state.tiles: HashMap<AtlasKey, AtlasTile>` never
   evicts. Edge-case (only active between `term_window`/`init_window`), but a
   resume storm could rasterize glyphs into the void and hold stale entries.
   Fix: no-op without calling `build()`, or clear on `init_window`.

9. **[GPU] Draw path is clean; no readbacks** — positive finding:
   `WgpuRenderer::draw` renders scene batches straight to the surface with
   `Mailbox` present and no `map_async`/readback (the only `map_async`,
   `wgpu_renderer.rs:2170`, is in headless `render_scene_to_image`, unused by
   Android). `device.poll(Wait)` appears only at resize (finding 3) and device
   creation (`wgpu_context.rs:39-46`). Text rasterization stays CPU-side
   (swash via `CosmicTextSystem`, `platform.rs:530`) — inherent to the GPUI
   design, not this crate. No GPU-opportunity action items beyond upstream.

10. **[Alloc] Minor per-event/per-frame chatter** — every
    `dispatch_on_main_thread` boxes the closure *and* writes a wake byte
    (`dispatcher.rs:512-521`); `dispatch_after` re-sorts a `Vec` per insertion
    (`dispatcher.rs:535`) and `tick()` allocates a fresh `ready` `Vec` every
    frame (`dispatcher.rs:296`); every a11y tree update triggers a JNI
    `notifyAccessibilityChanged` (`accessibility.rs:82-86`, `jni.rs:1270`);
    every touch/key event takes+puts the callback `Mutex` twice
    (`window.rs:813-835`, `845-857`). All small; batch or debounce if
    profiling shows them.

Existing perf notes: no TODO/FIXME/perf annotations in the crate; the
frame-rate request is cached via `OnceLock` (`window.rs:148-172`) and scroll
coalescing is documented in code (`window.rs:97-105`). No criterion benches or
allocation-count tests for this crate.

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | Block on looper when idle; drop the 500 µs spin; throttle thermal JNI to ~1 Hz | 1 | M | Battery/CPU, largest win |
| 2 | Font loading: paths/mmap instead of eager `fs::read`; lazy Noto/emoji | 2 | M | Startup time + tens of MB RSS |
| 3 | `&'static str` key table; avoid per-key JNI `KeyEvent` | 4 | S | Per-keystroke allocs → ~0 |
| 4 | Cache IMM/decor-view global refs; skip unchanged IME bounds | 5 | S | Typing latency on JNI-heavy devices |
| 5 | Empty-check before `drain_ime_events` collect | 6 | S | Removes a 120 Hz alloc |
| 6 | Coalesce/suppress `MouseMove` during scroll gestures | 7 | S | Fewer layout passes while scrolling (needs profiling) |
| 7 | Avoid blocking `device.poll(Wait)` on the main thread at resize | 3 | M | Rotation jank |
| 8 | Make `FallbackAtlas` not run `build()` / clear on surface recreate | 8 | S | Resume-path waste |

## Quick wins

- Guard `drain_ime_events` with `is_empty()` (finding 6) — a few lines.
- `android_keycode_to_key` → `Option<&'static str>`, push `String` conversion
  to the single `Keystroke` construction site (finding 4).
- Throttle `check_thermal_state` with a `last_check: Instant` ≥ 1 s
  (finding 1, partial — the spin loop itself is the bigger fix).
- Early-out in `update_ime_position` when bounds unchanged (finding 5).
- `FallbackAtlas::get_or_insert_with`: return `Ok(None)` without invoking
  `build()` (finding 8).
