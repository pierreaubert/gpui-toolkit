"""In-process (embedded) session transport tests.

Mobile hosts embed CPython and drive ``App.serve`` over injected streams
instead of spawning a desktop child process over stdio. These tests pin
that contract: same session-v1 wire messages, no ``sys.stdin``/``sys.stdout``
involved unless the caller leaves the streams at their defaults.
"""

import contextlib
import io
import json
import os
import sys
import unittest
from unittest.mock import patch

from gpui_toolkit.app import App, Event, SessionContext, section


class EmbeddedApp(App):
    def on_action(self, event: Event, context: SessionContext) -> None:
        context.patch(
            [{"op": "set", "id": "label", "property": "text", "value": "tapped"}],
            request_id=event.id,
        )


def build_app() -> EmbeddedApp:
    return EmbeddedApp(
        title="Embedded Test",
        sections=[section("main", "Main", {"kind": "text", "id": "label", "text": "hi"})],
        serial_reducer=True,
    )


def session_input(*messages: dict) -> bytes:
    return "".join(json.dumps(message) + "\n" for message in messages).encode()


INITIALIZE = {"type": "initialize", "session_version": 1, "capabilities": ["events", "patches"]}
SHUTDOWN = {"type": "shutdown", "reason": "test"}


class EmbeddedServeTests(unittest.TestCase):
    def test_serve_runs_full_session_over_injected_binary_streams(self):
        incoming = io.BytesIO(
            session_input(
                INITIALIZE,
                {
                    "type": "event",
                    "id": "event-1",
                    "node_id": "button",
                    "event": "click",
                    "action": "tap",
                    "payload": {},
                    "sequence": 1,
                },
                SHUTDOWN,
            )
        )
        outgoing = io.BytesIO()
        stdout = io.StringIO()

        with contextlib.redirect_stdout(stdout):
            build_app().serve(input_stream=incoming, output_stream=outgoing)

        messages = [json.loads(line) for line in outgoing.getvalue().splitlines()]
        self.assertEqual([message["type"] for message in messages], ["ready", "snapshot", "patch"])
        self.assertEqual(messages[1]["app_ir"]["title"], "Embedded Test")
        self.assertEqual(messages[2]["request_id"], "event-1")
        self.assertEqual(messages[2]["ops"], [{"op": "set", "id": "label", "property": "text", "value": "tapped"}])
        self.assertEqual(stdout.getvalue(), "")

    def test_binary_frames_go_to_the_injected_output_stream(self):
        outgoing = io.BytesIO()
        stdout = io.StringIO()
        context = SessionContext(output=outgoing)

        with contextlib.redirect_stdout(stdout):
            context.dataset_frame(
                resource_id="wave",
                generation=1,
                sequence=0,
                chunk_count=1,
                schema_fingerprint="test",
                payload=b"arrow-ish",
            )

        frame = outgoing.getvalue()
        header, payload, trailing = frame.split(b"\n", 2)
        self.assertEqual(json.loads(header)["type"], "dataset_frame")
        self.assertEqual(payload, b"arrow-ish")
        self.assertEqual(trailing, b"")
        self.assertEqual(stdout.getvalue(), "")

    def test_run_serves_instead_of_execing_a_host_when_embedded(self):
        incoming = io.StringIO(
            "".join(
                json.dumps(message) + "\n" for message in (INITIALIZE, SHUTDOWN)
            )
        )
        outgoing = io.StringIO()

        with (
            patch.dict(os.environ, {"GPUI_TOOLKIT_EMBEDDED_SESSION": "1"}),
            patch.object(sys, "stdin", incoming),
            contextlib.redirect_stdout(outgoing),
        ):
            build_app().run()

        messages = [json.loads(line) for line in outgoing.getvalue().splitlines()]
        self.assertEqual([message["type"] for message in messages], ["ready", "snapshot"])


if __name__ == "__main__":
    unittest.main()
