# Perf review: gpui-hello-web

Date: 2026-08-22

## Role and hot paths

`gpui-hello-web` is a 55-line wasm spike (`crates/gpui-hello-web/src/main.rs`): one
`Render` impl drawing a static two-quad + one-text scene (main.rs:11-25), started via
`gpui_miniapp::web_init()` + `Application::with_platform(...).run_embedded(...)`
(main.rs:28-49). The crate itself has **no per-frame code** — no animation, no timers,
no event-driven updates beyond GPUI's defaults.

The actual hot path is the platform chain the spike exercises, which is shared by
`gpui-showcase` and `gpui-px-showcase` (all go through `gpui_miniapp::current_platform()`
→ `gpui_web::WebPlatform`, crates/gpui-miniapp/src/misc.rs:34-37):

- Frame pump: `WebWindowInner::create_raf_closure` (crates/3rdparties/gpui_web/src/window.rs:324-352).
- Frame dispatch: GPUI core `on_request_frame` handler (crates/3rdparties/gpui/src/window.rs:1463-1552)
  and `Window::present` (crates/3rdparties/gpui/src/window.rs:2773-2779).
- Rasterization: `WgpuRenderer::draw` (crates/3rdparties/gpui_wgpu/src/wgpu_renderer.rs:1066-1198).
- Resize: `WebWindow::draw` → `WgpuRenderer::update_drawable_size`
  (gpui_web/src/window.rs:719-735, wgpu_renderer.rs:929-979).
- Events: `gpui_web/src/events.rs` pointer/wheel handlers — no per-event heap churn of note.

No TODO/FIXME/perf notes in the crate; no benches; `qa/perf/` has no hello-web entries.
`just wasm-visual hello 8080 gpui-hello-web` is the only existing perf-adjacent harness.

## Findings

1. **[GPU|Alloc] Idle page re-renders at full display refresh rate, forever.**
   The rAF loop calls `request_frame` with `require_presentation: true` on every vsync
   and unconditionally re-schedules itself (gpui_web/src/window.rs:329-345). In GPUI core
   this forces `needs_present = true` every frame (gpui/src/window.rs:1522-1524) and also
   disables the frame throttle (gpui/src/window.rs:1478-1489 — the `!require_presentation`
   guard fails), so even with a clean invalidator `window.present()` runs
   (gpui/src/window.rs:1541-1544), which calls `platform_window.draw(&rendered_frame.scene)`
   (gpui/src/window.rs:2774). Result: `WgpuRenderer::draw` re-acquires the surface texture,
   creates a fresh command encoder, re-uploads globals (3× `queue.write_buffer`,
   wgpu_renderer.rs:1172-1186) and **re-uploads every primitive batch's instance data**
   (`write_to_instance_buffer`, wgpu_renderer.rs:1950-1968) at 60–120 Hz for a scene that
   never changes. Contrast: gpui_windows passes `require_presentation: false`
   (crates/3rdparties/gpui_windows/src/events.rs:1192-1194). Impact: permanent 100% GPU/CPU
   duty cycle on an idle static page — battery, thermals, and it starves the browser
   compositor. This is the single biggest perf item in the hello-web path, and it is
   inherited verbatim by the showcase apps.

2. **[Alloc] No scene-unchanged early-out in `WgpuRenderer::draw`.**
   Even where re-presentation is legitimately requested, draw re-encodes all batches and
   re-writes the instance buffer from scratch (wgpu_renderer.rs:1189-1230, 1628-1650,
   1950-1968); nothing keys off a scene epoch/dirty flag. For hello-web the payload is
   ~3 instances (negligible), but the same renderer serves the px/showcase scenes where an
   unchanged-scene fast path would skip nearly all per-frame upload work
   (needs profiling to size; depends on fixing #1 first, since #1 is what makes idle
   re-draws happen at all).

3. **[Roundtrip] Blocking `device.poll` on the resize path is a no-op on wasm.**
   `update_drawable_size` calls `device.poll(PollType::Wait)` to "wait for any in-flight
   GPU work to complete before destroying textures" (wgpu_renderer.rs:951-965), and this
   runs on wasm: `WebWindow::draw` applies pending resize → `update_drawable_size`
   (gpui_web/src/window.rs:719-735). On the webgpu backend, `poll` returns
   `Ok(QueueEmpty)` immediately ("Device is polled automatically", zed wgpu fork
   `wgpu/src/backend/webgpu.rs:2560-2562`), so it does not hang — but the synchronization
   the comment promises does not exist on web. Currently survivable (WebGPU refcounts
   resources), but it is a latent hazard if native-style in-flight assumptions creep in,
   and it is the only `PollType::Wait` on the wasm runtime path.

4. **[Roundtrip] The only GPU→CPU readback is correctly out of the wasm path.**
   `copy_texture_to_buffer` + `map_async` + `PollType::Wait` exist in
   `WgpuHeadlessRenderer::readback` (wgpu_renderer.rs:2146-2201) but are gated behind
   `#[cfg(feature = "headless-qa")]` (wgpu_renderer.rs:2073) — never compiled into
   hello-web. No action; noted to confirm the canonical roundtrip anti-pattern from the
   d3rs `gpu2d` review is not present here.

5. **[Alloc|Latency] Main-thread runnable dispatch uses `setTimeout(0)`.**
   Non-realtime `dispatch_on_main_thread` schedules via
   `set_timeout_with_callback_and_timeout_and_arguments_0(callback, 0)`
   (gpui_web/src/dispatcher.rs:313-318, with a `TODO-Wasm` at line 314). Browsers clamp
   nested timeouts to ~4 ms, adding latency to GPUI effect/notification flushing after
   input. A `MessageChannel` port-post or `queueMicrotask` loop (already used for
   `RealtimeAudio`, dispatcher.rs:310-312) avoids the clamp. Minor for hello-web; matters
   for interactive wasm apps.

6. **[GPU] No missed GPU opportunity inside the crate itself.**
   The spike's scene is two colored quads plus one text run, all rasterized GPU-side by
   `WgpuRenderer`; there is no CPU rasterization, transform, or marshalling work in
   `crates/gpui-hello-web/src/main.rs` to move. One-time startup costs (8 embedded TTF
   fonts decoded at boot, gpui_web/src/platform.rs:23-32,73-79) are out of the hot path.

## Recommendations

| # | Action | Finding | Effort | Expected payoff |
|---|--------|---------|--------|-----------------|
| 1 | Pass `require_presentation: false` in the web rAF loop (keep the loop as the frame pump; let the dirty flag gate `present()`), matching gpui_windows | 1 | S | Idle wasm pages drop from 60–120 Hz re-render to ~0 GPU work; largest win, one-line class change — verify text-cursor blink and resize still repaint (they set dirty) |
| 2 | Add a scene-unchanged early-out (epoch/compare) in `WgpuRenderer::draw` to skip batch re-upload when presenting an identical scene | 2 | M | Cuts per-frame `write_buffer` churn on re-present; biggest for showcase/px scenes |
| 3 | Remove or cfg-gate the `PollType::Wait` in `update_drawable_size` on wasm, and fix the comment so the false guarantee doesn't get relied on | 3 | S | Hygiene; prevents a latent wasm correctness trap |
| 4 | Replace `setTimeout(0)` main-thread scheduling with `MessageChannel`/`queueMicrotask` | 5 | S | ~4 ms lower input→effect latency on wasm |
| 5 | Before/after measurement for #1/#2 with `gpui-profiler` + a `just wasm-visual` frame capture, since qa/perf has no wasm baseline yet | 1,2 | S | Turns the top two findings from code-read evidence into measured numbers |

## Quick wins

- Finding 1: flip `require_presentation: true` → `false` at gpui_web/src/window.rs:334 and
  smoke-test with `just wasm-serve-hello` + `just wasm-visual hello 8080 gpui-hello-web`;
  expect the idle page's GPU timeline to go flat.
- Finding 3: `#[cfg(not(target_family = "wasm"))]` the blocking poll (or delete it — it is
  a no-op on web) and correct the misleading comment at wgpu_renderer.rs:951.
- Finding 5: capture one idle-minute profile of hello-web in Chrome (Performance panel or
  `about:tracing` gpu category) to confirm finding 1's duty-cycle claim before landing.
