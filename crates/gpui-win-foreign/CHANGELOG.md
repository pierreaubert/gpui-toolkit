# Unreleased

# 0.11.0

# 0.11.0

## Added

- New Windows foreign-parent binding (VST3): GPUI editors in host-owned
  `HWND`s via the `WS_CHILD` creation path in the `gpui_windows` fork
  (`WindowsPlatform::new_foreign`, pending-foreign routing, synchronous
  `run`). The host pumps frames, owns the message loop, and forwards
  resize/focus; C ABI (`gpui_win_create/destroy/request_frame/resize/
  set_active/set_hovered/frame_stats/child_hwnd`) with content injection.
