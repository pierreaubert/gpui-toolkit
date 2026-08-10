"""Axisymmetric section/revolve declaration example."""
import json
import os

from gpui_toolkit import App, Section, meshplot, ui

mesh = meshplot.geometry(
    [[0.1, 0.0, 0.0], [0.3, 0.0, 0.0], [0.1, 1.0, 0.0]], [[0, 1, 2]], id="profile"
)
root = ui.mesh_plot(
    meshplot.plot(mesh, view="axisymmetric_revolve", interactions=("zoom", "select", "fit")),
    selection_action="selected",
)


def build_app() -> App:
    return App(title="Axisymmetric mesh plot", sections=[Section("profile", "Profile", root)])


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
