# Unreleased

## Added

- Split Android event handling into a tested `event_stages` module and added
  a `packages` module with deep-link and media stubs.
- Cached JNI class lookups and coalesced input events to cut per-event work.

## 0.9.5 - 2026-08-23

### Performance

- Retained Android keyboard-character-map state and reduced dispatcher and platform hot-path churn.

## Fixed

- Removed unsound `Send` transmutes from platform callbacks. Non-`Send`
  callbacks now remain on their owner thread, while off-thread quit requests
  are queued for delivery by the Android event loop.

# 0.9.3

## New

- Added native Android clipboard integration through `ClipboardManager`.
- Added Android Keystore-backed AES-GCM credential storage with a username
  index for enumerating saved credentials.
- Added complete IME bridging for committed and composing text, composition
  clearing, and backward deletion through the canonical `GpuiActivity`.
- Added an AccessKit-backed virtual accessibility provider for TalkBack.
- Added an initial Android GPUI platform backend scaffold, adapted from
  `itsbalamurali/gpui-mobile`, with NativeActivity lifecycle wiring, Android
  window/input modules, and wgpu/Vulkan surface plumbing.

## Fixed

- Added Android target and Java host compilation coverage to CI.
- Handle NativeActivity launches where a native window is already available
  before a fresh `InitWindow` event reaches the event loop, and mark Android
  windows active after surface initialization.
- Improve Android renderer startup diagnostics for emulator GPU stacks that
  only expose GLES without the storage-buffer limits required by GPUI.
- Resolve GPUI's `.SystemUI` virtual font family through the configured Android
  system fallback so Android themes can use the same system font alias as iOS.
