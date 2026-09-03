# Code Review: gpui-android — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-android` (Rust ~10k + Java 858 LOC)

## 1. Purpose / role
Android `Platform` backend via JNI + `NativeActivity`: `ANativeWindow` + `gpui_wgpu`, IME/keyboard/JNI bridge, Java `GpuiActivity` host. Largest: `android/window.rs` (2646), `android/platform.rs` (2222), `android/jni.rs` (1596), `android/keyboard.rs` (639), `GpuiActivity.java` (629), `display.rs` (402), `platform_view.rs` (404).

Public API (43 symbols): `current_platform(headless)` (`lib.rs:20`, panics off-Android), `SystemChromeStyle/StatusBarContentStyle/set_system_chrome()`, `KeyboardType/show/hide_keyboard`, `TEXT_INPUT_DIRTY`, `ImeEvent::{Commit,SetComposing,FinishComposing,DeleteSurrounding}` + `IME_EVENTS` queue, `credential_alias()`; `jni::{with_env,obtain_env,run_event_loop,poll_events,show_keyboard_android}` (`jni.rs:112,558,1020`); stub `deeplink/media_session` no-ops (`lib.rs:200-215`).

## 2. SOTA gap analysis (vs Jetpack Compose)
1. **No predictive-back / insets controller** — manual `getInsetsController` strings (`jni.rs:1193-1243`).
2. **No Material-You dynamic color**; dark-theme only via `query_night_mode_via_jni()` (`:349`).
3. **No foldable/window-size-class, PiP, multi-window.**
4. **Dead stubs** — `deeplink`/`media_session` have no `Intent` filter, Media3 session, or notification channels.
5. **No BiometricPrompt, CameraX, WorkManager, DataStore** — `platform.rs:268` Keystore host is in-memory double.
6. **No AppWidget, shortcuts/tiles, splash-screen API.**
7. **No navigation parity** — `platform_view.rs:113-115` visibility only; `GpuiActivity.java:457 createAccessibilityNodeInfo` 104 lines/cyclo 22, untested.

## 3. Performance evaluation
- `jni.rs:558 run_event_loop()` 315 lines/cyclo 48/cogn 173/MI 12.7/fan-out 58 — any JNI throw stalls loop.
- `window.rs:1623 on_input()` 314 lines/cyclo 36/nesting 7 — decode + `state.lock()` per event.
- `on_request_frame:1454` (168 lines) drains IME then draws; `update_ime_position:2052` (167 lines) calls `with_env()` per bounds change — JNI attach + lookup per keystroke.
- `process_input_events:366` (169 lines/cyclo 29) copies every IME string (`jni.rs:149-153`).
- ~20× `state.lock()` (`parking_lot::Mutex`) on draw/input/resize (`window.rs:474-1006`); `momentum.lock()` (`:847`) inside `draw(:802)` risks jitter.

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Split `run_event_loop`/`on_input` into poll→decode→dispatch→draw + tests (CRAP 2352/1332) | M | testability |
| 2 | Cache `jmethodID`/`jclass` (e.g. `setVisible`, `gpuiShowKeyboard:1286`) — current lookup per call | S | JNI overhead |
| 3 | Coalesce `update_ime_position` bounds updates; avoid `with_env` per keystroke | S | input latency |
| 4 | Copy `appearance/scale` out, release lock before `renderer.draw()` (`window.rs:813` hazard) | S | frame jitter |
| 5 | Implement or delete `deeplink`/`media_session` stubs | S | API honesty |

## 5. Verdict
Functional backend with real JNI costs on input/draw paths. SOTA = Compose services (back, dynamic color, foldables, media). Perf = cache JNI ids, shorten locks, batch IME.
