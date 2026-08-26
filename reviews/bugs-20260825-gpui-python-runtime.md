# Bug Review: gpui-python-runtime — 2026-08-25

Scope: full read of `crates/gpui-python-runtime` — `src/` (~13k lines across 15 modules plus the `scene3d/` submodule), the five `bin/` targets (including the 13k-line `bin/showcase/python_ir_showcase.rs` host), the Python package `python/gpui_toolkit/` (28 modules, ~7k lines), the capability registry (`python-surface.toml`, `scripts/generate_python_capabilities.py`, `scripts/check_python_surface.py`), packaging metadata, and CI wiring. Emphasis on bugs, performance, threading/session correctness, GPU data flow (the crate reaches wgpu through `gpui-d3rs`), and UI/UX consistency between the Python-facing surface and the wrapped crates.

Verification run for this review: `cargo test -p gpui-python-runtime --lib` (135 passed), `cargo test -p gpui-python-runtime --features showcase --lib` (159 passed), `cargo test -p gpui-python-runtime --features showcase --bins` (37 passed, **1 failed** — see L1), `python3 scripts/check_python_surface.py --strict` (exit 0, 53 capabilities, 21 crates). The patch-protocol finding (H1) was reproduced empirically with a scratch crate under `tmp/` driving `apply_patch` against nodes nested in dialog/tooltip/accordion containers (scratch removed afterwards).

## Findings

Ranked by severity: 0 Critical, 1 High, 4 Medium, 8 Low.

### High

**H1 — Patch ops cannot reach nodes nested in non-`children` containers.**
`crates/gpui-python-runtime/src/ui_ir.rs:203` (`find_node_mut`) and `ui_ir.rs:551` (`remove_child`) only recurse into `"children"` arrays. Nodes that live inside other container fields — dialog `content`/`footer`, popover `trigger`/`content`, tooltip `child`, accordion `items[].children`, empty_state `action` — are invisible to `Set`/`Replace`/`Insert`/`Remove`/`Reorder` and to every mesh-plot patch op; they all fail with `UnknownNodeId`. Reproduced directly: `Set` on a text node inside a dialog's `content`, inside a tooltip's `child`, and inside an accordion item all return `Err(UnknownNodeId)`.

This is worse than a missing feature because validation disagrees with patching: `child_contains_id` (`ui_ir.rs:791`) *does* traverse those containers, so a Python app that references such a node id validates cleanly and then cannot be patched at runtime — the failure only appears when the first patch arrives.

Fix: extend `find_node_mut`/`remove_child` to descend into the same container fields `child_contains_id` walks (`content`, `footer`, `trigger`, `child`, `action`, and `items[]` arrays), so the patch protocol and validation agree. Related protocol limitation (Low): top-level sections themselves cannot be added/removed/reordered by patch, because lookup only starts at section content — worth documenting or lifting at the same time.

### Medium

**M1 — Python/Rust version drift, and no CI gate that would catch it.**
Workspace `Cargo.toml:51` is at `version = "0.9.11"`, but `crates/gpui-python-runtime/pyproject.toml:7` and `python/gpui_toolkit/__init__.py:13` still say `0.9.7`, and `dist/gpui_toolkit-0.9.7-py3-none-any.whl` is a stale build. The crate README (lines 107–109) states the versions must match. Neither `.github/workflows/python-package.yml` nor `.github/workflows/ci.yml` checks parity, and neither runs `scripts/check_python_surface.py --strict` or the Python test suite — so drift and a red bin test (L1) both reached HEAD unnoticed.

Fix: bump the two Python-side versions to 0.9.11, rebuild the wheel, and add a parity check plus `check_python_surface.py --strict` to `python-package.yml`.

**M2 — Per-render full re-validation and re-serialization of MeshPlot specs in the host.**
`bin/showcase/python_ir_showcase.rs:10732` (`render_meshplot`) runs, on every render, `MeshPlotSpec::from_value(node.spec.clone())` (a full JSON clone plus validation), `native_mesh_plot::options`, and `build_native_mesh_plot` → `native_mesh_plot::build` (`src/native_mesh_plot.rs:611`). That path recomputes `structural_fingerprint(geometry)` — which serializes the entire geometry JSON (`src/cache.rs:264`) — and projects all positions O(n) even when a retained state already exists: bounds are computed *before* the retained-state check (`native_mesh_plot.rs:688-712`). `mesh_plots.upsert(spec.clone())` re-fingerprints per render as well. For inline (non-resource) plots with large geometry this is multi-MB of serialization and projection work per frame.

Fix: key the whole pipeline on `(id, revision, content fingerprint)` and skip the rebuild entirely when unchanged; only compute bounds when creating fresh state.

**M3 — Per-patch whole-app JSON round trips.**
`apply_patch_message` (`bin/showcase/python_ir_showcase.rs:12866`) serializes the entire app IR to a `serde_json::Value` for each patch (`serde_json::to_value` near line 12881 — required for the transactional rollback, which is fine), but `record_mesh_patch_error` (6283), `clear_mesh_patch_errors` (6306), and `record_mesh_resource_error` (6322) each call `serde_json::to_value(app)` *again* for bookkeeping. That is O(app size) several times per patch.

Fix: compute the `Value` once in `apply_patch_message` and pass it (or an already-serialized error record) down to the bookkeeping helpers.

**M4 — Capability/surface check and Python tests are not wired into CI.**
`scripts/check_python_surface.py --strict` passes today (53 capabilities, 21 crates), but nothing runs it in CI, so the manifest↔code parity it enforces can silently rot. Same for the pytest suite under `crates/gpui-python-runtime/python/`. This is the process root cause behind M1 and L1 and is called out separately because the fix is one workflow step, not a code change.

### Low

**L1 — A committed bin test fails: assertion message drift.**
`cargo test -p gpui-python-runtime --features showcase --bins` fails in `showcase::python_ir_showcase::mesh_resource_decode_tests::resource_backed_infinity_is_rejected_even_when_nan_masking_is_requested` (`bin/showcase/python_ir_showcase.rs:2852`). The behavior is still correct — infinity in a field resource is rejected at `src/mesh_frames.rs:343` — but the error text is `"field resource contains non-finite value"` while the test asserts `"field resource contains a non-finite value"` (a stray "a"). Either the message or the assertion needs a one-word fix; until then the full showcase-feature test run is red.

**L2 — `superseded_requests` grows unboundedly.**
`bin/showcase/python_ir_showcase.rs:6074` — the `HashSet<String>` is inserted into (13033) and queried (12873, 13027) but never pruned, so long sessions accumulate one entry per superseded patch request. Fix: clear it on session reset and/or drop entries once the corresponding revision is consumed.

**L3 — Window-size persistence is effectively dead for default-sized apps.**
`bin/showcase.rs:71` (`miniapp_config`) reads `app.width`/`app.height`, which serde-default to 1240/820 (`ui_ir.rs:2928` `default_width`, `:2936` `default_height`). The `else { saved.width }` fallback therefore only fires if Python explicitly sends non-positive dimensions; a saved `PresentationState` size is ignored on relaunch for every app that didn't set an explicit size. Fix: make width/height `Option<f32>` in the IR, or treat the default sentinel as "unspecified".

**L4 — Chart `color_scale` silently falls back to Viridis; meshplot validation and native renderer disagree.**
`src/showcase.rs:22` maps any unrecognized chart `color_scale` string — including `"turbo"`, `"cividis"`, and typos — to Viridis, and `ChartNode::validate` (`ui_ir.rs:~2497`) never checks the field, so misspellings render with the wrong colors and no error. Meshplot is stricter but inconsistent in the other direction: `meshplot.rs:256` validation *accepts* `"cividis"`/`"turbo"`, which `native_mesh_plot::options` then rejects at build time (`src/native_mesh_plot.rs:398`) — a two-phase accept/reject wart. Fix: validate chart `color_scale` in `ui_ir.rs`, and align the meshplot accepted set with what the native renderer actually supports.

**L5 — GPU uploads expand indexed meshes into per-triangle vertex soup.**
`mesh_gpu_upload` (`src/gpui_adapter.rs:357`) and `scene_gpu_upload` (`:393`) duplicate every vertex per triangle (positions ×3) and emit a trivial sequential index buffer `0,1,2,…`; the scene wireframe path also pushes 3 edges per triangle so shared edges are drawn twice. Memory and bandwidth are ~3× what the source geometry needs. Fix: where association is per-vertex, upload shared vertices with the original index buffer; dedupe wireframe edges.

**L6 — Scene-embedded surfaces ignore log axes and labels.**
The `Surface` branch of `scene_gpu_upload` (`src/gpui_adapter.rs:448-528`) tessellates grids on CPU and ignores `x_log`/`y_log`/`z_log` and axis labels, while a standalone `surface` node goes through `Surface3DElement` (`gpui_adapter.rs:660`) which applies log scaling. The same surface spec renders differently depending on whether it sits inside a composed `SceneSpec`. Fix: route the scene branch through the same transform, or reject log axes in composed scenes with a clear error.

**L7 — Global cache lock and clone in `default_axis_values`; unchecked size multiplies.**
`src/scene3d/surface_spec.rs:21-29` uses a `OnceLock<Mutex<HashMap>>` with `lock().unwrap()` (a poisoned mutex would panic in the render path) and clones the whole `Vec` per call. Minor: use `Arc` or a once-per-size value. Also `GridData::validate` (`src/scene3d/grid_data.rs`) and the heatmap check (`ui_ir.rs:2691`) compute `width * height` as unchecked `usize` multiplies — only theoretical overflow given the 4 MB message caps, but `checked_mul` is cheap hardening.

**L8 — Dirty check clones the full color buffer.**
`gpui_adapter.rs:244` compares `current.vertex_colors != Some(vertex_colors.clone())`, cloning the entire color `Vec` on every rebuild just to test for change. Compare with `as_deref()`/slice equality instead.

## Python interface coverage

The Python package (`python/gpui_toolkit/`, 28 modules) covers the wrapped crates as follows:

- **App/session IR** — `app.py` (App, SessionContext, section), `state.py`, `effects.py`, `events.py`, `commands.py`, `i18n.py`, `accessibility.py`, `keybindings.py`, `miniapp.py`, `platform.py`: good coverage of the session protocol itself.
- **UI nodes** — `ui.py` (52 top-level defs, ~45 node constructors): layout (vstack/hstack/wrap), heading, text, code, card, form, button, badge, metric, progress, spinner, thinking_orb, breadcrumbs, alert, toast, tooltip, empty_state, dialog, confirm_dialog, menu/menu_bar/context_menu, popover, tabs, stepper, accordion, list_editor, table, all form inputs (text/number/slider/select/color_picker/path_input/checkbox/toggle), divider, spacer, scene3d, mesh_plot.
- **Charts** — `charts.py`: Chart plus scatter/line/bar/heatmap/area/boxplot/contour/isoline/pie/donut/treemap and reports.
- **3D** — `scene3d.py`: OrbitCamera, PerspectiveCamera, Material, Surface, LineStrip, Lines, Mesh, Light, Scene. `meshplot.py`: MeshGeometry, MeshScalarField, MeshRevolve, MeshPlotSpec, resource handles — good `gpui-px::mesh_plot` coverage.
- **d3rs** — `d3.py`: typed bindings for Zoom, Array, Statistics, Tick, Scale, plus a generic `AlgorithmRequest` bridge.
- **Layout solver** — `layout.py`: full builder solver surface (solve/solve_matrix/solve_chassis), the deepest binding in the package.
- **Design/text/themes** — `design.py` (48 defs), `text.py` (31), `themes.py` (14).
- **Tooling** — `audio.py`, `profiler.py`, `lab.py`, `scaffolder.py`, `tooling.py`, `reports.py`, `resources.py`, `capabilities.py` (generated from `python-surface.toml`; parity enforced by `scripts/check_python_surface.py`, currently green but not in CI — M4).

Concrete gaps (public modules of wrapped crates with no typed Python binding):

- **gpui-ui-kit**: avatar, button_set, icon_button, image_view, keyboard_shortcut_label, qr, search_bar, step_indicator (Python `stepper` is hand-rolled divs in the host at `python_ir_showcase.rs:9313`, *not* ui-kit's StepIndicator — see UI/UX notes), wizard, loading_overlay, pane_divider, settings_form, sidebar, split_pane, status_bar, collection_diff, data_navigation, adaptive_overflow, mobile, security_surface, plus newer modules (command_palette, drag_list, notification, plot_toolbar, tag, toolbar, tree_view, workflow, swipe_panel).
- **gpui-px**: `StaticSvgOptions`/static export is reachable only via the host commands `chart.export_svg`/`chart.export_png` (no typed Python API); `Surface3DChart` is reachable through scene3d surfaces (acceptable indirection).
- **gpui-d3rs**: of 43 public modules, only array/zoom/scale/statistics/tick have typed bindings; the rest (geo, force, sankey, hierarchy, chord, delaunay, hexbin, contour, brush, drag, ease, format, interpolate, polygon, quadtree, random, tile, time, timer, transition, gpu2d, gpu3d, sphere_gallery, vello2d, …) are reachable only through the generic `AlgorithmRequest` bridge or not at all.
- **gpui-pretext**: prepare/analyze reachable via commands; no declarative text-layout node beyond `ui.text`/`ui.code`.
- Deliberate dispositions recorded in the manifest (not bugs): gpui-au, gpui-ios, gpui-android are platform-unavailable; gpui-ui-kit-macros and gpui-hello-web are non-consumer crates.

## GPU/CPU data-flow notes

The crate itself contains no direct wgpu calls (no `device.poll`/`pollster`/`map_async` anywhere — verified by grep), and no CPU→GPU→CPU readback cycles; all GPU work is delegated to `gpui-d3rs`/`gpui-px` elements. The data-flow concerns are on the CPU side of the upload boundary:

- Scene surfaces and meshes are tessellated/expanded on CPU into vertex soup before upload (L5, L6). The height-field case could stay an indexed grid (or a GPU-side height texture) and the wireframe could dedupe shared edges; today every geometry change re-expands and re-uploads ~3× the necessary bytes.
- The MeshPlot host path re-serializes and re-projects full geometry per render even when retained GPU state exists (M2) — the fingerprint/bounds work should be gated on an actual content change, not done unconditionally before the retained-state check.
- No blocking-poll hazards for a future wasm target were found in this crate; the only `std::thread::sleep` is in the profiler subscription background thread, which has proper unsubscribe/shutdown cleanup.

## UI/UX consistency

- **Stepper divergence**: Python `stepper` renders as hand-rolled divs in the host (`python_ir_showcase.rs:9313`) instead of gpui-ui-kit's `StepIndicator`, so it drifts from the design system's styling, sizing, and accessibility behavior. Either bind `StepIndicator` or document the intentional divergence.
- **Silent color fallback**: chart `color_scale` typos render as Viridis with no error (L4), so a Python user sees plausible-but-wrong colors rather than a validation message — the worst failure mode for a plotting API.
- **Two-phase meshplot validation**: `color_scale` values accepted by spec validation are later rejected by the native renderer (L4), moving a static error to render time.
- **Scene vs. standalone surface**: the same surface spec renders with different axis semantics (log scaling, labels) depending on composition context (L6) — visible inconsistency across two documented ways to draw the same thing.

## Clean bill

Areas reviewed and found sound:

- **Session protocol** (`src/session.rs`): JobRegistry state machine is idempotent; revision/generation staleness handling is well tested.
- **Python bridge** (`bin/showcase/python.rs`): stdout/stderr drained on dedicated threads, `sync_channel(256)` backpressure documented, the wake future uses the correct re-check pattern, binary framing for audio/mesh frames validates sizes and stays stream-synchronized (test-covered), and shutdown has a 2 s timeout before kill.
- **Host state persistence** (`src/host_state.rs`): coalesced writes on a dedicated thread; no lock is held across blocking I/O.
- **Mesh frame store** (`src/mesh_frames.rs`): bounded budget, FIFO eviction, reference counting, and recoverable generations, all well tested; non-finite rejection works (the L1 failure is assertion-text drift, not behavior).
- **Spec cache** (`src/cache.rs`) and the production `unwrap`/`expect`/`unreachable!` sites: all genuinely infallible (serialization of known-good structs, `unreachable!` after dtype checks).
- **Lib test suites**: 135 tests (default) and 159 tests (`--features showcase`) pass; `check_python_surface.py --strict` passes with 53 capabilities across 21 crates.
- No data races, deadlocks, or GPU readback hazards were found in the crate's own code.

## Resolution status

- [x] L4 (chart palette contract). **Unsupported chart color scales** (2026-08-26): UI-IR validation now rejects chart `color_scale` values outside the native `gpui-px::ColorScale` set, including misspellings and `turbo`, before rendering can silently fall back to Viridis. The accepted aliases remain case-, hyphen-, and underscore-insensitive. Verified `cargo test -p gpui-python-runtime --lib rejects_invalid_chart_shapes_annotations_and_options`.

- [x] H1. **Nested UI-IR patch targets** (2026-08-26): mutable node lookup now traverses all JSON object fields and arrays, so dialog, popover, tooltip, accordion, and other non-`children` containers are patchable. Verified by `cargo test -p gpui-python-runtime --lib ui_ir` (35 passed), including `nested_form_targets_cover_every_retained_container_and_control`.
- [x] M1. **Python/Rust version parity** (2026-08-26): package metadata and `gpui_toolkit.__version__` now match the workspace `0.9.11` release. The packaging workflow validates both values against `[workspace.package]` and runs `check_python_surface.py --strict`; the parity assertion and strict checker both pass locally.
- [x] M2. **MeshPlot render preparation** (2026-08-26): the showcase caches the parsed specification and prepared mesh/field data by immutable UI-IR node identity, so unchanged renders skip the JSON clone/validation, geometry and field fingerprints/decoding, resource-ref synchronization, and `MeshPlotRegistry` upsert. Preparation now owns the scalar field as well as geometry, avoiding a second field decode for diagnostics; retained-state bounds are calculated only for a new plot state. Changed specs still run stale-revision and resource validation before preparation. Verified by the showcase-bin suite (38 passed) and native MeshPlot tests (10 passed).
- [x] M4. **Python surface and declaration-test CI coverage** (2026-08-26): every wheel-build matrix job now runs the strict Python surface check and the complete `crates/gpui-python-runtime/python/tests` unittest discovery before building artifacts. The workflow YAML parses, and the strict surface check and version-parity command pass locally. (The local global editable package metadata is stale at `0.9.7`, so its installed-distribution assertion fails outside a clean CI environment; the workflow runs before any package installation.)
- [x] M3. **Patch-error JSON bookkeeping** (2026-08-26): a patch now serializes the committed app only once and reuses that `Value` for patching, resource validation, and MeshPlot error aliases. Accepted snapshots and patches retain that JSON value for later resource errors; the lazy fallback exists only for direct host/test construction. Focused patch-transaction and resource-recovery regressions pass.
- [x] L1. **Showcase-bin assertion drift** (2026-08-26): corrected the stale expected error text for resource-backed infinity validation. Verified by the exact showcase-bin regression (1 passed).
- [x] L2. **Unbounded superseded-request set** (2026-08-26): matching patch and rejection messages now consume their superseded request ID, bounding retained entries to requests still in flight. Verified by `cargo check -p gpui-python-runtime --features showcase --bin gpui-python-showcase`.
- [x] L3. **Default window-size persistence** (2026-08-26): app IR dimensions are optional rather than eagerly defaulted. Unspecified Python apps now omit both fields and the miniapp shell restores the saved presentation size; explicit finite, positive dimensions remain authoritative. Verified by the focused Rust window-size test and the Python miniapp test suite (4 passed).
- [x] L7 (dimension-overflow portion). **Grid size arithmetic** (2026-08-26): scene-grid and heatmap validation now use checked multiplication, returning the normal dimension-mismatch result instead of overflowing in debug builds; construction no longer preallocates from an unchecked product. Verified by `cargo test -p gpui-python-runtime --lib grid_data`.
- [x] L7 (axis-cache portion). **Default surface axes** (2026-08-26): cached axes are immutable process-lifetime slices behind a poison-tolerant `RwLock`; implicit `x_values`/`y_values` borrow those slices through `Cow` rather than cloning a `Vec` for every render. Verified `cargo test -p gpui-python-runtime --lib scene3d::tests::surface_default_axis_values_are_cached`.
- [x] L8. **Color-buffer dirty check** (2026-08-26): scene GPU state compares retained colors with the incoming slice, avoiding a full clone before determining whether an upload is needed. Verified by the full Python-runtime library suite (135 passed).
- [x] L4 (MeshPlot palette contract). **Accepted color scales** (2026-08-26): MeshPlot validation now rejects palettes that `gpui-px::ColorScale` cannot render, including `cividis` and `turbo`, rather than accepting them and silently using Viridis. Verified by focused MeshPlot validation tests (14 passed).
- [x] L5. **Indexed mesh uploads and scene wireframes** (2026-08-26): standalone meshes and unlit scene meshes with uniform or vertex-associated fields now retain their original vertex/index topology and a matching per-vertex color buffer. Cell-associated fields and lit scene meshes deliberately retain flat per-face expansion because their colors are face-dependent. Scene-surface wireframes now emit each shared grid edge once. Verified by 15 showcase-feature GPUI-adapter tests, including the vertex/cell topology and wireframe regressions.
- [x] L6. **Composed-surface axis semantics** (2026-08-26): composed scene rendering still does not own standalone-surface axes or labels, so `SceneNode::validate` now rejects surface log axes and any axis/title labels with a clear `UnsupportedNode` error instead of rendering silently different semantics. The standalone `Surface3DElement` remains the supported route for those features. Verified by focused label and positive-log-axis scene validation regressions.
