# Unreleased

## Added

- Added `token_export` (CSS-variable/Style-Dictionary round-trip with
  `gpui-design`) and `contrast_fix` (nearest passing WCAG color + editor
  badge).
- Theme mode preference now live-resolves OS dark mode, and transitions
  honor the reduced-motion gate.
- Added empty `gpu-2d`/`gpu-3d` feature flags for workspace feature
  unification.
- Checked `text_primary/background` before `text_on_accent/accent` in
  `accessibility_issues` so the most common failure reports first.

# 0.9.6 - 2026-08-23

## Fixed

- Restored child notification propagation for edited and built-in theme state.

# 0.7.4

## Maintenance

- Version bump; no user-facing changes.

# 0.6.2

## New

- Added a markdown editor as a demo for gpui-toolkit
- Started to migrate to new design/builder pattern
- Added a long list of new components wave 1

## Changes

- Feat(elisp): 100% subr.el compatibility (494/494 forms)
- Started the fully automated UI generation for plugins
- Manual review of sotf engine: found a lot of small bugs esp. on latency management
- Made the themes uniform
- Splitted autoeq UI from the the UI Kit
- Move crates around to match what is on crates.io and make it easier to update
