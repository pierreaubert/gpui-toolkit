import unittest
import contextlib
import io
import json

from gpui_toolkit import SessionContext
from gpui_toolkit.commands import CommandResult
from gpui_toolkit.d3 import ArrayOperation, ArrayRequest, ScaleKind, ScaleOutput, ScaleRequest, StatisticsOperation, StatisticsRequest, TickOperation, TickRequest, ZoomOperation, ZoomRequest, ZoomResult, reports_from_command, request_reports


class D3ZoomTests(unittest.TestCase):
    def test_zoom_request_is_typed_and_serializable(self):
        request = ZoomRequest(
            (0, 100), (-20, 20),
            [ZoomOperation.zoom_to((20, 80), (-10, 10)), ZoomOperation.back()],
        )
        self.assertEqual(request.to_spec()["operations"][0]["kind"], "zoom_to")
        self.assertEqual(request.to_spec()["operations"][1], {"kind": "back"})

    def test_zoom_result_requires_a_successful_host_response(self):
        result = CommandResult.from_wire(
            "zoom", {"ok": True, "x": [20, 80], "y": [-10, 10], "zoomed": True, "level": 1, "back_results": []},
        )
        self.assertEqual(ZoomResult.from_command(result).x, (20.0, 80.0))
        with self.assertRaises(ValueError):
            ZoomRequest((1, 1), (0, 1))

    def test_array_requests_validate_native_operation_arguments(self):
        request = ArrayRequest(ArrayOperation.BISECT_RIGHT, [1, 2, 2, 4], value=2)
        self.assertEqual(request.to_spec()["operation"], "bisect_right")
        result = CommandResult.from_wire("bisect", {"ok": True, "value": 3})
        self.assertEqual(ArrayRequest.value_from_command(result), 3)
        with self.assertRaises(ValueError):
            ArrayRequest(ArrayOperation.QUANTILE, [1, 2]).to_spec()

    def test_statistics_and_ticks_use_typed_native_commands(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            StatisticsRequest(StatisticsOperation.VARIANCE, [1, 2, 3]).send(SessionContext(), "stats")
        self.assertEqual(json.loads(output.getvalue())["command"], "d3.statistics")
        self.assertEqual(StatisticsRequest.value_from_command(CommandResult.from_wire("stats", {"ok": True, "value": 1.0})), 1.0)
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            TickRequest(TickOperation.LOG, 1, 1000, base=10).send(SessionContext(), "ticks")
        self.assertEqual(json.loads(output.getvalue())["arguments"]["base"], 10)

    def test_parity_and_benchmark_reports_are_typed(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output): request_reports(SessionContext(), "reports")
        self.assertEqual(json.loads(output.getvalue())["command"], "d3.reports")
        reports = reports_from_command(CommandResult.from_wire("reports", {"ok": True,
            "parity": {"entries": [{"id": "array", "d3_area": "d3-array", "gpui_d3rs_modules": "array", "status": "complete", "evidence": "tests", "release_requirement": "none"}], "markdown": "| area |"},
            "benchmark": {"cases": [{"id": "force", "module": "force", "bench_target": "large", "benchmark_group": "force", "benchmark_id": "force-10k", "dataset_scale": "10k", "evidence": "criterion"}], "markdown": "| case |"},
        }))
        self.assertEqual(reports.parity_entries[0].d3_area, "d3-array")
        self.assertEqual(reports.benchmark_cases[0].module, "force")

    def test_continuous_and_categorical_scales_are_typed(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            ScaleRequest(ScaleKind.SYMLOG, [-100, 100], [0, 1], [-10, 0, 10], constant=2).send(SessionContext(), "scale")
        command = json.loads(output.getvalue())
        self.assertEqual(command["command"], "d3.scale")
        self.assertEqual(command["arguments"]["constant"], 2)
        output = ScaleOutput.from_command(CommandResult.from_wire("scale", {"ok": True, "output": {"values": [0.25, 0.5, 0.75], "ticks": [-100, 0, 100]}}))
        self.assertEqual(output.values[1], 0.5)
        band = ScaleOutput.from_command(CommandResult.from_wire("band", {"ok": True, "output": {"values": [0, 50], "bandwidth": 40, "step": 50}}))
        self.assertEqual(band.bandwidth, 40)
