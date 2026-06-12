# gpui-python-runtime

Retained scene specifications for the GPUI Python wrapper.

Python declares UI and `scene3d` objects. Rust validates the scene, tracks
stable ids, classifies dirty resources, and adapts supported nodes to
`gpui-d3rs` 3D elements. Raw `wgpu` devices, queues, buffers, pipelines, and
shaders remain private to the renderer.

## V1 Scene3D Scope

- Surfaces: row-major `z` grids with optional `x`/`y` axes, log axes, z range,
  labels, colormaps, wireframe mode, orbit cameras, and interactions.
- Lines: retained orbit camera state and CPU-projected `Lines3DElement`
  segments/line strips.
- Meshes, materials, perspective cameras, and lights: validated spec objects
  for the lower-level scene API. Mesh rendering is intentionally not bound to a
  GPUI element yet.

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

## Showcase Application

Run the Python-authored native GPUI showcase with retained 3D scenes and
embedded `gpui-px` charts:

```bash
cargo run -p gpui-python-runtime --features showcase --bin gpui-python-showcase -- crates/gpui-toolkit/gpui-python-runtime/python/showcase.py
PYTHONPATH=crates/gpui-toolkit/gpui-python-runtime/python ./venv/bin/python crates/gpui-toolkit/gpui-python-runtime/python/showcase.py
```

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
