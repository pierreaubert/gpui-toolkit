# Unreleased

## Fixes

- Added `UIApplicationSupportsIndirectInputEvents` to the showcase app plist so
  iPad pointer and Simulator wheel input use the modern indirect-input path.
- Kept the showcase on the legacy `AppDelegate` lifecycle and `UIWindow`
  creation path after validating that the attempted `UIScene` migration caused
  a black screen in the Simulator.
- Verified Simulator wheel scrolling works when the Simulator input-capture
  button is enabled.

# 0.1.1

## Maintenance

- Version bump; no user-facing changes.

# 0.1.0

## Changes

- AU plugins are working and I can load them but without a proper UI
- More or less working version on iOS
