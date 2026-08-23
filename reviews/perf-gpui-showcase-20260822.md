# Perf review: gpui-showcase

Date: 2026-08-22

## Role and hot paths

`gpui-showcase` is the component-gallery app for `gpui-ui-kit` (native bin + wasm
`start` entry in `src/main.rs:40-45`, shared boot via `run_showcase` in
`src/lib.rs:16-26`). It contains no GPU/readback code of its own; its perf surface
is the render tree:

- `Showcase::render` (`src/showcase.rs:351-450`) rebuilds the root layout and
  conditionally syncs three persistent child entities (sidebar/header/content).
- `ShowcaseContent::render` (`src/showcase.rs:897-992`) calls
  `render_section_content` (`src/showcase.rs:455-549`), which dispatches to ~44
  per-section builders in `src/showcase/sections/render_*.rs`. This is the main
  per-event hot path.
- Event handlers throughout (`handle_key_down` at `src/showcase.rs:553-642`,
  nav clicks at `src/showcase.rs:727-734/791-798`, table/sort/select callbacks in
  `sections/render_table.rs:49-81`) mutate state and call `cx.notify()`.
- The crate embeds animated components from other crates: two `AnimatedQrCode`
  entities (`src/showcase.rs:205-208`) and audio-kit `SpectrumElement`s
  (`sections/render_audio_visuals.rs`), which drive frame-rate activity.

No TODO/FIXME, no criterion benches, no allocation-count tests, no `qa/perf`
references in the crate. `release_artifacts.rs` is static manifest data (startup
only, not hot).

## Findings

1. **[Alloc] Every interaction rebuilds the entire active section tree.**
   `ShowcaseContent::render` calls `parent.render_section_content(...)`
   unconditionally (`src/showcase.rs:903-906`), so every redraw of the showcase
   rebuilds all elements of the current section. The persistent-child-entity
   scheme does not prevent this: the children are inserted as plain
   `.child(entity.clone())` (`src/showcase.rs:416, 440-448`), i.e. non-cached
   `AnyView`s, and gpui re-renders a non-cached view whenever its ancestor tree is
   laid out (`crates/3rdparties/gpui/src/view.rs:117-128`); the prepaint/paint
   reuse path is only reachable for `.cached()` views (`view.rs:155-170, 214`).
   `mark_view_dirty` propagates dirtiness to ancestors, not descendants
   (`crates/3rdparties/gpui/src/window.rs:1812-1824`), so a single `cx.notify()`
   on `Showcase` re-renders everything. The comments at `src/showcase.rs:358-359`
   and `655-657`/`870-876` ("not rebuilt every frame", "only re-renders when...")
   overstate the actual isolation. Impact: hundreds of divs + `SharedString`s
   rebuilt per keystroke, slider tick, or selection change.

2. **[Alloc] ~30 fps whole-window re-render while the QR section is visible.**
   `AnimatedQrCode` spawns a 33 ms timer that calls `cx.notify()` on itself in a
   loop (`crates/gpui-ui-kit/src/qr/animated_qr_code.rs:131-147`). Because dirty
   marks walk up to ancestors (`window.rs:1812-1824`), each tick re-renders the
   full showcase tree, including the sidebar and the whole QR section rebuild via
   finding 1. The showcase creates two such entities eagerly in `Showcase::new`
   (`src/showcase.rs:205-208`) — the timers run regardless of which section is
   selected. Impact: continuous 30 Hz full-tree churn; especially costly on wasm.

3. **[Alloc] Audio-visuals section: per-rebuild Vec churn + painter re-registration.**
   Each section rebuild recomputes a 32-element `magnitudes` Vec and clones it
   three times (`sections/render_audio_visuals.rs:15-17, 19, 62, 66`), and
   constructs three `SpectrumElement`s, each owning a fresh `VelloScenePainter`
   (`crates/gpui-audio-kit/src/spectrum/spectrum_element.rs:24, 42`). Dropping the
   old painter unregisters its custom draw
   (`crates/gpui-d3rs/src/vello2d/element.rs:266-271`) and the next paint
   re-resolves the backend and re-registers (`element.rs:103-117`) — global
   custom-draw registry churn on every redraw of this section.

4. **[Roundtrip] The renderer QA matrix pins the CPU raster→re-upload path.**
   The "Vello · CPU" cells force `VelloBackend::Cpu`
   (`sections/render_audio_visuals.rs:62-75`), which rasterizes on CPU and uploads
   via `paint_image` (`element.rs:207-231`). Since `SpectrumElement::paint` builds
   a fresh `ChartScene` every paint (`spectrum_element.rs:202`), the revision
   always changes, so every repaint re-rasterizes and re-uploads even though the
   demo data is static. This is a deliberate QA matrix, so the roundtrip is
   intentional — but the demo data is constant, so raster output is identical
   between repaints; a scene-content hash instead of revision would make the CPU
   path repaint-free (needs profiling to confirm repaint frequency matters).

5. **[Alloc] Per-render clones and `format!` ids in stable subtrees.**
   - Table section clones `self.users`, `self.sort_state`,
     `self.selected_users`, `self.pagination` on every rebuild
     (`sections/render_table.rs:26, 45-48, 68, 75`).
   - Form-controls path clones `edit_text`, `input_value`, `input_edit_text`,
     and four buttonset `SharedString`s per rebuild (`src/showcase.rs:480-487`).
   - Sidebar builds `SharedString::from(format!("nav-{:?}", section))` for each
     of ~44 nav items and `group.label().to_uppercase()` per group on every
     sidebar render (`src/showcase.rs:705, 758, 767`).
   Individually small, but they execute on every redraw per finding 1.

6. **[Alloc] Scroll diagnostics allocate per scroll event.**
   Both scroll handlers build a `format!` string on every `on_scroll_wheel` event
   (`src/showcase.rs:814-828, 953-977`) and pass it to `showcase_scroll_diag`,
   which discards it on non-iOS (`src/showcase.rs:88-89`). On iOS each event also
   opens/appends a log file (`src/showcase.rs:78-85`).

7. **[GPU] No missed GPU opportunity inside the crate.**
   The only GPU surface is inherited: audio visuals default to the zero-copy
   `WgpuCustomDraw` vello path (good; `render_audio_visuals.rs:8-14` asserts this
   is the default). Nothing in the crate itself rasterizes or transforms on CPU
   that should move to GPU.

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| R1 | Wrap stable child entities with `.cached(style)` (or move section rendering fully into `ShowcaseContent` so unrelated notifies skip it) | 1, 2 | M | High — eliminates full-tree rebuilds per event |
| R2 | Start the `AnimatedQrCode` timers lazily (only while the QR section is visible) and/or make the animation invalidate only the element — cross-crate fix in gpui-ui-kit | 2 | S–M | High on wasm; removes 30 Hz idle churn |
| R3 | Hoist `magnitudes` (and other constant demo data) into `Showcase` fields computed once in `new` | 3, 5 | S | Medium |
| R4 | Keep `SpectrumElement`/painter instances alive across rebuilds (persistent per-section element state) instead of recreating per render | 3 | M | Medium — stops custom-draw register/unregister churn |
| R5 | Gate `showcase_scroll_diag` formatting behind a cfg/debug flag | 6 | S | Low–medium |
| R6 | Use precomputed static element ids for nav items instead of `format!("nav-{:?}")` per render | 5 | S | Low |
| R7 | Add a gpui-profiler allocation baseline for "switch section / type in input / QR visible" flows | all | S | Enables regression tracking |

## Quick wins

- Gate the scroll diagnostics (R5) — a few lines in `src/showcase.rs`.
- Hoist the `magnitudes` Vec into `Showcase::new` and share one `Arc<[f32]>`
  across the three spectrum demos (R3; `SpectrumElement::new` already takes
  `impl Into<Arc<[f32]>>`).
- Only spawn the animated-QR timer when the QR section is active (R2 showcase
  side: create the `AnimatedQrCode` entities on first visit to the section).
- Replace `format!`-derived nav ids with `SharedString::from_static` lookups (R6).
- Experiment: `.cached()` on `sidebar_entity`/`header_entity` clones in
  `Showcase::render` (R1 partial) and measure with gpui-profiler.
