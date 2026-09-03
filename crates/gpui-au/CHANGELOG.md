# Unreleased

## Added

- Added a `params` bridge module with lazy wgpu initialization and new FFI
  entries (plus `gpui_au.h` declarations) for host parameter exchange.
- Cleaned up `NSLog` hygiene and added renderer safety counters.

## 0.9.8 - 2026-08-23

### Performance

- Reused caller-owned glyph-layout buffers in the Audio Unit text system to reduce realtime UI allocation and lock contention.

# 0.9.7

## New

- Added key-down and key-up FFI entry points for forwarding AppKit keyboard
  events into embedded GPUI views.
- Added FFI entry points for UTF-8 text commits, marked-text updates,
  composition clearing, and backward deletion.
- Added the public C integration header at `include/gpui_au.h` and documented
  the corresponding Swift/AppKit host wiring.
- Added a dedicated Audio Unit build and test lane to CI.

# 0.7.5

## Performance

- Added a shaped-line layout cache to `AuTextSystemState`, keyed by text, font
  size, and run hash, so repeated layout calls reuse shaped `LineLayout`
  results.

# 0.6.3

## New

- Started to migrate to new design/builder pattern

## Fixes

- Refactoring: fixed remaining tests, all green

## Changes

- Started the fully automated UI generation for plugins
- Road the working AU plugins
- Continue to work on AU plugins (still not working)
- Next step of UI implementation for plugins
