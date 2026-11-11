# Python guide

Build native desktop apps, charts, and 3D scenes in pure Python — rendered
on the GPU by GPUI. You declare the UI; a native host binary bundled in the
wheel renders it in a real OS window. No Rust toolchain, no Electron, no
browser, no HTML. The base wheel has zero runtime dependencies and ships
typed stubs (`py.typed`).

## Install

Python 3.10 or newer:

```bash
pip install gpui-toolkit
```

Each platform wheel bundles its own native host, so that one command is the
whole setup. Verify the install (the second command prints the bundled host
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

## Step 1 — your first window

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
lay out widgets — text, metrics, buttons, cards, selects, sliders, and
more. Any widget Python later updates needs a stable `id`.

For window chrome and a built-in theme switcher, run the app inside the
native mini-app shell: pass `miniapp=MiniAppConfig(title=…, width=…,
height=…, with_theme=True)` from `gpui_toolkit.miniapp`. The
[tutorial demo](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/tutorial/demo_app.py)
shows it end to end.

## Step 2 — add a chart

Charts bind to a `Dataset` — column data the host owns. Declare every
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
`px.treemap` — same builder shape. Charts get wheel zoom, drag pan, and
double-click reset in the host; `.on_viewport_change("…")` and selection
events come back to Python as actions. Dense 2D grids (heatmap, contour,
isoline) bind a two-dimensional `ArrayData` instead of a `Dataset`.

Datasets stay host-side: values travel as Arrow IPC / memory-mapped frames,
never inside the UI JSON. `DatasetView` adds one projection, one typed
filter, one `group_by(…).aggregate(…)`, one stable sort, and bounded
windows, all executed by the host.

## Step 3 — handle clicks

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

The loop is always the same:

1. Native event → `on_action`, matched on `event.action`.
2. `context.acknowledge(event)` confirms it.
3. `context.patch(ops, request_id=event.id)` updates widgets by stable
   `id`; one op per widget. The `request_id` lets the host discard a patch
   a newer event already superseded.

Input controls (sliders, selects, text inputs, …) follow the same loop and
carry their new value in `event.payload`. Patched values must match the
widget schema — a metric's `value` is a string, hence `str(self.clicks)`;
a raw integer fails host validation and drops the session to an error
screen.

## Iterate headlessly

Check the UI spec without opening a window — this prints the exact JSON
your script hands to the host, which is handy for debugging and CI:

```bash
GPUI_TOOLKIT_DUMP_IR=1 python hello.py
```

## How it works

Python builds a JSON UI description ("IR", `schema_version: 1`);
`App.run()` launches the bundled Rust host (`gpui-python-host`), which owns
the window, GPUI rendering, themes, and GPU state, then re-executes your
script under it as a session. Dataset and array payloads move as binary
frames beside the JSON, and user input returns as typed events. Your code
stays pure Python — the host is a renderer, not an interpreter you embed.
Additive IR fields stay on v1; breaking shape changes require a schema
bump with a migration path.

## The modules

| Module | What it does |
|---|---|
| `gpui_toolkit.ui` | Layout (`vstack`, `hstack`, cards) and widgets (button, metric, select, slider, text input, progress, badges…) |
| `gpui_toolkit.px` | Chart builders: line, scatter, area, bar, boxplot, pie, donut, heatmap, contour, isoline, treemap |
| `gpui_toolkit.data` | `Dataset` / `ArrayData` resources, views, filters, group-by aggregation |
| `gpui_toolkit.scene3d` | Retained 3D: surfaces, meshes, line orbits, cameras, colormaps |
| `gpui_toolkit.scene2d` | 2D drawing surface (rects, circles, paths, gradients) with pointer/keyboard input and timed ticks — boards and small games |
| `gpui_toolkit.meshplot` | Unstructured triangle-mesh plots with scalar fields |
| `gpui_toolkit.miniapp` | Native mini-app shell: window chrome + theme switcher |
| `gpui_toolkit.d3rs`, `gpui_toolkit.native` | D3-style scales, ticks, statistics, color interpolation — Rust-backed |

## Examples

Full apps in the repository, each runnable against the installed wheel:

| Example | Shows |
|---|---|
| [Tutorial `demo_app.py`](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/tutorial/demo_app.py) | Start here, after this page: mini-app shell, theme picker, two charts, live patch loop |
| [`chart_gallery.py`](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/examples/chart_gallery.py) | Every `px` chart type |
| [`spinorama_demo.py`](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/examples/spinorama_demo.py) | CEA2034 speaker measurements: SPL charts, contour, 3D surface — compare with the [live Spinorama demo](/spinorama/) |
| [`surface3d_demo.py`](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/examples/surface3d_demo.py) | 3D surface modes with orbit camera |
| [`games_demo.py`](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/examples/games_demo.py) | Scene2D boards: Sudoku, Chess, Tetris, Othello, Queens, Zip — chess also runs in the [UI kit demo](/showcase/?section=chess) |
| [`showcase.py`](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/showcase.py) | The full component catalog, background jobs, live patching |

The [tutorial walkthrough](https://github.com/pierreaubert/gpui-toolkit/blob/main/crates/gpui-python-runtime/python/tutorial/README.md)
explains `demo_app.py` piece by piece, including how the shell theme
switcher stays in sync with app state.

## Troubleshooting

| Symptom | Cause / fix |
|---|---|
| `No GPUI native host was found` | Wheel installed without its bundled binary. `pip install --force-reinstall gpui-toolkit`, or point `GPUI_TOOLKIT_HOST` at a host binary. |
| `ImportError` on `gpui_toolkit` inside the window | The host spawned a different interpreter than the one holding the wheel. Set `GPUI_PYTHON=$(which python)`. |
| `App requires at least one section` | `App(sections=[])` — add at least one `section(...)`. |
| Button does nothing | The button's `action` must match `event.action` in `on_action`, and the patched `id` must exist in the spec. |
| Chart is empty | Its `Dataset`/`ArrayData` is missing from `app.resources`, so the host has nothing to bind. |

Working from a source checkout instead of the wheel? Build the host once
(`cargo build -p gpui-python-runtime --features showcase --bin
gpui-python-host --release`), set `GPUI_TOOLKIT_HOST` to it, and put
`crates/gpui-python-runtime/python` on `PYTHONPATH`.
