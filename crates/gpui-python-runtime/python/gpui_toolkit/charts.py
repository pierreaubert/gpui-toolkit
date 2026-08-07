"""Declarative gpui-px chart helpers for the GPUI Python wrapper."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Sequence, TYPE_CHECKING
from .commands import CommandResult, CommandStatus
if TYPE_CHECKING: from .app import SessionContext


@dataclass(frozen=True)
class Series:
    id: str
    x: Sequence[float]
    y: Sequence[float]
    label: str = ""
    color: str | None = None
    visible: bool = True
    stroke_width: float | None = None
    point_radius: float | None = None

    def to_spec(self) -> dict[str, Any]:
        return {
            "id": self.id, "x": [float(value) for value in self.x],
            "y": [float(value) for value in self.y], "label": self.label,
            "color": self.color, "visible": self.visible,
            "stroke_width": self.stroke_width, "point_radius": self.point_radius,
        }

@dataclass(frozen=True)
class TreemapNode:
    name: str
    value: float = 0.0
    children: Sequence["TreemapNode"] = ()
    def __post_init__(self) -> None:
        if not self.name or self.value < 0: raise ValueError("invalid treemap node")
    def to_spec(self) -> dict[str, Any]:
        return {"name": self.name, "value": float(self.value), "children": [child.to_spec() for child in self.children]}


@dataclass(frozen=True)
class Chart:
    chart: str
    id: str
    title: str
    x: Sequence[float] | None = None
    y: Sequence[float] | None = None
    categories: Sequence[str] | None = None
    values: Sequence[float] | None = None
    z: Sequence[float | None] | None = None
    width_count: int | None = None
    height_count: int | None = None
    color: str | None = None
    color_scale: str = "viridis"
    x_log: bool = False
    y_log: bool = False
    width: float = 360.0
    height: float = 260.0
    point_radius: float = 4.0
    stroke_width: float = 2.0
    series: Sequence[Series] = ()
    x_label: str | None = None
    y_label: str | None = None
    x_range: tuple[float, float] | None = None
    y_range: tuple[float, float] | None = None
    color_label: str | None = None
    color_unit: str | None = None
    color_range: tuple[float, float] | None = None
    aspect_ratio: float | None = None
    y0: Sequence[float] | None = None
    thresholds: Sequence[float] | None = None
    levels: Sequence[float] | None = None
    opacity: float = 1.0
    inner_radius: float = 0.0
    num_bins: int | None = None
    treemap: TreemapNode | None = None
    tiling_method: str = "squarify"
    padding: float = 1.0

    def to_spec(self) -> dict[str, Any]:
        return {
            "kind": "chart",
            "chart": self.chart,
            "id": self.id,
            "title": self.title,
            "x": None if self.x is None else [float(value) for value in self.x],
            "y": None if self.y is None else [float(value) for value in self.y],
            "categories": None if self.categories is None else [str(value) for value in self.categories],
            "values": None if self.values is None else [float(value) for value in self.values],
            "z": None if self.z is None else [None if value is None else float(value) for value in self.z],
            "width_count": self.width_count,
            "height_count": self.height_count,
            "color": self.color,
            "color_scale": self.color_scale,
            "x_log": bool(self.x_log),
            "y_log": bool(self.y_log),
            "width": float(self.width),
            "height": float(self.height),
            "point_radius": float(self.point_radius),
            "stroke_width": float(self.stroke_width),
            "series": [item.to_spec() for item in self.series],
            "x_label": self.x_label,
            "y_label": self.y_label,
            "x_range": None if self.x_range is None else [float(value) for value in self.x_range],
            "y_range": None if self.y_range is None else [float(value) for value in self.y_range],
            "color_label": self.color_label,
            "color_unit": self.color_unit,
            "color_range": None if self.color_range is None else [float(value) for value in self.color_range],
            "aspect_ratio": self.aspect_ratio,
            "y0": None if self.y0 is None else [float(value) for value in self.y0],
            "thresholds": None if self.thresholds is None else [float(value) for value in self.thresholds],
            "levels": None if self.levels is None else [float(value) for value in self.levels],
            "opacity": float(self.opacity),
            "inner_radius": float(self.inner_radius),
            "num_bins": self.num_bins,
            "treemap": None if self.treemap is None else self.treemap.to_spec(),
            "tiling_method": self.tiling_method,
            "padding": float(self.padding),
        }


def scatter(id: str, x: Sequence[float], y: Sequence[float], *, title: str = "", **kwargs: Any) -> Chart:
    return Chart("scatter", id=id, title=title, x=x, y=y, **kwargs)


def line(id: str, x: Sequence[float], y: Sequence[float], *, title: str = "", **kwargs: Any) -> Chart:
    return Chart("line", id=id, title=title, x=x, y=y, **kwargs)


def bar(
    id: str,
    categories: Sequence[str],
    values: Sequence[float],
    *,
    title: str = "",
    **kwargs: Any,
) -> Chart:
    return Chart("bar", id=id, title=title, categories=categories, values=values, **kwargs)


def heatmap(
    id: str,
    z: Sequence[float | None],
    width_count: int,
    height_count: int,
    *,
    title: str = "",
    **kwargs: Any,
) -> Chart:
    return Chart(
        "heatmap",
        id=id,
        title=title,
        z=z,
        width_count=int(width_count),
        height_count=int(height_count),
        **kwargs,
    )

def area(id: str, x: Sequence[float], y: Sequence[float], *, title: str = "", **kwargs: Any) -> Chart:
    return Chart("area", id=id, title=title, x=x, y=y, **kwargs)

def boxplot(id: str, x: Sequence[float], y: Sequence[float], *, title: str = "", **kwargs: Any) -> Chart:
    return Chart("box_plot", id=id, title=title, x=x, y=y, **kwargs)

def contour(id: str, z: Sequence[float], width_count: int, height_count: int, *, title: str = "", **kwargs: Any) -> Chart:
    return Chart("contour", id=id, title=title, z=z, width_count=width_count, height_count=height_count, **kwargs)

def isoline(id: str, z: Sequence[float], width_count: int, height_count: int, *, title: str = "", **kwargs: Any) -> Chart:
    return Chart("isoline", id=id, title=title, z=z, width_count=width_count, height_count=height_count, **kwargs)

def pie(id: str, labels: Sequence[str], values: Sequence[float], *, title: str = "", **kwargs: Any) -> Chart:
    return Chart("pie", id=id, title=title, categories=labels, values=values, **kwargs)

def donut(id: str, labels: Sequence[str], values: Sequence[float], *, title: str = "", inner_radius: float = 0.5, **kwargs: Any) -> Chart:
    return Chart("donut", id=id, title=title, categories=labels, values=values, inner_radius=inner_radius, **kwargs)

def treemap(id: str, root: TreemapNode, *, title: str = "", **kwargs: Any) -> Chart:
    return Chart("treemap", id=id, title=title, treemap=root, **kwargs)

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
    context.command(request_id, "px.reports")

def reports_from_command(result: CommandResult) -> ChartReports:
    if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "chart reports failed")
    capability = result.data["capability"]
    visual = result.data["visual"]
    return ChartReports(
        ChartCapabilityReport(
            int(capability["schema_version"]), str(capability["report_type"]), str(capability["reviewed_on"]), bool(capability["all_release_ready"]),
            tuple(ChartCapabilityEntry(str(entry["id"]), str(entry["capability"]), tuple(str(value) for value in entry["chart_families"]), tuple(str(value) for value in entry["story_ids"]), tuple(str(value) for value in entry["test_contracts"]), str(entry["status"]), str(entry["evidence"]), str(entry["release_requirement"])) for entry in capability["entries"]),
            str(capability["markdown"]),
        ),
        ChartVisualRegressionReport(
            int(visual["schema_version"]), str(visual["report_type"]), str(visual["crate_name"]), str(visual["crate_version"]),
            int(visual["capture_count"]), int(visual["expected_capture_count"]), bool(visual["unique_capture_ids"]),
            tuple(str(value) for value in visual["chart_families"]), str(visual["markdown"]),
        ),
    )
