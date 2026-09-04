# Changelog

## Unreleased

### Added

- `?view=quad|text|both` query-param selection and a page-lifetime
  `ResizeObserver` example that logs body size changes to the console,
  both in the defensive boot-error fallback style.

### Fixed

- Boot `expect`s now render a `console_error` fallback instead of
  white-screening on browsers without WebGPU.
- Documented the page-lifetime-only `mem::forget` handle and asserted
  `web_mark_ready` in the smoke test.

## 0.1.1 - 2026-08-23

### Performance

- Added an opt-in browser scheduling-baseline harness for capturing `MessageChannel`, timer, and animation-frame dispatch latency.
