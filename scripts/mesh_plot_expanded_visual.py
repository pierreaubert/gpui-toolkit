"""Validation contract for the expanded MeshPlot adapter-state report."""

from __future__ import annotations

import json
import math
from pathlib import Path
from typing import Any


EXPECTED_EXPANDED_IDS = frozenset(
    {
        "px.mesh_plot.state.camera",
        "px.mesh_plot.state.range",
        "px.mesh_plot.state.masked",
    }
)
REPORT_TYPE = "gpui-mesh-plot-cross-adapter-visual-diff"


class ExpandedVisualError(RuntimeError):
    """Raised when expanded adapter-state evidence is not release-safe."""


def _finite_number(value: object) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(float(value))
    )


def validate_expanded_report(path: Path) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ExpandedVisualError(f"invalid expanded visual report {path}: {error}") from error
    if not isinstance(report, dict):
        raise ExpandedVisualError("expanded visual report must be a JSON object")
    if (
        report.get("schema_version") != 1
        or report.get("report_type") != REPORT_TYPE
        or report.get("passed") is not True
        or not isinstance(report.get("left_renderer"), str)
        or not isinstance(report.get("right_renderer"), str)
        or report["left_renderer"] == report["right_renderer"]
        or report.get("max_channel_delta") != 0
        or report.get("max_changed_fraction") != 0.0
        or report.get("compared_count") != len(EXPECTED_EXPANDED_IDS)
        or report.get("failed_count") != 0
    ):
        raise ExpandedVisualError("expanded visual report does not describe a three-case exact pass")

    cases = report.get("cases")
    if not isinstance(cases, list) or len(cases) != len(EXPECTED_EXPANDED_IDS):
        raise ExpandedVisualError("expanded visual report must contain exactly three cases")
    seen: set[str] = set()
    for case in cases:
        if not isinstance(case, dict):
            raise ExpandedVisualError("expanded visual report contains a malformed case")
        case_id = case.get("id")
        if not isinstance(case_id, str) or case_id not in EXPECTED_EXPANDED_IDS or case_id in seen:
            raise ExpandedVisualError("expanded visual report case IDs are not canonical")
        seen.add(case_id)
        if (
            case.get("status") != "Passed"
            or case.get("changed_pixels") != 0
            or case.get("max_channel_delta") != 0
            or case.get("changed_fraction") != 0.0
        ):
            raise ExpandedVisualError(f"expanded visual case {case_id} is not an exact pass")
        if not isinstance(case.get("left_path"), str) or not case["left_path"]:
            raise ExpandedVisualError(f"expanded visual case {case_id} has no image paths")
        if not isinstance(case.get("right_path"), str) or not case["right_path"]:
            raise ExpandedVisualError(f"expanded visual case {case_id} has no image paths")
        if not _finite_number(case.get("mean_channel_delta")):
            raise ExpandedVisualError(f"expanded visual case {case_id} has an invalid mean delta")
    if seen != EXPECTED_EXPANDED_IDS:
        raise ExpandedVisualError("expanded visual report case IDs are incomplete")
    return report
