# gpui-audio-kit

Audio-focused GPUI controls and visualizations for `gpui-toolkit`.

## Public Surface

- `Potentiometer`, `VerticalSlider`, `VolumeKnob`
- `AudioDesignTokens`, `AudioScale`, and audio interaction helpers
- `LevelMeterElement`, `MeterColors`, `HorizontalMeterTheme`,
  `render_horizontal_meter_bar`, and `render_horizontal_meter_bar_with`
- `SpectrumElement`, `SpectrumColors`, `MeterData`, `SpectrumAxisTheme`,
  `spectrum_frequency_axis_labels`, `spectrum_db_axis_labels`,
  `render_spectrum_frequency_axis`, and `render_spectrum_db_axis`
- `TickConfig`, `TickMark`, `ScaleType`, and `render_tick_row`
- `AudioToggleExt` for applying audio design tokens to `gpui_ui_kit::Toggle`

`gpui-ui-kit` intentionally does not re-export these APIs.

## Component Lab Coverage

`gpui-component-lab` includes renderer-backed stories for `Potentiometer`,
`VerticalSlider`, `VolumeKnob`, level meters, horizontal meter bars, spectrum
elements, and reusable spectrum axes.
