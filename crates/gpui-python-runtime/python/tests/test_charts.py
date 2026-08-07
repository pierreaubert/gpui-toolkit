import unittest
import contextlib
import io
import json

from gpui_toolkit import SessionContext
from gpui_toolkit.charts import (
    AnnotationTarget,
    ChartAnnotation,
    CurveType,
    LegendPosition,
    Series,
    StrokeDash,
    TreemapNode,
    area,
    bar,
    boxplot,
    contour,
    donut,
    isoline,
    line,
    pie,
    reports_from_command,
    request_reports,
    treemap,
)
from gpui_toolkit.commands import CommandResult


class ExtendedChartTests(unittest.TestCase):
    def test_line_serializes_full_native_style_and_secondary_axis_surface(self):
        chart = line(
            "response",
            [20.0, 100.0, 1000.0],
            [0.0, -1.0, 2.0],
            curve=CurveType.MONOTONE_X,
            dash=StrokeDash.DASHED,
            legend_position=LegendPosition.BOTTOM,
            y2_label="Phase",
            y2_range=(-180.0, 180.0),
            series=(
                Series(
                    "phase",
                    [20.0, 100.0, 1000.0],
                    [10.0, 20.0, 30.0],
                    opacity=0.5,
                    secondary_y=True,
                    dash=StrokeDash.DASH_DOT,
                ),
            ),
            annotations=(
                ChartAnnotation(
                    "crossover",
                    "Crossover",
                    AnnotationTarget.X_VALUE,
                    x=1000.0,
                ),
            ),
        )

        spec = chart.to_spec()
        self.assertEqual(spec["curve"], "monotone_x")
        self.assertEqual(spec["dash"], "dashed")
        self.assertEqual(spec["legend_position"], "bottom")
        self.assertEqual(spec["y2_range"], [-180.0, 180.0])
        self.assertTrue(spec["series"][0]["secondary_y"])
        self.assertEqual(spec["series"][0]["dash"], "dash_dot")
        self.assertEqual(spec["annotations"][0]["target"], "x_value")

    def test_grouped_bar_serializes_multiple_series(self):
        chart = bar(
            "levels",
            ["L", "R"],
            [-3.0, -2.0],
            series=(Series("peak", [0.0, 1.0], [-1.0, -0.5], opacity=0.75),),
            legend_position=LegendPosition.TOP,
        )

        spec = chart.to_spec()
        self.assertEqual(spec["chart"], "bar")
        self.assertEqual(spec["series"][0]["id"], "peak")
        self.assertEqual(spec["series"][0]["opacity"], 0.75)

    def test_all_native_gpui_px_families_have_typed_specs(self):
        grid = [0.0, 1.0, 2.0, 3.0]
        charts = (
            area("area", [1, 2], [2, 3], y0=[0, 0], opacity=0.4),
            boxplot("box", [1, 1], [2, 3], num_bins=1),
            contour("contour", grid, 2, 2, thresholds=[1, 2]),
            isoline("isoline", grid, 2, 2, levels=[1.5]),
            pie("pie", ["A", "B"], [1, 2]),
            donut("donut", ["A", "B"], [1, 2]),
            treemap(
                "tree",
                TreemapNode("root", children=(TreemapNode("A", 1), TreemapNode("B", 2))),
                tiling_method="binary",
            ),
        )
        self.assertEqual(
            [chart.to_spec()["chart"] for chart in charts],
            ["area", "box_plot", "contour", "isoline", "pie", "donut", "treemap"],
        )
        self.assertEqual(charts[0].to_spec()["y0"], [0.0, 0.0])
        self.assertEqual(charts[2].to_spec()["thresholds"], [1.0, 2.0])
        self.assertEqual(charts[-1].to_spec()["treemap"]["children"][1]["value"], 2.0)

    def test_native_capability_and_visual_reports_are_typed(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            request_reports(SessionContext(), "reports")
        self.assertEqual(json.loads(output.getvalue())["command"], "px.reports")
        reports = reports_from_command(CommandResult.from_wire("reports", {
            "ok": True,
            "capability": {"schema_version": 1, "report_type": "chart-capabilities", "reviewed_on": "2026-08-07", "all_release_ready": False, "entries": [{"id": "line", "capability": "line", "chart_families": ["line"], "story_ids": ["line-basic"], "test_contracts": ["line_test"], "status": "complete", "evidence": "native", "release_requirement": "tests"}], "markdown": "| capability |"},
            "visual": {"schema_version": 1, "report_type": "chart-visual-regression", "crate_name": "gpui-px", "crate_version": "0.1.0", "capture_count": 3, "expected_capture_count": 3, "unique_capture_ids": True, "chart_families": ["line"], "markdown": "| capture |"},
        }))
        self.assertEqual(reports.capability.entries[0].chart_families, ("line",))
        self.assertTrue(reports.visual.unique_capture_ids)


if __name__ == "__main__":
    unittest.main()
