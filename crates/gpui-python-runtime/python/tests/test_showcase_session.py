import contextlib
import io
import json
import time
import unittest

from gpui_toolkit import Event, SessionContext
from showcase import RuntimeShowcase


class RuntimeShowcaseSessionTests(unittest.TestCase):
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
            time.sleep(0.55)

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


if __name__ == "__main__":
    unittest.main()
