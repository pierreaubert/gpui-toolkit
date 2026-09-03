import io
import json
import sys
import unittest
from unittest.mock import patch

from gpui_toolkit.app import (
    App,
    MeshFrameAcknowledgement,
    PYTHON_SESSION_CAPABILITIES,
    ResourceBackpressureError,
    Section,
    SessionContext,
    _negotiate_capabilities,
)
from gpui_toolkit.resources import (
    MAX_MESH_FRAME_BYTES,
    MeshDtype,
    MeshFrame,
    MeshFrameKind,
    ResourceStore,
)


class _BinaryStdout:
    def __init__(self) -> None:
        self.buffer = io.BytesIO()

    def flush(self) -> None:
        pass

    def write(self, value: str) -> int:
        self.buffer.write(value.encode("utf-8"))
        return len(value)


class MeshFrameTransportTests(unittest.TestCase):
    def test_session_context_writes_json_header_and_exact_payload(self):
        stdout = _BinaryStdout()
        payload = b"0123456789abcdef"  # two little-endian f64 values
        frame = MeshFrame(
            resource_id="geometry",
            generation=1,
            sequence=0,
            chunk_count=1,
            kind=MeshFrameKind.GEOMETRY,
            dtype=MeshDtype.F64LE,
            shape=(2,),
            payload=payload,
        )
        with patch.object(sys, "stdout", stdout):
            SessionContext().mesh_frame(frame.header(), payload)

        encoded = stdout.buffer.getvalue()
        header, framed_payload = encoded.split(b"\n", 1)
        self.assertEqual(json.loads(header)["type"], "mesh_frame")
        self.assertEqual(json.loads(header)["byte_length"], len(payload))
        self.assertEqual(json.loads(header)["checksum"], frame.checksum)
        self.assertEqual(framed_payload[:-1], payload)

    def test_mesh_capabilities_are_negotiated_and_advertised(self):
        self.assertIn("meshplot", PYTHON_SESSION_CAPABILITIES)
        self.assertIn("mesh_binary_frames", PYTHON_SESSION_CAPABILITIES)
        self.assertIn("mesh_frame_ack", PYTHON_SESSION_CAPABILITIES)
        negotiated = _negotiate_capabilities(
            ["meshplot", "mesh_binary_frames", "events"],
            ["meshplot", "mesh_binary_frames"],
        )
        self.assertEqual(negotiated, ["events", "mesh_binary_frames", "meshplot"])

    def test_mesh_frame_decode_rejects_missing_and_mismatched_checksums(self):
        payload = b"\x00" * 8
        frame = MeshFrame(
            resource_id="field",
            generation=1,
            sequence=0,
            chunk_count=1,
            kind=MeshFrameKind.FIELD,
            dtype=MeshDtype.F64LE,
            shape=(1,),
            payload=payload,
        )
        header = frame.header()
        self.assertEqual(MeshFrame.decode(header, payload), frame)
        del header["checksum"]
        with self.assertRaisesRegex(ValueError, "checksum"):
            MeshFrame.decode(header, payload)
        header["checksum"] = frame.checksum + 1
        with self.assertRaisesRegex(ValueError, "checksum"):
            MeshFrame.decode(header, payload)

    def test_mesh_acknowledgements_release_shared_backpressure_budget(self):
        stdout = _BinaryStdout()
        payload = b"\x00\x00\x00\x00\x00\x00\xf0?"
        frame = MeshFrame(
            resource_id="field",
            generation=2,
            sequence=0,
            chunk_count=1,
            kind=MeshFrameKind.FIELD,
            dtype=MeshDtype.F64LE,
            shape=(1,),
            payload=payload,
        )
        context = SessionContext(max_outstanding_resource_bytes=10)
        with patch.object(sys, "stdout", stdout):
            context.mesh_frame(frame.header(), payload)
            self.assertEqual(context.outstanding_resource_bytes, 8)
            with self.assertRaises(ResourceBackpressureError):
                context.dataset_frame(
                    resource_id="table",
                    generation=1,
                    sequence=0,
                    chunk_count=1,
                    schema_fingerprint="schema",
                    payload=b"abc",
                )
            with self.assertRaisesRegex(ValueError, "already outstanding"):
                context.mesh_frame(frame.header(), payload)

        acknowledgement = context._acknowledge_mesh_frame(
            {
                "resource_id": "field",
                "generation": 2,
                "sequence": 0,
                "byte_length": 8,
                "complete": True,
                "accepted": True,
                "error": None,
            }
        )
        self.assertIsInstance(acknowledgement, MeshFrameAcknowledgement)
        self.assertEqual(context.outstanding_resource_bytes, 0)

    def test_rejected_mesh_ack_is_typed_recorded_and_length_checked(self):
        stdout = _BinaryStdout()
        payload = b"\x00" * 8
        frame = MeshFrame(
            resource_id="field",
            generation=3,
            sequence=0,
            chunk_count=1,
            kind=MeshFrameKind.FIELD,
            dtype=MeshDtype.F64LE,
            shape=(1,),
            payload=payload,
        )
        context = SessionContext(max_outstanding_resource_bytes=8)
        with patch.object(sys, "stdout", stdout):
            context.mesh_frame(frame.header(), payload)
        wire = {
            "resource_id": "field",
            "generation": 3,
            "sequence": 0,
            "byte_length": 7,
            "complete": False,
            "accepted": False,
            "error": "checksum mismatch",
        }
        with self.assertRaisesRegex(ValueError, "byte_length mismatch"):
            context._acknowledge_mesh_frame(wire)
        self.assertEqual(context.outstanding_resource_bytes, 8)

        wire["byte_length"] = 8
        rejected = context._acknowledge_mesh_frame(wire)
        self.assertIsInstance(rejected, MeshFrameAcknowledgement)
        self.assertFalse(rejected.accepted)
        self.assertEqual(context.mesh_frame_rejections, (rejected,))
        self.assertEqual(context.outstanding_resource_bytes, 0)
        with self.assertRaisesRegex(ValueError, "outstanding frame"):
            context._acknowledge_mesh_frame(wire)

    def test_app_session_consumes_mesh_frame_acknowledgements(self):
        payload = b"\x00" * 8
        frame = MeshFrame(
            resource_id="field",
            generation=1,
            sequence=0,
            chunk_count=1,
            kind=MeshFrameKind.FIELD,
            dtype=MeshDtype.F64LE,
            shape=(1,),
            payload=payload,
        )

        class AcknowledgingApp(App):
            outstanding_at_shutdown: int | None = None

            def on_session_ready(self, context: SessionContext) -> None:
                context.mesh_frame(frame.header(), payload)

            def on_session_shutdown(self, context: SessionContext) -> None:
                self.outstanding_at_shutdown = context.outstanding_resource_bytes

        app = AcknowledgingApp(sections=(Section("main", "Main", {}),))
        stream = _BinaryStdout()
        initialize = {
            "type": "initialize",
            "session_version": 1,
            "capabilities": ["mesh_frame_ack"],
        }
        acknowledgement = {
            "type": "mesh_frame_result",
            "resource_id": "field",
            "generation": 1,
            "sequence": 0,
            "byte_length": 8,
            "complete": True,
            "accepted": True,
            "error": None,
        }
        with (
            patch("gpui_toolkit.app.sys.stdout", stream),
            patch("gpui_toolkit.app._read_message", return_value=initialize),
            patch(
                "gpui_toolkit.app._messages",
                return_value=iter((acknowledgement, {"type": "shutdown"})),
            ),
        ):
            app.serve()
        self.assertEqual(app.outstanding_at_shutdown, 0)
        self.assertIn(b'"type":"mesh_frame"', stream.buffer.getvalue())

    def test_typed_arrays_pack_little_endian_mesh_values(self):
        store = ResourceStore(256)
        resource = store.put_mesh_array(
            "positions",
            [[1.0, 2.0], [3.0, 4.0]],
            shape=(2, 2),
            dtype=MeshDtype.F32LE,
        )
        self.assertEqual(resource.dtype, "f32le")
        frames = list(store.iter_mesh_frames(resource, MeshFrameKind.GEOMETRY, max_frame_bytes=8))
        self.assertEqual([header["sequence"] for header, _ in frames], [0, 1])
        self.assertEqual(b"".join(payload for _, payload in frames), store.read(resource))
        self.assertEqual(frames[0][0]["chunk_count"], 2)

    def test_boolean_masks_support_packed_and_byte_formats(self):
        store = ResourceStore(64)
        packed = store.put_mesh_array(
            "packed", [True, False, True, True, False], shape=(5,), dtype="bool_packed"
        )
        byte_mask = store.put_mesh_array(
            "bytes", [True, False, True, True, False], shape=(5,), dtype="bool_bytes"
        )
        self.assertEqual(store.read(packed), b"\x0d")
        self.assertEqual(store.read(byte_mask), b"\x01\x00\x01\x01\x00")

    def test_mesh_frame_sender_rejects_oversized_payloads(self):
        with self.assertRaises(ValueError):
            SessionContext().mesh_frame({}, b"x" * (MAX_MESH_FRAME_BYTES + 1))


if __name__ == "__main__":
    unittest.main()
