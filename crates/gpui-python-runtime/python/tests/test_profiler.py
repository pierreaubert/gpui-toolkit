import unittest
import contextlib
import io
import json

from gpui_toolkit.profiler import (
    AllocationBudget,
    AllocatorTelemetryMode,
    AllocProbe,
    AllocSnapshot,
    telemetry,
    snapshot_from_command,
    sample_from_wire,
    subscribe,
    unsubscribe,
    request_snapshot,
)
from gpui_toolkit.commands import CommandResult
from gpui_toolkit import SessionContext


class ProfilerTests(unittest.TestCase):
 def test_feature_disabled_shape_matches_native_zero_mode(self):
  probe = AllocProbe()
  self.assertEqual(probe.sample("render"), AllocSnapshot())
  self.assertEqual(probe.telemetry.mode, AllocatorTelemetryMode.ZERO)
  self.assertFalse(probe.telemetry.is_counting)
 def test_zero_mode_is_explicit_not_an_inferred_measurement(self):
  state = telemetry()
  self.assertEqual(state.capability, "gpui-profiler.allocation-snapshots")
  self.assertEqual(state.mode, AllocatorTelemetryMode.ZERO)
 def test_host_snapshot_carries_its_instrumentation_mode(self):
  mode, sample = snapshot_from_command(CommandResult.from_wire("profile", {
   "ok": True, "mode": "counting_allocator", "bytes": 12, "count": 1,
  }))
  self.assertTrue(mode.is_counting)
  self.assertEqual(sample, AllocSnapshot(12, 1))
 def test_budgets_enforce_both_dimensions(self):
  budget = AllocationBudget("render", 1, 8)
  self.assertTrue(budget.contains(AllocSnapshot(8, 1)))
  with self.assertRaises(AssertionError): budget.assert_contains(AllocSnapshot(9, 1))
 def test_subscription_samples_are_typed_and_sequence_checked(self):
  sample = sample_from_wire({"subscription_id": "render", "sequence": 2, "sample": {
   "mode": "counting_allocator", "bytes": 12, "count": 1,
  }})
  self.assertEqual(sample.subscription_id, "render")
  self.assertEqual(sample.snapshot, AllocSnapshot(12, 1))
  with self.assertRaises(ValueError): sample_from_wire({"subscription_id": "", "sequence": 0})
 def test_subscription_rejects_unbounded_rates(self):
  with self.assertRaises(ValueError): subscribe(None, "start", "render", 49)
 def test_subscription_commands_use_explicit_ids_and_bounded_interval(self):
  output = io.StringIO()
  with contextlib.redirect_stdout(output):
   context = SessionContext()
   request_snapshot(context, "snapshot")
   subscribe(context, "start", "render", 500)
   unsubscribe(context, "stop", "render")
  self.assertEqual([json.loads(line) for line in output.getvalue().splitlines()], [{
   "type": "command", "request_id": "snapshot", "command": "profiler.snapshot", "arguments": {},
  }, {
   "type": "command", "request_id": "start", "command": "profiler.subscribe",
   "arguments": {"subscription_id": "render", "interval_ms": 500},
  }, {
   "type": "command", "request_id": "stop", "command": "profiler.unsubscribe",
   "arguments": {"subscription_id": "render"},
  }])
if __name__ == "__main__": unittest.main()
