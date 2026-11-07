# Unreleased

# 0.11.0

# 0.11.0

## Added

- New platform-neutral core for foreign-parent editor embedding (GPUI inside
  host-owned views): keyed `EditorRegistry`, `FrameThrottle` /
  `SyncFrameThrottle`, `ResizeDebounce` / `SyncResizeDebounce`,
  take-invoke-restore `CallbackSlot` / `TransitionCell`, and null-safe FFI
  buffer helpers (`fill_out_buffer`, `optional_c_string`).
- Contract: `docs/foreign-parent-editor-embedding.md` (M1).
