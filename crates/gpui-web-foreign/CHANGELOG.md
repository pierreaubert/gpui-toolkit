# Unreleased

# 0.11.1

## Changed

- The `requestAnimationFrame` pump runs through the shared
  `FrameScheduler` policy and stops while the editor is clean:
  invalidations (input, resize, composition, focus) restart it.
  Browsers no longer wake idle editors 60-120x/s forever.

# 0.11.0

# 0.11.0

## Added

- New browser foreign-parent binding: page-owned canvas host shell
  (`host.rs`: observers, rAF pump, listener lifecycle with drop/frame
  counters) plus pure-DOM pointer/keyboard/composition tables (`input.rs`).
- Headless until the renderer-provider seam lands: painting needs the §16
  `gpui_web` fork change; WebGPU-only with async device init.
