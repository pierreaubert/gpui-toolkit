# Unreleased

# 0.11.1

## Added

- `FrameScheduler`: the common dirty/throttle/resize-debounce frame
  policy for host pumps (clean ticks render nothing, bursts
  coalesce), with rendered/idle/coalesced counters.

## Removed

- **Breaking:** the borrowed-pointer `EditorRegistry` (and
  `no_thread_assertion`) is deleted. Every binding migrated to
  `OwnedEditorRegistry` tokens, leased window slots, or `Arc` leases;
  the raw-pointer registry remains only in git history.

# 0.11.0

# 0.11.0

## Added

- New platform-neutral core for foreign-parent editor embedding (GPUI inside
  host-owned views): keyed `EditorRegistry`, `FrameThrottle` /
  `SyncFrameThrottle`, `ResizeDebounce` / `SyncResizeDebounce`,
  take-invoke-restore `CallbackSlot` / `TransitionCell`, and null-safe FFI
  buffer helpers (`fill_out_buffer`, `optional_c_string`).
- Contract: `docs/foreign-parent-editor-embedding.md` (M1).
