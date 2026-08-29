# Bug Review: gpui-component-lab — 2026-08-25

Scope: full read of the crate — `src/lib.rs`, all of `src/lib/*.rs` (registry,
conformance validation, visual manifest/diff/baseline/gallery artifacts), all of
`src/lab_ui/*.rs` (interactive lab view, ~4,000-line `component_lab.rs`, misc
caches, visual capture), `src/bin/gpui_component_lab.rs`, and both integration
tests under `tests/` (~25 files, ~11.4k lines). To judge the lab's rendering
assumptions I also read the vendored GPUI invalidation paths
(`crates/3rdparties/gpui/src/window.rs`, `view.rs`, `elements/div.rs`) and
representative widget redraw behavior in `gpui-ui-kit` (`button.rs`, `input.rs`).
This is a UI/crate-scaffolding crate: no wgpu code of its own, no threads beyond
one `cx.spawn` polling loop, no unsafe code. Findings are ranked by severity;
line numbers refer to files under `crates/gpui-component-lab/`.

## Findings

## Resolved during follow-up (2026-08-26)

- **Sidebar and controls scrolling:** both fixed-height side panels now establish a minimum-height-zero scroll container, so every registered story and the complete property/layout editor remain reachable in a small window.
- **State invalidation:** story, property, viewport, theme, motion, matrix, and layout mutations now take their owning GPUI context and call `cx.notify()` after a real state change. The allocation-contract mutation helper follows the same path, so programmatic updates repaint without relying on incidental widget refreshes.
- **Retained child entities:** sidebar, toolbar, controls, and preview children retain only compact dirty keys; their render methods read the authoritative parent state. The obsolete whole-story, preset, and responsive-matrix mirror copies are gone.
- **Render allocation keys:** clean renders compare borrowed ids/status values and allocate replacement keys only when a child must be refreshed. A story revision invalidates the controls and preview after a property change, so the explicit notification also updates the visible preview.
- **Allocation contract:** the profiler test now separates a scheduled render from the following mutation interval, so it measures each operation rather than attributing the preceding frame's allocations to the next prop edit.
- **Element ID collisions:** `lab_id` now verifies raw cache parts within each hash bucket, and its visible fragments escape every non-alphanumeric UTF-8 byte. Distinct punctuation-only variants therefore remain distinct instead of sharing an element ID.
- **Story and visual-artifact filename collisions:** new story and screenshot path fragments encode raw bytes rather than collapsing punctuation or case. Saving migrates an existing legacy story filename only after confirming that its JSON has the same story id, preserving existing designer state while preventing future overwrites.
- **Manual Reload state restoration:** manual and live reload now share selected-document restoration for viewport, theme, motion, matrix mode, and layout constraints. Replacing the selected document clears stale layout-dirty state and advances the story revision so retained controls and preview redraw from disk. `manual_reload_restores_selected_document_layout` covers the manual Reload path under `visual-capture`.
- **Stateful preview retention:** the selected ColorPicker, AnimatedQrCode, WorkflowCanvas, or embedded showcase is now created outside the render path and retained only while its story remains selected. Prop/document replacement refreshes that entity deliberately; unrelated parent redraws retain interaction and animation state. `stateful_preview_survives_parent_layout_redraw` verifies entity identity across a layout redraw under `visual-capture`.
- **Constructor/rebuild panic paths (disproved):** `ComponentLab::new` and `rebuild_derived_state` are private to `lab_ui`; all direct callers construct the built-in registry internally. A loaded story document only inserts or replaces a map entry, so it cannot remove the selected built-in document guarded by the internal registry invariant. No public registry injection path exists, so making the internal constructor fallible would not fix a reachable user failure.
- **Showcase entity retention:** `ui_showcases` now retains at most the selected showcase. On every story switch, inactive showcase entities release their self-handle before removal, avoiding both unbounded retained demo trees and a self-reference cycle. `switching_showcase_stories_releases_inactive_showcase` verifies the cache remains bounded.
- **CLI and visual-manifest robustness:** `--child-command` now uses shell-style tokenization without invoking a shell, so quoted paths remain one argument and unterminated quotes fail clearly; bin tests cover both cases. Representative-case selection now performs one explicit clone per borrowed case. Pixel diffs process raw RGBA chunks rather than repeated bounds-checked image access; `visual_manifest_diff_compares_png_captures_and_writes_diff` verifies unchanged diff output.

### High

1. **Sidebar and controls panel do not scroll; most of the ~150 stories are unreachable.**
   `src/lab_ui/component_lab.rs:836-887` (`render_sidebar`) stacks one `Button`
   per story in a fixed 300px column with `.h_full()` and no
   `overflow_y_scroll()`; `render_controls_panel` (`component_lab.rs:1011-1020`)
   is the same pattern for a 340px column holding props plus ten layout-control
   groups. `run_lab_app` sets `MiniAppConfig::scrollable(false)`
   (`component_lab.rs:340-343`), so nothing above provides scrolling either.
   The registry contains ~150 stories (6 base + 73 exported ui-kit + 40 showcase
   + 13 px + 11 mesh variants + 7 audio), i.e. several thousand pixels of list
   in a 920px window; everything below the fold is clipped/unclickable.
   Fix: wrap the story list and the controls column in a scrollable container
   (e.g. `.overflow_y_scroll()` on the list `div()` with `min_h_0()`/`flex_1()`),
   matching how `ui-kit.showcase-component` already uses `overflow_y_scroll()`
   at `component_lab.rs:2293`.

### Medium

2. **The retained-child-entity "optimization" is inert in this GPUI fork, and its
   hand-rolled dirty checks buy nothing while adding per-render clones.**
   `ComponentLab` keeps four child entities (`sidebar_entity`,
   `toolbar_entity`, `controls_panel_entity`, `preview_area_entity`,
   `component_lab.rs:374-378`) with doc comments claiming they "only re-render
   when X changes" (`component_lab.rs:3879-3987`). In the vendored GPUI, once a
   view is dirty it re-renders with `window.refreshing = true` for its whole
   subtree (`crates/3rdparties/gpui/src/view.rs:155-182`), so every
   `ComponentLab` re-render re-renders all four children regardless of the
   dirty-check blocks at `component_lab.rs:3716-3799`. Additionally, ui-kit
   widgets and clickable divs call `window.refresh()` on interaction
   (`div.rs:2689`, `input.rs:499` etc.), which bypasses all view caching for the
   frame — so each click rebuilds the entire ~150-button sidebar and the chart
   preview. Meanwhile the sync blocks clone state that is never read for
   rendering: `LabPreviewArea`/`LabControlsPanel` store full clones of
   `ComponentStory`, `ViewportPreset`, `ThemePreset`, `MotionPreset`, and the
   whole `ResponsivePreviewMatrix` (`component_lab.rs:3759, 3783-3796`), but
   their `Render` impls just delegate to the parent (`component_lab.rs:4016-4025`),
   which re-reads the authoritative state from `self.documents`. Fix options:
   (a) drop the mirrored fields and the sync blocks entirely — keep the child
   entities only if you actually push rendering down into them; or (b) render
   from the child's own stored state so the caches mean something. Either way,
   compare dirty keys without allocating (see finding 3).

3. **Unconditional `String` clones per render for dirty-check keys.**
   `component_lab.rs:3752` (`self.selected_story().id.clone()`) and
   `component_lab.rs:3771-3774` (story/viewport/theme/motion id clones) run on
   every `render()` even when nothing changed — odd for a crate that ships an
   `AllocProbe` overlay and an interaction allocation contract
   (`tests/component_lab_interaction_allocation_contract.rs`). Fix: compare
   against `&self.selected_story_id`, `self.selected_viewport().id` etc.
   directly and clone only inside the `if` body that performs the update.

4. **State mutators never call `cx.notify()`; redraws happen only by side effect.**
   `set_prop` (`component_lab.rs:674-681`), `select_story` (647-661),
   `set_viewport`/`set_theme`/`set_motion`/`set_layout_*`/`toggle_matrix`
   (702-775), and the child-entity sync updates (3723-3797) mutate state without
   `cx.notify()`. In this GPUI, `Entity::update` alone does not schedule a
   redraw (`crates/3rdparties/gpui/src/app.rs:2557-2571`); today the UI still
   updates because every pointer interaction path calls `window.refresh()` from
   widget/div internals. Any future trigger that doesn't go through a clickable
   element (programmatic prop set, keyboard-driven change on a widget that
   forgets to refresh, the profiler mouse-move sampler at 3808-3842) leaves
   stale pixels until the next unrelated event. Fix: call `cx.notify()` at the
   end of `set_prop`, `select_story`, `mark_layout_state_dirty` callers, and
   inside each child-entity `update` closure — cheap, idempotent, and it makes
   the code correct by construction instead of by accident.

5. **Stateful preview components are re-created as fresh entities on every
   preview re-render.** `render_exported_ui_kit_component_story` calls
   `cx.new(|_| ColorPickerView::new(...))` (`component_lab.rs:1896-1898`),
   `cx.new(|cx| AnimatedQrCode::new(...))` (1990-1992),
   `cx.new(|cx| WorkflowCanvas::with_graph(...))` (2287), and
   `cx.new(|cx| Showcase::embedded_section(...))` (2294) inside the render path.
   Because the preview subtree is rebuilt on every interaction (see finding 2),
   these components lose all internal state (chosen color, animation phase,
   canvas drag state) on any unrelated change such as toggling the layout
   border, and each recreation pays full construction (a Showcase section is a
   large tree; `AnimatedQrCode` restarts its animation). The showcase-story path
   already solved this with the retained `ui_showcases` map
   (`component_lab.rs:663-672`); the same retention pattern should be applied
   here, keyed by story id.

### Low

6. **`lab_id` cache is keyed only by a u64 hash and its fragment sanitizer is
   lossy.** `src/lab_ui/misc.rs:302-345`: `lab_id` hashes the raw parts with
   `DefaultHasher` and returns the cached string for the hash without verifying
   equality — a collision silently hands out a wrong element id. Separately,
   `id_fragment` maps every non-alphanumeric char to `-`, so distinct values
   (`"a.b"` vs `"a-b"`, `"a b"`) produce identical element ids in the same
   window. Fix: key the cache by the owned joined string (or `Vec<String>`),
   and either include a disambiguator in `id_fragment` or assert uniqueness of
   story/prop ids after sanitization.

7. **`story_file_name` sanitization can make two stories share one JSON file.**
   `src/lab_ui/story.rs:4-15` maps all non-alphanumerics to `_`, so story ids
   that differ only in punctuation collide (`a.b-c` and `a_b.c` →
   `a_b_c.story.json`); a save from one silently overwrites the other's designer
   state. Same collision class exists in `sanitize_path_part` for capture ids
   (`src/lib/visual_regression_manifest.rs:526-539`), where two colliding cases
   would write the same baseline/actual/diff paths. Fix: detect collisions at
   registration/manifest build time and bail, or encode a short hash suffix.

8. **Manual "Reload" and live reload treat the selected story inconsistently.**
   `apply_live_reload` re-reads the selected story's persisted
   viewport/theme/motion/layout (`component_lab.rs:588-595`), but
   `reload_documents` (820-834) only merges documents and leaves the in-memory
   selection untouched, so clicking Reload after editing a `*.story.json` on
   disk does not actually apply that file's saved layout to the UI. Fix: share
   the `selected_reloaded` block between both paths.

9. **Panic paths on registry consistency.** `ComponentLab::new` uses
   `.expect("builtin story registry")`, `.expect("selected story exists")`, and
   `.unwrap()` on the selected document (`component_lab.rs:390-391, 417, 424`),
   and `rebuild_derived_state` unwraps again (504). These are unreachable while
   the builtin registry is non-empty and visual-capture ids are pre-filtered,
   but a future caller-constructed registry or a documents map built without
   builtins turns them into panics. Fix: return `Result` from `new` or fall
   back to the first document.

10. **`ui_showcases` grows monotonically for the app's lifetime.**
    `ensure_ui_showcase` (`component_lab.rs:663-672`) inserts one retained
    `Entity<Showcase>` per visited showcase story and never evicts; only the
    visual-capture path releases them (`component_lab.rs:3583-3591`). Browsing
    all ~40 showcase sections keeps ~40 full demo trees alive. Fix: evict on
    story switch (keep only the current showcase) or cap the map.

11. **CLI robustness/perf nits.**
    - `spawn_child` splits `--child-command` on whitespace
      (`src/bin/gpui_component_lab.rs:553-559`), so quoted arguments or paths
      with spaces are silently mangled. Fix: take structured args or use
      `shell_words`.
    - `representative_cases` does `cases[index].clone().clone()` — a redundant
      double clone (`src/lib/visual_regression_manifest.rs:237, 256`).
    - `diff_visual_case` compares images pixel-by-pixel via bounds-checked
      `get_pixel` calls (`visual_regression_manifest.rs:434-452`). Fine at CI
      cadence, but `chunks_exact(4)` over `as_raw()` would be ~10× faster and
      avoids per-pixel bounds checks.

## GPU/CPU data-flow notes

The crate owns no wgpu state; rendering is delegated to gpui-px/d3rs chart
builders and GPUI's Metal renderer. No GPU→CPU→GPU cycles were found:

- Story/mesh data is deliberately kept CPU-side and shared via `OnceLock<Arc<…>>`
  fixtures (`component_lab.rs:103-336`) and `Mutex<HashMap>` caches
  (`src/lab_ui/misc.rs:160-260`, `src/lab_ui/types.rs:98-211`), so re-renders do
  not regenerate or re-clone chart buffers — the allocation-contract tests
  confirm this. Any GPU re-upload cost on chart rebuild lives in gpui-px/d3rs,
  not here.
- The only GPU→CPU readback is `cx.capture_screenshot` in the headless capture
  lane (`src/lab_ui/visual_capture.rs:179`), which is inherent to screenshot
  testing, runs once per case, and its CPU image is only written to PNG — never
  re-uploaded. `MetalHeadlessRenderer` is created once per capture run (145-150),
  which is correct.
- One minor CPU-side waste: each capture case constructs a fresh
  `ComponentLab`, which rebuilds the full builtin registry (~150 stories with
  metadata) per case (`visual_capture.rs:159-173`). Harmless at current case
  counts; worth revisiting only if the manifest grows large.

## UI/UX consistency

- The blocking issue is finding 1 (no scrolling in sidebar/controls).
- Spacing/typography consistently uses ui-kit `Text`/`Heading` sizes and theme
  tokens (`theme.surface`, `theme.border`, `TextSize::Xs/Sm`) throughout — no
  ad-hoc colors or hardcoded font sizes found.
- The lab chrome relies entirely on ui-kit `Button`/`Toggle`/`NumberInput`, so
  keyboard activation and ARIA come along for free; there is no `FocusGroup`
  around the ~150-entry sidebar, so Tab navigation through it is linear and
  there is no search/filter — acceptable for an internal tool, but a filter
  input would pay off as the registry grows.
- Minor: the prop editor forces `.decimals(2)` on all number props
  (`component_lab.rs:1186`) even for integer-count props (`bars`, `points`,
  `size`, `slices`, `groups`, `selected` — cf. `number_step` at
  `src/lab_ui/number.rs:4-12`), so counts display as e.g. "8.00".

## Clean bill

- Live-preview polling loop (`component_lab.rs:512-551`) is sound: runs on the
  background executor, breaks on entity drop, consumes failed reloads' mtimes
  to avoid hot-looping on a broken file, and calls `cx.notify()` on the result.
- No mutex/lock issues: the static data caches take short `Mutex` guards with no
  callbacks held across them; `thread_local` `RefCell`s (`misc.rs:283-299`) are
  borrowed in narrow scopes. No blocking GPU calls on the UI thread anywhere in
  the crate.
- Conformance validation (`src/lib/validate.rs`, 1,023 lines) is purely
  additive finding collection — no panics, range checks match the UI-side
  clamps (`clamp_f32` bounds in `component_lab.rs:722-764` match the
  `validate_layout_constraints` ranges).
- Manifest/diff/baseline/gallery code (`visual_regression_manifest.rs`,
  `visual_artifacts.rs`) validates dimensions, blank frames, and renderer
  namespaces before use, and paths are sanitized against traversal.
- Data-driven registration tables (`consts.rs`, `register.rs`) are consistent;
  duplicate id registration is rejected (`story_registry.rs:17-23`,
  `story_renderer_registry.rs:17-23`).

Note: this review was static analysis only; I did not build or run the crate.
Finding 4's impact would be confirmed by driving a prop change through
`VisualTestContext` without any pointer event and asserting the preview
redraws.
