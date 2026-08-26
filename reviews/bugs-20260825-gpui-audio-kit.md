# Bug Review: gpui-audio-kit — 2026-08-25

Scope: full scan of `crates/gpui-audio-kit` — all 25 files under `src/` (~6.4k
lines: `potentiometer.rs`, `vertical_slider.rs`, `volume_knob.rs` and their
submodules, `meter.rs`, `ticks.rs`, `scale.rs`, `spectrum*`,
`audio_accessibility.rs`, `audio_design_tokens.rs`,
`audio_automation_patterns.rs`, `audio_visual_regression.rs`), plus
`Cargo.toml`, examples, and the test tree. Where the crate leans on shared
machinery I followed it into `gpui-ui-kit` (`interaction.rs`, `scale.rs`),
`gpui-d3rs` (`vello2d` painter), and the vendored `gpui` keystroke table —
those cross-crate reads were context only, no files were modified. Source
review only; I did not run the crate's test suite.

## Findings

Ranked by severity.

### High

1. **`VerticalSlider` double-fires `on_select` and the double-click reset when
   the pointer is over the track** — `src/audio/vertical_slider.rs:833-858`
   (track `on_mouse_down` calls `on_select_track` and deliberately does *not*
   stop propagation, per the comment at line 834) and
   `src/audio/vertical_slider.rs:574-587` (container `on_mouse_down` calls
   `on_select_container` again). One click on the track therefore invokes the
   user's `on_select` twice. The same duplication hits double-click reset: the
   container registers `on_click` → reset at lines 592-597 and the track
   registers its own at lines 861-868, with a comment claiming
   "stop_propagation prevents container from getting it" — but nothing calls
   `stop_propagation()` on either handler, so a double-click on the track runs
   the reset handler twice. Suggested fix: have the track handlers call
   `cx.stop_propagation()` (after doing their work) or drop the redundant
   track-level `on_select`/`on_click` registrations and correct the stale
   comment.

2. **`VolumeKnob::new()` auto-generates a fresh `ElementId` from a global
   counter on every construction** — `src/audio/volume_knob.rs:71-74`
   (`VOLUME_KNOB_COUNTER.fetch_add`). GPUI views rebuild components on every
   render, so a caller who omits `.id(...)` gets a new id per frame: the drag
   key (`drag_key = self.id`, line 326) changes mid-gesture, so the first
   `cx.notify()` during a drag makes `get_drag_state(&drag_key_move)` miss and
   the drag silently dies; `on_commit` never fires; and the fill element's
   retained Vello state (`paint_retained` keyed by `GlobalElementId`) is
   re-created every frame. Every in-repo caller (examples, component-lab)
   sets `.id(...)`, so this only bites external users of the default — but the
   default is a trap. Suggested fix: require an id in `new(id)` like
   `Potentiometer::new(id)`, or derive a `CodeLocation` id via
   `#[track_caller]` as `SpectrumElement` does.

3. **`VerticalSlider` value badge can display an unclamped, out-of-range
   value, and its `%` semantics disagree with `Potentiometer`** —
   `src/audio/vertical_slider.rs:361-372` formats `self.value` raw (no clamp)
   into `formatted_value` at builder time; `render()` then clamps `self.value`
   (line 392) but displays the stale cached string (lines 411-412), so
   `VerticalSlider::new("x").value(150.0)` renders a badge reading "150.0"
   over a track filled to 100%. `Potentiometer::format_value_only`
   (`potentiometer.rs:375-394`) clamps at format time, so the two sibling
   controls behave differently. Additionally the slider's `%` branch is
   `value * 100.0` (assumes a 0..1 range) while the potentiometer's is
   range-relative `(value - min) / (max - min) * 100.0` — the same
   `.unit("%")` means different things on the two controls. Suggested fix:
   clamp inside `format_value()` and compute `%` range-relative (or recompute
   `formatted_value` after the render-time clamp).

### Medium

4. **Potentiometer click-step fires on any release whose value equals the
   drag-start value** — `src/audio/potentiometer.rs:626-632`: exact float
   equality `final_value == state.start_value` turns "no movement" into a 10%
   step. `handle_drag` ignores movement under 2 px
   (`gpui-ui-kit/src/interaction.rs:283-285`), so a press + sub-threshold
   wiggle + release is treated as a click and jumps the parameter by 10%; a
   drag that returns exactly to its start value also steps. Suggested fix:
   track a `moved: bool` (set when `handle_drag` first returns `Some`) instead
   of comparing floats, or compare against a small epsilon.

5. **Media-key handling is internally inconsistent and mostly unreachable** —
   `src/audio/volume_knob.rs:454` matches `"audiovolumemute"` and `"f10"`,
   while the shared keyboard handler matches `"audiolowervolume"` /
   `"audioraisevolume"` (`gpui-ui-kit/src/interaction.rs:185-194`). The
   vendored GPUI keystroke/platform layer contains no media-key names at all
   (`crates/3rdparties/gpui/src/platform/keystroke.rs:378-430` — only
   `f1..f35` and navigation keys are non-printable), so the
   `"audiovolume*"`/`"audio*volume"` spellings likely never arrive; meanwhile
   the documented F11/F12 fallback (module doc at `volume_knob.rs:14`) is not
   handled anywhere — `"f11"`/`"f12"` fall through to `handle_keyboard`, which
   returns `None`. Net effect: volume media keys work only if the platform
   happens to emit `"f10"` for mute. Suggested fix: pick one spelling set,
   verify what the macOS/Windows backends actually emit for media keys, and
   add explicit `"f11"`/`"f12"` handling in `VolumeKnob`'s key handler or fix
   the doc comment.

6. **Releasing a drag outside the element loses `on_commit` and leaves stale
   drag state** — `src/audio/potentiometer.rs:625-638`,
   `src/audio/vertical_slider.rs:891-898`,
   `src/audio/volume_knob.rs:421-428`: drag state is cleared and `on_commit`
   fired only from the element's own `on_mouse_up`. GPUI hit-tests mouse
   events at the cursor position (no implicit pointer capture on divs), so a
   press-drag-release ending outside the element never runs the handler: the
   semantic commit the automation contract promises
   (`audio_automation_patterns.rs` "drag" → commit on release) is silently
   dropped, and the `DRAG_STATES` entry persists until the next press
   overwrites it. Suggested fix: listen for mouse-up on the window while a
   drag is active (e.g. `window.on_mouse_up` / a capture-phase handler), or
   clear-and-commit on the first `on_mouse_move` that observes
   `pressed_button == None`.

7. **`SpectrumElement::bar_gap()` is a no-op** —
   `src/spectrum/spectrum_element.rs:21` (field), `:85-87` (builder), but
   `paint()` (`:160` onwards) computes `step_width = bounds.size.width /
   bar_count` and never reads `bar_gap` in either the Vello or the legacy
   path. Callers reasonably expect gaps between bars. Suggested fix: subtract
   the gap from the bar width in both paint paths (`x0 + gap/2 .. x1 -
   gap/2`), or remove the builder method.

8. **Child/custom element ids collide across siblings** —
   `src/audio/potentiometer.rs:817` and `:884` hard-code
   `ElementId::named_usize("potentiometer-ticks", 0)` /
   `("potentiometer-arc", 0)`; `src/audio/volume_knob.rs:481` hard-codes
   `("volume-knob-fill", 0)`; `src/meter.rs:362` and `:440` derive ids from a
   hash of only the label/channel name. Two meters with the same channel name
   ("L" appears twice in a mid/side pair, two "Wet" bars, etc.) under the
   same parent share a `GlobalElementId`, and with it the retained Vello
   backend state (`paint_retained` → `window.with_element_state` in
   `gpui-d3rs/src/vello2d/element.rs:147-163`), risking cross-talk/flicker.
   Suggested fix: incorporate the parent/element id (or a caller-supplied
   discriminant) into these child ids, and document the uniqueness
   requirement on `LevelMeterElement::new`.

9. **Hover steals keyboard focus on all three controls** —
   `src/audio/potentiometer.rs:705-712`,
   `src/audio/vertical_slider.rs:622-631`,
   `src/audio/volume_knob.rs:407-412`: every `on_mouse_move` with no button
   pressed calls `fh.focus(window, cx)`. Sweeping the mouse across a mixer
   surface yanks focus away from whatever the user was editing (e.g. a text
   field) without any click. Marked intentional ("keyboard follows hover"),
   but it violates platform focus conventions and is inconsistent with
   `gpui-ui-kit` siblings, which focus on click only. Suggested fix: gate
   behind an opt-in builder flag, or at minimum do not steal focus from text
   inputs.

### Low

10. **Unbounded thread-local caches and registries** —
    `src/meter.rs:15-17` (`METER_VALUE_LABEL_CACHE`),
    `src/audio/vertical_slider/calculate.rs:218-220` (`TICK_CACHE`),
    `src/audio/potentiometer/tick_element.rs:95-97` (`GEOMETRY_CACHE`),
    `src/spectrum.rs:28-31` (`FREQUENCY_AXIS_LABEL_CACHE`), and
    `src/audio/vertical_slider.rs:35-38` (`VERTICAL_SLIDER_FOCUS_HANDLES`,
    which also accumulates a `FocusHandle` per distinct slider id forever).
    None have eviction; only `TICK_SCENE_CACHE` is capped (64). For
    long-lived hosts that sweep parameter ranges or create dynamically-id'd
    sliders these grow for the process lifetime. Suggested fix: a small LRU
    cap, or reuse the 64-entry clear policy already used for tick scenes.

11. **Tick scene cache keyed on an `Arc` pointer** —
    `src/audio/potentiometer/tick_element.rs:45-50` uses
    `ticks.as_ptr() as usize` in the key, which is ABA-fragile if the
    geometry cache ever gains eviction (today `GEOMETRY_CACHE` pins the Arcs
    forever, so it is safe by accident). Also lines 416-422 do a full
    `cache.clear()` at 64 entries, so an app with >64 distinct knob
    geometries rebuilds every scene on the next paint. Suggested fix: key by
    the `GeometryCacheKey` content hash instead of the pointer, and evict
    one entry (not all) at the cap.

12. **Suspect `abs()` in linear tick label detection** —
    `src/audio/vertical_slider/calculate.rs:60-63`: `((tick_value -
    first_label_tick) / label_step).round().abs() * label_step +
    first_label_tick` mirrors tick values below `first_label_tick` to the
    wrong side when the rounded offset is negative (e.g. rounds to −1 →
    treated as +1). Impact is limited to a tick occasionally being
    mislabeled/unlabeled near the range start; suggest dropping the `.abs()`.

13. **`format_meter_value` NaN poisons key 0** — `src/meter.rs:34-44`:
    `(value * 10.0).round() as i64` maps NaN to 0, so a NaN meter reading
    inserts/displays "0.0" (or a cached "0.0" entry masks a real NaN).
    Suggested fix: early-return `"NaN".into()` for non-finite values before
    computing the key.

14. **`AudioToggleExt::design_tokens` silently drops two toggle variants** —
    `src/lib.rs:74-81`: only `TOGGLE_SEGMENTED` is honored;
    `TOGGLE_THUMB_ON_TRACK` and `TOGGLE_PILL` fall through to `Sliding`.
    `gpui-ui-kit`'s `ToggleStyle` currently has only `Sliding`/`Segmented`
    (`gpui-ui-kit/src/toggle/types.rs:6-12`), so this is a documented token
    with no effect rather than a wrong mapping. Suggested fix: note the
    limitation in the token docs, or extend `ToggleStyle`.

15. **`VolumeKnob` ARIA registration disagrees with its own accessibility
    summary** — `src/audio/volume_knob.rs:263-267` registers
    `value_range(self.value, ...)` raw (unclamped, and not zeroed when muted),
    while `accessibility_summary()` (`:214-218`) clamps and reports 0 when
    muted. Suggested fix: register the same effective value the summary uses.

16. **`VolumeKnob` has no `disabled` state** — both siblings
    (`Potentiometer`, `VerticalSlider`) support `disabled` (cursor, opacity,
    handler gating, ARIA state); `VolumeKnob` does not, so a disabled monitor
    control stays fully interactive. Suggested fix: add the same
    `disabled(bool)` builder + gating pattern.

17. **Spectrum "allocation-free" scratch buffer is per-instance, not
    per-frame-persistent** — `src/spectrum/spectrum_element.rs:22,189-202`:
    `scratch_heights` lives on the element, but GPUI views construct a fresh
    `SpectrumElement` every render, so the buffer is reallocated on the first
    paint of each frame anyway unless the caller caches the element (the unit
    test at `:364-388` simulates reuse on one instance, which real render
    flows don't do). Suggested fix: move the scratch into the retained element
    state (`window.with_element_state`) or a thread-local buffer, as the
    other caches do.

18. **Commit noise on `VolumeKnob` clicks** —
    `src/audio/volume_knob.rs:353-363` stores drag state on every mouse-down
    and `:421-428` fires `on_commit` whenever drag state existed, so a plain
    click commits an unchanged value, and a double-click mute fires two
    spurious commits in addition to the mute toggle. Suggested fix: commit
    only when the value actually changed during the gesture (same `moved`
    flag as finding 4).

## GPU/CPU data-flow notes

The crate's Vello paths (`KnobArcElement`, `PotentiometerTickLinesElement`,
`VolumeKnobFillElement`, `LevelMeterElement`, `GradientMeterFillElement`,
`SpectrumElement`) all encode a CPU-side `ChartScene` per paint and hand it to
`VelloScenePainter::paint_retained`, which keeps a retained
`WgpuCustomDraw`/vello backend per `GlobalElementId`
(`gpui-d3rs/src/vello2d/element.rs:140-164`). I found **no GPU→CPU readbacks
and no GPU→CPU→GPU cycles** in this crate: all source data (magnitudes,
levels, values) originates on the CPU by nature, and the rendered output stays
on the GPU. The remaining per-frame costs are CPU-side: every paint rebuilds
its `ChartScene`/`BezPath` vectors except the potentiometer tick lines, which
are cached in `TICK_SCENE_CACHE` (see findings 11 and 17 for that cache's
pointer keying and the spectrum scratch buffer). If profiling shows scene
encoding is hot for meters at audio refresh rates, the next step is keying a
cached scene by quantized level (as the tick cache does) rather than adding
any readback — the data already flows one way.

## UI/UX consistency

- `%` unit semantics differ between `VerticalSlider` (`value * 100`) and
  `Potentiometer` (range-relative) — finding 3.
- Focus behavior: all three controls steal focus on hover (finding 9);
  `gpui-ui-kit` siblings focus on click.
- `VolumeKnob` lacks `disabled`, `on_select`, and Escape-to-reset, all of
  which both siblings have (finding 16); its keyboard docs promise media keys
  that are not wired up (finding 5).
- Fine-control step differs by input path without documentation: Shift+scroll
  is 0.5% (`interaction.rs:250`) while Shift+arrow is 1%
  (`interaction.rs:167-173`); module docs mention only the scroll figures.
- Design-token coverage is uneven: `AudioDesignTokens::from(&DesignSystem)`
  hardcodes `meter_use_gradient: false`, `knob_arc_glow: 0.0`,
  `knob_label_style`/`knob_indicator_style` to defaults
  (`src/audio_design_tokens.rs:164-173`), so several tokens are only reachable
  via manual `design_tokens(...)` calls — fine, but worth a doc line since
  `gpui-design` has no corresponding fields to source them from.

## Resolved during follow-up

- Fixed `VerticalSlider` duplicate track callbacks (#1). The track now invokes
  focus, selection, and drag-start hooks once, then stops propagation; its
  double-click reset likewise cannot reach the container reset handler. The
  reset interaction test now asserts both the single callback and restored
  default value. Verified with `cargo test -p gpui-audio-kit vertical_slider`,
  `cargo fmt --check`, and `git diff --check`.
- Fixed unstable default `VolumeKnob` IDs (#2). `new()` and `default()` now
  derive the default `ElementId` from their caller location instead of a global
  counter, so a render call site retains focus, drag, and Vello state across
  renders. Repeated controls from one loop still require their existing `.id()`
  override. Added same-call-site and distinct-call-site identity tests; verified
  with `cargo test -p gpui-audio-kit volume_knob`, `cargo fmt --check`, and
  `git diff --check`.
- Fixed `VerticalSlider` display/range disagreement (#3). It now clamps before
  ARIA registration and display formatting, and `%` is calculated relative to
  `min..max`, matching `Potentiometer`. Added clamped-value and offset-range
  percentage coverage; verified with `cargo test -p gpui-audio-kit format_value`.
- Fixed Potentiometer click-step gesture classification (#4). Shared interaction
  state now records pointer movement until release; the Potentiometer applies
  its legacy 10% click step only to a true click, not a drag returning to its
  start value or a sub-threshold wiggle. Added state-reset and real pointer
  sequence regressions, while retaining the click-step test; verified with
  `cargo test -p gpui-ui-kit drag_movement_state_is_reset_for_each_gesture` and
  the targeted `gpui-audio-kit` click/movement tests.
- Fixed `VolumeKnob` documented media-key behavior (#5). The vendored backends
  do not currently emit the review's proposed `audio*volume` key strings, but
  they do provide F-key identities; shared media handling now supports F11
  lower-volume and F12 raise-volume alongside the documented aliases, and mute
  accepts `audiomute`, the prior alias, and F10. Added alias coverage with
  `cargo test -p gpui-ui-kit media_key_aliases_adjust_volume_when_enabled`.
- Fixed `SpectrumElement::bar_gap` (#7). Both Vello and the legacy renderer
  now derive each bar's centered horizontal bounds from the configured gap;
  the legacy path uses independent subpaths so it cannot bridge the gaps.
  Negative gaps clamp to zero and oversized gaps collapse safely to zero-width
  bars. Added geometry coverage and verified with
  `cargo test -p gpui-audio-kit spectrum_element`.
- Clarified and fixed retained meter IDs (#8). Potentiometer and VolumeKnob
  children already inherit their unique parent `GlobalElementId` path, so their
  fixed child segments cannot collide across controls. The real collisions were
  label-hashed `LevelMeterElement` and gradient fill IDs: defaults now combine
  caller location with the label, and `LevelMeterElement::id(...)` supports
  repeated labels from a single loop. Added stable/scoped/override identity
  coverage; verified with
  `cargo test -p gpui-audio-kit meter_default_ids_are_stable_and_channel_scoped`.
- Fixed hover-driven focus theft (#9). Potentiometer, VerticalSlider, and
  VolumeKnob no longer focus merely because the pointer moves over them; their
  existing mouse-down focus behavior remains the explicit keyboard interaction
  path. Verified with the full `cargo test -p gpui-audio-kit` suite (94 unit,
  1 allocation-contract, 28 component, 2 design-token, and 123 integration
  tests passed).
- Finding #12 was disproved. Linear intermediate ticks are exactly half a
  label step apart. A tick below `first_label_tick` can only be the intervening
  half-step, which matches neither the signed nor `abs()`-mirrored label
  boundary; all actual label boundaries are at or above `first_label_tick`.
  Removing `abs()` would therefore be a non-functional cleanup, not a bug fix.
- Fixed non-finite meter-format cache poisoning (#13). `format_meter_value`
  now formats NaN and infinities directly instead of converting them to the
  finite-value cache key (where NaN became zero). Added coverage proving a NaN
  cannot affect the normal `0.0` label; verified with
  `cargo test -p gpui-audio-kit format_meter_value`.
- Fixed VolumeKnob's ARIA/display mismatch (#15). Its rendered accessibility
  range, visible fill, and non-rendering summary now share one effective value
  calculation: muting reports zero and all other values clamp to `0.0..=1.0`.
  Added clamped-summary coverage; verified with
  `cargo test -p gpui-audio-kit volume_knob_accessibility_summary`.
- Fixed spurious VolumeKnob commits (#18). A pointer release now commits only
  after real pointer movement that leaves a changed value; plain clicks,
  double-click mute gestures, and drags returning to their start value emit no
  automation commit. Added gesture-classification coverage; verified with
  `cargo test -p gpui-audio-kit volume_knob_commits_only_changed_drags`.
- Fixed missing VolumeKnob disabled state (#16). Added the `disabled(bool)`
  builder, disabled cursor/opacity, ARIA disabled state and summary metadata,
  and gated every pointer, scroll, mute, and keyboard handler. Added
  accessibility-summary coverage; verified with
  `cargo test -p gpui-audio-kit volume_knob_accessibility_summary_includes_disabled_state`.
- Fixed audio-toggle token flattening (#14). `ToggleStyle` now represents the
  documented Material `ThumbOnTrack` and Fluent `Pill` variants, with distinct
  track/thumb visuals; `AudioToggleExt` maps all four documented audio token
  values explicitly. Added mapping and render coverage; verified with
  `cargo test -p gpui-audio-kit audio_toggle_tokens_map_every_supported_platform_style`
  and `cargo test -p gpui-ui-kit test_toggle_all_styles`.
- Fixed lost outside-release drag lifecycle (#6). GPUI's capture-phase
  `on_mouse_up_out` API now finalizes Potentiometer, VerticalSlider, and
  VolumeKnob gestures when the pointer is released beyond their hitbox. The
  handlers apply the final drag delta, avoid duplicate change callbacks, emit
  the semantic commit, and always clear retained drag state. Verified with
  `cargo check -p gpui-audio-kit`.
- Fixed unbounded audio-kit caches and focus registry (#10). Meter labels,
  frequency-axis labels, vertical-slider ticks and fallback focus handles, and
  potentiometer geometry now have small bounded caches; insertion evicts one
  old entry rather than growing for the process lifetime. Added cache-cap
  coverage; verified with `cargo test -p gpui-audio-kit cache_is_bounded`.
- Fixed tick-scene cache identity and eviction (#11). Vello tick scenes now
  use a content hash of tick geometry rather than an `Arc` address, remaining
  valid when geometry entries are evicted; the 64-entry scene cache evicts one
  old entry instead of clearing all scenes. Added independent-allocation hash
  coverage; verified with `cargo test -p gpui-audio-kit
  equivalent_tick_geometry_has_the_same_scene_cache_hash`.
- Fixed SpectrumElement scratch-buffer lifetime (#17). Height scratch storage
  is now retained through GPUI element state under the element GlobalElementId,
  so reconstructed spectrum elements reuse it and GPUI releases it when the
  element disappears. Added reusable-buffer coverage; verified with
  `cargo test -p gpui-audio-kit scratch_height_update_reuses_the_caller_buffer`.

## Clean bill

- Log/linear scale math (`Scale::value_to_normalized` / `normalized_to_value`,
  log tick generation, `logarithmic_frequency_position`) is guarded against
  degenerate ranges and NaN inputs and round-trips correctly in tests.
- The dB axis label positions (`spectrum/types.rs:15-35`) match
  `SpectrumElement::db_to_height` exactly; `db_to_position` piecewise mapping
  is tested across all regions.
- No `unwrap`/panic paths reachable on production geometry: the one
  `partial_cmp().unwrap()` (`calculate.rs:194`) sorts values derived from
  sanitized finite ranges; `format_label`'s `chars().next().unwrap()` is
  provably on a found char boundary.
- No threads, mutexes, channels, or blocking GPU calls (no `device.poll` /
  `pollster`) anywhere in the crate; thread-locals are the only shared state
  and are `RefCell`-scoped without re-entrant borrows.
- The automation-pattern and visual-regression manifest modules are pure data
  with consistent, test-pinned contracts.
