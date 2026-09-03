# Code Review: gpui-ios — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-ios` (~10k LOC)

## 1. Purpose / role
iOS Metal platform backend: vendored `Platform` impl with `CAMetalLayer` + `gpui_wgpu`, touch/keyboard/safe-area/text via ObjC FFI. Largest: `ios/window/ios_window.rs` (2159), `ios/window/register.rs` (690), `ios/text_system/ios_text_system_state.rs` (565), `accessibility.rs` (529), `native.rs` (349), `ios/platform.rs` (326).

Public API (38 symbols): `IosPlatform`, `current_platform()` (cfg ios/tvos, `lib.rs:216-219`), `safe_area_insets()` (`:166`), `show/hide_keyboard[_with_type]` (`:111-133`), `KeyboardType/StatusBarContentStyle`, `TEXT_INPUT_DIRTY`, `keyboard_height`, `scene_metrics()`, `native_bridge_report()`, `begin/end_metal_capture()`; `momentum` (`VelocityTracker/MomentumScroller`), `pencil` (`IosPencilSample`), `native` (`SizeClass/DynamicTypeCategory`), `widget` (`WidgetSnapshotRequest`).

## 2. SOTA gap analysis (vs SwiftUI/UIKit)
1. **No declarative navigation** (`NavigationStack`, deep-link routing) — only `document_picker.rs`.
2. **No haptics** (`CoreHaptics`/`UIFeedbackGenerator`) — zero hits in `src/`.
3. **No LocalAuthentication** (FaceID/TouchID) — only `ios/keychain.rs:133` Keychain.
4. **No UserNotifications/push, BackgroundTasks.**
5. **No share-sheet, drag-and-drop, context menus.**
6. **No Camera/Photos/HealthKit/StoreKit/Siri/App-Clip bridges** — `widget.rs:1-61` is snapshot-files only, not WidgetKit.
7. **Incomplete tvOS focus engine** (cfg-gated `lib.rs:210-213`, no focus nav API).
8. **No DynamicType live scaling** beyond `native.rs:34-68` enum.

## 3. Performance evaluation
- `ios_window.rs:213 new()` 262 lines/fan-out 61 — CAMetalLayer + wgpu init inline; slow first-frame.
- `handle_touch_inner:765` 284 lines/cyclo 22/cogn 54/nesting 6 — touch→mouse/scroll synthesis on UI thread.
- `refresh_accessibility:1216` 195 lines/cyclo 22/5 loops — full-tree diff per-frame risk; only `accessibility.rs:401` diff helper is tested.
- `draw:2118` holds `renderer.lock()` across draw; `sprite_atlas()` (`:2127-2142`) double-locks; contends with `:452` writer.
- `rasterize_glyph_into:314` (107 lines) + `layout_line_uncached:466` (95 lines/2 unsafe) — no glyph-cache eviction; `HashMap` inserts (`:236,:264`) unbounded.

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Split `ios_window.rs` — extract touch/a11y/renderer modules + per-method benches | M | maintainability |
| 2 | Narrow `renderer.lock()` in `draw()`; reuse `sprite_atlas` without second lock | S | frame jitter |
| 3 | Throttle `refresh_accessibility()` via incremental `compute_accessibility_diff_into` | S | per-frame diff cost |
| 4 | Add font/glyph LRU with size cap (`ios_text_system_state.rs:174`) | S | memory bound |
| 5 | Add haptics + biometric + notification FFI before SOTA claims | M | platform parity |

## 5. Verdict
Credible Metal backend; SOTA gap is OS-service bridges, perf gap is locking + unbounded caches. Note `publish=false` due to `gpui_wgpu` git dep (`Cargo.toml:13`).
