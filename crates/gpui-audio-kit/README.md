# gpui-audio-kit

Audio-focused GPUI controls and visualizations for `gpui-toolkit`.

## Public Surface

- `Potentiometer`, `VerticalSlider`, `VolumeKnob`
- `AudioDesignTokens`, `AudioScale`, and audio interaction helpers
- `audio_automation_pattern_report()` for release-ready automation-control
  patterns covering gain, frequency, fader, monitor volume, and meter feedback
- `audio_visual_regression_manifest()` for CI-ready audio control screenshot
  coverage across component-lab story ids, viewports, and color schemes
- `LevelMeterElement`, `MeterColors`, `HorizontalMeterTheme`,
  `render_horizontal_meter_bar`, `render_horizontal_meter_bar_with`, and
  `horizontal_meter_accessibility_summary`
- `SpectrumElement`, `SpectrumColors`, `MeterData`, `SpectrumAxisTheme`,
  `spectrum_frequency_axis_labels`, `spectrum_db_axis_labels`,
  `render_spectrum_frequency_axis`, and `render_spectrum_db_axis`
- `TickConfig`, `TickMark`, `ScaleType`, and `render_tick_row`
- `AudioToggleExt` for applying audio design tokens to `gpui_ui_kit::Toggle`

`gpui-ui-kit` intentionally does not re-export these APIs.

## Accessibility Metadata

Interactive audio controls register ARIA-style metadata during render and also
expose non-rendering summaries for hosts, tests, and future native accessibility
bridges. `AudioAccessibilitySummary` captures the control type, label, ARIA
role, value/range text, normalized position, state flags, peak values, and a
screen-reader-friendly description.

```rust
use gpui_audio_kit::{Potentiometer, AudioScale};

let summary = Potentiometer::new("freq")
    .label("Frequency")
    .value(1000.0)
    .min(20.0)
    .max(20_000.0)
    .unit("Hz")
    .scale(AudioScale::Logarithmic)
    .accessibility_summary();

assert_eq!(summary.control_type, "potentiometer");
assert_eq!(summary.value_text.as_deref(), Some("1000 Hz"));
```

## Automation Patterns

`audio_automation_pattern_report()` is a non-rendering release artifact for
plugin and playback control automation. It records stable patterns for
continuous gain/mix controls, logarithmic frequency controls, vertical channel
faders, monitor-volume mute controls, and read-only meter feedback. Each row
names the recommended control, scale, expected interactions, accessibility
summary contract, and release evidence.

```rust
use gpui_audio_kit::audio_automation_pattern_report;

let report = audio_automation_pattern_report();
assert!(report.blocking_entries().is_empty());
assert!(report.pattern("log-frequency").is_some());
```

## Visual Regression Manifest

`audio_visual_regression_manifest()` is a stable capture inventory for audio UI
release QA. It expands the renderer-backed component-lab audio stories across
desktop-panel and compact-strip viewports plus light, dark, and high-contrast
color schemes. Each capture records deterministic baseline, actual, and diff
artifact paths under `artifacts/gpui-audio-kit/visual/`.

```rust
use gpui_audio_kit::audio_visual_regression_manifest;

let manifest = audio_visual_regression_manifest();
assert_eq!(manifest.capture_count(), manifest.expected_capture_count());
assert!(manifest.validate_unique_capture_ids());
```

## Component Lab Coverage

`gpui-component-lab` includes renderer-backed stories for `Potentiometer`,
`VerticalSlider`, `VolumeKnob`, level meters, horizontal meter bars, spectrum
elements, and reusable spectrum axes. The visual regression manifest uses the
same story ids so screenshot runners can capture and diff those surfaces without
duplicating the audio-kit coverage list.
