import unittest
import contextlib
import io
import json

from gpui_toolkit import SessionContext
from gpui_toolkit.commands import CommandResult
from gpui_toolkit.d3 import AlgorithmOperation, AlgorithmRequest, AlgorithmResult, ArrayOperation, ArrayRequest, D3BridgeKind, EaseKind, RandomKind, ScaleKind, ScaleOutput, ScaleRequest, StatisticsOperation, StatisticsRequest, TickOperation, TickRequest, ZoomOperation, ZoomRequest, ZoomResult, module_catalog_from_command, reports_from_command, request_module_catalog, request_reports


class D3ZoomTests(unittest.TestCase):
    def test_algorithm_catalog_covers_every_renderer_independent_family(self):
        self.assertEqual(
            {operation.value for operation in AlgorithmOperation},
            {
                "color_interpolate", "color_convert", "format", "format_prefix",
                "time_interval", "time_scale", "csv_parse", "dsv_parse", "dsv_format",
                "interpolate_number", "interpolate_array", "interpolate_string",
                "interpolate_transform_css", "interpolate_transform_svg", "interpolate_zoom",
                "ease", "selection_join", "brush_gesture", "drag_gesture",
                "transition_sample", "random_uniform", "random", "shuffle",
            },
        )
        self.assertEqual(len(EaseKind), 25)
        self.assertEqual(len(RandomKind), 8)

    def test_algorithm_builders_validate_and_serialize_family_arguments(self):
        easing = AlgorithmRequest.easing(EaseKind.ELASTIC_IN_OUT, [0.0, 0.5, 1.0])
        self.assertEqual(easing.arguments["kind"], "elastic_in_out")

        random = AlgorithmRequest.random(
            RandomKind.NORMAL, count=4, seed=7, mean=0.0, deviation=1.0,
        )
        self.assertEqual(random.arguments["seed"], 7)
        self.assertEqual(random.arguments["kind"], "normal")

        dsv = AlgorithmRequest.dsv_parse("name\tvalue\na\t1", "\t")
        self.assertEqual(dsv.operation, AlgorithmOperation.DSV_PARSE)
        formatted = AlgorithmRequest.dsv_format([{"name": "a"}], ["name"])
        self.assertEqual(formatted.arguments["delimiter"], ",")

        join = AlgorithmRequest.selection_join(["a", "b"], ["b", "c"])
        self.assertEqual(join.arguments["new_keys"], ["b", "c"])
        brush = AlgorithmRequest.brush_gesture([(10.0, 20.0), (30.0, 40.0)])
        self.assertEqual(brush.operation, AlgorithmOperation.BRUSH_GESTURE)
        drag = AlgorithmRequest.drag_gesture([(0.0, 0.0), (4.0, 3.0)], click_distance=4.0)
        self.assertEqual(drag.arguments["click_distance"], 4.0)
        transition = AlgorithmRequest.transition_sample(
            0.0, 10.0, 100.0, [25.0, 25.0, 50.0], easing=EaseKind.CUBIC_IN_OUT,
        )
        self.assertEqual(transition.arguments["kind"], "cubic_in_out")

        with self.assertRaises(ValueError):
            AlgorithmRequest.random(RandomKind.UNIFORM, count=-1, seed=1)
        with self.assertRaises(ValueError):
            AlgorithmRequest.dsv_parse("a", "::")

    def test_pure_algorithm_families_use_one_typed_native_command(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            AlgorithmRequest(
                AlgorithmOperation.INTERPOLATE_NUMBER,
                {"start": 0.0, "end": 10.0, "values": [0.0, 0.5, 1.0]},
            ).send(SessionContext(), "algorithm")
        message = json.loads(output.getvalue())
        self.assertEqual(message["command"], "d3.algorithms")
        self.assertEqual(message["arguments"]["operation"], "interpolate_number")

        result = AlgorithmResult.from_command(
            CommandResult.from_wire(
                "algorithm",
                {"ok": True, "operation": "interpolate_number", "value": [0.0, 5.0, 10.0]},
            )
        )
        self.assertEqual(result.value, [0.0, 5.0, 10.0])

    def test_native_module_catalog_covers_algorithm_render_and_interaction_bridges(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            request_module_catalog(SessionContext(), "modules")
        self.assertEqual(json.loads(output.getvalue())["command"], "d3.modules")

        result = CommandResult.from_wire(
            "modules",
            {
                "ok": True,
                "modules": [
                    {"module": "array", "bridge": "direct_command", "python_path": "gpui_toolkit.d3", "evidence": "native"},
                    {"module": "shape", "bridge": "chart_spec", "python_path": "gpui_toolkit.charts", "evidence": "native"},
                    {"module": "zoom", "bridge": "host_interaction", "python_path": "gpui_toolkit.events", "evidence": "native"},
                    {"module": "gpu3d", "bridge": "scene_spec", "python_path": "gpui_toolkit.scene3d", "evidence": "native"},
                ],
            },
        )
        catalog = module_catalog_from_command(result)
        self.assertEqual(catalog.by_name("array").bridge, D3BridgeKind.DIRECT_COMMAND)
        self.assertEqual(catalog.by_name("zoom").bridge, D3BridgeKind.HOST_INTERACTION)

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
