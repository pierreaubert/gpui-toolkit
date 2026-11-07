# Unreleased

# 0.11.0

# 0.11.0

## Added

- New Linux foreign-parent binding: X11 child windows (VST3, CLAP-x11) and
  Wayland subsurfaces (CLAP-wayland) via the `gpui_linux` fork paths, with
  host input-forwarding FFI for both backends and content injection.
- Known Wayland gaps: touch, key-repeat timers, text-input protocol,
  cursor-shape, clipboard/data-device, and output tracking.
