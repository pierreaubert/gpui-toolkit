# Perf review: gpui-audio-kit

Date: 2026-08-22

## Role and hot paths

Audio-focused GPUI controls: `Potentiometer`, `VerticalSlider`, `VolumeKnob`
(interaction-driven, re-render on every drag/scroll/key event) and
visualizations: `SpectrumElement`, `LevelMeterElement`,
`render_horizontal_meter_bar*` (audio-rate, 30–60 fps repaints). All
custom-painted elements default to the d3rs `vello2d` path
(`Renderer2D::Vello`, `vello` feature on by default, Cargo.toml:12-14) with a
legacy `paint_quad`/`paint_path` fallback. Per-frame work: build a
`ChartScene`, hand it to an owned `VelloScenePainter`, paint. Per-event work:
drag state via `gpui-ui-kit` thread-local `DRAG_STATES`
(gpui-ui-kit/src/interaction.rs:48), value formatting, accessibility
registration.

Already-good discipline (worth keeping): `MeterData::update` is in-place and
zero-alloc (src/spectrum/meter_data.rs:18-29), pinned by
tests/allocation_contracts.rs:31-52; `format_meter_value` is thread-locale
cached (src/meter.rs:33-43, contract at allocation_contracts.rs:12-29);
`SpectrumElement::scratch_heights` is reused across paints
(src/spectrum/spectrum_element.rs:20,177-190, tested at :351-376); tick
geometry and frequency labels are cached (src/audio/potentiometer/
tick_element.rs:64-118, src/spectrum.rs:28-83).

## Findings

1. **[Alloc/GPU] `VelloScenePainter` is created and dropped per frame per
   element.** Every custom element owns a painter constructed in its builder:
   spectrum_element.rs:21-24,42; meter.rs:203,369 and :419,434;
   knob_arc_element.rs:42; tick_element.rs:252 (built per render at
   potentiometer.rs:827); volume_knob_fill_element.rs:15,35 (built at
   volume_knob.rs:480). These are `RenderOnce`/stateless elements
   (`id()` returns `None`), so a drag or meter tick rebuilds them every
   frame. Each fresh painter's first paint runs `resolve()` →
   `register_custom_draw` + new `WgpuVelloDraw`
   (crates/gpui-d3rs/src/vello2d/element.rs:103-127), whose draw then lazily
   creates a `vello::Renderer` (vello2d/wgpu_draw.rs:143-168 — pipeline
   compilation), an offscreen texture (wgpu_draw.rs:171-188), and a
   `CompositePipeline` (shader module + render pipeline,
   wgpu_draw.rs:189-191,282-369); `Drop` unregisters (element.rs:266-272).
   Net: GPU pipeline/renderer churn per frame per element during knob drags
   and meter animation — the per-device-sharing gap from
   reviews/20260822-vello.md amplified from per-chart to per-frame. Impact:
   severe under interaction (needs profiling to quantify; structurally
   unambiguous from the ownership chain).

2. **[Alloc] Full scene copied twice per frame.** A fresh `ChartScene::new()`
   per paint (spectrum_element.rs:202, meter.rs:274/596,
   knob_arc_element.rs:122, tick_element.rs:328,
   volume_knob_fill_element.rs:124) always carries a new revision
   (d3rs vello2d/scene.rs:40; test at :320-323), so the painter clones the
   entire scene every paint (element.rs:181-184), and `draw_wgpu` re-encodes
   a fresh `vello::Scene` from it (wgpu_draw.rs:129-141,
   vello2d/gpu_scene.rs:10-27). For a 1024-bin spectrum that is build → clone
   → encode of ~3k path elements per frame.

3. **[Roundtrip] CPU fallback re-rasterizes every frame.** Because the
   painter is not retained (finding 1), the `CpuState.rendered` memoization
   (element.rs:195) never hits: every frame runs `rasterize` → full-buffer
   channel swap (element.rs:216-218) → `RgbaImage`/`RenderImage` alloc
   (element.rs:219-221) → `paint_image` atlas upload (element.rs:222-228).
   This is the CPU-side analog of the gpu2d offscreen→readback→re-upload
   anti-pattern, active on any host where `wgpu_custom_draw_available()` is
   false (element.rs:107,250-256).

4. **[Alloc] Knob drag rebuilds static tick/arc scenes per mouse-move.** Tick
   *geometry* is cached, but `PotentiometerTickLinesElement::paint`
   re-triangulates every tick into fresh `BezPath`s + `ChartScene` on every
   paint (tick_element.rs:328-372) although ticks are value-independent. The
   arc element builds up to 5 sector `BezPath`s per paint (glow halo ×3,
   value, track; knob_arc_element.rs:123-203). One potentiometer drag frame
   therefore rebuilds 2 vello scenes; with the volume knob, 3.

5. **[Alloc] Per-render string/handle churn in controls.**
   `format!("{}-track", element_id)` per render
   (src/audio/vertical_slider.rs:713); focus-handle cache insert path
   (vertical_slider.rs:494-501); fresh `Rc<Cell>` value tracker +
   `InteractionConfig` per render (potentiometer.rs:508-510,
   volume_knob.rs:323-325); `cx.register_accessible` with cloned label every
   render (potentiometer.rs:405-411, vertical_slider.rs:379-380); tick-label
   `div`s rebuilt per render from cached data (potentiometer.rs:835-849).
   Individually small, but they run at drag/audio rates (needs profiling to
   rank against findings 1–4).

6. **[GPU] Gradient/glow effects use strip/fake-bloom rasterization instead
   of GPU gradients.** Gradient meters emit 10–12 alpha strips per segment
   (meter.rs:621-635 vello, :703-739 legacy; GradientMeterFillElement
   meter.rs:276-294); the knob glow is 3 concentric filled sectors
   (knob_arc_element.rs:162-178,252-267). On the vello path a single
   `peniko::Gradient` fill replaces both loops with one draw command.

7. **[Alloc, API-level] `SpectrumElement` takes fresh `Arc<[f32]>` magnitudes
   plus a separate `previous` arc per frame** (spectrum_element.rs:28,57-60)
   and smooths on the main thread in paint (:177-190). This nudges callers
   into per-audio-callback `Vec`→`Arc` allocation; an in-place double-buffer
   contract (like `MeterData::update`) would be allocation-free. Caller-side
   cost is outside this crate (needs profiling in the host app).

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| R1 | Retain painter/GPU state across frames: share a per-device `vello::Renderer` + composite pipeline pool in d3rs `vello2d` (best, fixes all dependents), or keep `VelloScenePainter` in component state instead of in the per-render element | 1, 3 | M | High |
| R2 | Pass `ChartScene` by value (or `Arc<ChartScene>`) into `VelloScenePainter::paint` to kill the per-frame full-scene clone (d3rs element.rs:163,181-184) | 2 | S | High |
| R3 | Cache the tick `ChartScene` alongside `PotentiometerTickGeometry`, keyed by the existing geometry key + colors; rebuild only on key change | 4 | S | Medium |
| R4 | Replace strip-loop gradients and 3-sector glow with vello gradient brushes on the vello path | 6 | S | Medium |
| R5 | Cache `ElementId::Name(format!("{}-track", …))` and gate `register_accessible`/label-div rebuilds on value change | 5 | S | Low–Med |
| R6 | Extend tests/allocation_contracts.rs with a knob-drag / spectrum-paint probe; d3rs `PainterTestStats` (element.rs:43-50) already exposes registration counters to assert painter reuse | 1, 5 | S | Guardrail |
| R7 | Document/add an in-place smoothing buffer contract for spectrum magnitudes | 7 | S | Low |

## Quick wins

- R2 (by-value scene handoff) — one signature change in d3rs, deletes a
  full-scene clone per paint.
- Cache the `{}-track` ElementId (vertical_slider.rs:713).
- Cache the static tick scene next to the cached tick geometry
  (tick_element.rs:64-118 vs :328-372).
- Add painter-reuse assertions to allocation_contracts.rs via
  `PainterTestStats` so finding 1 cannot regress silently.
