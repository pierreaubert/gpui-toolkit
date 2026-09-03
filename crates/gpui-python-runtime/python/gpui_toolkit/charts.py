"""Strict compatibility names for the resource-backed :mod:`gpui_toolkit.px` API.

Version 2 deliberately has no inline chart document.  These functions preserve
the familiar chart-family names while returning immutable px builders that
require an explicit Dataset, DatasetView, or ArrayData binding.
"""

from __future__ import annotations

from dataclasses import dataclass
from collections.abc import Mapping
from typing import TYPE_CHECKING, Any

from . import px
from .commands import CommandResult, CommandStatus

if TYPE_CHECKING:
    from .app import SessionContext


Annotation = px.Annotation
Chart = px.ChartBuilder
ChartBuilder = px.ChartBuilder
ColorScale = px.ColorScale
CurveType = px.CurveType
LegendPosition = px.LegendPosition
Lod = px.Lod
StaticSvgOptions = px.StaticSvgOptions
TilingMethod = px.TilingMethod
TreemapNode = px.TreemapNode
TreemapRect = px.TreemapRect


def scatter(id: str = "scatter") -> ChartBuilder:
    return px.scatter(id)


def line(id: str = "line") -> ChartBuilder:
    return px.line(id)


def area(id: str = "area") -> ChartBuilder:
    return px.area(id)


def boxplot(id: str = "boxplot") -> ChartBuilder:
    return px.boxplot(id)


def heatmap(id: str = "heatmap") -> ChartBuilder:
    return px.heatmap(id)


def contour(id: str = "contour") -> ChartBuilder:
    return px.contour(id)


def isoline(id: str = "isoline") -> ChartBuilder:
    return px.isoline(id)


def surface(id: str = "surface") -> ChartBuilder:
    return px.surface(id)


def pie(id: str = "pie") -> ChartBuilder:
    return px.pie(id)


def donut(id: str = "donut") -> ChartBuilder:
    return px.donut(id)


def bar(id: str = "bar") -> ChartBuilder:
    return px.bar(id)


def treemap(id: str = "treemap") -> ChartBuilder:
    return px.treemap(id)


def mesh(id: str = "mesh_plot") -> px.MeshPlotBuilder:
    return px.mesh(id)


@dataclass(frozen=True)
class ChartCapabilityEntry:
    id: str
    capability: str
    chart_families: tuple[str, ...]
    story_ids: tuple[str, ...]
    test_contracts: tuple[str, ...]
    status: str
    evidence: str
    release_requirement: str


@dataclass(frozen=True)
class ChartCapabilityReport:
    schema_version: int
    report_type: str
    reviewed_on: str
    all_release_ready: bool
    entries: tuple[ChartCapabilityEntry, ...]
    markdown: str


@dataclass(frozen=True)
class ChartVisualRegressionReport:
    schema_version: int
    report_type: str
    crate_name: str
    crate_version: str
    capture_count: int
    expected_capture_count: int
    unique_capture_ids: bool
    chart_families: tuple[str, ...]
    markdown: str


@dataclass(frozen=True)
class ChartReports:
    capability: ChartCapabilityReport
    visual: ChartVisualRegressionReport


def request_reports(context: "SessionContext", request_id: str) -> None:
    if not isinstance(request_id, str) or not request_id.strip():
        raise ValueError("chart report request id must be non-empty")
    context.command(request_id, "px.reports")


def _mapping(value: Any, name: str) -> Mapping[str, Any]:
    if not isinstance(value, Mapping):
        raise ValueError(f"chart reports missing {name} object")
    return value


def reports_from_command(result: CommandResult) -> ChartReports:
    if result.status is not CommandStatus.SUCCEEDED:
        raise RuntimeError(result.error or "chart reports failed")
    payload = _mapping(result.data, "result")
    capability = _mapping(payload.get("capability"), "capability")
    visual = _mapping(payload.get("visual"), "visual")
    entries_value = capability.get("entries")
    if not isinstance(entries_value, list):
        raise ValueError("chart capability report entries must be a list")
    entries = tuple(
        ChartCapabilityEntry(
            id=str(entry["id"]),
            capability=str(entry["capability"]),
            chart_families=tuple(str(value) for value in entry["chart_families"]),
            story_ids=tuple(str(value) for value in entry["story_ids"]),
            test_contracts=tuple(str(value) for value in entry["test_contracts"]),
            status=str(entry["status"]),
            evidence=str(entry["evidence"]),
            release_requirement=str(entry["release_requirement"]),
        )
        for entry_value in entries_value
        for entry in (_mapping(entry_value, "capability entry"),)
    )
    return ChartReports(
        capability=ChartCapabilityReport(
            schema_version=int(capability["schema_version"]),
            report_type=str(capability["report_type"]),
            reviewed_on=str(capability["reviewed_on"]),
            all_release_ready=bool(capability["all_release_ready"]),
            entries=entries,
            markdown=str(capability["markdown"]),
        ),
        visual=ChartVisualRegressionReport(
            schema_version=int(visual["schema_version"]),
            report_type=str(visual["report_type"]),
            crate_name=str(visual["crate_name"]),
            crate_version=str(visual["crate_version"]),
            capture_count=int(visual["capture_count"]),
            expected_capture_count=int(visual["expected_capture_count"]),
            unique_capture_ids=bool(visual["unique_capture_ids"]),
            chart_families=tuple(str(value) for value in visual["chart_families"]),
            markdown=str(visual["markdown"]),
        ),
    )


__all__ = [
    "Annotation",
    "Chart",
    "ChartBuilder",
    "ChartCapabilityEntry",
    "ChartCapabilityReport",
    "ChartReports",
    "ChartVisualRegressionReport",
    "ColorScale",
    "CurveType",
    "LegendPosition",
    "Lod",
    "StaticSvgOptions",
    "TilingMethod",
    "TreemapNode",
    "TreemapRect",
    "area",
    "bar",
    "boxplot",
    "contour",
    "donut",
    "heatmap",
    "isoline",
    "line",
    "mesh",
    "pie",
    "reports_from_command",
    "request_reports",
    "scatter",
    "surface",
    "treemap",
]
