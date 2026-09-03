import contextlib
import io
import json
import os
from pathlib import Path
from queue import Empty, Queue
import subprocess
import sys
import textwrap
from threading import Thread
import time
import unittest

from gpui_toolkit import Event, SessionContext
from showcase import NATIVE_SECTION_ORDER, RuntimeShowcase, build_app, native_demo_sections


class RuntimeShowcaseCatalogTests(unittest.TestCase):
    def test_component_catalog_tracks_native_showcase_sections(self):
        app = build_app()
        self.assertEqual(
            [section.id for section in app.sections[:len(NATIVE_SECTION_ORDER)]],
            [section_id for section_id, _ in NATIVE_SECTION_ORDER],
        )
        self.assertEqual(
            [section.id for section in app.sections[len(NATIVE_SECTION_ORDER):]],
            ["charts", "surface", "lines", "scene-specs"],
        )
        self.assertEqual(
            [section.id for section in native_demo_sections()],
            [section_id for section_id, _ in NATIVE_SECTION_ORDER],
        )
        self.assertTrue(all("kind" in section["content"] for section in app.to_spec()["sections"]))

    def test_python_chart_gallery_uses_only_v2_resource_bindings(self):
        app = build_app()
        chart_section = next(section for section in app.to_spec()["sections"] if section["id"] == "charts")

        def nodes(value):
            if isinstance(value, dict):
                yield value
                for child in value.values():
                    yield from nodes(child)
            elif isinstance(value, list):
                for child in value:
                    yield from nodes(child)

        charts = [
            node
            for node in nodes(chart_section["content"])
            if node.get("kind") == "px_chart_v2"
        ]
        self.assertEqual(len(charts), 4)
        self.assertNotIn('"values"', json.dumps(charts))
        self.assertEqual(len(app.resources), 4)
        line = next(chart for chart in charts if chart["id"] == "response")
        self.assertEqual(line["data"]["roles"]["series"], "series")
        self.assertEqual(line["data"]["roles"]["color"], "color")
        self.assertTrue(line["x_log"])
        scatter = next(chart for chart in charts if chart["id"] == "latency")
        self.assertEqual(scatter["point_radius"], 4.0)


def read_process_stdout(process: subprocess.Popen[bytes], length: int, timeout: float = 5.0) -> bytes:
    """Read a subprocess pipe with a timeout on every supported platform."""
    result: Queue[bytes] = Queue(maxsize=1)
    Thread(target=lambda: result.put(process.stdout.read(length)), daemon=True).start()
    try:
        return result.get(timeout=timeout)
    except Empty:
        process.kill()
        _, stderr = process.communicate()
        raise AssertionError(f"timed out waiting for session output: {stderr!r}") from None


class RuntimeShowcaseSessionTests(unittest.TestCase):
    def test_orb_controls_patch_every_native_orb(self):
        output = io.StringIO()
        context = SessionContext()
        event = Event(
            id="event-orb-size",
            sequence=1,
            node_id="orb-size",
            event="change",
            action="set_orb_size",
            payload={"value": 2.0},
        )
        with contextlib.redirect_stdout(output):
            RuntimeShowcase().on_action(event, context)

        messages = [json.loads(line) for line in output.getvalue().splitlines()]
        patch = next(message for message in messages if message.get("type") == "patch")
        sphere_ops = [
            op
            for op in patch["ops"]
            if op.get("property") == "size" and op.get("id", "").startswith("thinking-orb-")
        ]
        cell_ops = [op for op in patch["ops"] if op.get("property") == "width"]
        self.assertEqual(len(sphere_ops), 9)
        self.assertTrue(all(op["value"] == 192.0 for op in sphere_ops))
        self.assertEqual(len(cell_ops), 9)
        self.assertTrue(all(op["value"] == 192.0 for op in cell_ops))

    def test_run_action_streams_a_job_and_result_patch(self):
        output = io.StringIO()
        context = SessionContext()
        event = Event(
            id="event-run",
            sequence=1,
            node_id="run-showcase-simulation",
            event="click",
            action="run-showcase-simulation",
            payload={},
        )

        with contextlib.redirect_stdout(output):
            RuntimeShowcase().on_action(event, context)
            deadline = time.monotonic() + 60.0
            while time.monotonic() < deadline:
                history = context.job_history()
                if any(
                    job["id"] == "showcase-simulation"
                    and job["state"] in {"succeeded", "failed", "cancelled"}
                    for job in history
                ):
                    break
                time.sleep(min(5.0, deadline - time.monotonic()))
            else:
                self.fail("showcase simulation did not reach a terminal state within one minute")

        messages = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertIn({"type": "acknowledged", "request_id": "event-run"}, messages)
        self.assertTrue(any(
            message.get("type") == "job"
            and message.get("id") == "showcase-simulation"
            and message.get("state") == "running"
            for message in messages
        ))
        self.assertTrue(any(
            message.get("type") == "job_log"
            and message.get("line", {}).get("message") == "Completed band 5/5"
            for message in messages
        ))
        self.assertTrue(any(
            message.get("type") == "patch"
            and message["ops"] == [{
                "op": "set", "id": "simulation-result",
                "property": "value", "value": "Ready",
            }]
            for message in messages
        ))

    def test_meshplot_subprocess_session_round_trips_snapshot_and_field_patch(self):
        child = textwrap.dedent(
            """
            from gpui_toolkit.app import App, section
            from gpui_toolkit import meshplot, ui

            geometry = meshplot.geometry(
                [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                [[0, 1, 2]],
                id="session-mesh",
            )

            class MeshPlotSessionApp(App):
                def __init__(self):
                    self.spec = meshplot.plot(
                        geometry,
                        meshplot.scalar_field(
                            [0.0, 0.5, 1.0], id="pressure", label="Pressure", unit="Pa"
                        ),
                        id="session-plot",
                        mode="scalar_fill",
                    )
                    super().__init__(
                        title="MeshPlot session",
                        required_capabilities=("meshplot", "patches"),
                        sections=[
                            section(
                                "main",
                                "Main",
                                ui.mesh_plot(self.spec, selection_action="inspect"),
                            )
                        ],
                    )

                def on_action(self, event, context):
                    if event.action == "replace-field":
                        context.replace_mesh_field(
                            "session-plot",
                            2,
                            meshplot.scalar_field(
                                [1.0, 1.5, 2.0],
                                id="pressure",
                                label="Pressure",
                                unit="Pa",
                            ),
                            request_id=event.id,
                        )

            MeshPlotSessionApp().serve()
            """
        )
        package_root = Path(__file__).resolve().parents[1]
        environment = os.environ.copy()
        environment["PYTHONPATH"] = os.pathsep.join(
            value for value in (str(package_root), environment.get("PYTHONPATH")) if value
        )
        process = subprocess.Popen(
            [sys.executable, "-u", "-c", child],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
        )

        def read_message():
            line = bytearray()
            while True:
                chunk = read_process_stdout(process, 1)
                if not chunk:
                    stderr = process.stderr.read()
                    self.fail(f"session exited without output: {stderr!r}")
                if chunk == b"\n":
                    return json.loads(line.decode("utf-8"))
                line.extend(chunk)

        def send_message(message):
            process.stdin.write((json.dumps(message) + "\n").encode("utf-8"))
            process.stdin.flush()

        try:
            send_message({
                "type": "initialize",
                "session_version": 1,
                "capabilities": ["meshplot", "patches"],
            })
            ready = read_message()
            snapshot = read_message()
            self.assertEqual(ready["type"], "ready")
            self.assertEqual(ready["capabilities"], ["meshplot", "patches"])
            self.assertEqual(snapshot["type"], "snapshot")
            node = snapshot["app_ir"]["sections"][0]["content"]
            self.assertEqual(node["kind"], "mesh_plot")
            self.assertEqual(node["spec"]["field"]["values"], [0.0, 0.5, 1.0])

            send_message({
                "type": "event",
                "id": "replace-field-event",
                "sequence": 1,
                "node_id": "session-plot",
                "event": "click",
                "action": "replace-field",
                "payload": {},
            })
            patch = read_message()
            self.assertEqual(patch["type"], "patch")
            self.assertEqual(patch["revision"], 1)
            self.assertEqual(patch["request_id"], "replace-field-event")
            self.assertEqual(patch["ops"][0]["op"], "replace_mesh_field")
            self.assertEqual(patch["ops"][0]["plot_id"], "session-plot")
            self.assertEqual(patch["ops"][0]["generation"], 2)
            self.assertEqual(patch["ops"][0]["field"]["values"], [1.0, 1.5, 2.0])

            send_message({"type": "shutdown"})
            self.assertEqual(process.wait(timeout=5), 0, process.stderr.read())
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            for stream in (process.stdin, process.stdout, process.stderr):
                if stream is not None:
                    stream.close()

    def test_meshplot_subprocess_session_streams_retained_resources_and_patches(self):
        child = textwrap.dedent(
            """
            from gpui_toolkit.app import App, section
            from gpui_toolkit import meshplot, ui
            from gpui_toolkit.resources import MeshFrameKind, ResourceStore

            store = ResourceStore(4096)
            positions = store.put_mesh_array(
                "session-positions",
                [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                shape=(3, 3), dtype="f64le",
            )
            triangles = store.put_mesh_array(
                "session-triangles", [[0, 1, 2]], shape=(1, 3), dtype="u32le"
            )
            values = store.put_mesh_array(
                "session-values", [0.0, 0.5, 1.0], shape=(3,), dtype="f64le"
            )
            mask = store.put_mesh_array(
                "session-mask", [True, True, True], shape=(3,), dtype="bool_bytes"
            )

            def stream(resource, kind, context):
                store.send_mesh_frames(context, resource, kind, max_frame_bytes=64)

            class MeshResourceSessionApp(App):
                def __init__(self):
                    self.store = store
                    geometry = meshplot.resource_geometry_from_resources(
                        positions, triangles, id="session-resource-mesh"
                    )
                    field = meshplot.resource_field(
                        values.id,
                        values.generation,
                        id="pressure",
                        label="Pressure",
                        unit="Pa",
                        valid_resource_id=mask.id,
                        valid_generation=mask.generation,
                    )
                    self.spec = meshplot.plot(
                        geometry, field, id="session-resource-plot", mode="scalar_fill"
                    )
                    super().__init__(
                        title="MeshPlot resource session",
                        required_capabilities=("meshplot", "mesh_binary_frames", "patches"),
                        sections=[section("main", "Main", ui.mesh_plot(self.spec))],
                    )

                def on_session_ready(self, context):
                    stream(positions, MeshFrameKind.GEOMETRY, context)
                    stream(triangles, MeshFrameKind.GEOMETRY, context)
                    stream(values, MeshFrameKind.FIELD, context)
                    stream(mask, MeshFrameKind.MASK, context)

                def on_action(self, event, context):
                    if event.action == "replace-field":
                        replacement = self.store.put_mesh_array(
                            "session-values", [1.0, 1.5, 2.0], shape=(3,), dtype="f64le"
                        )
                        stream(replacement, MeshFrameKind.FIELD, context)
                        context.replace_mesh_field(
                            "session-resource-plot",
                            2,
                            meshplot.resource_field(
                                replacement.id,
                                replacement.generation,
                                id="pressure",
                                label="Pressure",
                                unit="Pa",
                                valid_resource_id="session-mask",
                                valid_generation=1,
                            ),
                            request_id=event.id,
                        )
                    elif event.action == "replace-geometry":
                        replacement = self.store.put_mesh_array(
                            "session-positions",
                            [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
                            shape=(3, 3), dtype="f64le",
                        )
                        stream(replacement, MeshFrameKind.GEOMETRY, context)
                        context.replace_mesh_geometry(
                            "session-resource-plot",
                            3,
                            meshplot.resource_geometry_from_resources(
                                replacement, triangles, id="session-resource-mesh"
                            ),
                            request_id=event.id,
                        )
                    elif event.action == "set-camera":
                        context.set_mesh_plot_camera(
                            "session-resource-plot",
                            4,
                            {"azimuth": 0.75, "elevation": 0.35, "distance": 3.0},
                            request_id=event.id,
                        )
                    elif event.action == "set-viewport":
                        context.set_mesh_plot_viewport(
                            "session-resource-plot",
                            5,
                            {"x": [0.1, 1.9], "y": [0.2, 1.8]},
                            request_id=event.id,
                        )
                    elif event.action == "set-selection":
                        context.set_mesh_plot_selection(
                            "session-resource-plot",
                            6,
                            {"cell_index": 0, "vertex_id": 10, "displayed_value": 1.5},
                            request_id=event.id,
                        )
                    elif event.action == "reset-view-state":
                        context.reset_mesh_plot_camera(
                            "session-resource-plot", 7, request_id=event.id
                        )
                        context.reset_mesh_plot_viewport(
                            "session-resource-plot", 8, request_id=event.id
                        )
                        context.clear_mesh_plot_selection(
                            "session-resource-plot", 9, request_id=event.id
                        )
                    elif event.action == "future-schema":
                        context.set_mesh_plot_prop(
                            "session-resource-plot",
                            10,
                            "schema_version",
                            2,
                            request_id=event.id,
                        )

            MeshResourceSessionApp().serve()
            """
        )
        package_root = Path(__file__).resolve().parents[1]
        environment = os.environ.copy()
        environment["PYTHONPATH"] = os.pathsep.join(
            value for value in (str(package_root), environment.get("PYTHONPATH")) if value
        )
        process = subprocess.Popen(
            [sys.executable, "-u", "-c", child],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
        )

        def read_bytes(length):
            payload = bytearray()
            while len(payload) < length:
                chunk = read_process_stdout(process, length - len(payload))
                if not chunk:
                    stderr = process.stderr.read()
                    self.fail(f"session exited without bytes: {stderr!r}")
                payload.extend(chunk)
            return bytes(payload)

        def read_line():
            line = bytearray()
            while True:
                chunk = read_process_stdout(process, 1)
                if not chunk:
                    stderr = process.stderr.read()
                    self.fail(f"session exited without line: {stderr!r}")
                if chunk == b"\n":
                    return json.loads(line.decode("utf-8"))
                line.extend(chunk)

        def read_message():
            return read_line()

        def read_mesh_frame():
            header = read_line()
            self.assertEqual(header["type"], "mesh_frame")
            payload = read_bytes(header["byte_length"])
            self.assertEqual(read_bytes(1), b"\n")
            return header, payload

        def send_message(message):
            process.stdin.write((json.dumps(message) + "\n").encode("utf-8"))
            process.stdin.flush()

        try:
            send_message({
                "type": "initialize",
                "session_version": 1,
                "capabilities": ["meshplot", "mesh_binary_frames", "patches"],
            })
            ready = read_message()
            self.assertEqual(ready["type"], "ready")
            self.assertEqual(
                ready["capabilities"], ["mesh_binary_frames", "meshplot", "patches"]
            )

            frames = [read_mesh_frame() for _ in range(5)]
            self.assertEqual(
                [(header["resource_id"], header["kind"], header["generation"]) for header, _ in frames],
                [
                    ("session-positions", "geometry", 1),
                    ("session-positions", "geometry", 1),
                    ("session-triangles", "geometry", 1),
                    ("session-values", "field", 1),
                    ("session-mask", "mask", 1),
                ],
            )
            self.assertEqual(frames[0][0]["chunk_count"], 2)
            self.assertEqual(
                frames[0][1] + frames[1][1],
                b"".join(
                    __import__("struct").pack("<d", value)
                    for value in (0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0)
                ),
            )

            snapshot = read_message()
            self.assertEqual(snapshot["type"], "snapshot")
            node = snapshot["app_ir"]["sections"][0]["content"]
            self.assertEqual(node["spec"]["geometry"]["positions"]["resource_id"], "session-positions")
            self.assertEqual(node["spec"]["geometry"]["triangles"]["resource_id"], "session-triangles")
            self.assertEqual(node["spec"]["field"]["valid"]["resource_id"], "session-mask")

            send_message({
                "type": "event",
                "id": "resource-field-event",
                "sequence": 1,
                "node_id": "session-resource-plot",
                "event": "click",
                "action": "replace-field",
                "payload": {},
            })
            replacement_header, replacement_payload = read_mesh_frame()
            self.assertEqual(replacement_header["resource_id"], "session-values")
            self.assertEqual(replacement_header["generation"], 2)
            self.assertEqual(
                replacement_payload,
                b"".join(__import__("struct").pack("<d", value) for value in (1.0, 1.5, 2.0)),
            )
            field_patch = read_message()
            self.assertEqual(field_patch["request_id"], "resource-field-event")
            self.assertEqual(field_patch["revision"], 1)
            self.assertEqual(field_patch["ops"][0]["op"], "replace_mesh_field")
            self.assertEqual(field_patch["ops"][0]["field"]["generation"], 2)

            send_message({
                "type": "event",
                "id": "resource-geometry-event",
                "sequence": 2,
                "node_id": "session-resource-plot",
                "event": "click",
                "action": "replace-geometry",
                "payload": {},
            })
            geometry_header, geometry_payload = read_mesh_frame()
            geometry_chunks = [geometry_payload]
            for _ in range(geometry_header["chunk_count"] - 1):
                next_header, next_payload = read_mesh_frame()
                self.assertEqual(next_header["resource_id"], "session-positions")
                self.assertEqual(next_header["generation"], 2)
                geometry_chunks.append(next_payload)
            geometry_payload = b"".join(geometry_chunks)
            self.assertEqual(geometry_header["resource_id"], "session-positions")
            self.assertEqual(geometry_header["generation"], 2)
            self.assertNotEqual(geometry_payload, frames[0][1] + frames[1][1])
            geometry_patch = read_message()
            self.assertEqual(
                geometry_patch.get("request_id"),
                "resource-geometry-event",
                geometry_patch,
            )
            self.assertEqual(geometry_patch["revision"], 2)
            self.assertEqual(geometry_patch["ops"][0]["op"], "replace_mesh_geometry")
            self.assertEqual(geometry_patch["ops"][0]["geometry"]["positions"]["generation"], 2)

            def send_action(action, sequence):
                send_message({
                    "type": "event",
                    "id": f"{action}-event",
                    "sequence": sequence,
                    "node_id": "session-resource-plot",
                    "event": "click",
                    "action": action,
                    "payload": {},
                })
                patch = read_message()
                self.assertEqual(patch["type"], "patch")
                self.assertEqual(patch["request_id"], f"{action}-event")
                return patch

            camera_patch = send_action("set-camera", 3)
            self.assertEqual(camera_patch["revision"], 3)
            self.assertEqual(camera_patch["ops"][0]["op"], "set_mesh_plot_camera")
            self.assertEqual(camera_patch["ops"][0]["generation"], 4)
            self.assertEqual(camera_patch["ops"][0]["camera"]["azimuth"], 0.75)

            viewport_patch = send_action("set-viewport", 4)
            self.assertEqual(viewport_patch["revision"], 4)
            self.assertEqual(viewport_patch["ops"][0]["op"], "set_mesh_plot_viewport")
            self.assertEqual(viewport_patch["ops"][0]["generation"], 5)
            self.assertEqual(viewport_patch["ops"][0]["viewport"]["x"], [0.1, 1.9])

            selection_patch = send_action("set-selection", 5)
            self.assertEqual(selection_patch["revision"], 5)
            self.assertEqual(selection_patch["ops"][0]["op"], "set_mesh_plot_selection")
            self.assertEqual(selection_patch["ops"][0]["generation"], 6)
            self.assertEqual(selection_patch["ops"][0]["selection"]["vertex_id"], 10)

            send_message({
                "type": "event",
                "id": "reset-view-state-event",
                "sequence": 6,
                "node_id": "session-resource-plot",
                "event": "click",
                "action": "reset-view-state",
                "payload": {},
            })
            reset_camera_patch = read_message()
            reset_viewport_patch = read_message()
            reset_selection_patch = read_message()
            for reset_patch in (reset_camera_patch, reset_viewport_patch, reset_selection_patch):
                self.assertEqual(reset_patch["request_id"], "reset-view-state-event")
            self.assertEqual(
                [reset_camera_patch["revision"], reset_viewport_patch["revision"], reset_selection_patch["revision"]],
                [6, 7, 8],
            )
            self.assertEqual(
                [
                    reset_camera_patch["ops"][0]["op"],
                    reset_viewport_patch["ops"][0]["op"],
                    reset_selection_patch["ops"][0]["op"],
                ],
                [
                    "reset_mesh_plot_camera",
                    "reset_mesh_plot_viewport",
                    "clear_mesh_plot_selection",
                ],
            )
            self.assertEqual(
                [
                    reset_camera_patch["ops"][0]["generation"],
                    reset_viewport_patch["ops"][0]["generation"],
                    reset_selection_patch["ops"][0]["generation"],
                ],
                [7, 8, 9],
            )

            future_schema_patch = send_action("future-schema", 7)
            self.assertEqual(future_schema_patch["revision"], 9)
            self.assertEqual(future_schema_patch["ops"][0]["op"], "set_mesh_plot_prop")
            self.assertEqual(future_schema_patch["ops"][0]["property"], "schema_version")
            self.assertEqual(future_schema_patch["ops"][0]["value"], 2)

            send_message({"type": "shutdown"})
            self.assertEqual(process.wait(timeout=5), 0, process.stderr.read())
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            for stream in (process.stdin, process.stdout, process.stderr):
                if stream is not None:
                    stream.close()

    def test_meshplot_subprocess_session_supports_inline_resource_cross_products(self):
        child = textwrap.dedent(
            """
            from gpui_toolkit.app import App, section
            from gpui_toolkit import meshplot, ui
            from gpui_toolkit.resources import MeshFrameKind, ResourceStore

            store = ResourceStore(4096)
            positions = store.put_mesh_array(
                "cross-positions",
                [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                shape=(3, 3), dtype="f64le",
            )
            triangles = store.put_mesh_array(
                "cross-triangles", [[0, 1, 2]], shape=(1, 3), dtype="u32le"
            )
            values = store.put_mesh_array(
                "cross-values", [0.0, 0.5, 1.0], shape=(3,), dtype="f64le"
            )
            inline_geometry = meshplot.geometry(
                [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                [[0, 1, 2]],
                id="cross-inline-mesh",
            )
            resource_geometry = meshplot.resource_geometry_from_resources(
                positions, triangles, id="cross-resource-mesh"
            )
            inline_field = meshplot.scalar_field(
                [0.0, 0.5, 1.0], id="cross-inline-field", label="Inline"
            )
            resource_field = meshplot.resource_field(
                values.id, values.generation, id="cross-resource-field", label="Resource"
            )

            class CrossProductApp(App):
                def __init__(self):
                    self.inline_geometry_spec = meshplot.plot(
                        inline_geometry, resource_field, id="cross-inline-geometry"
                    )
                    self.resource_geometry_spec = meshplot.plot(
                        resource_geometry, inline_field, id="cross-resource-geometry"
                    )
                    super().__init__(
                        title="MeshPlot cross products",
                        required_capabilities=("meshplot", "mesh_binary_frames", "patches"),
                        sections=[
                            section("inline", "Inline geometry", ui.mesh_plot(self.inline_geometry_spec)),
                            section("resource", "Resource geometry", ui.mesh_plot(self.resource_geometry_spec)),
                        ],
                    )

                def on_session_ready(self, context):
                    for resource, kind in (
                        (positions, MeshFrameKind.GEOMETRY),
                        (triangles, MeshFrameKind.GEOMETRY),
                        (values, MeshFrameKind.FIELD),
                    ):
                        store.send_mesh_frames(context, resource, kind, max_frame_bytes=4096)

            CrossProductApp().serve()
            """
        )
        package_root = Path(__file__).resolve().parents[1]
        environment = os.environ.copy()
        environment["PYTHONPATH"] = os.pathsep.join(
            value for value in (str(package_root), environment.get("PYTHONPATH")) if value
        )
        process = subprocess.Popen(
            [sys.executable, "-u", "-c", child],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
        )

        def read_line():
            line = bytearray()
            while True:
                chunk = read_process_stdout(process, 1)
                if not chunk:
                    stderr = process.stderr.read()
                    self.fail(f"cross-product session exited without output: {stderr!r}")
                if chunk == b"\n":
                    return json.loads(line.decode("utf-8"))
                line.extend(chunk)

        def read_bytes(length):
            payload = bytearray()
            while len(payload) < length:
                chunk = read_process_stdout(process, length - len(payload))
                if not chunk:
                    stderr = process.stderr.read()
                    self.fail(f"cross-product session exited during frame: {stderr!r}")
                payload.extend(chunk)
            return bytes(payload)

        def read_frame():
            header = read_line()
            self.assertEqual(header["type"], "mesh_frame")
            payload = read_bytes(header["byte_length"])
            self.assertEqual(read_bytes(1), b"\n")
            return header, payload

        try:
            process.stdin.write(
                (
                    json.dumps(
                        {
                            "type": "initialize",
                            "session_version": 1,
                            "capabilities": ["meshplot", "mesh_binary_frames", "patches"],
                        }
                    )
                    + "\n"
                ).encode("utf-8")
            )
            process.stdin.flush()
            ready = read_line()
            self.assertEqual(ready["type"], "ready")
            for expected_id in ("cross-positions", "cross-triangles", "cross-values"):
                header, _ = read_frame()
                self.assertEqual(header["resource_id"], expected_id)
            snapshot = read_line()
            self.assertEqual(snapshot["type"], "snapshot")
            nodes = [section["content"] for section in snapshot["app_ir"]["sections"]]
            inline_node, resource_node = nodes
            self.assertEqual(
                inline_node["spec"]["geometry"]["id"], "cross-inline-mesh"
            )
            self.assertEqual(
                inline_node["spec"]["field"]["resource_id"], "cross-values"
            )
            self.assertEqual(
                resource_node["spec"]["geometry"]["positions"]["resource_id"],
                "cross-positions",
            )
            self.assertEqual(
                resource_node["spec"]["field"]["values"], [0.0, 0.5, 1.0]
            )
            process.stdin.write(b'{"type":"shutdown"}\n')
            process.stdin.flush()
            self.assertEqual(process.wait(timeout=5), 0, process.stderr.read())
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            for stream in (process.stdin, process.stdout, process.stderr):
                if stream is not None:
                    stream.close()

    def test_meshplot_subprocess_session_recovers_from_malformed_messages_and_shuts_down(self):
        child = textwrap.dedent(
            """
            from gpui_toolkit.app import App, section
            from gpui_toolkit import meshplot, ui

            geometry = meshplot.geometry(
                [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                [[0, 1, 2]],
                id="recovery-mesh",
            )

            class RecoveryApp(App):
                def __init__(self):
                    self.spec = meshplot.plot(
                        geometry,
                        meshplot.scalar_field([0.0, 0.5, 1.0], id="recovery-field"),
                        id="recovery-plot",
                    )
                    super().__init__(
                        title="MeshPlot recovery",
                        required_capabilities=("meshplot", "patches"),
                        sections=[section("main", "Main", ui.mesh_plot(self.spec))],
                    )

                def on_session_shutdown(self, context):
                    context.error(None, "shutdown_observed", "recovery session shut down")

            RecoveryApp().serve()
            """
        )
        package_root = Path(__file__).resolve().parents[1]
        environment = os.environ.copy()
        environment["PYTHONPATH"] = os.pathsep.join(
            value for value in (str(package_root), environment.get("PYTHONPATH")) if value
        )
        process = subprocess.Popen(
            [sys.executable, "-u", "-c", child],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
        )

        def read_message():
            line = bytearray()
            while True:
                chunk = read_process_stdout(process, 1)
                if not chunk:
                    stderr = process.stderr.read()
                    self.fail(f"recovery session exited without output: {stderr!r}")
                if chunk == b"\n":
                    return json.loads(line.decode("utf-8"))
                line.extend(chunk)

        try:
            process.stdin.write(
                (
                    json.dumps({
                        "type": "initialize",
                        "session_version": 1,
                        "capabilities": ["meshplot", "patches"],
                    })
                    + "\n"
                ).encode("utf-8")
            )
            process.stdin.flush()
            ready = read_message()
            snapshot = read_message()
            self.assertEqual(ready["type"], "ready")
            self.assertEqual(snapshot["type"], "snapshot")

            process.stdin.write(b"not-json\n")
            process.stdin.flush()
            malformed_json = read_message()
            self.assertEqual(malformed_json["type"], "error")
            self.assertEqual(malformed_json["code"], "malformed_message")
            self.assertIsNone(malformed_json["request_id"])

            process.stdin.write(b"[\"not an object\"]\n")
            process.stdin.flush()
            malformed_object = read_message()
            self.assertEqual(malformed_object["type"], "error")
            self.assertEqual(malformed_object["code"], "malformed_message")

            process.stdin.write(b'{"type":"heartbeat","id":"recovery-heartbeat"}\n')
            process.stdin.flush()
            self.assertEqual(
                read_message(), {"type": "heartbeat", "id": "recovery-heartbeat"}
            )
            self.assertIsNone(process.poll(), "malformed messages must not terminate the session")

            process.stdin.write(b'{"type":"shutdown"}\n')
            process.stdin.flush()
            shutdown = read_message()
            self.assertEqual(shutdown["type"], "error")
            self.assertEqual(shutdown["code"], "shutdown_observed")
            self.assertEqual(process.wait(timeout=5), 0, process.stderr.read())
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            for stream in (process.stdin, process.stdout, process.stderr):
                if stream is not None:
                    stream.close()


if __name__ == "__main__":
    unittest.main()
