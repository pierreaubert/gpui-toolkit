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
