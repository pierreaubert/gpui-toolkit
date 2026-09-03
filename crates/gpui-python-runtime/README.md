# gpui-python-runtime

Retained scene specifications for the GPUI Python wrapper.

Python declares UI and `scene3d` objects. Rust validates the scene, tracks
stable ids, classifies dirty resources, and adapts supported nodes to
`gpui-d3rs` 3D elements. Raw `wgpu` devices, queues, buffers, pipelines, and
shaders remain private to the renderer.

## Scene3D Scope

- Surfaces: row-major `z` grids with optional `x`/`y` axes, log axes, z range,
  labels, colormaps, wireframe mode, orbit cameras, and interactions.
- Lines: retained orbit camera state and CPU-projected `Lines3DElement`
  segments/line strips.
- Meshes and surfaces consume retained `ArrayData` geometry and scalar-field
  resources; materials, perspective cameras, and lights remain declarative
  scene objects while GPU state stays host-owned.

## Audited Python Surface

`python-surface.toml` is the reviewed Python mapping registry. The generated
`python-rustdoc-inventory.json` freezes the all-feature macOS public API of the
current priority crates, `gpui-d3rs` and `gpui-px`, including inherent methods,
fields, callable parameters, and return types. Run the freshness gate with:

```bash
just qa-python-rustdoc-inventory
```

The gate uses pinned `nightly-2026-09-02` rustdoc JSON and rejects public Rust
surface changes until the snapshot is reviewed and regenerated. Inventory
coverage does not itself claim Python parity; stable v2 remains blocked until
every inventory entry has a reviewed binding or exclusion classification.

## JSON Schema Contract

Python-authored payloads are versioned at the JSON boundary:

- app IR uses `schema_version: 1`, exposed as
  `PYTHON_APP_IR_SCHEMA_VERSION`.
- Scene3D specs use `schema_version: 1`, exposed as
  `SCENE3D_SPEC_SCHEMA_VERSION`.

New Python emitters write the current schema version. Rust treats omitted
`schema_version` fields as v1 so early examples and local scripts keep loading,
but validation rejects unsupported future versions before rendering or reusing a
cached spec.

Compatibility policy:

1. Additive optional fields may stay on v1 when Rust gives them safe defaults.
2. Renaming fields, removing fields, changing data-shape semantics, or changing
   renderer meaning requires a schema-version bump.
3. A schema-version bump must include compatibility tests for previous v1
   payloads and a migration path before Python emitters start writing it.
4. Consumers should parse JSON, validate `PythonAppIr`, then parse and validate
   Scene3D specs through `TypedSpecCache` or
   `validate_scene3d_spec_schema_version`.

## Resource Model

`RetainedSceneCache` fingerprints geometry, material, and camera state
separately:

- unchanged scenes do no renderer work,
- camera-only changes update uniforms/state,
- color/material changes update small renderer state,
- data/mesh changes reupload affected geometry.

`Gpui3DCache` is available behind the `gpui` feature and keeps
`Surface3DElement` / line camera state keyed by stable ids.

## Python Examples

The examples build JSON-serializable scene specs that the Rust runtime can
validate and adapt to GPUI elements:

```bash
PYTHONPATH=python python examples/surface_dispersion.py
PYTHONPATH=python python examples/lines_orbit.py
PYTHONPATH=python python examples/mesh_scene.py
```

- `surface_dispersion.py` shows a log-frequency surface with orbit controls.
- `lines_orbit.py` shows line strips, axis references, and a shared orbit camera.
- `mesh_scene.py` shows the future lower-level scene shape with mesh, path, and
  light nodes.

The larger app-authored demos mirror the Rust showcase programs:

```bash
# Dump the native app IR without requiring a GPUI host.
GPUI_TOOLKIT_DUMP_IR=1 PYTHONPATH=python python examples/spinorama_demo.py
GPUI_TOOLKIT_DUMP_IR=1 PYTHONPATH=python python examples/surface3d_demo.py
GPUI_TOOLKIT_DUMP_IR=1 PYTHONPATH=python python examples/chart_gallery.py

# Without a host-related environment variable, each script prints JSON too.
PYTHONPATH=python python examples/spinorama_demo.py > /tmp/spinorama-python.json
```

- `spinorama_demo.py` contains CEA2034, horizontal SPL, vertical SPL,
  contour, and retained 3D surface sections using deterministic local data.
- `surface3d_demo.py` contains the sinc, spinorama-style, and saddle surface
  modes from the Rust `surface3d_demo`.
- `chart_gallery.py` covers scatter, line, area, heatmap, contour, isoline, bar,
  pie, donut, box-plot, and treemap declarations.

Replace the local data-builder functions with measurement or application data
when adapting a demo; the chart and Scene3D declarations remain unchanged.

## Python Package

The Python declarations are packaged as `gpui-toolkit` with the import package
`gpui_toolkit`. Wheels contain a private PyO3 `abi3-py310` extension for pure
native computations, while Rust/GPUI remains the rendering host. The base
wheel has no Python runtime dependencies.

```bash
python -m pip install gpui-toolkit
python -c "from gpui_toolkit import native; print(native.AVAILABLE)"
```

Build a platform wheel with maturin (rather than invoking Cargo's `cdylib`
directly):

```bash
maturin build --manifest-path Cargo.toml --release
```

The installed extension is `gpui_toolkit._native`; public code should import
`gpui_toolkit.native` or the corresponding `gpui_toolkit.d3rs` functions.
Native d3rs array statistics, search, tick generation, numeric/array and
RGB/HSL/Lab/HCL/Cubehelix interpolation, color operations, and `linear_scale`
execute the corresponding `gpui-d3rs` implementation. Substantial array work
releases the GIL.
Command request objects remain available for deliberately host-owned execution.

The package version matches the Rust crate version. Update both
`pyproject.toml` and `Cargo.toml` together when releasing a new Python-facing
runtime.

## Showcase Application

Run the Python-authored native GPUI showcase with retained 3D scenes and
embedded `gpui-px` charts:

```bash
cargo run -p gpui-python-runtime --features showcase --bin gpui-python-showcase -- crates/gpui-toolkit/gpui-python-runtime/python/showcase.py
PYTHONPATH=crates/gpui-toolkit/gpui-python-runtime/python ./venv/bin/python crates/gpui-toolkit/gpui-python-runtime/python/showcase.py
```

The complete `gpui-d3rs` gallery can use the same generic host with no
d3rs-showcase-specific Rust route:

```bash
cargo run -p gpui-python-runtime --features showcase --bin gpui-python-showcase -- crates/gpui-python-runtime/python/d3_showcase.py
```

The abi3 wheel publishes nullable Boolean, integer, floating-point, and UTF-8
`Dataset` columns as Arrow IPC without Python dependencies. Install the
optional Arrow adapter when nested or temporal columns need PyArrow's richer
conversion support:

```bash
python -m pip install 'gpui-toolkit[arrow]'
```

Declare every live `Dataset` and `ArrayData` object on the application:

```python
app = App(sections=(...), resources=(events, spectrum_grid))
```

The base `App.on_session_ready` publishes initial binary generations and
subscribes to later commits. Overrides must call
`super().on_session_ready(context)` before issuing application-specific
commands. Resource values never enter the JSON application snapshot.

The base wheel remains dependency-free. Source-only use without either the
native extension or PyArrow can still dump declarations, while starting a live
dataset app raises a typed transport error instead of displaying an
unpopulated chart.

The installed host negotiates `resource_mmap_frames` and creates a private
`0700` directory with a high-entropy session token. Python publishes each
complete dataset or `ArrayData` generation as a `0600` file; the host validates
the token, local filename, size, permissions, and checksum, maps it read-only,
and unlinks it immediately on Unix. The retained mapping is consumed directly
by Arrow/table/chart and dense-array readers. Bounded binary stdout frames are
used only when mmap transport is unavailable. Dataset and mesh frames carry
payload checksums. Per-frame acknowledgements drive shared Python in-flight
byte accounting, typed rejection reporting, backpressure, and deterministic
fallback cleanup.
Native consumers acquire generation-scoped leases. Publishing a new generation
keeps an older mapped generation alive only while a retained consumer still
owns it; explicit drops cannot invalidate active owners, and the final release
reclaims both the mapping and its byte-budget charge.

Resource-backed `scatter`, `line`, `area`, `boxplot`, `bar`, `pie`, and
`donut` declarations execute bounded LOD sampling from Dataset/DatasetView or
numeric ArrayData frames. `heatmap`, `contour`, and `isoline` consume only a
two-dimensional ArrayData grid (`[height, width]`) and never flatten it into
JSON. One projection, one stable sort, and one `range(...)` execute in the
host. Tables, aggregate pipelines, resource-backed charts, and static export
execute the typed `DatasetView.filter` AST, including comparisons, boolean
composition, membership, and null checks. Projection is enforced by Python,
Rust IR validation, and each consumer. Tables sort before extracting
their visible window. Tables, resource-backed charts, and static chart export
execute one `group_by(...).aggregate(...)` stage over Arrow batches, then sort
and consume only the bounded aggregate window. Supported reducers are count,
sum, mean, min, max, first, and last. Scatter and line charts sort on their
bound x or y role.
Chart sort-plus-range remains explicitly rejected until it can preserve full
pipeline ordering. Tables retain primary-key selection, scatter/line charts
emit keyed point selections and viewport changes, treemaps emit keyed
selection, and surfaces emit retained camera viewport changes.

Scene3D surfaces, line-strip points, mesh vertices, triangle indices, and
scalar fields accept `ArrayData`. Their declarations contain only resource
descriptors; the host resolves the exact generation and validates shape and
dtype before creating native geometry or GPU state.

The showcase app, sections, UI kit demos, chart data, and `scene3d` specs live
in Python. Rust loads the JSON UI IR, then owns GPUI, retained 3D renderer
state, chart widgets, and theme integration.

## Platform Notes

The renderer path is inherited from `wgpu` via `gpui-d3rs`:

- macOS/iOS: Metal,
- Linux: Vulkan where available,
- Windows: DirectX 12 or Vulkan depending adapter support,
- Android: Vulkan once a GPUI Android backend exists.

The Python API is intended to stay the same across platforms; only GPUI backend
initialization should differ.
