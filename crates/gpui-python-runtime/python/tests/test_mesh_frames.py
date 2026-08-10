import io
import json
import sys
import unittest
from unittest.mock import patch

from gpui_toolkit.app import (
    PYTHON_SESSION_CAPABILITIES,
    SessionContext,
    _negotiate_capabilities,
)
from gpui_toolkit.resources import (
    MAX_MESH_FRAME_BYTES,
    MeshDtype,
    MeshFrameKind,
    ResourceStore,
)


class _BinaryStdout:
    def __init__(self) -> None:
        self.buffer = io.BytesIO()

    def flush(self) -> None:
        pass


class MeshFrameTransportTests(unittest.TestCase):
    def test_session_context_writes_json_header_and_exact_payload(self):
        stdout = _BinaryStdout()
        payload = b"0123456789abcdef"  # two little-endian f64 values
        with patch.object(sys, "stdout", stdout):
            SessionContext().mesh_frame(
                {
                    "resource_id": "geometry",
                    "generation": 1,
                    "sequence": 0,
                    "chunk_count": 1,
                    "kind": "geometry",
                    "dtype": "f64le",
                    "shape": [2],
                },
                payload,
            )

        encoded = stdout.buffer.getvalue()
        header, framed_payload = encoded.split(b"\n", 1)
        self.assertEqual(json.loads(header)["type"], "mesh_frame")
        self.assertEqual(json.loads(header)["byte_length"], len(payload))
        self.assertEqual(framed_payload[:-1], payload)

    def test_mesh_capabilities_are_negotiated_and_advertised(self):
        self.assertIn("meshplot", PYTHON_SESSION_CAPABILITIES)
        self.assertIn("mesh_binary_frames", PYTHON_SESSION_CAPABILITIES)
        negotiated = _negotiate_capabilities(
            ["meshplot", "mesh_binary_frames", "events"],
            ["meshplot", "mesh_binary_frames"],
        )
        self.assertEqual(negotiated, ["events", "mesh_binary_frames", "meshplot"])

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
