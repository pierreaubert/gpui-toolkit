import io
import json
import os
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from threading import Event

from gpui_toolkit.app import MAX_SESSION_MESSAGE_BYTES, Event as AppEvent, SessionContext
from pathlib import Path

from gpui_toolkit.state import Computed, State, StateError, StateStore, ValidationResult, ValidationSeverity, application_data_dir


class StateStoreTests(unittest.TestCase):

    def test_state_binding_and_computed_values_are_json_safe(self):
        frequency = State(20.0)
        changes = []
        unsubscribe = frequency.subscribe(lambda value, revision: changes.append((value, revision)))
        binding = frequency.bind()
        binding.set(40.0)
        binding.set(40.0)
        doubled = Computed(lambda: frequency.value * 2.0)
        self.assertEqual(binding.to_spec(), 40.0)
        self.assertEqual(doubled.bind().to_spec(), 80.0)
        self.assertEqual(changes, [(40.0, 1)])
        unsubscribe()

    def test_validation_result_is_typed_and_serializable(self):
        result = ValidationResult(ValidationSeverity.ERROR, "range", "Frequency must be positive")
        self.assertFalse(result.valid)
        self.assertEqual(result.to_spec()["severity"], "error")

    def test_application_model_can_own_mutable_domain_state(self):
        from gpui_toolkit import App

        app = App(title="Before")
        app.title = "After"
        self.assertEqual(app.title, "After")
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.old_root = os.environ.get("GPUI_TOOLKIT_DATA_DIR")
        os.environ["GPUI_TOOLKIT_DATA_DIR"] = self.temp.name

    def tearDown(self):
        if self.old_root is None:
            os.environ.pop("GPUI_TOOLKIT_DATA_DIR", None)
        else:
            os.environ["GPUI_TOOLKIT_DATA_DIR"] = self.old_root
        self.temp.cleanup()

    def test_save_is_versioned_and_atomic(self):
        store = StateStore("org.example.sim")
        store.save({"gain": 3.0}, version=2)
        loaded = store.load(version=2)
        self.assertEqual(loaded.version, 2)
        self.assertEqual(loaded.state, {"gain": 3.0})
        self.assertFalse(list(store.directory.glob("*.tmp")))

    def test_migration_does_not_overwrite_original_on_failure(self):
        store = StateStore("org.example.sim")
        store.save({"old": True}, version=1)
        original = store.path.read_bytes()
        with self.assertRaises(StateError):
            store.load(version=2, migrate=lambda *_: (_ for _ in ()).throw(ValueError("bad")))
        self.assertEqual(store.path.read_bytes(), original)

    def test_ids_cannot_escape_data_root(self):
        with self.assertRaises(ValueError):
            application_data_dir("../outside")
        self.assertTrue(application_data_dir("org.example.sim").is_dir())

    def test_background_job_reports_cooperative_cancellation(self):
        class RecordingContext(SessionContext):
            def __init__(self):
                super().__init__()
                self.messages = []

            def send(self, message):
                self.messages.append(message)

        context = RecordingContext()
        finished = Event()

        def worker(token):
            while not token.cancelled:
                token._cancelled.wait(0.01)
            finished.set()

        context.spawn_job("solve", worker)
        self.assertTrue(context.cancel_job("solve"))
        self.assertTrue(finished.wait(1))
        states = [message["state"] for message in context.messages if message["type"] == "job"]
        self.assertIn("cancelling", states)
        self.assertIn("cancelled", states)

    def test_resource_tags_serialize_jobs_without_blocking_submission(self):
        class RecordingContext(SessionContext):
            def __init__(self):
                super().__init__()
                self.messages = []

            def send(self, message):
                self.messages.append(message)

        context = RecordingContext()
        context.set_resource_limit("gpu", 1)
        first_started = Event()
        release_first = Event()
        second_started = Event()

        def first(_token):
            first_started.set()
            release_first.wait(1)

        def second(_token):
            second_started.set()

        context.spawn_job("first", first, resource_tags=["gpu"])
        self.assertTrue(first_started.wait(1))
        context.spawn_job("second", second, resource_tags=["gpu"])
        self.assertFalse(second_started.wait(0.1))
        release_first.set()
        self.assertTrue(second_started.wait(1))

    def test_process_job_streams_stdout_and_stderr_without_a_shell(self):
        class RecordingContext(SessionContext):
            def __init__(self):
                super().__init__()
                self.messages = []

            def send(self, message):
                self.messages.append(message)

        context = RecordingContext()
        finished = Event()
        original_job = context.job

        def record_terminal(id, state, **values):
            original_job(id, state, **values)
            if state in {"succeeded", "failed", "cancelled"}:
                finished.set()

        context.job = record_terminal
        context.spawn_process_job(
            "solver",
            [sys.executable, "-c", "import sys; print('progress'); print('diagnostic', file=sys.stderr)"],
        )
        self.assertTrue(finished.wait(5))
        logs = [message["line"] for message in context.messages if message["type"] == "job_log"]
        self.assertIn({"severity": "info", "message": "progress"}, logs)
        self.assertIn({"severity": "error", "message": "diagnostic"}, logs)
        states = [message["state"] for message in context.messages if message["type"] == "job"]
        self.assertIn("succeeded", states)

    def test_process_job_terminates_cooperatively_when_cancelled(self):
        class RecordingContext(SessionContext):
            def __init__(self):
                super().__init__()
                self.messages = []
                self.started = Event()
                self.finished = Event()

            def send(self, message):
                self.messages.append(message)
                if message["type"] == "job_log" and message["line"]["message"] == "ready":
                    self.started.set()
                if message["type"] == "job" and message["state"] in {"cancelled", "failed", "succeeded"}:
                    self.finished.set()

        context = RecordingContext()
        context.spawn_process_job(
            "long-solver",
            [sys.executable, "-c", "import time; print('ready', flush=True); time.sleep(30)"],
        )
        self.assertTrue(context.started.wait(5))
        self.assertTrue(context.cancel_job("long-solver"))
        self.assertTrue(context.finished.wait(5))
        states = [message["state"] for message in context.messages if message["type"] == "job"]
        self.assertIn("cancelling", states)
        self.assertEqual(states[-1], "cancelled")

    def test_job_history_is_serializable_and_restorable(self):
        class RecordingContext(SessionContext):
            def __init__(self):
                super().__init__()
                self.messages = []

            def send(self, message):
                self.messages.append(message)

        context = RecordingContext()
        context.job("remote-solve", "running", completed=4, total=10, message="fetching")
        saved = context.job_history()
        self.assertEqual(saved, [{"id": "remote-solve", "state": "running", "completed": 4, "total": 10, "message": "fetching"}])
        restored = RecordingContext()
        restored.restore_job_history(saved)
        self.assertEqual(restored.job_history(), saved)
        with self.assertRaises(ValueError):
            restored.restore_job_history([{"id": "bad", "state": "unknown"}])

    def test_credential_store_helper_uses_typed_effect_arguments(self):
        class RecordingContext(SessionContext):
            def __init__(self):
                super().__init__()
                self.messages = []

            def send(self, message):
                self.messages.append(message)

        context = RecordingContext()
        context.credential_store("store-token", "remote-token", secret="not-rendered")
        context.credential_store("delete-token", "remote-token", delete=True)
        self.assertEqual(context.messages[0]["effect"], "credential_store")
        self.assertEqual(context.messages[0]["arguments"], {"operation": "store", "reference": "remote-token", "secret": "not-rendered"})
        self.assertEqual(context.messages[1]["arguments"], {"operation": "delete", "reference": "remote-token"})
        with self.assertRaises(ValueError):
            context.credential_store("bad", "", secret="x")

    def test_chart_series_helpers_emit_stable_patch_operations(self):
        class RecordingContext(SessionContext):
            def __init__(self):
                super().__init__()
                self.operations = []

            def patch(self, ops):
                self.operations.extend(ops)

        context = RecordingContext()
        context.append_chart_series("response", "measured", [20.0], [0.0])
        context.replace_chart_series(
            "response", {"id": "target", "x": [20.0], "y": [0.0]}
        )
        self.assertEqual(context.operations[0]["op"], "append_chart_series")
        self.assertEqual(context.operations[0]["series_id"], "measured")
        self.assertEqual(context.operations[1]["op"], "replace_chart_series")
        with self.assertRaises(ValueError):
            context.append_chart_series("response", "measured", [20.0], [])

    def test_action_outcomes_are_correlated_with_event_ids(self):
        class RecordingContext(SessionContext):
            def __init__(self):
                super().__init__()
                self.messages = []

            def send(self, message):
                self.messages.append(message)

        context = RecordingContext()
        older = AppEvent("evt-1", 1, "gain", "change", "set-gain", {"value": 1})
        newer = AppEvent("evt-2", 2, "gain", "change", "set-gain", {"value": 2})
        context.acknowledge(older)
        context.supersede(older, newer)
        context.reject(newer, "out_of_range", "Gain must be below 12 dB")
        self.assertEqual(
            context.messages,
            [
                {"type": "acknowledged", "request_id": "evt-1"},
                {"type": "superseded", "request_id": "evt-1", "superseded_by": "evt-2"},
                {
                    "type": "rejected",
                    "request_id": "evt-2",
                    "code": "out_of_range",
                    "message": "Gain must be below 12 dB",
                },
            ],
        )

    def test_patches_can_be_correlated_with_their_source_event(self):
        context = SessionContext()
        output = io.StringIO()
        with redirect_stdout(output):
            context.patch([{"op": "set", "id": "gain", "property": "value", "value": 2}], request_id="evt-1")
        message = json.loads(output.getvalue())
        self.assertEqual(message["request_id"], "evt-1")
        self.assertEqual(message["revision"], 1)

    def test_oversized_session_messages_are_rejected_before_stdout(self):
        context = SessionContext()
        with self.assertRaisesRegex(ValueError, "exceeds"):
            context.send({"type": "log", "message": "x" * MAX_SESSION_MESSAGE_BYTES})


if __name__ == "__main__":
    unittest.main()
