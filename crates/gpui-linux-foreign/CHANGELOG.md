# Unreleased

# 0.11.1

## Changed

- Editors are owned by a thread-local `OwnedEditorRegistry` under
  non-reused checked tokens (the numeric editor id is the token, not
  a pointer); closing during a callback defers destruction safely.
- Foreign-thread calls are rejected in release builds (previously a
  debug-only assertion).
- The host pump drives the shared `FrameScheduler` policy, and all
  22 C exports run inside `ffi_boundary`.

# 0.11.0

# 0.11.0

## Added

- New Linux foreign-parent binding: X11 child windows (VST3, CLAP-x11) and
  Wayland subsurfaces (CLAP-wayland) via the `gpui_linux` fork paths, with
  host input-forwarding FFI for both backends and content injection.
- Known Wayland gaps: touch, key-repeat timers, text-input protocol,
  cursor-shape, clipboard/data-device, and output tracking.
