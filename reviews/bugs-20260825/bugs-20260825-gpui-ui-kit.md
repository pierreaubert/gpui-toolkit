# Bug Review: gpui-ui-kit — 2026-08-25


## Resolved during follow-up — 2026-08-26

- [x] **Dialog and context-menu click propagation:** dialog content already stopped backdrop propagation; ContextMenu now does the same explicitly, so choosing an item runs its Menu selection path before the outside-dismiss handler.
- [x] **Command Palette mouse activation:** enabled result rows now invoke the configured selection handler on click, with propagation stopped at the row.
- [x] **Slider focus and arithmetic:** passive mouse movement no longer assigns focus; focus occurs on an actual press/drag. Zero or non-finite step values fall back to a finite clamped value rather than yielding NaN.
- [x] **Wizard callbacks:** Next validates the current step before calling Next or step-change handlers; Back publishes its target step as well. Validation and transition callbacks can now drive controlled wizard state as their public API promises.
- [x] **Grid focus navigation:** up/down moves by the configured column count, including bounded wraparound behavior, instead of incorrectly behaving as previous/next.
- [x] **Input key propagation:** Input and NumberInput now stop propagation only for keys they actually handle; Tab, unknown shortcuts, and application-level keys continue to parent contexts.
- [x] **Animation repeat/alternate:** repeat counts additional cycles, `alternate` reverses odd cycles, and completion/total-duration calculations include every cycle.
- [x] **Scoped component IDs and TreeView mouse support:** table, split-pane, wizard, menu, command-palette, drag-list, and tree rows derive child IDs from their owning component. Tree rows now focus/select on click; branch arrows are separate propagation-stopping toggle controls.
- [x] **Focus-out subscriptions:** Select and NumberInput replace their blur subscription on every render (and Select removes it when no toggle callback is present), so callbacks and window ownership cannot become stale.
- [x] **Bounded text history and toast contract:** EditState retains at most 200 undo snapshots; Toast documentation now accurately describes display-duration metadata consumed by its host instead of promising an unimplemented timer.
- [x] **Command Palette local cache lifecycle:** per-palette leaked hover-handler maps were removed in favor of a per-render Copy closure, and filtered-index entries are capped at 64 before eviction.
- [x] **Other thread-local stores:** input, button, icon-button, Select, Tabs, and SwipePanel identity stores are bounded (weak entity caches also prune dead entries); drag-only SplitPane/DragList/interaction maps remove their state on mouse-up/end-drag.
- [x] **Wizard README drift:** the referenced stale Wizard example is absent from the current README, so it cannot mislead users about removed API.
- [x] **Progress and pagination edge cases:** zero/non-finite progress maxima render 0% rather than NaN; pagination reports (0, 0) for empty results and clamps an out-of-range page to the final valid page.
- [x] **Popover and drag guards:** popover panels now stop their mouse event before it reaches the dismissing backdrop; drag math returns no update for zero or non-finite track dimensions rather than dividing into non-finite values.
- [x] **Slider min/max documentation disposition:** current setters explicitly document their panic invariant and range validates min <= max, so the review’s documentation-drift finding is no longer present.

Verified cargo test -p gpui-ui-kit (1,319 passed, 47 ignored).
Scope: full scan of `crates/gpui-ui-kit/src/` (187 Rust files, ~42k lines; 383 files /
~72k lines including tests and examples), prioritising interaction-heavy components
(slider, input, number_input, select, menu, context_menu, dialog, popover,
command_palette, table, tree_view, tabs, focus, wizard, toast, drag_list, split_pane,
swipe_panel, thinking_orb, qr, animation, interaction/scale helpers, accessibility,
data_navigation, collection_diff, momentum). The 2026-08-22 perf review of this crate
(`reviews/perf-gpui-ui-kit-20260822.md`) already covers per-frame allocation/tessellation;
this review targets correctness, event-dispatch, memory growth, and UI/UX consistency, and
does not repeat those perf findings. GPUI mouse-event semantics were verified against the
vendored fork (`crates/3rdparties/gpui/src/window.rs:4840-4848`): bubble-phase listeners
fire for every ancestor in the dispatch path until a handler calls
`cx.stop_propagation()`.

## Findings

Ranked by severity. All line numbers verified against the working tree.

### Critical

None.

### High

1. **Dialog closes when clicking anywhere inside it.**
   `src/dialog.rs:43` defines `ignore_mouse_down` as an empty closure, attached at
   `src/dialog.rs:307-308` with the comment "Stop propagation so clicking dialog doesn't
   close it" — but it never calls `cx.stop_propagation()`. The dialog is a *child* of the
   backdrop (`backdrop.child(dialog)`, `src/dialog.rs:413`), and the backdrop closes the
   dialog on `on_mouse_down` (`src/dialog.rs:286-291`). Any click inside the dialog body
   bubbles to the backdrop and fires `on_close` whenever `close_on_backdrop` is true
   (the default). Fix: make the inner handler `|_, _, cx| cx.stop_propagation()`. The
   correct pattern already exists in-crate at `src/select.rs:527-529` and
   `src/workflow/canvas/workflow_canvas.rs:1320`.

2. **ContextMenu: clicking an item can close the menu before `on_select` runs.**
   Same shape: the positioned menu wrapper is a child of the dismiss-on-`on_mouse_down`
   backdrop (`src/context_menu.rs:177-180`, wrapper at `:208-215`), and the wrapper's
   `on_mouse_down` is a no-op despite the comment "Stop propagation so clicking menu
   doesn't close it" (`:213-214`). Menu items select on `on_mouse_up`
   (`src/menu.rs:333-335`), which fires *after* mouse-down — so the backdrop's close
   handler runs first; an app that unmounts the menu in `on_close` never delivers
   `on_select`. Clicking menu padding/separators also dismisses. Fix:
   `cx.stop_propagation()` in the wrapper's mouse-down handler (and consider stopping
   propagation in item `on_mouse_up`).

3. **CommandPalette is unusable with a mouse: no click handler on items, and any click
   inside dismisses it.** Item rows get `cursor_pointer()` and a hover style but no
   `on_click`/`on_mouse_down` selection handler at all (`src/command_palette.rs:481-521`).
   Worse, the palette container's `on_mouse_down` is an empty closure whose comment claims
   it stops propagation (`:426-428`), while the ancestor overlay dismisses on
   `on_mouse_up` (`:409-411`) — so clicking anywhere inside the palette (including on an
   item) bubbles up and closes it without selecting. Fix: add a real click-select handler
   per row that calls `cx.stop_propagation()`, and make the container's handler actually
   stop propagation.

4. **Slider steals keyboard focus on hover.** In `on_mouse_move`, when no button is
   pressed and the slider isn't focused, the handler calls `focus_hover.focus(window, cx)`
   (`src/slider.rs:486-489`). Merely moving the mouse across a slider yanks focus away
   from whatever the user was editing (e.g. an Input), breaking keyboard input and
   focus-visible UX. No sibling component does this. Fix: drop the hover-focus block; the
   `on_mouse_down` handler already focuses correctly (`:475`).

### Medium

5. **NumberInput and Input swallow every keystroke while focused.**
   `NumberInputEntity::handle_key_down` calls `_cx.stop_propagation()` unconditionally at
   the top (`src/number_input.rs:365`), and `InputEntity::handle_key_down` does the same
   after its focus check (`src/input.rs:608`). Unhandled keys (Tab for focus traversal,
   app-level function-key bindings, etc.) are consumed and never reach parent key
   contexts. `Select` documents the opposite discipline — "B3 fix: cx.stop_propagation()
   is only called for keys we actually handle" (`src/select.rs:344,403-405`). Fix: move
   `stop_propagation` into the arms that actually handle the key, as Select does.

6. **`Animation::repeat` / `alternate` are dead public API.** Both are settable
   (`src/animation.rs:107-116`) but never read: `progress()` (`:148-161`) and
   `is_complete()` (`:164-166`) ignore them, and nothing else in the crate reads
   `self.repeat`. Callers get a silently non-functional repeat/ping-pong feature. Fix:
   either implement repeat/alternate in `progress()`/`is_complete()` or remove the
   setters and fields.

7. **Wizard: `on_validate` and `on_step_change` are stored but never invoked.**
   Module docs promise "Step dependencies (can only advance if validation passes)"
   (`src/wizard/mod.rs:7`), but a grep over the crate shows `on_validate` is only ever
   assigned (`:84,186-189`) and never called; same for `on_step_change` (`:53,176-182`).
   "Next" fires `on_next` unconditionally (`:413-417`). Validation-gated flows silently
   advance invalid steps. Fix: call `on_validate(current_step)` in the Next handler and
   suppress the transition on `false`; fire `on_step_change` from the navigation handlers.

8. **FocusGroup `Grid` navigation ignores the `columns` parameter.**
   For `FocusDirection::Grid { columns }`, up/down map to `FocusMove::Previous/Next`
   (`src/focus.rs:316-333`), i.e. index ±1 — identical to a vertical list. `columns` is
   accepted but never used (also ignored in layout at `:396-399`). In a real grid, "down"
   should move ±columns. Fix: pass `columns` into `target_index` and step by it for
   vertical movement in grid mode.

9. **Hardcoded/unsanitised stateful element IDs collide across component instances.**
   GPUI keys interaction/hover/focus state by `ElementId`, and several components mint
   non-unique ids:
   - `src/table.rs:403` `.id("table-body")` and `:490` rows keyed by bare
     `ElementId::from(row_idx)` (a `usize`) — two tables in one window share row ids
     `0..n` and the same `"table-body"`.
   - `src/split_pane.rs:212,233` `.id("split-divider")` regardless of the pane's own id.
   - `src/wizard/mod.rs:368,388,402` hardcoded `"wizard-cancel" / "wizard-back" /
     "wizard-next"` — two wizards collide.
   - `src/menu.rs:294`, `src/command_palette.rs:482`, `src/drag_list.rs:148` use raw
     caller-supplied item ids without scoping to the parent component id.
   Fix: scope every child id under the component id, e.g.
   `ElementId::from((self.id.clone(), row_idx))`, as Select already does for options
   (`src/select.rs:456-460`).

10. **Thread-local state maps grow without bound.** Many components keep
    `thread_local!` `RefCell<HashMap<ElementId, _>>` state that is only ever inserted:
    `FILTERED_INDICES_CACHE` grows per distinct query string typed into a palette
    (`src/command_palette.rs:159-161,293-312` — every keystroke with a new query adds a
    permanent entry); `PALETTE_HOVER_HANDLERS` is intentionally `Box::leak`ed per id
    (`:186-191`) and `PALETTE_HOVER_BG` never shrinks (`:165,176-178`); focus-handle maps
    in `src/button.rs:23`, `src/icon_button.rs:31`, `src/tabs.rs:29`,
    `src/swipe_panel.rs:37`, `src/interaction.rs:48` have no cleanup at all.
    `src/input.rs` / `src/number_input.rs` at least document the growth and expose
    cleanup functions. Fix: add eviction (LRU or clear-on-dismiss for the palette caches)
    and cleanup functions mirroring `input/cleanup.rs`.

11. **NumberInput/Select focus-out subscriptions go stale across windows and prop
    changes.** `NUMBER_INPUT_FOCUS_SUBS` registers `window.on_focus_out` once per id and
    keeps it forever (`src/number_input.rs:989-1002`); if the same id is later rendered
    in a different window, the old window's subscription is retained and blur handling in
    the new window never registers. `SELECT_FOCUS_SUBS` has the same once-per-id pattern
    and additionally captures the *first render's* `on_toggle` closure
    (`src/select.rs:298-309`). Fix: key subscriptions by (window, id) or re-register when
    the entity/handler changes.

12. **EditState undo history is unbounded.** Every text-mutating keystroke pushes a
    full-text `EditSnapshot` (`src/input/edit_state.rs:63-66`) with no depth cap; a long
    editing session on a large buffer grows memory linearly per keystroke. Fix: cap the
    undo stack (e.g. 200 entries) and/or coalesce consecutive inserts.

13. **Pagination label is wrong for empty lists.** `PaginationState::page_range` returns
    `(1, 0)` when `total_items == 0` (`src/table/pagination_state.rs:21-29`), and Table
    prints it verbatim → "Showing 1 to 0 of 0 items" (`src/table.rs:622-627`). Fix:
    return `(0, 0)` when `total_items == 0`.

### Low

14. **`Slider::step(0.0)` produces NaN values.** `snap_value` divides by `step`
    unchecked (`src/slider.rs:236-243`); `step(0.0)` yields NaN/∞ passed to `on_change`.
    Fix: assert `step > 0.0` in the setter or ignore non-positive steps.

15. **Progress with `max == 0.0` renders "NaN%".** `self.value / self.max * 100.0` with
    `max == 0.0` yields NaN, which survives `.clamp(0.0, 100.0)` and is formatted into the
    label (`src/progress.rs:158,278`). Fix: guard `max > 0.0`.

16. **Toast advertises auto-dismiss that nothing implements.** `Toast::new` documents
    "auto-dismisses after 5 seconds by default" (`src/toast.rs:85`) and stores
    `duration_secs`, but no timer in the crate consumes `get_duration_secs/ms`
    (`:147-155`); dismissal only happens via the close button. Either implement the timer
    (weak-entity scoped, like `swipe_panel.rs:424-447`) or fix the doc comment.

17. **TreeView rows are mouse-inert.** Rows are built with hover styling but no
    `on_click`/`on_mouse_down` and no expand-on-click
    (`src/tree_view.rs:263-315`); selection and expand/collapse are keyboard-only, unlike
    sibling Table which supports mouse selection (`src/table.rs:512-536`). Fix: attach
    click handlers that fire `on_select`/`on_toggle` (and give rows scoped ids — see #9).

18. **Slider `min()`/`max()` docs lie about panicking.** Both setters claim
    "# Panics ... if min > max after this call" (`src/slider.rs:118-134`) but contain no
    assertion; only `range()` validates (`:140-150`). Fix: remove the doc lines or add
    the assert.

19. **Stale README Wizard example.** `README.md:693-714` shows
    `WizardStep::new(...).content(...)` and `WizardStepStatus::Completed`, but
    `WizardStep` has no `content` field (`src/wizard/wizard_step.rs:5-18`) and the enum is
    `StepStatus` (`src/wizard/types.rs`). Fix: update the README to the real API.

20. **Popover's "prevent click-through" handler is dead code.** The panel's empty
    `on_mouse_down` (`src/popover.rs:166-167`) claims to stop propagation but doesn't; it
    happens to be harmless because the backdrop there is a sibling, not an ancestor
    (`:234-238`). Fix: delete it or make it `cx.stop_propagation()` for clarity.

21. **`handle_drag` divides by `track_size` unchecked** (`src/interaction.rs:288`); a
    zero-width control yields inf/NaN. Only reachable with `width(0.0)`-style misuse, but
    a `track_size.max(1.0)` guard is one line.

## GPU/CPU data-flow notes

The crate has essentially no direct GPU surface: no `map_async`, `read_texture`,
`device.poll`, or `pollster` usage. The one `paint_image` path (`src/qr/qr_code.rs:113-120`)
rasterizes the QR matrix to a bitmap once at build time and paints a single image quad —
the pattern the perf review recommended; no GPU→CPU→GPU round trip exists. `ThinkingOrb`
(feature `vello`) delegates rasterization to gpui-d3rs's vello2d path and is out of this
crate's direct control. No action needed.

## UI/UX consistency

- Overlay dismissal is the big inconsistency: Select (`src/select.rs:527-529`) and the
  workflow canvas menu (`src/workflow/canvas/workflow_canvas.rs:1320`) correctly stop
  propagation so inside clicks don't dismiss, while Dialog, ContextMenu, and
  CommandPalette ship no-op handlers with comments claiming they do (findings 1-3, 20).
- Keyboard propagation discipline diverges: Select stops only handled keys while
  NumberInput/Input stop everything (finding 5).
- Focus acquisition diverges: Slider focuses on hover; every other component focuses on
  mouse-down only (finding 4).
- Mouse support is uneven across selection lists: Table rows are clickable, Menu items
  are clickable (mouse-up), but CommandPalette items and TreeView rows are keyboard-only
  (findings 3, 17).
- ARIA registration is uniform and follows the `AGENTS.md` default-role table; spot-checks
  (button, checkbox, slider, select, dialog, toast, table, tree, menu) all register with
  the documented roles. Note the documented caveat stands: `build()` /
  `build_with_theme()` paths (e.g. `Menu::build_with_theme`, `CommandPalette::build_with_theme`)
  bypass `register_accessible`, so direct `build()` users lose the tree entries.

## Clean bill

- `src/data_navigation.rs`, `src/collection_diff.rs`, `src/input/edit_state.rs` (char/byte
  boundary math), `src/mobile/momentum.rs` (velocity regression + guards),
  `src/animation/keyframe_animation.rs`, `src/scale.rs`, and `src/arc.rs` are carefully
  clamped and well unit-tested; no bugs found.
- Timer loops in `swipe_panel.rs:420-447` and `thinking_orb/mod.rs:240-267` are
  weak-entity scoped, exit on entity drop/pause, and (orb) use a generation counter
  against stale tickers — no leak or deadlock. No mutex/RwLock anywhere; all shared state
  is single-threaded `thread_local!`/`Rc`/`RefCell`, and no RefCell borrow was observed
  held across a callback that re-enters the same cell (handlers consistently
  `drop(state)` before firing user callbacks, e.g. `src/number_input.rs:478-484`).
- QR (`src/qr/`) and pagination arithmetic (`total_pages`, spacer heights) handle zero/NaN
  geometry; `src/accessibility.rs` frame generation pruning (`begin_frame`/`end_frame`)
  is sound.
