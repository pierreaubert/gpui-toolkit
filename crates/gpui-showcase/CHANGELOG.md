# Unreleased

# 0.11.0

## Added

- Added a `retained_frames` criterion benchmark (steady-state frames:
  one notified panel of sixty, a scrolling 5000-row uniform list, and a
  refreshed Table section), each run retained vs from-scratch via
  `Window::set_view_retention`, wired into the `qa-perf` gate. First
  paired record: `qa/perf/20261107-retained-frames-paired.json`.
- Added an `e2e_frames` criterion benchmark (cold-open plus first frame
  for the buttons, table, and tree-view sections) wired into the
  `qa-perf` non-regression gate.

## Fixed

- Set the `I18nState` global in the headless `--visual-capture` harness
  (mirroring `MiniApp::run` with `with_i18n(true)`); translated titles
  previously rendered as `???`, breaking wasm-vs-native parity.
- Switched the headless `--visual-capture` text stack to
  `CosmicTextSystem` with `bundled_ui_fonts()` (no system fonts), matching
  the wasm platform exactly; the CoreText system stack resolved "IBM Plex
  Sans" to a wider fallback face whose extra wrapped lines shifted layout
  (dialog/popover/tooltip parity failures).
- Memoized showcase `User` cell strings (`SharedString` fields plus
  `id_text`) so table cell renderers do Rc bumps instead of per-frame
  `String` clones and integer formatting.

# 0.10.6

## Added

- Added showcase sections for the 30 new UI-kit components, with
  interactive state for calendar month navigation/day picking,
  pagination page windows, and list row selection.
- Added `showcase_interactions` integration tests and a `showcase_group`
  module; fixed form/qr section rendering and release-artifact reporting.

## Fixed

- Stopped panicking on keystroke parse in the allocation-contracts input
  test; a failed parse now resets `input_editing` and returns early.
