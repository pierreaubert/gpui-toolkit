import unittest

from gpui_toolkit import meshplot, ui
from gpui_toolkit.app import SessionContext, _negotiate_capabilities
from gpui_toolkit.resources import (
    MeshDtype,
    MeshFrame,
    MeshFrameKind,
    ResourceKind,
    ResourceStore,
    StaleResourceError,
)


class MeshPlotProtocolTests(unittest.TestCase):
    def test_inline_geometry_field_and_selection_round_trip(self):
        geometry = meshplot.geometry([[0, 0, 0], [1, 0, 0], [0, 1, 0]], [[0, 1, 2]])
        field = meshplot.scalar_field([0.0, 1.0, 0.5])
        node = ui.mesh_plot(meshplot.plot(geometry, field), selection_action="select")
        spec = node.to_spec()
        self.assertEqual(spec["kind"], "mesh_plot")
        self.assertEqual(spec["selection_action"], "select")
        self.assertEqual(spec["spec"]["field"]["association"], "vertex")

    def test_resource_geometry_ids_field_and_mask_round_trip(self):
        store = ResourceStore(512)
        positions = store.put_mesh_array(
            "positions", [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            shape=(3, 3), dtype="f64le"
        )
        triangles = store.put_mesh_array(
            "triangles", [[0, 1, 2]], shape=(1, 3), dtype="u32le"
        )
        vertex_ids = store.put_mesh_array(
            "vertex_ids", [10, 20, 30], shape=(3,), dtype="u64le"
        )
        cell_ids = store.put_mesh_array("cell_ids", [99], shape=(1,), dtype="u64le")
        values = store.put_mesh_array(
            "values", [1.0, 2.0, 3.0], shape=(3,), dtype="f64le"
        )
        mask = store.put_mesh_array(
            "mask", [True, False, True], shape=(3,), dtype="bool_packed"
        )
        geometry = meshplot.resource_geometry_from_resources(
            positions, triangles, vertex_ids_resource=vertex_ids, cell_ids_resource=cell_ids
        )
        field = meshplot.resource_field(
            values.id, values.generation, valid_resource_id=mask.id, valid_generation=mask.generation
        )
        spec = meshplot.plot(geometry, field, mode="scalar_fill").to_spec()

        self.assertEqual(spec["geometry"]["positions"]["dtype"], "f64le")
        self.assertEqual(spec["geometry"]["triangles"]["dtype"], "u32le")
        self.assertEqual(spec["geometry"]["vertex_ids"]["dtype"], "u64le")
        self.assertEqual(spec["geometry"]["cell_ids"]["resource_id"], "cell_ids")
        self.assertEqual(spec["field"]["valid"]["dtype"], "bool_bytes")

    def test_binary_mesh_frame_round_trip_preserves_chunk_order_and_metadata(self):
        store = ResourceStore(1024)
        resource = store.put_mesh_array(
            "values", [1.0, 2.0, 3.0], shape=(3,), dtype=MeshDtype.F64LE
        )
        frames = list(store.iter_mesh_frames(resource, MeshFrameKind.FIELD, max_frame_bytes=16))
        self.assertEqual([header["sequence"] for header, _ in frames], [0, 1])
        decoded = [MeshFrame.decode(header, payload) for header, payload in frames]
        self.assertEqual([frame.kind for frame in decoded], [MeshFrameKind.FIELD] * 2)
        self.assertEqual(b"".join(frame.payload for frame in decoded), store.read(resource))
        encoded = decoded[0].encode()
        header_end = encoded.index(b"\n")
        self.assertEqual(
            MeshFrame.decode(encoded[:header_end], encoded[header_end + 1 : -1]), decoded[0]
        )

    def test_malformed_frame_shapes_and_future_metadata_are_rejected(self):
        frame = MeshFrame(
            resource_id="field",
            generation=1,
            sequence=0,
            chunk_count=1,
            kind=MeshFrameKind.FIELD,
            dtype=MeshDtype.F32LE,
            shape=(2,),
            payload=b"\x00\x00\x80\x3f",
        )
        with self.assertRaises(ValueError):
            frame.validate()
        with self.assertRaises(ValueError):
            MeshFrame.decode({"type": "future_mesh_frame"}, b"")

    def test_mesh_patch_helpers_emit_correlated_revision_and_capability_negotiation(self):
        self.assertEqual(
            _negotiate_capabilities(["meshplot", "patches", "events"], ["meshplot"]),
            ["events", "meshplot", "patches"],
        )
        context = SessionContext()
        import contextlib
        import io

        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            context.replace_mesh_field("plot", 4, {"values": [1.0]}, request_id="event-4")
        message = __import__("json").loads(output.getvalue())
        self.assertEqual(message["request_id"], "event-4")
        self.assertEqual(message["ops"][0]["generation"], 4)
        with self.assertRaises(ValueError):
            context.replace_mesh_field("plot", 0, {"values": [0.0]})

    def test_oversized_inline_payload_requires_retained_resources(self):
        geometry = meshplot.geometry(
            [[float(index), 0.0, 0.0] for index in range(30_000)], [], id="large"
        )
        with self.assertRaisesRegex(ValueError, "ResourceStore"):
            meshplot.plot(geometry).to_spec()

    def test_rapid_patch_revisions_and_eviction_drop_old_mesh_handles(self):
        context = SessionContext()
        import contextlib
        import io

        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            context.set_mesh_plot_prop("plot", 1, "mode", "mesh")
            context.set_mesh_plot_prop("plot", 2, "mode", "scalar_fill")
        messages = [__import__("json").loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual([message["revision"] for message in messages], [1, 2])

        store = ResourceStore(8)
        first = store.put("first", b"12345678", kind=ResourceKind.MESH)
        store.put("second", b"abcdefgh", kind=ResourceKind.MESH)
        with self.assertRaises(StaleResourceError):
            store.read(first)


if __name__ == "__main__":
    unittest.main()
