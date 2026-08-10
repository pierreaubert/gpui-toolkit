"""Small inline mesh plot protocol example."""
import json
import os

from gpui_toolkit import App, Section, meshplot, ui

mesh = meshplot.geometry(
    [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], [[0, 1, 2]], id="demo"
)
field = meshplot.scalar_field([0.0, 1.0, 0.5], id="pressure", unit="Pa")
plot = meshplot.plot(
    mesh,
    field,
    mode="scalar_fill",
    equal_aspect=True,
    interactions=("pan", "zoom", "inspect", "select", "reset", "fit"),
)
root = ui.mesh_plot(plot, selection_action="mesh_selected")


def build_app() -> App:
    return App(title="Mesh plot demo", sections=[Section("mesh", "Mesh", root)])


def main() -> None:
    app = build_app()
    if (
        os.environ.get("GPUI_TOOLKIT_DUMP_IR") == "1"
        or os.environ.get("GPUI_TOOLKIT_SESSION") == "1"
        or bool(os.environ.get("GPUI_TOOLKIT_HOST"))
    ):
        app.run()
    else:
        print(json.dumps(app.to_spec(), indent=2))


if __name__ == "__main__":
    main()
