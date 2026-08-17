from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from mesh_plot_expanded_visual import ExpandedVisualError, validate_expanded_report


CASE_IDS = (
    "px.mesh_plot.state.camera",
    "px.mesh_plot.state.range",
    "px.mesh_plot.state.masked",
)


def report_fixture() -> dict[str, object]:
    return {
        "schema_version": 1,
        "report_type": "gpui-mesh-plot-cross-adapter-visual-diff",
        "passed": True,
        "left_renderer": "metal-headless",
        "right_renderer": "wgpu-headless",
        "max_channel_delta": 0,
        "max_changed_fraction": 0.0,
        "compared_count": 3,
        "failed_count": 0,
        "cases": [
            {
                "id": case_id,
                "left_path": f"target/qa/visual/mesh-plot-expanded/metal/{index}.png",
                "right_path": f"target/qa/visual/mesh-plot-expanded/wgpu/{index}.png",
                "status": "Passed",
                "changed_pixels": 0,
                "changed_fraction": 0.0,
                "max_channel_delta": 0,
                "mean_channel_delta": 0.0,
            }
            for index, case_id in enumerate(CASE_IDS)
        ],
    }


class MeshPlotExpandedVisualTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.report = self.root / "expanded.json"
        self.report.write_text(json.dumps(report_fixture()), encoding="utf-8")

    def test_valid_three_case_report_passes(self) -> None:
        report = validate_expanded_report(self.report)
        self.assertEqual(report["compared_count"], 3)

    def test_failed_case_is_rejected(self) -> None:
        report = report_fixture()
        report["cases"][1]["status"] = "Failed"  # type: ignore[index]
        self.report.write_text(json.dumps(report), encoding="utf-8")
        with self.assertRaisesRegex(ExpandedVisualError, "is not an exact pass"):
            validate_expanded_report(self.report)

    def test_duplicate_case_id_is_rejected(self) -> None:
        report = report_fixture()
        report["cases"][2]["id"] = CASE_IDS[0]  # type: ignore[index]
        self.report.write_text(json.dumps(report), encoding="utf-8")
        with self.assertRaisesRegex(ExpandedVisualError, "case IDs are not canonical"):
            validate_expanded_report(self.report)

    def test_missing_case_id_is_rejected(self) -> None:
        report = report_fixture()
        report["cases"] = report["cases"][:2]  # type: ignore[index]
        report["compared_count"] = 2
        self.report.write_text(json.dumps(report), encoding="utf-8")
        with self.assertRaisesRegex(ExpandedVisualError, "three-case exact pass"):
            validate_expanded_report(self.report)

    def test_malformed_report_is_rejected(self) -> None:
        self.report.write_text("not json", encoding="utf-8")
        with self.assertRaisesRegex(ExpandedVisualError, "invalid expanded visual report"):
            validate_expanded_report(self.report)


if __name__ == "__main__":
    unittest.main()
