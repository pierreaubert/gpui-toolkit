# gpui-toolkit Python tutorial

Build a native desktop app in pure Python. You declare the UI, the bundled
Rust host (`gpui-python-host`) renders it with GPUI. No Rust toolchain needed.

Requires `gpui-toolkit` 0.9.27 or newer (the `data` + `px` builder API used
below). Earlier wheels expose the older `charts.*(id, x, y)` functions instead.

## 1. Install the wheel

Python 3.10 or newer, then:

```bash
pip install gpui-toolkit
```

Platform wheels (macOS arm64/x86_64, Windows x64, Linux manylinux) each bundle
their own native host binary, so the install is self-contained. Verify:

```bash
python -c "import gpui_toolkit; print(gpui_toolkit.__version__)"
python -c "from gpui_toolkit.app import _host_binary; print(_host_binary())"
```

The second command prints the bundled host path. If it prints a path,
windowing will work.

## 2. Run the demo app

From this directory:

```bash
python demo_app.py
```

A native window opens with **Overview** and **Chart** sections in the sidebar.
`build_app().run()` locates the bundled host and re-executes your script under
it as a session — you never touch the host directly. With an installed wheel
the same file runs from anywhere; from this repo the host also accepts a
script path explicitly:

```bash
cargo run -p gpui-python-runtime --features showcase --bin gpui-python-host -- python/tutorial/demo_app.py
```

## 3. How `demo_app.py` fits together

- `App(title=..., sections=[...])` is the whole window and needs at least one
  `section(id, label, content)`; each section becomes a sidebar entry.
- `ui.vstack / hstack / text / metric / button / section_header` are layout
  and widgets. Any widget Python later updates needs a stable `id`.
- `data.Dataset.from_mapping({...}, id="demo-wave")` holds chart data as a
  host-owned resource. Declare every live resource on the app:
  `app.resources = (wave,)`, otherwise the chart has nothing to bind.
- `px.line("demo-line").data(wave).x("x").y("y")...` builds a native line
  chart. `.series(...)` / `.color(...)` name the columns holding series names
  and per-series colors. Sibling builders: `px.bar`, `px.scatter`, `px.area`,
  `px.boxplot`, `px.pie`, `px.donut`, `px.heatmap`, `px.contour`.

## 4. Interact: the action loop

Press **Click me**. The Clicks metric increments through this round-trip:

1. Native click → Python `DemoApp.on_action`, with `event.action` matching the
   button's `action="demo_bump"`.
2. `context.acknowledge(event)` confirms the event.
3. `context.patch([{"op": "set", "id": "demo-clicks", "property": "value",
   "value": ...}], request_id=event.id)` re-renders one widget. Passing
   `request_id=event.id` lets the host discard the patch if a newer event
   already superseded it.

All input controls (`ui.slider`, `ui.text_input`, …) follow the same loop and
carry their new value in `event.payload`.

## 5. Iterate

Check the UI spec without opening a window (prints the JSON your script
produces — handy for debugging layout and data issues):

```bash
GPUI_TOOLKIT_DUMP_IR=1 python demo_app.py
```

Add a second chart section with one line, reusing the same dataset pattern:

```python
px.bar("demo-bar").data(wave).x("series").y("y").title("Totals")
```

## 6. Keep going: larger examples

[`../examples/`](../examples/) holds full apps, each with `build_app()` and
covered by `test_examples.py`:

- [`chart_gallery.py`](../examples/chart_gallery.py) — every px chart type:
  scatter, line, area, heatmap, contour, isoline, bar, pie, donut, box-plot,
  treemap. Read this next.
- [`spinorama_demo.py`](../examples/spinorama_demo.py) — CEA2034, horizontal /
  vertical SPL, contour, and a retained 3D surface from deterministic data.
- [`surface3d_demo.py`](../examples/surface3d_demo.py) — sinc, spinorama-style,
  and saddle surface modes.
- [`mesh_plot_demo.py`](../examples/mesh_plot_demo.py),
  [`mesh_plot_revolve_demo.py`](../examples/mesh_plot_revolve_demo.py),
  [`mesh_plot_resource_demo.py`](../examples/mesh_plot_resource_demo.py) —
  unstructured triangle-mesh plots.
- [`../showcase.py`](../showcase.py) — the component catalog the native
  `gpui-python-showcase` binary runs, including background jobs
  (`context.spawn_job`) and live patching.

Replace the data-builder functions with measurement or application data when
adapting a demo; the chart declarations stay unchanged.

## Troubleshooting

| Symptom | Cause / fix |
|---|---|
| `No GPUI native host was found` | Wheel installed without the bundled binary. `pip install --force-reinstall gpui-toolkit`, or point `GPUI_TOOLKIT_HOST` at a host binary. |
| `ImportError` on `gpui_toolkit` inside the window | The host spawned a different interpreter than the one holding the wheel. Set `GPUI_PYTHON=$(which python)`. |
| `App requires at least one section` | `App(sections=[])` — add at least one `section(...)`. |
| Button does nothing | The button's `action` must match `event.action` in `on_action`, and the patched `id` must exist in the spec. |
| Chart is empty | Its `Dataset`/`ArrayData` is missing from `app.resources`, so the host has nothing to bind. |
