# Unreleased

## Added

- Added showcase sections for the 30 new UI-kit components, with
  interactive state for calendar month navigation/day picking,
  pagination page windows, and list row selection.
- Added `showcase_interactions` integration tests and a `showcase_group`
  module; fixed form/qr section rendering and release-artifact reporting.

## Fixed

- Stopped panicking on keystroke parse in the allocation-contracts input
  test; a failed parse now resets `input_editing` and returns early.
