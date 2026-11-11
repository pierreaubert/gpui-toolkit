# gpui-toolkit

[![PyPI version](https://img.shields.io/pypi/v/gpui-toolkit.svg)](https://pypi.org/project/gpui-toolkit/)
[![Python versions](https://img.shields.io/pypi/pyversions/gpui-toolkit.svg)](https://pypi.org/project/gpui-toolkit/)
[![License: ISC](https://img.shields.io/pypi/l/gpui-toolkit.svg)](https://github.com/pierreaubert/gpui-toolkit/blob/main/LICENSE)

**Build native desktop apps, charts, and 3D scenes in pure Python — rendered
on the GPU by [GPUI](https://www.gpui.rs/) (the UI framework behind Zed).**

You declare the UI in Python. A native host binary bundled in the wheel
renders it in a real OS window: Metal on macOS, DirectX 12/Vulkan on Windows,
Vulkan on Linux. No Rust toolchain, no Electron, no browser, no HTML.

- **Declarative UI** — layout, widgets, sidebars, themes, mini-app shell.
- **Charts** — Plotly-Express-style builders (`px.line`, `px.scatter`,
  `px.heatmap`, …) over typed datasets, with zoom/pan/selection in the host.
- **2D & 3D scenes** — retained 3D surfaces, meshes and line orbits
  (`scene3d`), plus a 2D drawing surface with pointer/keyboard input
  (`scene2d`) suitable for boards and small games.
- **Fast native core** — statistics, ticks, color scales and interpolation
  from the Rust `gpui-d3rs` crate, exposed through a private `abi3`
  extension and typed stubs (`py.typed`).
- **Zero runtime dependencies** for the base wheel.

## Install

Python 3.10 or newer:

```bash
pip install gpui-toolkit
```

Each platform wheel bundles its own native host, so that one command is the
whole setup. Verify the install (second command prints the bundled host
path — if it prints one, windowing will work):

```bash
python -c "import gpui_toolkit; print(gpui_toolkit.__version__)"
python -c "from gpui_toolkit.app import _host_binary; print(_host_binary())"
```

| Platform | Wheels |
|---|---|
| macOS | arm64, x86_64 |
| Windows | x64 |
| Linux | x86_64 (manylinux, glibc 2.39+) |

Optional: richer Arrow conversion for nested or temporal dataset columns:

```bash
pip install 'gpui-toolkit[arrow]'
```

## Quickstart

### Step 1 — your first window

Save as `hello.py` and run `python hello.py`. A native window opens with a
sidebar ("Home") and the content below:

```python
from gpui_toolkit import App, section, ui

app = App(
    title="Hello GPUI",
    sections=[
        section(
            "home",
            "Home",
            ui.vstack(
                [
                    ui.section_header(
                        "Hello from Python",
                        "Declared in Python, rendered natively by GPUI.",
                    ),
                    ui.text("No Rust, no browser, no HTML — a real native window."),
                ],
                gap=12.0,
            ),
        )
    ],
)

if __name__ == "__main__":
    app.run()
```

An `App` is the whole window. It needs at least one `section(id, label,
content)`; each section becomes a sidebar entry. `ui.vstack` / `ui.hstack`
lay out widgets — text, metrics, buttons, cards, selects, sliders, and more.

### Step 2 — add a chart

Charts bind to a `Dataset` (column data the host owns). Declare every
dataset a chart uses in `resources=`, otherwise the host has nothing to
bind:

```python
import math

from gpui_toolkit import App, data, px, section, ui

xs = [float(i) for i in range(72)]
wave = data.Dataset.from_mapping(
    {
        "x": xs + xs,
        "y": [math.sin(x * 0.35) for x in xs] + [math.cos(x * 0.35) for x in xs],
        "series": ["Sine"] * len(xs) + ["Cosine"] * len(xs),
    },
    id="wave",
)

chart = (
    px.line("wave-chart")
    .data(wave)
    .x("x")
    .y("y")
    .series("series")
    .title("Waveforms")
    .x_label("Sample")
    .y_label("Amplitude")
)

app = App(
    title="Hello GPUI",
    sections=[
        section(
            "home",
            "Home",
            ui.vstack(
                [
                    ui.section_header(
                        "Hello from Python",
                        "Declared in Python, rendered natively by GPUI.",
                    ),
                    ui.text("No Rust, no browser, no HTML — a real native window."),
                ],
                gap=12.0,
            ),
        ),
        section("chart", "Chart", chart),
    ],
    resources=(wave,),  # every Dataset a chart binds must be declared here
)

if __name__ == "__main__":
    app.run()
```

Swap `px.line` for `px.scatter`, `px.area`, `px.bar`, `px.boxplot`,
`px.pie`, `px.donut`, `px.heatmap`, `px.contour`, `px.isoline`, or
`px.treemap` — same builder shape.

### Step 3 — handle clicks

Widgets send actions; your `on_action` replies with patches. Subclass
`App` as a dataclass to keep state:

```python
from dataclasses import dataclass

from gpui_toolkit import App, Event, SessionContext, section, ui


@dataclass
class CounterApp(App):
    clicks: int = 0

    def on_action(self, event: Event, context: SessionContext) -> None:
        if event.action != "bump":
            return
        self.clicks += 1
        context.acknowledge(event)
        context.patch(
            [
                {
                    "op": "set",
                    "id": "clicks",
                    "property": "value",
                    "value": str(self.clicks),
                }
            ],
            request_id=event.id,
        )


app = CounterApp(
    title="Counter",
    sections=[
        section(
            "home",
            "Home",
            ui.vstack(
                [
                    ui.metric("Clicks", 0, id="clicks"),
                    ui.button("Click me", id="bump-button", action="bump"),
                ],
                gap=12.0,
            ),
        )
    ],
)

if __name__ == "__main__":
    app.run()
```

The loop is always the same: native event → `on_action` (match on
`event.action`) → `context.patch(...)` updates widgets by their stable
`id`. Input controls (sliders, selects, text inputs) carry their new value
in `event.payload`. Note that patched values must match the widget schema —
a metric's `value` is a string, hence `str(self.clicks)`.

### Step 4 — iterate headlessly

Check the UI spec without opening a window — this prints the exact JSON
your script hands to the host, which is handy for debugging and CI:

```bash
GPUI_TOOLKIT_DUMP_IR=1 python hello.py
```

## How it works

Python builds a JSON UI description ("IR"); `App.run()` launches the
bundled Rust host, which owns the window, GPUI rendering, themes, and GPU
state. Dataset and array payloads travel as Arrow IPC / memory-mapped
frames (never inside the JSON), and user input comes back as typed events.
Your process stays pure Python — the host is a renderer, not an interpreter
you have to embed.

## What's in the box

| Module | What it does |
|---|---|
| `gpui_toolkit.ui` | Layout (`vstack`, `hstack`, cards) and widgets (button, metric, select, slider, text input, progress, badges…) |
| `gpui_toolkit.px` | Chart builders: line, scatter, area, bar, boxplot, pie, donut, heatmap, contour, isoline, treemap |
| `gpui_toolkit.data` | `Dataset` / `ArrayData` resources, views, filters, group-by aggregation |
| `gpui_toolkit.scene3d` | Retained 3D: surfaces, meshes, line orbits, cameras, colormaps |
| `gpui_toolkit.scene2d` | 2D drawing surface (rects, circles, paths, gradients) with pointer/keyboard input and ticks |
| `gpui_toolkit.meshplot` | Unstructured triangle-mesh plots with scalar fields |
| `gpui_toolkit.miniapp` | Native mini-app shell: window chrome + theme switcher |
| `gpui_toolkit.d3rs`, `gpui_toolkit.native` | D3-style scales, ticks, statistics, color interpolation — Rust-backed |

## Examples

Full apps in the repository, each runnable against the installed wheel:

| Example | Shows |
|---|---|
| [Tutorial `demo_app.py`](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/tutorial/demo_app.py) | Start here: metrics, theme picker, two charts, live patch loop |
| [`chart_gallery.py`](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/examples/chart_gallery.py) | Every `px` chart type |
| [`spinorama_demo.py`](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/examples/spinorama_demo.py) | CEA2034 speaker measurements: SPL charts, contour, 3D surface |
| [`surface3d_demo.py`](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/examples/surface3d_demo.py) | 3D surface modes with orbit camera |
| [`games_demo.py`](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/examples/games_demo.py) | Scene2D boards: Sudoku, Chess, Tetris, Othello, Queens, Zip |
| [`showcase.py`](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/showcase.py) | The full component catalog, background jobs, live patching |

## Documentation

- **[Python tutorial](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/tutorial/README.md)** — the step-by-step
  walkthrough this page abbreviates: install, run, how `demo_app.py` fits
  together, the action loop in depth, troubleshooting.
- **[Toolkit tutorial, §12 Python Runtime Demo](https://github.com/pierreaubert/gpui-toolkit/blob/main/TUTORIAL.md#12-python-runtime-demo)** —
  the same Python path in the wider toolkit tour (scaffolding, themes,
  design system, building and distributing apps).
- **[Examples directory](https://github.com/pierreaubert/gpui-toolkit/tree/main/crates/gpui-python-runtime/python/examples)** —
  every demo above, plus mesh-plot variants.
- **[Repository README](https://github.com/pierreaubert/gpui-toolkit#readme)** — the full Rust
  workspace this package is part of.
- **[Changelog](https://github.com/pierreaubert/gpui-toolkit/blob/main/CHANGELOG.md)**.
- Contributing or hacking on the runtime itself? The
  [crate README](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/README.md)
  documents the IR/schema contracts, resource model, and dev tooling.

## Troubleshooting

| Symptom | Cause / fix |
|---|---|
| `No GPUI native host was found` | Wheel installed without its bundled binary. `pip install --force-reinstall gpui-toolkit`, or point `GPUI_TOOLKIT_HOST` at a host binary. |
| `ImportError` on `gpui_toolkit` inside the window | The host spawned a different interpreter than the one holding the wheel. Set `GPUI_PYTHON=$(which python)`. |
| `App requires at least one section` | `App(sections=[])` — add at least one `section(...)`. |
| Button does nothing | The button's `action` must match `event.action` in `on_action`, and the patched `id` must exist in the spec. |
| Chart is empty | Its `Dataset`/`ArrayData` is missing from `app.resources`, so the host has nothing to bind. |

## Status

Alpha (`0.x`): the Python API and the v1 JSON schema are usable today and
versioned — additive fields stay on v1, breaking shape changes require a
schema bump with a migration path. The package version tracks the Rust
workspace version. Issues and questions:
[github.com/pierreaubert/gpui-toolkit/issues](https://github.com/pierreaubert/gpui-toolkit/issues).

## License

ISC — see [LICENSE](https://github.com/pierreaubert/gpui-toolkit/blob/main/LICENSE).
