from dataclasses import dataclass
from typing import TypeAlias
from .app import SessionContext
from .commands import CommandResult
from .px import (
    Annotation,
    ChartBuilder,
    ColorScale,
    CurveType,
    LegendPosition,
    Lod,
    MeshPlotBuilder,
    StaticSvgOptions,
    TilingMethod,
    TreemapNode,
    TreemapRect,
)

Chart: TypeAlias = ChartBuilder

def scatter(id: str = "scatter") -> ChartBuilder: ...
def line(id: str = "line") -> ChartBuilder: ...
def area(id: str = "area") -> ChartBuilder: ...
def boxplot(id: str = "boxplot") -> ChartBuilder: ...
def heatmap(id: str = "heatmap") -> ChartBuilder: ...
def contour(id: str = "contour") -> ChartBuilder: ...
def isoline(id: str = "isoline") -> ChartBuilder: ...
def surface(id: str = "surface") -> ChartBuilder: ...
def pie(id: str = "pie") -> ChartBuilder: ...
def donut(id: str = "donut") -> ChartBuilder: ...
def bar(id: str = "bar") -> ChartBuilder: ...
def treemap(id: str = "treemap") -> ChartBuilder: ...
def mesh(id: str = "mesh_plot") -> MeshPlotBuilder: ...

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

def request_reports(context: SessionContext, request_id: str) -> None: ...
def reports_from_command(result: CommandResult) -> ChartReports: ...
