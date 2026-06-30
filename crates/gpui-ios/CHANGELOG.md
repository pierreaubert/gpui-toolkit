# Unreleased

## Fixes

- Restored the iOS backend to the legacy `UIWindow` `initWithFrame` path after
  the attempted `UIScene` migration produced a black showcase screen.
- Added iPad pointer and Simulator wheel-scroll handling through an indirect
  scroll pan recognizer, including frame requests for wheel, touch, and
  momentum scroll updates.
- Improved one-finger direct-touch scrolling so vertical drags dispatch GPUI
  scroll-wheel events without accidentally activating menu rows or buttons.
- Filtered modifier-only hardware key presses so iOS no longer reports
  `unknown-e3` style key events for pure modifier changes.

# 0.7.6

## Maintenance

- Version bump; no user-facing changes.

# 0.6.2

## Fixes

- **safe_area_insets**: doc comment now matches the actual return tuple
  order — `(top, left, bottom, right)`, matching `UIEdgeInsets` field
  order. Code behavior unchanged; only the documentation was stale.

# 0.6.1

## New

- Added a markdown editor as a demo for gpui-toolkit
- Started to migrate to new design/builder pattern
- Merged a long list of new features into the apps (wired new engine, update of config files, export filters to new applications)
- Ios: added support for svg icons, added settings in menu, can now move the IIR in the EQ plugin

## Fixes

- Did a round of test fixing

## Changes

- More or less working version on iOS
