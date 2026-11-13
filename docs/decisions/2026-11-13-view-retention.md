# Decision: keep default-on view/Taffy retention (retroactive gate)

Date: 2026-11-13
Step: audit-20261113 §9 step 11 (closes §8 of the 2026-10-18 audit)

## Context

The 2026-10-18 audit proposed a fork-level retained-Taffy *prototype*
behind a 20 % decision gate. Instead, the November "gpui-fast" fork
port shipped retained views, Taffy nodes, and text measurements as the
default (`GPUI_VIEW_RETENTION=0` opts out) without the recorded gate
or an invalidation audit. This record is that gate, run retroactively.

## Paired evidence

`qa/perf/20261107-retained-frames-paired.json` (harness:
`crates/gpui-showcase/benches/retained_frames.rs`) measures warm
frames with retention on vs off (`Window::set_view_retention(false)`
is upstream GPUI's drawing model on the same build):

| Scenario | Retained | Scratch | Speedup |
|---|---:|---:|---:|
| 60 panels, one notified/frame | 179 µs | 428 µs | 2.39× |
| Table refresh | 523 µs | 1272 µs | 2.43× |

Both clear the audit's proposed 20 % threshold by a wide margin on
the workloads the toolkit actually ships (panel grids, tables).

Mechanism-level evidence is now asserted, not printed:
`crates/gpui-showcase/tests/retention_stats.rs` fails if a one-panel
change stops reusing layout nodes/views or if scratch mode stops
being distinguishable (view replay and style-compare counters via
`Window::layout_stats()`).

Matrix rows still unmeasured here: 100/1k/10k-node scaling, DPI/font
change storms, and device energy. The frame-time rows above plus the
stats gate are the recorded basis; energy remains a physical-device
item (audit §10).

## Production staleness audit

Retention replays a view's previous element tree when GPUI believes
nothing it read changed. Views that read untracked state
(interior-mutable cells, globals, clocks) during `render` can go
stale. Inventory of the in-repo views at this revision:

| View | Untracked state read in render | Verdict |
|---|---|---|
| `SpinoramaApp` | `contour_geometry_cache: Rc<RefCell<..>>` written during render | Keyed by `(contour_data_revision, mode)`; the revision is bumped at every data replacement site in `spinorama_app.rs`, so a hit implies identical inputs. Safe. (Cache-hit work order fixed alongside: the generator is no longer built on hits.) |
| `SpinoramaApp` | `*_chart_bounds: Rc<RefCell<Option<Bounds>>>`, `surface_state: Rc<RefCell<Surface3DState>>` | Bounds are written by layout callbacks and read by interaction handlers, not render output. Surface state changes go through event handlers that notify. Safe. |
| `Showcase` | No `RefCell` view state; sections are entities notified explicitly | Safe. The one known retention breakage (element-focus registry, `34e67181`) was a clean-window prepaint skip in the fork, worked around in the affected test by `window.refresh()` after focus changes. Any new focus-registration code must do the same until the fork registers focusable elements on retained frames. |
| `gpui-px` charts | `ChartInteraction: Rc<RefCell<..>>`, `MeshSceneState: Rc<RefCell<..>>` | Mutated by input handlers that then notify the owning view; render reads are consistent within a frame. Safe as long as every external mutation path notifies — chart builders that mutate shared state without notifying the view will go stale under retention (previously they were merely racy). |
| Theme/design globals | `ThemeState`/`DesignSystemState` read via `cx.global` in render | GPUI tracks global reads for invalidation in this fork generation; theme switches in the showcase/spinorama harnesses use `set_global` + notify paths exercised by their interaction tests. No stale instance found. |

No production `set_view_retention(false)` opt-outs are required today.

## Builder vs Taffy geometry ownership

`gpui-builder` contains no Taffy dependency (grep: 0 hits): it owns
*policy* (priority collapse, display tiers, auto-axis) and emits
ordinary GPUI elements with sizes; Taffy inside GPUI remains the
single geometry solver. Retained frames therefore cannot make the
two compute the same geometry twice — the builder solver runs only
when its owning view re-renders, and its own `RetainedLayoutSolver`
reuse is covered by its solver tests. No low-level Taffy adapter is
needed; revisit only if builder profiling shows duplicated work.

## Decision

**Keep retention default-on.** The gate that should have preceded it
now exists: paired numbers above threshold, an asserting stats test
in CI, and the staleness inventory above with no required opt-outs.
New views must either keep render a pure function of tracked state
(entity fields, globals) or key any render-time cache by every input
it reads, like `contour_geometry_cache`.
