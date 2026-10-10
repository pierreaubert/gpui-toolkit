# Unreleased

# 0.10.11

## Added

- Added a dedicated `mini_app_state` module with expanded config/shell
  options and host lifecycle tests.
- Added `run_multi` (one window per config, shared actions/menus) and
  signature-guarded `refresh_menus` that skips rebuilds with unchanged
  checked states.
- Added `MiniAppConfig::initial_design` plus `?style=` web query support
  so embeds can force a design language (`apple`, `material3`,
  `fluent`, `neutral`, ...); unknown or empty values keep the platform
  default.

## New

- Added Android platform support so mini-app based showcase binaries can run
  through the `gpui-android` backend.

# 0.7.7

## Maintenance

- Version bump; no user-facing changes.
