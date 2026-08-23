# Unreleased

## 0.9.6 - 2026-08-23

### Performance

- Retained Vello element state for meters, spectrum, and knob controls; added allocation-contract coverage for interactive rendering paths.

# 0.9.5

## Features

- Added semantic `on_commit` handlers to potentiometers, vertical sliders, and volume knobs for drag-release and discrete keyboard/scroll changes.
- Added native delta dragging to potentiometers while preserving click-step behavior for releases without movement.

# 0.7.3

## Features

- Added `gpui-audio-kit` as the dedicated home for audio controls,
  `AudioDesignTokens`, and audio-specific control helpers.
- Moved `Potentiometer`, `VerticalSlider`, `VolumeKnob`, audio interactions,
  scale utilities, meter elements, spectrum elements, meter data, and tick
  rendering helpers into the crate.
- Added reusable horizontal meter bar rendering helpers and
  `HorizontalMeterTheme` for LUFS, width, and peak-spread style meters.
- Added reusable spectrum frequency/dB axis label and render helpers for
  analyzer-style views.
- Added `AudioToggleExt` so UI-kit `Toggle` styling can still consume audio
  design tokens without re-exporting audio APIs from UI-kit.
- Added component-lab coverage for `VerticalSlider`, `VolumeKnob`, horizontal
  meter bars, and spectrum axes.
