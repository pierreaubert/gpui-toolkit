# GPUI Toolkit Requirements for Python Simulation Applications

**Status:** Draft requirements  
**Date:** 2026-08-06  
**Primary consumer:** Sonium Speaker Studio  
**Toolkit surface:** `gpui-python-runtime`, `gpui-ui-kit`, `gpui-px`, and retained 3D rendering

## 1. Purpose

This document defines the gpui-toolkit capabilities required to build Sonium
Speaker Studio as a fully interactive, Python-authored native application.
Speaker Studio configures product BEM and coupled whole-speaker simulations,
runs them locally or on a remote machine, tracks long-running jobs, and
visualizes scientific results.

The current application can declare and render a native dashboard, but it must
use a JSON file and CLI commands for configuration and execution. The target is
an application in which users can edit, validate, run, monitor, cancel, and
review simulations without leaving the GPUI window.

The requirements are intentionally domain-neutral. Implementing them should
also enable Python-authored optimization, measurement, CAD, data-analysis, and
batch-processing applications.

Normative terms **MUST**, **SHOULD**, and **MAY** have their usual RFC 2119
meanings.

## 2. Product outcome

The toolkit is sufficient when a Python application can:

1. Render a native, responsive form from typed Python declarations.
2. Receive control events and validate edits in Python.
3. Start local or remote work without blocking the GPUI event loop.
4. Stream structured progress, status, and bounded logs into the window.
5. Cancel a running job and distinguish cancellation from failure.
6. Incrementally replace charts, tables, metrics, and 3D data when results
   arrive.
7. Preserve application state and job history across restarts.
8. Run as an installed application without a gpui-toolkit source checkout or
   `cargo run`.

## 3. Current baseline and gaps

The baseline was verified against the current gpui-toolkit checkout on
2026-08-06.

| Area | Current capability | Gap for Speaker Studio |
|---|---|---|
| Python application model | Python emits `PythonAppIr` schema v1 as one JSON document. Rust parses it in a background thread. | Python exits after emitting the initial snapshot. There is no persistent session, event channel, or subsequent update. |
| Layout and display | Stacks, wrapping, headings, text, code, cards, badges, metrics, progress, spinner, tabs, tables, charts, and Scene3D nodes. | Most nodes are presentation-only. They cannot be bound to mutable Python state. |
| Buttons | A button carries an optional string `action`. | The host only implements the `select:<section>` prefix. Other action strings do nothing, and action payloads are untyped. |
| Form controls | None in Python IR v1. | Text, number, select, checkbox, toggle, slider, path, and collection editors are required. |
| Tabs and navigation | Tabs render an `active` index supplied in the snapshot. | Clicking a tab does not update Python or application state. |
| Progress and spinner | Static values render correctly. | No runtime progress or status stream exists. |
| Tables | Static string cells render correctly. | No selection, sorting, virtualization, incremental rows, typed cells, or row actions. |
| Charts | One `x`/`y` series, bar data, or heatmap data per chart; log-axis flags and basic styling. | Scientific work needs multiple named series, legends, axis metadata, inspection, zoom/pan, streaming replacement, and export. |
| 3D | Surface and line paths are rendered; mesh/material/camera/light specs validate. | Mesh rendering is explicitly not bound to a GPUI element. Scalar fields, selection, and large mesh updates are unavailable. |
| Python lifecycle | The Rust host starts a Python process, reads one JSON value, waits for exit, and renders it. | No supervision, heartbeat, restart, shutdown handshake, crash recovery, or stderr event stream. |
| Launch and packaging | `App.run()` shells out to `cargo run` after looking for a repository root. | Installed applications must find a bundled host binary without a toolkit checkout or Rust toolchain. |
| Compatibility | App IR and Scene3D schemas are versioned. | Python package metadata reports `0.8.2` while the Rust crate reports `0.9.4`; protocol feature negotiation is absent. |

Relevant current implementation surfaces:

- `crates/gpui-python-runtime/python/gpui_toolkit/app.py`
- `crates/gpui-python-runtime/python/gpui_toolkit/ui.py`
- `crates/gpui-python-runtime/python/gpui_toolkit/charts.py`
- `crates/gpui-python-runtime/src/ui_ir.rs`
- `crates/gpui-python-runtime/bin/showcase/python.rs`
- `crates/gpui-python-runtime/bin/showcase/python_ir_showcase.rs`

## 4. Ownership and architecture constraints

### 4.1 Separation of concerns

- Python application code MUST own domain state, validation, simulation
  configuration, job orchestration, and result normalization.
- Rust MUST own the GPUI event loop, native widgets, rendering, focus,
  accessibility, window lifecycle, and retained GPU resources.
- gpui-toolkit MUST provide transport and lifecycle primitives, but MUST NOT
  implement Sonium-specific solver or SSH business logic.
- The UI protocol MUST carry structured events and data. It MUST NOT treat
  arbitrary shell commands embedded in action strings as callbacks.

### 4.2 Protocol layering

The snapshot schema and the live-session protocol SHOULD be versioned
independently:

- `python_app_ir`: declarative UI tree and initial values;
- `python_app_session`: events, patches, effects, jobs, errors, and lifecycle.

An IR schema change must not be required merely to add an optional session
message. Both peers MUST negotiate supported schema and session versions during
initialization and fail with a user-readable compatibility error.

### 4.3 State model

Every interactive node MUST have a stable application-defined ID. Widget-local
transient state such as focus, selection, hover, and camera position remains in
Rust. Application state such as field values, active study, validation errors,
and job status remains authoritative in Python.

## 5. Functional requirements

Priorities are:

- **P0:** required for a usable simulation application;
- **P1:** required for a complete scientific desktop experience;
- **P2:** valuable follow-up capability.

### 5.1 Persistent Python session

| ID | Priority | Requirement |
|---|---:|---|
| PY-SESSION-001 | P0 | The host MUST support a persistent Python process for the lifetime of the application window. |
| PY-SESSION-002 | P0 | The host MUST perform an initialization handshake that includes protocol versions, toolkit capabilities, platform, theme, and initial window metadata. |
| PY-SESSION-003 | P0 | The host MUST send structured UI events to Python and accept snapshots or incremental patches in response. |
| PY-SESSION-004 | P0 | Messages MUST have request/event IDs so responses, errors, cancellation, and late results can be correlated. |
| PY-SESSION-005 | P0 | The host MUST implement graceful `shutdown`; it SHOULD detect a hung child and terminate it after a configurable timeout. |
| PY-SESSION-006 | P0 | A Python crash or malformed message MUST produce a recoverable native error view with stderr diagnostics and a restart action. |
| PY-SESSION-007 | P1 | The protocol SHOULD support heartbeats and expose child-process health without polling from application code. |

The transport MAY be newline-delimited JSON over stdio initially. The contract
must permit a future binary or local-socket transport for large arrays without
changing the Python application model.

### 5.2 Incremental rendering and state updates

| ID | Priority | Requirement |
|---|---:|---|
| PY-PATCH-001 | P0 | Python MUST be able to set properties on a node by stable ID without replacing the entire app snapshot. |
| PY-PATCH-002 | P0 | Python MUST be able to insert, remove, replace, and reorder child nodes. |
| PY-PATCH-003 | P0 | Patches MUST preserve unaffected native widget state, including focus, scroll, selection, and 3D camera state. |
| PY-PATCH-004 | P0 | The host MUST reject unknown node IDs and invalid property types with structured errors rather than panicking. |
| PY-PATCH-005 | P1 | Multiple patches SHOULD be applied atomically as a transaction to avoid intermediate invalid layouts. |
| PY-PATCH-006 | P1 | High-volume data nodes SHOULD support resource references or shared buffers rather than embedding every array in the UI tree. |

### 5.3 Typed form controls

Python IR MUST expose native gpui-ui-kit controls with the following common
properties:

- stable `id`;
- current `value` and optional `default_value`;
- `label`, `help`, `placeholder`, and `unit`;
- `disabled`, `read_only`, `required`, and visibility state;
- validation state with severity and message;
- a structured change/commit action;
- explicit width or responsive sizing rules.

| ID | Priority | Requirement |
|---|---:|---|
| PY-FORM-001 | P0 | Text and password inputs MUST support edit, commit, focus, selection, placeholder, and validation events. Password values MUST never appear in debug IR or logs. |
| PY-FORM-002 | P0 | Number inputs MUST support integer/float modes, min/max, step, decimal precision, scientific notation, and unit labels. Invalid intermediate text MUST be representable while editing. |
| PY-FORM-003 | P0 | Select controls MUST support typed option values, labels, disabled options, and keyboard navigation. |
| PY-FORM-004 | P0 | Checkbox and toggle controls MUST support boolean and optional indeterminate state. |
| PY-FORM-005 | P0 | Buttons MUST emit structured click events with an action ID and optional serializable payload. |
| PY-FORM-006 | P0 | Tabs, accordions, and steppers MUST be interactive and bindable to Python state. |
| PY-FORM-007 | P1 | Sliders MUST support continuous preview and committed-value events so expensive recomputation is not triggered for every pointer movement. |
| PY-FORM-008 | P1 | Repeated/list editors SHOULD support add, remove, reorder, and per-row validation for frequency lists and evaluation points. |
| PY-FORM-009 | P1 | Form-level validation MUST support a summary and focus the first invalid control. |

### 5.4 Actions, callbacks, and effects

| ID | Priority | Requirement |
|---|---:|---|
| PY-ACTION-001 | P0 | Events MUST identify the node, event kind, action ID, payload, and monotonic sequence number. |
| PY-ACTION-002 | P0 | Python handlers MAY be synchronous or asynchronous. The UI MUST remain responsive while a handler runs. |
| PY-ACTION-003 | P0 | A handler MUST be able to acknowledge, reject, or supersede an event. This is required for validation and rapidly changing values. |
| PY-ACTION-004 | P0 | The host MUST expose typed effects for notifications, confirmation dialogs, clipboard operations, opening a URL, file dialogs, and window close. |
| PY-ACTION-005 | P0 | Effects requiring user consent MUST only execute in direct response to an application request and MUST return a result to Python. |
| PY-ACTION-006 | P1 | Applications SHOULD be able to register keyboard commands and menus using action IDs without exposing Rust closures to Python. |

### 5.5 Long-running jobs

| ID | Priority | Requirement |
|---|---:|---|
| PY-JOB-001 | P0 | The runtime MUST provide a task API that executes Python handlers or child processes without blocking the render thread. |
| PY-JOB-002 | P0 | Jobs MUST expose `queued`, `running`, `cancelling`, `cancelled`, `succeeded`, and `failed` states. |
| PY-JOB-003 | P0 | Jobs MUST support determinate progress (`completed`, `total`, optional unit) and indeterminate progress. |
| PY-JOB-004 | P0 | Jobs MUST stream structured status messages and bounded stdout/stderr or application log records. |
| PY-JOB-005 | P0 | Users MUST be able to request cancellation. Python MUST receive a cancellation token/event and report whether cancellation completed. |
| PY-JOB-006 | P0 | Completion, cancellation, and failure MUST remain distinct terminal outcomes. Closing a window MUST NOT falsely mark a job successful. |
| PY-JOB-007 | P0 | A job result MUST be able to update metrics, tables, charts, and 3D resources without restarting the app. |
| PY-JOB-008 | P1 | The host SHOULD provide concurrency limits and resource tags so an application can serialize GPU jobs while allowing lightweight tasks. |
| PY-JOB-009 | P1 | Job state SHOULD be serializable so an application can restore history and reconnect to externally managed remote jobs. |

The toolkit does not need to understand SSH. Speaker Studio will implement SSH
and SCP in Python. The toolkit must provide responsive actions, secure inputs,
progress/log views, cancellation, and state updates around that orchestration.

### 5.6 Files, paths, and secrets

| ID | Priority | Requirement |
|---|---:|---|
| PY-FILE-001 | P0 | The runtime MUST expose native open-file, save-file, and directory dialogs with filters and optional initial location. |
| PY-FILE-002 | P0 | Dialog results MUST be returned as structured path values and cancellation MUST not be treated as an error. |
| PY-FILE-003 | P0 | A path input MUST support manual entry, browse, existence/type validation, and recent values. |
| PY-SECRET-001 | P0 | Passwords, tokens, and secret values MUST be redacted from protocol traces, logs, crash reports, and serialized snapshots. |
| PY-SECRET-002 | P1 | The runtime SHOULD offer platform credential-store effects and return opaque credential references to Python. |

### 5.7 Tables and logs

| ID | Priority | Requirement |
|---|---:|---|
| PY-TABLE-001 | P0 | Tables MUST accept typed cells, stable row IDs, selection, and row actions. |
| PY-TABLE-002 | P1 | Tables SHOULD support sorting, column sizing, pinned columns, and incremental row patches. |
| PY-TABLE-003 | P1 | Tables and log views MUST virtualize large datasets and avoid rebuilding all rows when one row changes. |
| PY-LOG-001 | P0 | A log view MUST support bounded retention, follow-tail, pause, copy, severity filtering, and clear/export actions. |

### 5.8 Scientific charts

| ID | Priority | Requirement |
|---|---:|---|
| PY-CHART-001 | P0 | A Cartesian chart MUST support multiple named series with independent color, line style, marker, visibility, and legend entry. |
| PY-CHART-002 | P0 | Axes MUST support title, unit, linear/log scale, explicit or automatic range, ticks, and shared-axis layout. |
| PY-CHART-003 | P0 | Charts MUST support replacing or appending series data by stable chart/series ID. |
| PY-CHART-004 | P1 | Charts SHOULD support pointer inspection, nearest-point tooltip, crosshair, zoom, pan, reset, and series visibility toggles. |
| PY-CHART-005 | P1 | Heatmaps MUST support explicit x/y coordinates, colorbar title and unit, color range, missing values, and aspect control. |
| PY-CHART-006 | P1 | Charts SHOULD export at least SVG and PNG and SHOULD expose the displayed data for CSV export. |
| PY-CHART-007 | P1 | The data contract MUST define behavior for `NaN`, infinities, complex values, and mismatched lengths; invalid input must return a structured error. |
| PY-CHART-008 | P2 | Linked axes and selection across charts MAY be supported for response/impedance comparison workflows. |

### 5.9 Retained 3D visualization

| ID | Priority | Requirement |
|---|---:|---|
| PY-3D-001 | P1 | Validated mesh nodes MUST render with indexed triangles, materials, camera, and lights. |
| PY-3D-002 | P1 | Meshes and surfaces MUST support per-vertex or per-cell scalar fields, colormaps, range, and colorbar metadata. |
| PY-3D-003 | P1 | Geometry, scalar fields, material, and camera MUST be independently patchable so result changes do not reset the camera or re-upload unchanged geometry. |
| PY-3D-004 | P1 | Orbit, pan, zoom, reset, and fit-to-bounds interactions MUST work without Python round trips. |
| PY-3D-005 | P2 | Picking SHOULD return stable object/cell IDs to Python for inspection. |
| PY-3D-006 | P2 | Large resources MAY use memory mapping, shared memory, or another non-JSON transfer path. |

### 5.10 Persistence and lifecycle

| ID | Priority | Requirement |
|---|---:|---|
| PY-STATE-001 | P0 | The runtime MUST provide an application-specific writable data directory and expose it to Python. |
| PY-STATE-002 | P0 | Window size, section, scroll position, and other host-owned presentation state SHOULD be restored automatically. |
| PY-STATE-003 | P0 | Python MUST be able to save versioned application state atomically and handle migration failures without data loss. |
| PY-STATE-004 | P1 | The host SHOULD distinguish user-requested close from process failure and allow an application to confirm closure while jobs run. |

### 5.11 Distribution and versioning

| ID | Priority | Requirement |
|---|---:|---|
| PY-DIST-001 | P0 | `App.run()` MUST locate and launch an installed native host binary; it MUST NOT require a gpui-toolkit checkout or invoke `cargo run` in production. |
| PY-DIST-002 | P0 | The Python package and Rust crate versions MUST be released from one source of truth and tested for equality. |
| PY-DIST-003 | P0 | The package MUST publish supported Python versions and fail early with a clear message when the embedded/native boundary is incompatible. |
| PY-DIST-004 | P0 | The host MUST reject unsupported future IR/session versions before rendering or processing events. |
| PY-DIST-005 | P1 | macOS, Linux, and Windows packaging SHOULD include the native host and required renderer assets with a consistent Python API. |

## 6. Non-functional requirements

### 6.1 Responsiveness and performance

- The GPUI render thread MUST NOT wait for Python, solver work, filesystem I/O,
  SSH, or child-process output.
- Dispatching a local control event inside the host SHOULD take less than 16 ms
  at the 95th percentile, excluding Python handler time.
- A small Python event/patch round trip SHOULD appear in the next rendered frame
  and SHOULD complete within 100 ms under normal local load.
- Log and table views MUST remain responsive with at least 10,000 retained rows.
- Updating one metric or progress value MUST be proportional to the affected
  nodes, not the size of the complete application tree.
- Chart data transfers above a configurable threshold SHOULD use a compact or
  out-of-band representation instead of JSON decimal arrays.
- Resource caches MUST have observable size limits and eviction behavior.

### 6.2 Reliability

- Every protocol message MUST be validated before it mutates UI state.
- A malformed patch, chart, or 3D resource MUST produce a localized error and
  MUST NOT crash the host.
- The runtime MUST drain stdout and stderr concurrently to prevent child-process
  deadlocks.
- Cancellation and shutdown MUST be idempotent.
- Late events from superseded requests MUST not overwrite newer state.

### 6.3 Security

- The host MUST never evaluate Python-provided shell strings.
- Effects MUST use typed arguments and platform APIs or argument-vector process
  execution.
- Secret controls and credential references MUST be redacted by default.
- File and URL effects MUST preserve platform consent and sandbox behavior.
- Protocol tracing MUST be opt-in and document its treatment of application
  data.

### 6.4 Accessibility and interaction

- All controls MUST expose roles, labels, values, disabled state, validation
  state, and keyboard operation through the platform accessibility bridge.
- Focus order MUST follow visual order unless the application explicitly
  declares otherwise.
- Forms, job controls, tables, charts, and dialogs MUST be usable without a
  pointer.
- Long labels and validation messages MUST wrap or receive stable space at
  narrow widths.
- Theme and contrast MUST come from gpui-design/gpui-ui-kit rather than
  application-defined duplicate theme state.

### 6.5 Observability

- The runtime SHOULD expose opt-in metrics for Python round-trip latency, patch
  count/size, dropped or superseded events, renderer resource uploads, job
  transitions, and cache use.
- Errors MUST include the node/action/request ID and a concise remediation hint.

## 7. Reference Python API shape

The exact API may differ, but it must preserve typed declarations, stable IDs,
structured actions, and explicit state ownership. A representative target is:

```python
from gpui_toolkit import App, action, ui


class SpeakerStudio(App):
    def view(self):
        return ui.form(
            id="simulation-form",
            children=[
                ui.select(
                    id="study",
                    label="Study",
                    value=self.state.study,
                    options=[
                        ("product_bem", "Product BEM"),
                        ("whole_speaker", "Whole speaker"),
                    ],
                    on_change=action("set-study"),
                ),
                ui.number_input(
                    id="frequency-start",
                    label="Start frequency",
                    unit="Hz",
                    value=self.state.frequency_start_hz,
                    min=1.0,
                    on_commit=action("set-frequency-start"),
                ),
                ui.path_input(
                    id="speaker-model",
                    label="Speaker model",
                    value=self.state.model_path,
                    mode="open_file",
                    filters=[("Speaker models", ["mlg", "json"])],
                    on_change=action("set-model"),
                ),
                ui.button(
                    id="run",
                    label="Run simulation",
                    action=action("run-simulation"),
                    disabled=not self.state.is_valid,
                ),
            ],
        )

    async def on_action(self, event, context):
        if event.action == "run-simulation":
            job = context.jobs.start("speaker-simulation")
            await self.runner.run(self.state.config, job=job)
```

This example does not require Python callbacks to execute on the render thread.
`context.jobs` reports progress and cancellation through the session protocol,
and `view()` or explicit patches update the retained native UI.

## 8. Reference session messages

This is an illustrative contract, not a mandated wire format.

Host to Python:

```json
{"type":"initialize","session_version":1,"capabilities":["forms","jobs","patches"]}
{"type":"event","id":"evt-42","node_id":"run","event":"click","action":"run-simulation","payload":{}}
{"type":"cancel","request_id":"job-7"}
{"type":"shutdown","reason":"window_closed"}
```

Python to host:

```json
{"type":"snapshot","app_ir":{}}
{"type":"patch","revision":12,"ops":[{"op":"set","id":"job-progress","property":"value","value":0.4}]}
{"type":"job","id":"job-7","state":"running","completed":4,"total":10,"message":"504 Hz"}
{"type":"effect","request_id":"dialog-3","effect":"open_file","filters":["mlg","json"]}
{"type":"error","request_id":"evt-42","message":"Frequency end must exceed start"}
```

Messages need explicit maximum sizes, ordering rules, revision handling, and
structured error codes before the protocol is considered stable.

## 9. Speaker Studio acceptance scenarios

The P0 implementation is accepted when all of the following work in one native
application session:

1. **Configure product BEM:** The user selects an MLG/JSON model, edits start
   and end frequencies, frequency count, solver tolerance, evaluation distance,
   and optional field-map settings. Invalid values show inline errors and
   disable Run.
2. **Configure whole-speaker modeling:** The user enters an ordered frequency
   list, voltage, acoustic-feedback mode, 3D-sector option, impedance/excursion
   gates, and optional paired calibration files.
3. **Configure execution:** The user switches between local and remote mode.
   Remote host, port, work directory, Python executable, Sonium root, and
   credential reference are editable and independently validated.
4. **Run locally:** Clicking Run creates a job, keeps the window responsive,
   streams progress/log output, and permits cancellation.
5. **Run remotely:** Clicking Run invokes application-owned SSH orchestration
   through a background task. The UI displays connection/staging/running/fetch
   phases without exposing credentials.
6. **Handle failure:** A missing model, invalid configuration, Python crash,
   solver failure, unreachable host, and malformed result each produce a
   distinct actionable error while preserving logs and configuration.
7. **Display results:** Completion updates job status and renders SPL and
   impedance together with legends and units, field-map heatmaps, QA tables,
   and artifact links without restarting the app.
8. **Cancel correctly:** Cancellation transitions through `cancelling` to
   `cancelled`; it is never reported as success or generic failure.
9. **Restore:** Closing and reopening restores window state, configuration, job
   history, and the last successful result.
10. **Distribute:** The installed command launches the native app without a
    gpui-toolkit source checkout, Cargo, or repository-relative path discovery.

P1 acceptance additionally requires rendered speaker meshes or field geometry,
interactive chart inspection/export, virtualized job/log tables, and secure
credential-store integration.

## 10. Required test coverage

### Protocol and schema

- Golden round trips for every node and message type.
- Unsupported-version and capability-negotiation failures.
- Unknown node/action IDs, invalid patches, stale revisions, and oversized
  payloads.
- Python crash, malformed JSON, truncated message, stderr flood, and graceful
  shutdown tests.

### Controls and accessibility

- Keyboard, focus, disabled/read-only, validation, and screen-reader metadata
  for every form control.
- Intermediate invalid number text and commit semantics.
- Narrow-width layout with long labels, units, and validation messages.

### Jobs

- Progress, indeterminate work, bounded logs, success, failure, cancellation,
  cancellation races, close-while-running, and late-result suppression.
- Confirmation that no Python or process work blocks the GPUI render thread.

### Visualization

- Multi-series chart updates preserving zoom/legend state.
- Heatmap coordinate/colorbar validation and missing-value behavior.
- Mesh geometry, scalar-only, material-only, and camera-only cache updates.
- Large table/log virtualization and retained-resource eviction.

### Packaging

- Clean-machine launch from an installed wheel/application bundle.
- Python/Rust version equality and compatible host discovery.
- Platform smoke tests on macOS, Linux, and Windows where supported.

## 11. Suggested delivery sequence

1. **Session foundation (P0):** persistent child, handshake, structured events,
   actions, patches, lifecycle, and compatibility tests.
2. **Forms and effects (P0):** input/select/number/toggle, validation, dialogs,
   dynamic tabs, and accessibility.
3. **Jobs (P0):** background handlers/processes, progress, bounded logs,
   cancellation, terminal states, and error recovery.
4. **Scientific results (P0/P1):** multi-series charts, incremental data,
   heatmap metadata, tables, and export.
5. **Retained 3D (P1):** mesh binding, scalar fields, resource patches, and
   picking follow-up.
6. **Distribution (P0):** bundled host discovery, unified versions, supported
   Python matrix, and clean-machine tests.

Session protocol, form controls, and jobs should land before application-level
live editing is attempted. Adding isolated Python control constructors without
an event/update protocol would render forms that cannot own reliable state.

## 12. Non-goals

- Moving Sonium solver logic into gpui-toolkit.
- Giving Rust direct knowledge of speaker, FEM, BEM, SSH, or job-result schemas.
- Running arbitrary Python callbacks on GPUI's render thread.
- Treating shell command strings as UI actions.
- Replacing application-owned durable job storage with hidden toolkit state.
- Requiring Python applications to manipulate raw GPUI, wgpu, buffer, pipeline,
  or shader objects.

## 13. Open design decisions

1. Whether Python remains an external supervised process or can optionally be
   embedded. The external process is the safer initial isolation boundary.
2. Whether patches are JSON Patch, a toolkit-specific typed operation list, or
   a reconciled `view()` diff. Stable IDs and revision semantics are required in
   every case.
3. Which large-array transport is portable enough for charts and 3D resources.
4. Whether job helpers live in `gpui-python-runtime` or a separate Python
   application-runtime package.
5. How application bundles select and provision a compatible Python runtime.
6. Which credential-store abstraction is feasible across supported platforms.

