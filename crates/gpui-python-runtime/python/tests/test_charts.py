import contextlib
import inspect
import io
import json
import unittest

from gpui_toolkit import SessionContext, data, px
from gpui_toolkit import charts
from gpui_toolkit.commands import CommandResult


class StrictChartShimTests(unittest.TestCase):
    def test_family_names_return_v2_builders_without_var_keywords(self) -> None:
        families = (
            charts.scatter,
            charts.line,
            charts.area,
            charts.boxplot,
            charts.heatmap,
            charts.contour,
            charts.isoline,
            charts.surface,
            charts.pie,
            charts.donut,
            charts.bar,
            charts.treemap,
        )
        for family in families:
            signature = inspect.signature(family)
            self.assertFalse(
                any(
                    parameter.kind is inspect.Parameter.VAR_KEYWORD
                    for parameter in signature.parameters.values()
                )
            )
            builder = family()
            self.assertIsInstance(builder, px.ChartBuilder)
            with self.assertRaisesRegex(ValueError, "requires .data"):
                builder.to_spec()

    def test_shims_emit_only_resource_backed_v2_ir(self) -> None:
        rows = data.Dataset.from_mapping(
            {"x": [1.0, 2.0], "y": [3.0, 4.0]},
            id="shim-rows",
        )
        line = charts.line("response").data(rows).x("x").y("y")
        spec = line.to_spec()

        self.assertEqual(spec["kind"], "px_chart_v2")
        self.assertEqual(spec["data"]["source"]["kind"], "dataset")
        self.assertNotIn("values", spec["data"]["source"])
        self.assertIs(charts.Chart, px.ChartBuilder)
        self.assertIs(charts.Annotation, px.Annotation)
        self.assertIs(charts.TreemapNode, px.TreemapNode)

    def test_native_capability_and_visual_reports_are_typed(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            charts.request_reports(SessionContext(), "reports")
        self.assertEqual(json.loads(output.getvalue())["command"], "px.reports")

        reports = charts.reports_from_command(
            CommandResult.from_wire(
                "reports",
                {
                    "ok": True,
                    "capability": {
                        "schema_version": 1,
                        "report_type": "chart-capabilities",
                        "reviewed_on": "2026-08-07",
                        "all_release_ready": False,
                        "entries": [
                            {
                                "id": "line",
                                "capability": "line",
                                "chart_families": ["line"],
                                "story_ids": ["line-basic"],
                                "test_contracts": ["line_test"],
                                "status": "complete",
                                "evidence": "native",
                                "release_requirement": "tests",
                            }
                        ],
                        "markdown": "| capability |",
                    },
                    "visual": {
                        "schema_version": 1,
                        "report_type": "chart-visual-regression",
                        "crate_name": "gpui-px",
                        "crate_version": "0.1.0",
                        "capture_count": 3,
                        "expected_capture_count": 3,
                        "unique_capture_ids": True,
                        "chart_families": ["line"],
                        "markdown": "| capture |",
                    },
                },
            )
        )
        self.assertEqual(reports.capability.entries[0].id, "line")
        self.assertTrue(reports.visual.unique_capture_ids)

    def test_report_request_and_payload_validation_are_strict(self) -> None:
        with self.assertRaisesRegex(ValueError, "non-empty"):
            charts.request_reports(SessionContext(), "")
        with self.assertRaises(RuntimeError):
            charts.reports_from_command(
                CommandResult.from_wire("reports", {"ok": False, "error": "failed"})
            )


if __name__ == "__main__":
    unittest.main()
