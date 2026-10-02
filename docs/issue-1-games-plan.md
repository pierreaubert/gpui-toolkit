## Outcome

Enable polished simple 2D games in gpui-toolkit, authored from Python and rendered by GPUI, with desktop keyboard/mouse and mobile finger input. The first consumers are Zip, Queens, Sudoku, and Tetris in crates/gpui-python-runtime/python/examples/games_demo.py. Audio playback and mixing belong to ../sotf-daw.

This is a planning issue. It does not claim the proposed APIs or mobile Python deployment exist.

## Evidence from the current worktree

- games_demo.py:44 creates board cells with ui.button. Zip (222), Queens (537), and Sudoku (922) are stacks of these buttons. ButtonNode has only id, label, action, selected, and disabled; render_button uses generic control padding/corners.
- Tetris board_node (1544) renders rows of glyphs as text, making geometry and appearance depend on fonts. Its background loop (1675) applies a gravity step then sleeps for tick_interval; Sudoku solver animation sleeps at 30 updates/second.
- The demo dispatches click/control actions and has no game keyboard bindings or continuous pointer handling. Generic button Enter/Space activation exists in the host; Python keybinding value declarations also exist.
- gpui-ui-kit already provides Animation, Easing, Spring, keyframes, and reduced-motion handling. GPUI Window provides on_next_frame/request_animation_frame and native quads, paths, text, images, and shadows.
- gpui-d3rs::vello2d already provides ChartScene paths, shapes, brushes, shaped text, and retained painters. Its WGPU path uses custom draws; its Metal painter calls snapshot_scene_gpu on changed revisions then paints an image. GPU-backed composition alone does not prove low-cost dynamic drawing.
- gpui-ios/src/ios/window/touch.rs defers click synthesis until finger release, routes vertical gestures to scrolling, and recognizes two contacts as pinch. Android window.rs similarly translates contacts into mouse/scroll/pinch. GPUI PlatformInput has no independent contact stream.
- PythonEventSink::send in bin/showcase/python.rs locks/writes/flushes child stdin from UI callbacks. Incoming Python messages have a bounded 256-message channel, but drain_session drains available messages without a work budget.
- App.serve submits actions to four workers. The demo lock prevents concurrent model mutation but does not establish event sequence order or atomic model-mutation/patch-publication ordering.
- The current native Python host launches a child process with stdio. Mobile platform declarations do not establish that this launcher or the Python demo runs in an iOS/Android package.
- Baseline: PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=crates/gpui-python-runtime/python python3 -m unittest discover -s crates/gpui-python-runtime/python/tests -p test_games_demo.py: 31 tests pass. These verify logic and IR, not appearance, timing, or device input.

## Proposed small toolkit surface

### 1. A retained 2D drawing element and Python declaration

Add Scene2D/GameSurface in gpui-ui-kit and a scene2d node, typed specs, validation, and adapter in gpui-python-runtime. Names are provisional. It should remain useful for diagrams and custom controls as well as games.

Start with an ordered display list of stable IDs: rectangles/rounded rectangles, circles, lines/paths, text, and small transform groups. Include solid/gradient brushes, strokes, opacity, clipping, and limited shadows. Add cached image references through existing GPUI image resources when needed; no mandatory sprite atlas.

Use GPUI native painting for ordinary moving shapes/text/images. Reuse d3rs geometry and existing Vello machinery for complex vector artwork after profiling the selected backend. Do not make GPU snapshot/readback part of every animation frame.

Support logical view bounds, contain/letterbox fitting, aspect ratio, display scale, and one shared forward/inverse coordinate transform for paint and hit testing. Stable IDs survive resize.

Retain static board geometry and text shaping; patch only changed objects or groups. Separate background, pieces, effects, and HUD. Transform/opacity animation must not reconstruct unrelated widgets or reload assets. Cache invalidation must include size, display scale, theme, and font changes.

Expose a small Python grid/board helper over this generic drawing API: cell geometry, cell picking, grid borders, selected/conflict overlays. Game rules remain in the demo. Support atomic multi-object changes; validate patches before committing and preserve the last valid scene on errors. Begin with JSON for these small boards, adding binary transport only if measurements justify it.

### 2. Focused keyboard and direct pointer input

Add a normalized pointer event stream with contact ID, mouse/touch device kind, down/move/up/cancel phase, monotonic timestamp, local logical coordinates, buttons/modifiers, and capture state. Pressure is optional.

Add explicit per-surface gesture ownership. A direct-contact surface claims contacts at finger-down, captures them through release/cancel, and opts out of scroll/pinch synthesis for those contacts. Contacts starting outside the board keep normal app behavior. Avoid duplicate synthetic mouse activation, and release all captured contacts on focus/lifecycle loss or removal.

Carry independent contacts through the shared GPUI dispatch layer and the iOS/Android adapters. Do not emulate multiple fingers with one pressed mouse button. Desktop mouse uses the same surface API.

Expose focused key-down/key-up and held actions with configurable repeat delay/rate. Register keyboard scope on the active board; text inputs/dialogs retain their normal focus behavior. Clear held state on focus loss, suspension, section change, and disconnect.

For ordinary taps emit semantic cell/object IDs. Continuous surfaces may opt into local pointer samples. Keep hit testing, hover/press feedback, gesture recognition, and capture native.

For Zip, preserve an ordered drag trajectory or emit every crossed grid cell. Latest-position-only coalescing can skip cells and break gameplay. Generic drags may use bounded latest-position updates; down/up/cancel and discrete actions must remain ordered.

### 3. Frame timing, animation, and ordered state updates

Create one cancellable frame-clock subscription per active surface using GPUI frame requests and monotonic elapsed time. Request frames only while animation or play needs them. Pause on hidden/inactive/suspended surfaces and cancel on removal. Resume without simulating the entire background interval.

Run visual interpolation in Rust. Python publishes a new target plus a transition specification once, and the host animates position, scale, opacity, color, or path reveal using existing easing/spring/keyframe code. Define interruption/retargeting from the current presented value, removal, completion IDs, and reduced-motion behavior. Collision/rule state always uses authoritative targets.

Offer an opt-in serial reducer/simulation lane in Python. Process ordered input and ticks on that lane; commit model mutation and outbound scene update together. Solver computation may stay on workers, but completion results return to the reducer and use generation checks.

For periodic logic, expose elapsed-time tick delivery with bounded outstanding requests, or a cancellable monotonic-deadline scheduler. Use an accumulator, bounded catch-up, and explicit suspend/resume reset. Tetris gravity stays independent of display refresh. Never make a frame wait for Python or send a per-frame round trip solely to animate a highlight.

Move host-to-Python writes to a supervised background writer. Bound messages and bytes, preserve discrete input order, and define overflow recovery that cancels held/captured input rather than silently dropping releases. Budget inbound message application per frame and reschedule remaining work. Preserve existing UI patch revisions: arbitrary delta patches cannot be dropped. Only explicitly self-contained visual updates may use latest-generation replacement.

### 4. Responsive layout and accessibility

Provide board aspect-fit layout, safe-area accommodation, portrait/landscape arrangements, and adaptive HUD/control placement using existing design/layout infrastructure. Desktop shows rules beside the board; phones prioritize the board and put rules behind a compact help action.

Use platform touch-target tokens for controls and measure available board cell sizes. On narrow phones, a 9x9 board cannot always give every cell a full control-sized target: support deliberate cell selection with an enlarged preview and an accessible keypad, or zoom where needed. Do not create overlapping padded cell hit areas.

Provide semantic grid/cell accessibility and keyboard selection without requiring a visual button per cell. Announce selection, conflicts, game status, and results; avoid per-frame announcements. Keep symbols/outline cues alongside color and preserve essential gameplay feedback with reduced motion.

## First visual/game conversions

| Game | Presentation | Desktop | Finger input |
| --- | --- | --- | --- |
| Zip | Thin grid, numbered checkpoint discs, continuous rounded path, animated head | Arrows extend; Backspace/Undo retract; mouse drag | Captured finger drag with ordered cell traversal |
| Queens | Actual colored regions, vector crowns, crosses, conflict borders | Arrows select; Enter/Space cycle; explicit mark action | Tap cycle; optional explicit crown/mark mode |
| Sudoku | Aligned 9x9 board, stronger 3x3 borders, given/user distinction, candidates, peer highlights | Arrows, digits 1-9, Delete/Backspace | Tap selection, large keypad, enlarged selection preview |
| Tetris | Colored geometric blocks, ghost piece, next preview, score HUD, brief lock/clear effects | Held Left/Right/Down, rotate key, Space hard drop, pause | Large hold controls supporting move+rotate with two fingers; optional swipes |

Tetris should remain responsive; avoid smoothing horizontal moves enough to obscure the real occupied cell. Line-clear effects need a defined transition so removed rows and new row positions are shown consistently.

Use one coherent game palette, readable typography, consistent board spacing, and restrained emphasis/win animations. Ordinary controls remain appropriate for New, Undo, Hint, Pause, settings, and menus.

## Ownership and scope boundary

- gpui-ui-kit: portable drawing surface, interaction state, board helpers, native visual animation.
- gpui-design/gpui-builder: reuse design tokens and responsive layout.
- GPUI fork + gpui-ios/gpui-android: contact dispatch, gesture ownership, lifecycle/frame integration.
- gpui-d3rs: reuse geometry/vector rendering; add only genuinely missing renderer primitives.
- gpui-python-runtime: typed declarations/events, retained cache/patches, ordering, scheduling, capability negotiation, transport, host integration.
- games_demo.py: rules, puzzle generation, Tetris state, score, and per-game presentation.
- ../sotf-daw: preloaded sound assets, cue playback, mixer, device lifecycle. The game controller emits cues such as place/error/rotate/lock/clear/win through an optional adapter. IDs deduplicate retried cues. No audio processing or loading on the UI frame thread.

Exclude ECS, physics, collision engines, camera worlds, scripting engines, scene editors, custom shader APIs, networking, and a separate renderer. General particles, sprite sheets, gamepads, and haptics can wait for an actual consumer.

## Delivery order and review gates

1. **Prove the platform path and measure baseline.** Define an on-device test harness for the same surface API. If the Python demo itself must ship on phones, prove embedded Python packaging plus in-process transport on both iOS and Android; the desktop child-process launcher is insufficient. Use the same drawing/input APIs for a native harness while that packaging work is assessed, but do not count it as proof of Python-on-mobile delivery. Instrument frame time, input-to-visible latency, queue depth, paint backend, bytes/patch, and resource counts.
2. **First vertical slice: Scene2D + Sudoku.** Typed schema, retained native painter, logical coordinate fitting/hit testing, keyboard selection/digits, mobile tap/keypad, accessibility. Demonstrate a crisp playable board after resize/rotation with no visual cell buttons.
3. **Direct contact + Zip.** Independent contact events, capture/cancel, gesture ownership, ordered drag traversal, nonblocking transport, serial reducer. Verify vertical dragging inside a scrollable shell and continuation outside board bounds.
4. **Native transitions + Queens/Sudoku polish.** Reuse easing/springs/keyframes, add interruption/removal/reduced-motion handling, and cache static work. Show crown placement, path reveal, selection/conflict effects, and solver reveal without per-frame Python patches.
5. **Frame scheduling + Tetris.** Replace glyph well and sleep loop, add held/repeat input, two-finger controls, lifecycle pause/reset, ghost/preview, and lock/clear transitions.
6. **Audio adapter and device QA.** Integrate the sotf-daw cue boundary; verify preload/play/pause behavior independently. Complete full-demo desktop and on-device mobile evidence, and mobile Python packaging if part of shipping scope.

Dependencies: steps 2/3 establish rendering and input; step 4 depends on retained object identity and frame clocks; step 5 depends on ordered input/scheduling. Shared queue safety belongs with step 3 rather than deferred cosmetic QA. Mobile packaging is an early feasibility gate.

## Acceptance targets (proposed, not measured results)

- All four games use drawn boards/pieces; game rules and existing logic tests remain valid.
- Focused desktop keyboard gameplay and mouse interaction; phone tap/drag/hold; no accidental page scrolling during owned gameplay gestures; other app scrolling remains usable.
- Two-finger move+rotate works; cancel, focus loss, pause, rotation, removal, suspension, and disconnect cannot leave input stuck.
- Paint and hit coordinates agree after resize, display scale changes, safe-area changes, and orientation changes.
- Choose named desktop and midrange iOS/Android devices. Target active frame time p95 <=16.7ms on 60Hz devices, host surface update/paint CPU p95 <=4ms, and instrumented input-to-present p95 <=50ms. At 120Hz measure against 8.3ms if that refresh rate is claimed. Record renderer/backend and build mode; CPU fallback has separately measured limits.
- Inactive boards cause no recurring animation work. A slow/stalled Python controller cannot block native UI rendering. Queue lengths/bytes stay bounded.
- Ten-minute play/reset/section-switch soak shows stable retained-resource counts and no growing backlog.
- Tests cover semantic input ordering, transforms/hit tests, crossed-cell drags, pointer cancellation/capture, multi-contact controls, repeat timing, fixed-step pause/resume, animation retargeting/removal, and malformed/stale scene updates. Reuse deterministic puzzle tests.
- Golden images for every game in light/dark, portrait/landscape, and small/large display sizes; on-device tests for platform gesture behavior and timing. Unit tests alone do not establish visual or touch quality.
- Add capability negotiation, Python typing/schema tests, and update the audited Python surface registry/rustdoc inventory when public APIs change. Run relevant narrow crate/tests and the repository's Python surface freshness gate before landing each slice.

The smallest useful first milestone is a polished Sudoku board with focused keyboard and finger selection. Zip then proves continuous input, and Tetris proves timing and multi-contact gameplay.


## Accepted delivery scope

Implementation follows Gitea issue #1. The user accepted native Rust mobile apps using the same Scene2D surface, with Python games on desktop. Mobile delivery is verified on the iOS simulator and Android emulator. Physical-device input latency and simultaneous finger operation remain separate, unmeasured checks.
