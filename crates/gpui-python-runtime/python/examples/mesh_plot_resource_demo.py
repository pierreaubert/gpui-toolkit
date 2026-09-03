"""Resource-backed MeshPlot host smoke example.

The geometry, stable IDs, scalar field, and validity mask travel as typed
little-endian mesh frames before the initial retained snapshot.  The example
is intentionally small enough for native-host screenshot and selection QA.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
from threading import Thread
from time import sleep

from gpui_toolkit import App, Section, px, ui
from gpui_toolkit.resources import MeshFrameKind, ResourceStore


resources = ResourceStore(4096)
positions = resources.put_mesh_array(
    "mesh-positions",
    [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
    shape=(4, 3),
    dtype="f64le",
)
triangles = resources.put_mesh_array(
    "mesh-triangles",
    [[0, 1, 2], [0, 2, 3]],
    shape=(2, 3),
    dtype="u32le",
)
vertex_ids = resources.put_mesh_array(
    "mesh-vertex-ids", [101, 102, 103, 104], shape=(4,), dtype="u64le"
)
cell_ids = resources.put_mesh_array(
    "mesh-cell-ids", [201, 202], shape=(2,), dtype="u64le"
)
field_values = resources.put_mesh_array(
    "mesh-pressure", [0.0, 0.5, 1.0, 0.25], shape=(4,), dtype="f64le"
)
validity = resources.put_mesh_array(
    "mesh-pressure-valid", [True, True, True, True], shape=(4,), dtype="bool_bytes"
)

geometry = px.mesh_geometry(
    positions,
    triangles,
    id="resource-baffle",
    vertex_ids=vertex_ids,
    cell_ids=cell_ids,
)
field = px.mesh_field(
    field_values,
    id="resource-pressure",
    label="Sound pressure level",
    unit="dB SPL",
    valid=validity,
)
plot = (
    px.mesh("resource-pressure-plot")
    .geometry(geometry)
    .field(field)
    .mode("fill_and_isolines")
    .equal_aspect(True)
    .interactions(("pan", "zoom", "inspect", "select", "reset", "fit"))
    .title("Resource-backed MeshPlot")
    .on_selection_change("resource_mesh_selected")
)
root = ui.mesh_plot(plot)


class ResourceMeshApp(App):
    def on_session_ready(self, context):
        for resource, kind in (
            (positions, MeshFrameKind.GEOMETRY),
            (triangles, MeshFrameKind.GEOMETRY),
            (vertex_ids, MeshFrameKind.IDS),
            (cell_ids, MeshFrameKind.IDS),
            (field_values, MeshFrameKind.FIELD),
            (validity, MeshFrameKind.MASK),
        ):
            resources.send_mesh_frames(context, resource, kind)

        close_after = float(os.environ.get("GPUI_TOOLKIT_QA_CLOSE_AFTER_SECS", "0"))
        if close_after > 0:
            Thread(target=self._close_after, args=(context, close_after), daemon=True).start()

    @staticmethod
    def _close_after(context, delay: float) -> None:
        sleep(delay)
        context.effect("qa-close-window", "close_window")

    def on_action(self, event, context):
        if event.action != "resource_mesh_selected":
            return
        destination = os.environ.get("GPUI_TOOLKIT_QA_SELECTION_LOG")
        if destination:
            destination_path = Path(destination)
            destination_path.parent.mkdir(parents=True, exist_ok=True)
            destination_path.write_text(
                json.dumps(
                    {
                        "event": event.event,
                        "action": event.action,
                        "node_id": event.node_id,
                        "payload": event.payload,
                    },
                    sort_keys=True,
                )
                + "\n",
                encoding="utf-8",
            )


def build_app() -> App:
    return ResourceMeshApp(
        title="Resource-backed MeshPlot",
        sections=[Section("resource-mesh", "Resource mesh", root)],
    )


def _should_run_native_host() -> bool:
    return any(
        os.environ.get(name)
        for name in (
            "GPUI_TOOLKIT_DUMP_IR",
            "GPUI_TOOLKIT_SESSION",
            "GPUI_TOOLKIT_HOST",
            "GPUI_TOOLKIT_QA_CLOSE_AFTER_SECS",
            "GPUI_TOOLKIT_QA_SELECTION_LOG",
            "GPUI_TOOLKIT_QA_AUTO_SELECT",
            "GPUI_TOOLKIT_QA_HOST_SELECTION_LOG",
            "GPUI_TOOLKIT_QA_POINTER_TRACE",
            "GPUI_TOOLKIT_QA_HIT_TRACE",
            "GPUI_TOOLKIT_QA_INNER_HIT_TRACE",
            "GPUI_TOOLKIT_QA_RENDER_TRACE",
            "GPUI_TOOLKIT_QA_LIVE_HIT_TRACE",
            "GPUI_TOOLKIT_QA_POINTER_POINTS",
        )
    )


def main() -> None:
    app = build_app()
    if _should_run_native_host():
        app.run()
    else:
        print(json.dumps(app.to_spec(), indent=2))


if __name__ == "__main__":
    main()
