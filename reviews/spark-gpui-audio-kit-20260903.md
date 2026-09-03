# Code Review: gpui-audio-kit — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-audio-kit` (~31 files, ~8.8k LOC)

## 1. Purpose / role
Audio plugin-style controls (knobs, sliders, meters, spectrum); general widgets stay in `ui-kit`. Largest: `audio/potentiometer.rs` (1236), `audio/vertical_slider.rs` (1232), `meter.rs` (1077), `volume_knob.rs` (585), `ticks.rs` (502).

Public API: `Potentiometer{,Scale,Size,Theme}`, `VerticalSlider{,Scale,Size,Theme}`, `VolumeKnob`, `DragState/InteractionConfig/ValueTracker/handle_drag/scroll/keyboard` (`lib.rs:20-30`), `LevelMeterElement/MeterColors/db_to_position/render_horizontal_meter_bar*` (`meter.rs:35,62,77,105,164,434`), `SpectrumElement::new(Arc<[f32]>)`, `TickConfig::generate_ticks()->Arc<[TickMark]>`, `AudioAutomationPatternReport/AudioVisualRegressionManifest/AudioDesignTokens`.

## 2. SOTA gap analysis (vs JUCE, iPlug2, Kontakt)
1. **No DSP/audio-thread bridge** — pure UI, no parameter-value-tree, no lock-free meter FIFO.
2. **No MIDI-learn / host-automation binding.**
3. **No filmstrip/SVG skinning** or resizable vector skins.
4. **Thin gesture model** — missing velocity/inertia, Shift-drag fine-adjust, double-click-reset (JUCE standard).
5. **No host snapshot testing** (AUv3/VST3 harness).
6. **Summary-string-only a11y** (`meter.rs:176`, `audio_accessibility.rs:60`) — no screen-reader value announcements.
7. **No preset / A-B comparison story.**

## 3. Performance evaluation
Coverage 5% (13/265 tested). `vertical_slider.rs:384 render` 687 lines/cyclo 86/cog 184/MI 0.0/fan-out 131; `potentiometer.rs:398 render` 676 lines/cyclo 99/cog 227; `volume_knob.rs:281 render` 304 lines/cyclo 47. Per-frame `format!` in render (`vertical_slider.rs:93,348,366-378`, `potentiometer.rs:335,355-392`, `meter.rs:523`, `volume_knob.rs:250`) + `Rc::new` handler re-wrap per render (`vertical_slider.rs:555-577`, `potentiometer.rs:568-570`). Untrusted-path `unwrap`: `potentiometer.rs:333`, `vertical_slider.rs:346` (`chars().next().unwrap()`), `calculate.rs:194` (`partial_cmp().unwrap()` NaN panic).

## 4. Recommendations
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Split `render` into track/fill/thumb/label/overlay (<150 lines each) | M | removes 0.0-MI functions |
| 2 | Hoist `format_value`/`ElementId` strings — cache `SharedString` on value change | S | per-frame allocs → ~0 |
| 3 | Replace `unwrap`s with `unwrap_or`/NaN guards (`:333/:346/:194`) | S | panic safety |
| 4 | Test `format_value_abbrev` (risk 1174), `db_to_position` (risk 499), `generate_ticks:184` | S | top-risk cover |
| 5 | Add meter FIFO (`Arc<TripleBuffer<Vec<f32>>>`) so spectrum need not imply per-frame `Rc<RefCell<Vec>>` (`spectrum_element.rs:220`) | M | realtime safety |

## 5. Verdict
Good control styling, missing realtime/audio-host plumbing. Split god renders, cache strings, fix NaN unwraps, add FIFO.
