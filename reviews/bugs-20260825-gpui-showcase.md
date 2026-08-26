# Bug Review: gpui-showcase — 2026-08-25

Scope: full scan of `crates/gpui-showcase` — all of `src/` (`lib.rs`, `main.rs`,
`release_artifacts.rs`, `showcase.rs`, the 3 `showcase/*` support modules and all
44 `showcase/sections/render_*.rs` files, ~7,600 lines of Rust), plus the iOS/tvOS
FFI glue (`ios/src/*.rs`), the Android `android_main` glue, and the two
profiler/allocation contract tests (~50 Rust files total). To judge behavioral
claims I also read the relevant vendored GPUI internals
(`crates/3rdparties/gpui/src/{app.rs,window.rs,elements/div.rs,app/entity_map.rs}`)
and the gpui-ui-kit components the showcase drives (`tabs.rs`, `accordion.rs`,
`toggle.rs`, `button.rs`). No wgpu/rendering code lives in this crate; GPU content
is instantiated from gpui-ui-kit/gpui-d3rs elements.

## Findings

Ranked by severity. Line numbers refer to files under `crates/gpui-showcase/`
unless noted.

## Resolved during follow-up (2026-08-26)

- **Reliable interactive invalidation (#1):** `ShowcaseHandle::update` now calls `cx.notify()` after each successful entity mutation. Every existing section callback using the shared weak handle now causes a GPUI repaint rather than depending on incidental hover or focus invalidation. Verified with `cargo test -p gpui-showcase --lib` (12 passed).
- **Pane divider outside-release cleanup (#3):** the demo container now uses capture-phase `on_mouse_up_out` to clear an active left-pane drag when release occurs outside its 600×200 hitbox. Re-entering the demo can no longer resume a stale drag. Verified with `cargo test -p gpui-showcase --lib` (12 passed).
- **Section scroll reset (#4):** selecting a section resets the persistent content `ScrollHandle` to `(0, 0)` and notifies the content entity, so each section opens at its top rather than inheriting the preceding section’s offset. Verified with `cargo test -p gpui-showcase --lib` (12 passed).
- **Independent static tab identities (#2):** the Pills and Enclosed tab examples now use distinct `Tabs` IDs (`tabs-pills` and `tabs-enclosed`), preventing their retained `TabsEntity` state from colliding. Verified with `cargo test -p gpui-showcase --lib` (12 passed).
- **Advertised window-size CLI option (#5):** startup is delegated to MiniApp’s validated parser, which advertises `--window-min-size WIDTHxHEIGHT`, rejects malformed dimensions, and clamps valid sizes to the visible display. `target/release/gpui-showcase --help` now prints the option.
- **Thinking Orbs grid solver (#8, disproved as a bug):** the two calls serve distinct layouts—sphere cards use their dynamic card width, while the controls use a fixed 400 px width. The grid remains intentionally solved through `gpui-builder`, as required for its adaptive layout behavior; focused boundary coverage verifies 1, 2, and 9 columns.
- **Navigation identity lookup (#9):** `ShowcaseSection` now has an explicit `usize` representation and stable index, letting cached navigation IDs use direct indexing instead of a linear search for every sidebar row. `section_indices_match_navigation_order` verifies the enum and navigation order remain aligned.
- **Table selection after sort (#6):** because the table component’s selection contract is row-index based, sorting now clears selection rather than applying stale indices to different users. Verified with `cargo test -p gpui-showcase --lib` (13 passed).
- **Checkbox size demo binding (#7):** the Large checkbox now uses the same checked value and update direction as the Small and Medium variants, so all three accurately demonstrate one shared selection state. Verified with `cargo test -p gpui-showcase --lib` (13 passed).
- **Editor backup files (#10, disproved as current):** no `*~` files remain under `crates/gpui-showcase`, so there is no source-tree pollution to remove in the current worktree.

### High

1. **Interactive sections mutate state with no `cx.notify()` and no window
   refresh, so the repaint depends on incidental invalidation.**
   `ShowcaseHandle::update` (`src/showcase.rs:291-299`) is just
   `WeakEntity::update`; vendored GPUI's `update_entity`
   (`crates/3rdparties/gpui/src/app.rs:2557-2571`) does not notify or dirty any
   window. The affected callbacks are the wizard buttons
   (`src/showcase/sections/render_wizard.rs:76-111`), all four accordion
   `on_change` handlers (`src/showcase/sections/render_accordion.rs:51-62`,
   `95-108`, `148-158`, `199-211`), the form toggles/checkboxes/slider/select/
   buttonsets (`src/showcase/sections/render_form.rs:50-84`, `101-136`,
   `161-168`, `341-366`, `398-484`), settings form
   (`render_settings_form.rs:35-39`), accessibility controls
   (`render_accessibility.rs:62-103`), search bar (`render_search_bar.rs:64-69`),
   drag lists (`render_drag_list.rs:32-69`), and workflow toolbar
   (`render_workflow.rs:51-70`). This matters because the underlying components
   trigger these handlers from `on_mouse_up` (e.g. accordion header,
   `crates/3rdparties/gpui-ui-kit/src/accordion.rs:343-348`; toggle,
   `toggle.rs:263-265`), and plain `on_mouse_up` listeners bind no refresh
   (`div.rs:206-220`) — unlike `on_click`, which refreshes in the capture phase
   (`div.rs:2795-2810`). A click that is not followed by a mouse-move,
   hover-change, or focus-change therefore leaves stale pixels until some
   unrelated redraw; sibling sections that do call `notify_content` (table,
   tabs, popover, tooltip) behave correctly, so this is also an internal
   inconsistency. Fix: call `this.notify_content(cx)` (or `cx.notify()`) in every
   state-mutating callback above. To confirm the visible lag before fixing:
   synthesize a `MouseUpEvent` on an accordion header in a `VisualTestContext`
   without moving the mouse and compare frames before/after.

### Medium

2. **Duplicate `Tabs::new("tabs")` element id shares one global state entity
   between the Pills and Enclosed demos.**
   `src/showcase/sections/render_tabs.rs:117` and `:144` both pass `"tabs"`.
   gpui-ui-kit caches a render entity per `ElementId` in a thread-local
   (`crates/3rdparties/gpui-ui-kit/src/tabs.rs:26-31`), so the two visible tab
   bars alias the same `TabsEntity` — hover, focus, and keyboard-navigation
   state leak between them, and removing one demo can drop the shared entity
   under the other. Fix: give each instance a unique id (e.g. `"tabs-pills"`,
   `"tabs-enclosed"`).

3. **Pane-divider drag sticks at the container edge and never clears if the
   mouse is released outside the demo box.**
   `src/showcase/sections/render_layout.rs:514-536` attaches `on_mouse_move` and
   `on_mouse_up` to the fixed 600×200 container. Both listeners are
   hitbox-gated (`div.rs:206-220`, and the move listener likewise), so dragging
   past the edge freezes the pane mid-drag, and releasing outside never fires
   the `on_mouse_up` handler — `pane_dragging_left` stays `true` and the pane
   jumps to the cursor the next time it re-enters the box. Fix: register
   capture-phase/window-level move/up handlers for the duration of the drag
   (or clear drag state from an `on_mouse_up_out` / mouse-exit path), as the
   component's own doc comment suggests handling up "on a parent element that
   covers the full drag area" — the fixed demo box does not.

4. **Content scroll position is not reset when switching sections.**
   `ShowcaseContent` keeps one persistent `ScrollHandle`
   (`src/showcase.rs:1019`, `:1029`), and `select_section`
   (`src/showcase.rs:451-461`) never resets it. Switching from a scrolled-down
   section to another long section opens the new section mid-scroll instead of
   at the top. Fix: `self.content_entity`'s scroll handle `set_offset` /
   `scroll_to_top` on section change.

### Low

5. **`--help` advertises an unimplemented flag; unknown args are silently
   ignored.** `src/main.rs:29-34` documents `--window-min-size WIDTHxHEIGHT`,
   but the argument parser has no arm for it — it (and any typo) falls through
   to `_ => {}` and the app launches normally. Fix: implement the flag or drop
   it from the help text, and error on unrecognized arguments.

6. **Table selection is index-based while sorting reorders the rows.**
   `src/showcase/sections/render_table.rs:51-64` sorts `self.users` in place
   but `selected_users` (`:67-74`) stores row indices, so after a sort the
   highlighted rows are different users than the ones the user selected. Fix:
   key selection by `User::id` (or clear selection on sort).

7. **The "Large" checkbox is bound to the inverted shared state.**
   `src/showcase/sections/render_form.rs:124-137` renders
   `.checked(!checkbox_checked)` and writes `checkbox_checked = !checked`, so
   clicking any of the three size-demo checkboxes flips all of them and the
   Large one always disagrees with Sm/Md. If the intent is "same state, three
   sizes", bind it like the others; if the intent is an inverted-binding demo,
   label it as such.

8. **`orb_grid_columns` runs the full constraint solver up to 9× per call,
   twice per render.** `src/showcase/sections/render_thinking_orb.rs:134-160`
   allocates a fresh `Vec<LayoutNode>` and calls `gpui_builder::solve()` for
   each candidate column count; `render` calls it at `:193` and again at
   `:194`. For fixed-size slots the answer is closed form
   (`floor((available + gap) / (card + gap))`, clamped to `1..=9`) — no solver
   or allocations needed. Only triggered on lab re-renders, so impact is small.

9. **Per-render linear lookup in `cached_navigation_id`.**
   `src/showcase.rs:41-45` does `ShowcaseSection::all().iter().position(...)`
   for every nav item on every sidebar build — 45 items × 45-entry scan.
   Trivial cost, but pairing ids with sections via `enumerate` in the
   `OnceLock` init (or caching `[(section, id)]` pairs) removes the O(n²)
   pattern entirely.

10. **Committed editor backup files.** `src/showcase/sections/render_wizard.rs~`,
    `src/showcase/sections/render_workflow.rs~`, and `Cargo.toml~` sit in the
    crate. They are not compiled (not declared in `mod.rs`), but they pollute
    the source tree and confuse greps; delete and gitignore `*~`.

## GPU/CPU data-flow notes

This crate contains no wgpu code of its own — no `device.poll`, no
`pollster::block_on`, no readbacks, no per-frame buffer creation. The GPU-backed
content (Vello spectrum/meters/knobs in `render_audio_visuals.rs`, `ThinkingOrb`
entities in `render_thinking_orb.rs`) is constructed and owned by gpui-ui-kit /
gpui-d3rs elements; the showcase only holds static CPU-side inputs
(`AudioVisuals::magnitudes` is a fixed `Arc<[f32]>`, cheaply cloned per render)
and reads CPU-side `frame_stats()` for the stats line. No GPU→CPU→GPU cycles to
fix here. `gpui::wgpu_custom_draw_available()` is queried on each lab render —
a cheap flag read, fine. The AGENTS.md caveat about gpu3d/compute paths on wasm
does not apply to anything this crate instantiates.

## UI/UX consistency

- Finding 1 is also the main consistency issue: half the interactive sections
  notify after mutation, half don't.
- Finding 7 (inverted checkbox) and finding 6 (index-based selection) are
  demo-visible behavior quirks.
- i18n is applied unevenly: section headers and some labels go through
  `cx.t(TranslationKey::…)`, but most body/help strings are hardcoded English
  (e.g. all of `render_progress.rs`, `render_table.rs`, `render_workflow.rs`).
  Acceptable for a demo, but the half-localized mix reads inconsistently when
  the language is switched.
- Hardcoded `rgba(0xffffffff)` for active nav text (`src/showcase.rs:856`,
  `:919`) bypasses the theme; an `on_accent`/`accent_foreground` token would
  survive accent-color changes. A few one-off pixel values (`py(px(5.0))`,
  sidebar `px(220.0)`) duplicate gpui-design spacing that other sections get
  via `StackSpacing`.
- ARIA/focus: the showcase root tracks a focus handle for its text-editing key
  handler, and the Accessibility section exercises aria labels; the custom tab
  bar (`render_tabs.rs:71-88`) and popover/tooltip triggers
  (`render_popover.rs:34-61`, `render_tooltip.rs:43-73`) are mouse-only with no
  focus handle or key handlers, unlike the ui-kit components they imitate
  (Tabs supports a `focus_handle`). Fine for a showcase, but it silently
  demonstrates a keyboard-inaccessible pattern next to components that are
  accessible.

## Clean bill

- No threads, mutexes, channels, or `RefCell` in the crate — nothing to
  deadlock; all state is single-threaded GPUI entities with weak handles, and
  the `WeakEntity` upgrade paths are checked.
- The `AnimatedQrCode` 30 Hz timer entities are deliberately dropped when
  leaving the QR section and re-created on entry (`src/showcase.rs:451-461`),
  avoiding idle redraws.
- Persistent child entities (`ShowcaseSidebar/Header/Content`) with
  compare-before-update syncing (`src/showcase.rs:489-530`) keep stable
  subtrees from being rebuilt every frame; warmed allocation-contract tests
  back the nav/typing/section-switch paths.
- All `unwrap`/`expect` sites are unreachable-in-practice (cached-id lookup,
  single-byte keystrokes, QR entities initialized before render) or test-only.
- iOS FFI entry point is wrapped in a `catch_unwind` guard (`ios/src/misc.rs`),
  and the Android glue logs and returns on platform-init failure instead of
  panicking.
- `release_artifacts.rs` JSON/slug generation handles its (fully static)
  inputs correctly and is covered by contract tests.
