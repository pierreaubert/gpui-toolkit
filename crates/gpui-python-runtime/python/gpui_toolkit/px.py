"""Strict v2 gpui-px declarations backed by data resources."""
from __future__ import annotations

from dataclasses import dataclass, field, replace
from enum import Enum
from math import isfinite
from typing import TYPE_CHECKING, Any, Callable

try:
    from ._native import (
        _PxChartInteraction as _NativePxChartInteraction,
        _PxMeshPickIndex as _NativePxMeshPickIndex,
        px_chart_capability_report as _px_chart_capability_report,
        px_chart_keyboard_action as _px_chart_keyboard_action,
        px_color_range_resolve as _px_color_range_resolve,
        px_color_scale_index as _px_color_scale_index,
        px_color_scale_map as _px_color_scale_map,
        px_treemap_layout as _px_treemap_layout,
    )
except ImportError:  # Source-only declarations remain importable without a built wheel.
    _NativePxChartInteraction = None
    _NativePxMeshPickIndex = None
    _px_chart_capability_report = None
    _px_chart_keyboard_action = None
    _px_color_range_resolve = None
    _px_color_scale_index = None
    _px_color_scale_map = None
    _px_treemap_layout = None

from .data import UNSET, ArrayData, DataBinding, Dataset, DatasetView
from .d3 import Renderer2D, VelloBackend
from .meshplot import MeshGeometry, MeshPlotSpec, MeshRevolve, MeshScalarField
from .meshplot import resource_field as _resource_mesh_field
from .meshplot import resource_geometry_from_resources as _resource_mesh_geometry
from .resources import Resource, StaleResourceError
from .commands import CommandResult, CommandStatus

if TYPE_CHECKING:
    from .app import SessionContext


class Lod(str, Enum):
    AUTO = "auto"
    OFF = "off"
    AGGRESSIVE = "aggressive"


class LegendPosition(str, Enum):
    RIGHT = "right"
    LEFT = "left"
    TOP = "top"
    BOTTOM = "bottom"
    HIDDEN = "hidden"


class TilingMethod(str, Enum):
    SQUARIFY = "squarify"
    BINARY = "binary"
    SLICE = "slice"
    DICE = "dice"
    SLICE_DICE = "slice_dice"


class CurveType(str, Enum):
    LINEAR = "linear"
    STEP = "step"
    STEP_BEFORE = "step_before"
    STEP_AFTER = "step_after"
    BASIS = "basis"
    CARDINAL = "cardinal"
    CATMULL_ROM = "catmull_rom"
    MONOTONE_X = "monotone_x"
    NATURAL = "natural"


@dataclass(frozen=True)
class TreemapNode:
    """Immutable hierarchy node for native renderer-independent treemap layout."""

    name: str
    value: float = 0.0
    children: tuple["TreemapNode", ...] = ()

    def __post_init__(self) -> None:
        object.__setattr__(self, "name", str(self.name))
        object.__setattr__(self, "value", float(self.value))
        object.__setattr__(self, "children", tuple(self.children))

    @classmethod
    def new(cls, name: str, value: float) -> "TreemapNode":
        return cls(str(name), float(value))

    @classmethod
    def with_children(
        cls, name: str, children: tuple["TreemapNode", ...] | list["TreemapNode"]
    ) -> "TreemapNode":
        return cls(str(name), 0.0, tuple(children))

    def add_child(self, child: "TreemapNode") -> "TreemapNode":
        if not isinstance(child, TreemapNode):
            raise TypeError("child must be a TreemapNode")
        return replace(self, children=(*self.children, child))

    def total_value(self) -> float:
        if self.children:
            return sum(child.total_value() for child in self.children)
        return self.value

    def is_leaf(self) -> bool:
        return not self.children


@dataclass(frozen=True)
class TreemapRect:
    x0: float
    y0: float
    x1: float
    y1: float
    name: str
    value: float
    depth: int
    category_index: int


class TreemapLayoutError(ValueError):
    """The native gpui-px treemap layout rejected its hierarchy or viewport."""


def treemap_layout(
    root: TreemapNode,
    width: float,
    height: float,
    method: TilingMethod = TilingMethod.SQUARIFY,
    padding: float = 1.0,
) -> tuple[TreemapRect, ...]:
    """Compute native gpui-px treemap geometry without creating a window."""

    if _px_treemap_layout is None:
        raise RuntimeError("native extension not installed")
    if not isinstance(root, TreemapNode):
        raise TypeError("root must be a TreemapNode")

    names: list[str] = []
    values: list[float] = []
    parents: list[int] = []

    def flatten(node: TreemapNode, parent: int) -> None:
        if not isinstance(node, TreemapNode):
            raise TypeError("all children must be TreemapNode instances")
        index = len(names)
        names.append(node.name)
        values.append(node.value)
        parents.append(parent)
        for child in node.children:
            flatten(child, index)

    flatten(root, -1)
    try:
        native_rects = _px_treemap_layout(
            names,
            values,
            parents,
            TilingMethod(method).value,
            float(padding),
            float(width),
            float(height),
        )
    except ValueError as error:
        raise TreemapLayoutError(str(error)) from error
    return tuple(TreemapRect(*rect) for rect in native_rects)


class InteractionMode(str, Enum):
    NONE = "none"
    BRUSH = "brush"
    PAN = "pan"
    ZOOM = "zoom"


class ChartKeyboardAction(str, Enum):
    ZOOM_IN = "zoom_in"
    ZOOM_OUT = "zoom_out"
    PAN_LEFT = "pan_left"
    PAN_RIGHT = "pan_right"
    PAN_UP = "pan_up"
    PAN_DOWN = "pan_down"
    RESET_ZOOM = "reset_zoom"


class ChartCapabilityStatus(str, Enum):
    IMPLEMENTED = "implemented"
    PARTIAL = "partial"
    MISSING = "missing"
    APP_BRIDGE_REQUIRED = "app-bridge-required"

    def is_release_ready(self) -> bool:
        return self is ChartCapabilityStatus.IMPLEMENTED


@dataclass(frozen=True)
class ChartCapabilityEntry:
    id: str
    capability: str
    chart_families: tuple[str, ...]
    story_ids: tuple[str, ...]
    test_contracts: tuple[str, ...]
    status: ChartCapabilityStatus
    evidence: str
    release_requirement: str


@dataclass(frozen=True)
class ChartCapabilityReport:
    schema_version: int
    report_type: str
    reviewed_on: str
    entries: tuple[ChartCapabilityEntry, ...]
    _blocking_ids: tuple[str, ...]
    _markdown: str

    def all_release_ready(self) -> bool:
        return not self._blocking_ids

    def blocking_entries(self) -> tuple[ChartCapabilityEntry, ...]:
        blocking = frozenset(self._blocking_ids)
        return tuple(entry for entry in self.entries if entry.id in blocking)

    def to_markdown_table(self) -> str:
        return self._markdown


def chart_capability_report() -> ChartCapabilityReport:
    if _px_chart_capability_report is None:
        raise RuntimeError("native extension not installed")
    (
        schema_version,
        report_type,
        reviewed_on,
        release_ready,
        native_entries,
        blocking_ids,
        markdown,
    ) = _px_chart_capability_report()
    entries = tuple(
        ChartCapabilityEntry(
            id=str(entry[0]),
            capability=str(entry[1]),
            chart_families=tuple(
                family.strip() for family in str(entry[2]).split(",") if family.strip()
            ),
            story_ids=tuple(str(value) for value in entry[3]),
            test_contracts=tuple(str(value) for value in entry[4]),
            status=ChartCapabilityStatus(entry[5]),
            evidence=str(entry[6]),
            release_requirement=str(entry[7]),
        )
        for entry in native_entries
    )
    report = ChartCapabilityReport(
        schema_version=int(schema_version),
        report_type=str(report_type),
        reviewed_on=str(reviewed_on),
        entries=entries,
        _blocking_ids=tuple(str(value) for value in blocking_ids),
        _markdown=str(markdown),
    )
    if bool(release_ready) != report.all_release_ready():
        raise RuntimeError("native gpui-px capability report is internally inconsistent")
    return report


def keyboard_action_for_key(key: str) -> ChartKeyboardAction | None:
    if _px_chart_keyboard_action is None:
        raise RuntimeError("native extension not installed")
    value = _px_chart_keyboard_action(str(key))
    return None if value is None else ChartKeyboardAction(value)


@dataclass(frozen=True)
class InteractionSelection:
    x0: float
    y0: float
    x1: float
    y1: float

    @classmethod
    def _from_native(cls, value: tuple[float, float, float, float]) -> "InteractionSelection":
        return cls(*(float(component) for component in value))


class ChartInteraction:
    """Native renderer-independent gpui-px interaction state."""

    def __init__(self, x_min: float, x_max: float, y_min: float, y_max: float) -> None:
        if _NativePxChartInteraction is None:
            raise RuntimeError("native extension not installed")
        self._native = _NativePxChartInteraction(
            float(x_min), float(x_max), float(y_min), float(y_max)
        )

    @classmethod
    def _from_native(cls, native: Any) -> "ChartInteraction":
        result = cls.__new__(cls)
        result._native = native
        return result

    def with_log_x(self, enabled: bool = True) -> "ChartInteraction":
        return self._from_native(self._native.with_log_x(bool(enabled)))

    def with_log_y(self, enabled: bool = True) -> "ChartInteraction":
        return self._from_native(self._native.with_log_y(bool(enabled)))

    def with_size(self, width: float, height: float) -> "ChartInteraction":
        return self._from_native(self._native.with_size(float(width), float(height)))

    def with_mode(self, mode: InteractionMode) -> "ChartInteraction":
        return self._from_native(self._native.with_mode(InteractionMode(mode).value))

    def start_brush(self, x: float, y: float) -> None:
        self._native.start_brush(float(x), float(y))

    def update_brush(self, x: float, y: float) -> None:
        self._native.update_brush(float(x), float(y))

    def end_brush(self, apply_zoom: bool = False) -> InteractionSelection | None:
        value = self._native.end_brush(bool(apply_zoom))
        return None if value is None else InteractionSelection._from_native(value)

    def cancel_brush(self) -> None:
        self._native.cancel_brush()

    def current_brush_selection(self) -> InteractionSelection | None:
        value = self._native.current_brush_selection()
        return None if value is None else InteractionSelection._from_native(value)

    def is_brushing(self) -> bool:
        return bool(self._native.is_brushing())

    def zoom_to(self, x_min: float, x_max: float, y_min: float, y_max: float) -> None:
        self._native.zoom_to(float(x_min), float(x_max), float(y_min), float(y_max))

    def set_viewport_without_history(
        self, x_min: float, x_max: float, y_min: float, y_max: float
    ) -> None:
        self._native.set_viewport_without_history(
            float(x_min), float(x_max), float(y_min), float(y_max)
        )

    def reset_zoom(self) -> None:
        self._native.reset_zoom()

    def zoom_back(self) -> bool:
        return bool(self._native.zoom_back())

    def is_zoomed(self) -> bool:
        return bool(self._native.is_zoomed())

    def x_domain(self) -> tuple[float, float]:
        return tuple(self._native.x_domain())

    def y_domain(self) -> tuple[float, float]:
        return tuple(self._native.y_domain())

    def zoom_level(self) -> int:
        return int(self._native.zoom_level())

    def point_to_domain(self, x: float, y: float) -> tuple[float, float]:
        return tuple(self._native.point_to_domain(float(x), float(y)))

    def update_hover_pixel(self, x: float, y: float) -> tuple[float, float] | None:
        value = self._native.update_hover_pixel(float(x), float(y))
        return None if value is None else tuple(value)

    def clear_hover(self) -> None:
        self._native.clear_hover()

    def hover_domain(self) -> tuple[float, float] | None:
        value = self._native.hover_domain()
        return None if value is None else tuple(value)

    def pan_by_pixels(self, dx: float, dy: float) -> None:
        self._native.pan_by_pixels(float(dx), float(dy))

    def zoom_around_pixel(self, x: float, y: float, factor: float) -> None:
        self._native.zoom_around_pixel(float(x), float(y), float(factor))

    def zoom_around_domain(self, x: float, y: float, factor: float) -> None:
        self._native.zoom_around_domain(float(x), float(y), float(factor))


@dataclass(frozen=True)
class MeshPick:
    plot_id: str
    mesh_id: str
    cell_index: int
    cell_id: int | None
    nearest_vertex_index: int | None
    vertex_id: int | None
    world_position: tuple[float, float, float]
    displayed_value: float | None
    field_id: str | None


class MeshPickIndex:
    """Retained native gpui-px spatial index for revisioned ArrayData.

    Construction snapshots each input generation once because Python 3.10's
    limited ABI does not expose the safe buffer-view API. Repeated picks reuse
    the native mesh and spatial index; mutation is detected before every query.
    """

    def __init__(
        self,
        positions: ArrayData,
        triangles: ArrayData,
        *,
        mesh_id: str = "mesh",
        plot_id: str = "mesh_plot",
        horizontal: str = "x",
        vertical: str = "y",
        field: ArrayData | None = None,
        field_association: str = "vertex",
        field_id: str = "field",
        valid: ArrayData | None = None,
        vertex_ids: ArrayData | None = None,
        cell_ids: ArrayData | None = None,
    ) -> None:
        if _NativePxMeshPickIndex is None:
            raise RuntimeError("native extension not installed")
        if not isinstance(positions, ArrayData) or not isinstance(triangles, ArrayData):
            raise TypeError("mesh pick index requires ArrayData positions and triangles")
        if len(positions.shape) != 2 or positions.shape[1] != 3:
            raise ValueError("mesh pick positions shape must be [vertices, 3]")
        if len(triangles.shape) != 2 or triangles.shape[1] != 3:
            raise ValueError("mesh pick triangles shape must be [triangles, 3]")
        if field is not None and not isinstance(field, ArrayData):
            raise TypeError("mesh pick field must be ArrayData or None")
        if field is None and valid is not None:
            raise ValueError("mesh pick validity requires a field")
        for name, source in (
            ("valid", valid),
            ("vertex_ids", vertex_ids),
            ("cell_ids", cell_ids),
        ):
            if source is not None and not isinstance(source, ArrayData):
                raise TypeError(f"mesh pick {name} must be ArrayData or None")

        sources = tuple(
            source
            for source in (positions, triangles, field, valid, vertex_ids, cell_ids)
            if source is not None
        )
        self._sources = tuple((source, source.generation) for source in sources)
        self._native = _NativePxMeshPickIndex(
            _array_data_bytes_view(positions),
            positions.dtype,
            positions.shape[0],
            _array_data_bytes_view(triangles),
            triangles.dtype,
            triangles.shape[0],
            str(mesh_id),
            str(plot_id),
            str(horizontal),
            str(vertical),
            None if field is None else _array_data_bytes_view(field),
            None if field is None else field.dtype,
            str(field_association),
            str(field_id),
            None if valid is None else _array_data_bytes_view(valid),
            None if vertex_ids is None else _array_data_bytes_view(vertex_ids),
            None if vertex_ids is None else vertex_ids.dtype,
            None if cell_ids is None else _array_data_bytes_view(cell_ids),
            None if cell_ids is None else cell_ids.dtype,
        )

    def _ensure_current(self) -> None:
        if self._native.is_closed():
            raise RuntimeError("mesh pick index is closed")
        for source, generation in self._sources:
            if source.generation != generation:
                raise StaleResourceError(
                    f"array {source.id!r} advanced from generation {generation} "
                    f"to {source.generation}; rebuild the mesh pick index"
                )

    def pick(self, x: float, y: float) -> MeshPick | None:
        self._ensure_current()
        value = self._native.pick(float(x), float(y))
        if value is None:
            return None
        return MeshPick(
            plot_id=str(value[0]),
            mesh_id=str(value[1]),
            cell_index=int(value[2]),
            cell_id=None if value[3] is None else int(value[3]),
            nearest_vertex_index=None if value[4] is None else int(value[4]),
            vertex_id=None if value[5] is None else int(value[5]),
            world_position=tuple(float(component) for component in value[6]),
            displayed_value=None if value[7] is None else float(value[7]),
            field_id=None if value[8] is None else str(value[8]),
        )

    @property
    def vertex_count(self) -> int:
        return int(self._native.vertex_count())

    @property
    def triangle_count(self) -> int:
        return int(self._native.triangle_count())

    def close(self) -> None:
        self._native.close()

    def __enter__(self) -> "MeshPickIndex":
        self._ensure_current()
        return self

    def __exit__(self, exc_type: Any, exc: Any, traceback: Any) -> None:
        self.close()


def _array_data_bytes_view(source: ArrayData) -> bytes:
    source._ensure_open()
    try:
        return source._data.cast("B").tobytes()
    except (TypeError, ValueError) as error:
        raise ValueError(f"array {source.id!r} must be C-contiguous") from error


class ColorScale(str, Enum):
    """Built-in gpui-px scalar colormaps backed by the native wheel."""

    VIRIDIS = "viridis"
    PLASMA = "plasma"
    INFERNO = "inferno"
    MAGMA = "magma"
    HEAT = "heat"
    COOLWARM = "coolwarm"
    GREYS = "greys"

    def map(self, value: float) -> str:
        if _px_color_scale_map is None:
            raise RuntimeError("native extension not installed")
        return str(_px_color_scale_map(self.value, float(value)))

    def to_colormap_index(self) -> int:
        if _px_color_scale_index is None:
            raise RuntimeError("native extension not installed")
        return int(_px_color_scale_index(self.value))

    def to_fn(self) -> Callable[[float], str]:
        return self.map

    @classmethod
    def custom(cls, callback: Callable[[float], str]) -> "CustomColorScale":
        return CustomColorScale(callback)


@dataclass(frozen=True)
class CustomColorScale:
    """Python-thread-only equivalent of ``gpui_px::ColorScale::Custom``."""

    callback: Callable[[float], str]

    def __post_init__(self) -> None:
        if not callable(self.callback):
            raise TypeError("custom color scale callback must be callable")

    def map(self, value: float) -> str:
        value = float(value)
        if not isfinite(value):
            raise ValueError("color scale value must be finite")
        return _normalize_hex_color(self.callback(min(1.0, max(0.0, value))))

    def to_colormap_index(self) -> int:
        return 0

    def to_fn(self) -> Callable[[float], str]:
        return self.map


@dataclass(frozen=True)
class AutoOrFixed:
    """Automatic or explicit extent for a symmetric color range."""

    value: float | None = None

    @classmethod
    def auto(cls) -> "AutoOrFixed":
        return cls()

    @classmethod
    def fixed(cls, value: float) -> "AutoOrFixed":
        value = float(value)
        if not isfinite(value) or value <= 0.0:
            raise ValueError("color range extent must be positive and finite")
        return cls(value)

    @property
    def is_auto(self) -> bool:
        return self.value is None


class ColorRangeError(ValueError):
    """A path-qualified gpui-px color-range validation failure."""

    def __init__(self, reason: str) -> None:
        self.path = "color_range"
        self.reason = reason
        super().__init__(f"{self.path}: {reason}")


@dataclass(frozen=True)
class ColorRange:
    """Immutable gpui-px scalar display-range declaration."""

    kind: str
    lower: float | None = None
    upper: float | None = None
    center: float | None = None
    extent: AutoOrFixed | None = None

    def __post_init__(self) -> None:
        if self.kind not in {"auto", "fixed", "symmetric"}:
            raise ColorRangeError(f"unsupported color range kind {self.kind!r}")
        if self.kind == "auto":
            if any(
                value is not None
                for value in (self.lower, self.upper, self.center, self.extent)
            ):
                raise ColorRangeError("automatic range does not accept explicit bounds")
            return
        if self.kind == "fixed":
            if self.lower is None or self.upper is None:
                raise ColorRangeError("fixed range requires lower and upper bounds")
            if (
                not isfinite(self.lower)
                or not isfinite(self.upper)
                or self.lower >= self.upper
            ):
                raise ColorRangeError("fixed bounds must be finite and lower < upper")
            return
        if self.center is None or not isfinite(self.center):
            raise ColorRangeError("symmetric center must be finite")
        if self.extent is not None and not isinstance(self.extent, AutoOrFixed):
            raise TypeError("symmetric color range extent must be AutoOrFixed or None")

    @classmethod
    def auto(cls) -> "ColorRange":
        return cls("auto")

    @classmethod
    def fixed(cls, lower: float, upper: float) -> "ColorRange":
        return cls("fixed", lower=float(lower), upper=float(upper))

    @classmethod
    def symmetric(
        cls, center: float = 0.0, extent: AutoOrFixed | None = None
    ) -> "ColorRange":
        if extent is not None and not isinstance(extent, AutoOrFixed):
            raise TypeError("symmetric color range extent must be AutoOrFixed or None")
        return cls("symmetric", center=float(center), extent=extent or AutoOrFixed.auto())

    def resolve(self, data_min: float, data_max: float) -> tuple[float, float]:
        if _px_color_range_resolve is None:
            raise RuntimeError("native extension not installed")
        try:
            result = _px_color_range_resolve(
                self.kind,
                float(data_min),
                float(data_max),
                self.lower,
                self.upper,
                self.center,
                None if self.extent is None else self.extent.value,
            )
        except ValueError as error:
            raise ColorRangeError(str(error)) from error
        return float(result[0]), float(result[1])

    def to_spec(self) -> str | tuple[float, float] | dict[str, float | str]:
        if self.kind == "auto":
            return "auto"
        if self.kind == "fixed":
            return (float(self.lower), float(self.upper))
        if self.kind == "symmetric":
            return {
                "kind": "symmetric",
                "center": float(self.center),
                "extent": (
                    "auto"
                    if self.extent is None or self.extent.value is None
                    else self.extent.value
                ),
            }
        raise ColorRangeError(f"unsupported color range kind {self.kind!r}")


class ColorbarOrientation(str, Enum):
    VERTICAL = "vertical"
    HORIZONTAL = "horizontal"


class MeshPlotBackend(str, Enum):
    """Retained renderer used by the native live MeshPlot host."""

    AUTO = "auto"
    WGPU = "wgpu"


def _mesh_color_range_spec(value: ColorRange) -> str | list[float] | dict[str, Any]:
    spec = value.to_spec()
    if isinstance(spec, tuple):
        return list(spec)
    if isinstance(spec, dict) and spec.get("kind") == "symmetric":
        return {"symmetric": {"center": spec["center"], "extent": spec["extent"]}}
    return spec


@dataclass(frozen=True)
class MeshColorbar:
    label: str
    _unit: str | None = None
    _scale: ColorScale = ColorScale.VIRIDIS
    _range: ColorRange = field(default_factory=ColorRange.auto)
    _ticks: tuple[float, ...] | None = None
    _orientation: ColorbarOrientation = ColorbarOrientation.VERTICAL

    def __post_init__(self) -> None:
        if not isinstance(self.label, str) or not self.label.strip():
            raise ValueError("colorbar label must be a non-empty string")

    def unit(self, value: str | None) -> "MeshColorbar":
        if value is not None and not isinstance(value, str):
            raise TypeError("colorbar unit must be str or None")
        return replace(self, _unit=value)

    def scale(self, value: ColorScale | str) -> "MeshColorbar":
        return replace(self, _scale=ColorScale(value))

    def range(self, value: ColorRange) -> "MeshColorbar":
        if not isinstance(value, ColorRange):
            raise TypeError("colorbar range must be ColorRange")
        return replace(self, _range=value)

    def ticks(self, values: tuple[float, ...] | list[float] | None) -> "MeshColorbar":
        if values is None:
            return replace(self, _ticks=None)
        return replace(self, _ticks=_finite_levels("colorbar ticks", values))

    def orientation(self, value: ColorbarOrientation | str) -> "MeshColorbar":
        return replace(self, _orientation=ColorbarOrientation(value))

    def to_spec(self) -> dict[str, Any]:
        return {
            "label": self.label,
            "unit": self._unit,
            "scale": self._scale.value,
            "range": _mesh_color_range_spec(self._range),
            "ticks": None if self._ticks is None else list(self._ticks),
            "orientation": self._orientation.value,
        }


def _normalize_hex_color(value: str) -> str:
    if not isinstance(value, str):
        raise TypeError("color must be a #RRGGBB string")
    normalized = value.removeprefix("#")
    if len(normalized) != 6 or any(
        character not in "0123456789abcdefABCDEF" for character in normalized
    ):
        raise ValueError("color must be a #RRGGBB string")
    return f"#{normalized.lower()}"


@dataclass(frozen=True)
class SvgExportResult:
    chart_id: str
    svg: str

    @classmethod
    def from_command(cls, result: CommandResult) -> "SvgExportResult":
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or "resource chart SVG export failed")
        chart_id = str(result.data.get("chart_id", ""))
        svg = str(result.data.get("svg", ""))
        if not chart_id or not svg.startswith("<svg"):
            raise RuntimeError("native resource chart export returned an invalid SVG result")
        return cls(chart_id, svg)


@dataclass(frozen=True)
class ChartViewportResult:
    """Retained native viewport state returned by the installed host."""

    chart_id: str
    x_domain: tuple[float, float]
    y_domain: tuple[float, float]
    zoom_level: int
    is_zoomed: bool

    @classmethod
    def from_command(cls, result: CommandResult) -> "ChartViewportResult":
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or "resource chart viewport query failed")
        chart_id = str(result.data.get("chart_id", ""))
        x_domain = result.data.get("x_domain")
        y_domain = result.data.get("y_domain")
        zoom_level = result.data.get("zoom_level")
        is_zoomed = result.data.get("is_zoomed")
        if not chart_id:
            raise RuntimeError("native viewport query returned an empty chart id")
        if not _valid_domain_result(x_domain) or not _valid_domain_result(y_domain):
            raise RuntimeError("native viewport query returned invalid domains")
        if not isinstance(zoom_level, int) or isinstance(zoom_level, bool) or zoom_level < 0:
            raise RuntimeError("native viewport query returned an invalid zoom level")
        if not isinstance(is_zoomed, bool):
            raise RuntimeError("native viewport query returned an invalid zoom state")
        return cls(
            chart_id,
            (float(x_domain[0]), float(x_domain[1])),
            (float(y_domain[0]), float(y_domain[1])),
            zoom_level,
            is_zoomed,
        )


@dataclass(frozen=True)
class SurfaceCameraResult:
    """Retained native Surface3D camera returned after query or reset."""

    chart_id: str
    distance: float
    azimuth: float
    elevation: float
    target: tuple[float, float, float]

    @classmethod
    def from_command(cls, result: CommandResult) -> "SurfaceCameraResult":
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or "surface camera request failed")
        chart_id = str(result.data.get("chart_id", ""))
        camera = result.data.get("camera")
        if not chart_id or not isinstance(camera, dict):
            raise RuntimeError("native surface camera returned an invalid result")
        distance = camera.get("distance")
        azimuth = camera.get("azimuth")
        elevation = camera.get("elevation")
        target = camera.get("target")
        scalars = (distance, azimuth, elevation)
        if any(
            not isinstance(value, (int, float))
            or isinstance(value, bool)
            or not isfinite(float(value))
            for value in scalars
        ) or float(distance) <= 0.0:
            raise RuntimeError("native surface camera returned invalid orbit values")
        if (
            not isinstance(target, (list, tuple))
            or len(target) != 3
            or any(
                not isinstance(value, (int, float))
                or isinstance(value, bool)
                or not isfinite(float(value))
                for value in target
            )
        ):
            raise RuntimeError("native surface camera returned an invalid target")
        return cls(
            chart_id,
            float(distance),
            float(azimuth),
            float(elevation),
            tuple(float(value) for value in target),
        )


def _valid_domain_result(value: Any) -> bool:
    return (
        isinstance(value, (list, tuple))
        and len(value) == 2
        and all(
            isinstance(component, (int, float))
            and not isinstance(component, bool)
            and isfinite(float(component))
            for component in value
        )
        and float(value[0]) < float(value[1])
    )


@dataclass(frozen=True)
class ChartAccessibilitySummary:
    """Typed result produced by a native gpui-px chart implementation."""

    chart_id: str
    chart_type: str
    title: str | None
    series_count: int
    datum_count: int
    x_range: tuple[float, float] | None
    y_range: tuple[float, float] | None
    value_range: tuple[float, float] | None
    x_scale: str | None
    y_scale: str | None
    series_labels: tuple[str, ...]
    description: str
    accessible_label: str
    accessible_value_text: str

    @classmethod
    def from_command(cls, result: CommandResult) -> "ChartAccessibilitySummary":
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or "native chart accessibility summary failed")
        chart_id = str(result.data.get("chart_id", ""))
        summary = result.data.get("summary")
        if not chart_id or not isinstance(summary, dict):
            raise RuntimeError("native chart summary returned an invalid result")
        chart_type = summary.get("chart_type")
        title = summary.get("title")
        series_count = summary.get("series_count")
        datum_count = summary.get("datum_count")
        labels = summary.get("series_labels")
        x_scale = summary.get("x_scale")
        y_scale = summary.get("y_scale")
        text_fields = (
            summary.get("description"),
            summary.get("accessible_label"),
            summary.get("accessible_value_text"),
        )
        if not isinstance(chart_type, str) or not chart_type:
            raise RuntimeError("native chart summary returned an invalid chart type")
        if title is not None and not isinstance(title, str):
            raise RuntimeError("native chart summary returned an invalid title")
        if any(
            not isinstance(value, int) or isinstance(value, bool) or value < 0
            for value in (series_count, datum_count)
        ):
            raise RuntimeError("native chart summary returned invalid counts")
        if not isinstance(labels, list) or any(not isinstance(label, str) for label in labels):
            raise RuntimeError("native chart summary returned invalid series labels")
        if x_scale not in {None, "linear", "log"} or y_scale not in {None, "linear", "log"}:
            raise RuntimeError("native chart summary returned an invalid scale")
        if any(not isinstance(value, str) for value in text_fields):
            raise RuntimeError("native chart summary returned invalid accessible text")
        return cls(
            chart_id=chart_id,
            chart_type=chart_type,
            title=title,
            series_count=series_count,
            datum_count=datum_count,
            x_range=_summary_range(summary.get("x_range"), "x_range"),
            y_range=_summary_range(summary.get("y_range"), "y_range"),
            value_range=_summary_range(summary.get("value_range"), "value_range"),
            x_scale=x_scale,
            y_scale=y_scale,
            series_labels=tuple(labels),
            description=text_fields[0],
            accessible_label=text_fields[1],
            accessible_value_text=text_fields[2],
        )


def _summary_range(value: Any, name: str) -> tuple[float, float] | None:
    if value is None:
        return None
    if (
        not isinstance(value, (list, tuple))
        or len(value) != 2
        or any(
            not isinstance(component, (int, float))
            or isinstance(component, bool)
            or not isfinite(float(component))
            for component in value
        )
        or float(value[0]) > float(value[1])
    ):
        raise RuntimeError(f"native chart summary returned an invalid {name}")
    return float(value[0]), float(value[1])


@dataclass(frozen=True)
class ChartLegendItem:
    series_index: int
    label: str
    color: int
    marker: str
    hidden: bool
    uses_secondary_axis: bool


@dataclass(frozen=True)
class ChartLegendSummary:
    chart_type: str
    visible: bool
    position: LegendPosition
    position_explicit: bool
    items: tuple[ChartLegendItem, ...]
    description: str

    @classmethod
    def _from_payload(cls, value: Any) -> "ChartLegendSummary":
        if not isinstance(value, dict):
            raise RuntimeError("native chart metadata returned an invalid legend")
        raw_items = value.get("items")
        if not isinstance(raw_items, list):
            raise RuntimeError("native chart metadata returned invalid legend items")
        items: list[ChartLegendItem] = []
        for item in raw_items:
            if not isinstance(item, dict):
                raise RuntimeError("native chart metadata returned an invalid legend item")
            index = item.get("series_index")
            color = item.get("color")
            label = item.get("label")
            marker = item.get("marker")
            hidden = item.get("hidden")
            secondary = item.get("uses_secondary_axis")
            if (
                not isinstance(index, int)
                or isinstance(index, bool)
                or index < 0
                or not isinstance(color, int)
                or isinstance(color, bool)
                or not 0 <= color <= 0xFFFFFFFF
                or not isinstance(label, str)
                or marker not in {"line", "circle", "square"}
                or not isinstance(hidden, bool)
                or not isinstance(secondary, bool)
            ):
                raise RuntimeError("native chart metadata returned an invalid legend item")
            items.append(ChartLegendItem(index, label, color, marker, hidden, secondary))
        chart_type = value.get("chart_type")
        visible = value.get("visible")
        position_explicit = value.get("position_explicit")
        description = value.get("description")
        if (
            not isinstance(chart_type, str)
            or not chart_type
            or not isinstance(visible, bool)
            or not isinstance(position_explicit, bool)
            or not isinstance(description, str)
            or value.get("item_count") != len(items)
        ):
            raise RuntimeError("native chart metadata returned an invalid legend summary")
        try:
            position = LegendPosition(value.get("position"))
        except (TypeError, ValueError) as error:
            raise RuntimeError("native chart metadata returned an invalid legend position") from error
        return cls(chart_type, visible, position, position_explicit, tuple(items), description)


@dataclass(frozen=True)
class ChartAnnotationTarget:
    kind: str
    x: float | None = None
    y: float | None = None
    category: str | None = None


@dataclass(frozen=True)
class ChartAnnotationItem:
    id: str
    label: str
    target: ChartAnnotationTarget
    color: int | None
    series_index: int | None


@dataclass(frozen=True)
class ChartAnnotationSummary:
    chart_type: str
    annotations: tuple[ChartAnnotationItem, ...]
    description: str

    @classmethod
    def _from_payload(cls, value: Any) -> "ChartAnnotationSummary":
        if not isinstance(value, dict) or not isinstance(value.get("annotations"), list):
            raise RuntimeError("native chart metadata returned invalid annotations")
        annotations: list[ChartAnnotationItem] = []
        for item in value["annotations"]:
            if not isinstance(item, dict):
                raise RuntimeError("native chart metadata returned an invalid annotation")
            annotation_id = item.get("id")
            label = item.get("label")
            color = item.get("color")
            series_index = item.get("series_index")
            if (
                not isinstance(annotation_id, str)
                or not annotation_id
                or not isinstance(label, str)
                or (color is not None and (not isinstance(color, int) or isinstance(color, bool)))
                or (
                    series_index is not None
                    and (
                        not isinstance(series_index, int)
                        or isinstance(series_index, bool)
                        or series_index < 0
                    )
                )
            ):
                raise RuntimeError("native chart metadata returned an invalid annotation")
            annotations.append(
                ChartAnnotationItem(
                    annotation_id,
                    label,
                    _annotation_target_from_payload(item.get("target")),
                    color,
                    series_index,
                )
            )
        chart_type = value.get("chart_type")
        description = value.get("description")
        if (
            not isinstance(chart_type, str)
            or not chart_type
            or not isinstance(description, str)
            or value.get("annotation_count") != len(annotations)
        ):
            raise RuntimeError("native chart metadata returned an invalid annotation summary")
        return cls(chart_type, tuple(annotations), description)


def _annotation_target_from_payload(value: Any) -> ChartAnnotationTarget:
    if not isinstance(value, dict) or value.get("kind") not in {
        "point",
        "x_value",
        "y_value",
        "category",
    }:
        raise RuntimeError("native chart metadata returned an invalid annotation target")
    kind = value["kind"]
    x = value.get("x")
    y = value.get("y")
    category = value.get("category")
    if x is not None and (
        not isinstance(x, (int, float)) or isinstance(x, bool)
    ):
        raise RuntimeError("native chart metadata returned an invalid annotation x value")
    if y is not None and (
        not isinstance(y, (int, float)) or isinstance(y, bool)
    ):
        raise RuntimeError("native chart metadata returned an invalid annotation y value")
    if x is not None:
        x = float(x)
    if y is not None:
        y = float(y)
    if (
        (x is not None and not isfinite(x))
        or (y is not None and not isfinite(y))
        or (category is not None and not isinstance(category, str))
        or (kind == "point" and (x is None or y is None))
        or (kind == "x_value" and x is None)
        or (kind == "y_value" and y is None)
        or (kind == "category" and not category)
    ):
        raise RuntimeError("native chart metadata returned an invalid annotation target")
    return ChartAnnotationTarget(kind, x, y, category)


@dataclass(frozen=True)
class ChartMetadataResult:
    chart_id: str
    accessibility: ChartAccessibilitySummary
    legend: ChartLegendSummary
    annotations: ChartAnnotationSummary

    @classmethod
    def from_command(cls, result: CommandResult) -> "ChartMetadataResult":
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or "native chart metadata request failed")
        chart_id = str(result.data.get("chart_id", ""))
        if not chart_id:
            raise RuntimeError("native chart metadata returned an empty chart id")
        accessibility = ChartAccessibilitySummary.from_command(
            CommandResult.from_wire(
                "chart-metadata-accessibility",
                {
                    "ok": True,
                    "chart_id": chart_id,
                    "summary": result.data.get("accessibility"),
                },
            )
        )
        return cls(
            chart_id,
            accessibility,
            ChartLegendSummary._from_payload(result.data.get("legend")),
            ChartAnnotationSummary._from_payload(result.data.get("annotations")),
        )


@dataclass(frozen=True)
class StaticSvgOptions:
    """Immutable options consumed by gpui-px's native SVG renderers."""

    width: float = 800.0
    height: float = 600.0
    _margin_left: float | object = UNSET
    _margin_right: float | object = UNSET
    _margin_top: float | object = UNSET
    _margin_bottom: float | object = UNSET
    _background: str | None | object = UNSET
    _show_axes: bool | object = UNSET

    def __post_init__(self) -> None:
        if (
            not isfinite(self.width)
            or not isfinite(self.height)
            or self.width <= 0.0
            or self.height <= 0.0
        ):
            raise ValueError("SVG export dimensions must be positive and finite")

    @classmethod
    def new(cls, width: float, height: float) -> "StaticSvgOptions":
        return cls(float(width), float(height))

    def margins(
        self,
        *,
        left: float | object = UNSET,
        right: float | object = UNSET,
        top: float | object = UNSET,
        bottom: float | object = UNSET,
    ) -> "StaticSvgOptions":
        values = {"left": left, "right": right, "top": top, "bottom": bottom}
        normalized: dict[str, float | object] = {}
        for name, value in values.items():
            if value is UNSET:
                normalized[name] = UNSET
                continue
            number = float(value)
            if not isfinite(number) or number < 0.0:
                raise ValueError(f"SVG export {name} margin must be non-negative and finite")
            normalized[name] = number
        return replace(
            self,
            _margin_left=(self._margin_left if left is UNSET else normalized["left"]),
            _margin_right=(self._margin_right if right is UNSET else normalized["right"]),
            _margin_top=(self._margin_top if top is UNSET else normalized["top"]),
            _margin_bottom=(
                self._margin_bottom if bottom is UNSET else normalized["bottom"]
            ),
        )

    def background(self, value: str | None) -> "StaticSvgOptions":
        return replace(
            self,
            _background=None if value is None else _normalize_hex_color(value),
        )

    def show_axes(self, value: bool) -> "StaticSvgOptions":
        if not isinstance(value, bool):
            raise TypeError("SVG export show_axes must be bool")
        return replace(self, _show_axes=value)

    def to_spec(self) -> dict[str, Any]:
        result: dict[str, Any] = {"width": self.width, "height": self.height}
        for name, value in (
            ("margin_left", self._margin_left),
            ("margin_right", self._margin_right),
            ("margin_top", self._margin_top),
            ("margin_bottom", self._margin_bottom),
            ("show_axes", self._show_axes),
        ):
            if value is not UNSET:
                result[name] = value
        if self._background is not UNSET:
            result["background"] = (
                None
                if self._background is None
                else int(self._background.removeprefix("#"), 16)
            )
        return result


@dataclass(frozen=True)
class Annotation:
    id: str
    label: str
    target: str
    _x: float | object = UNSET
    _y: float | object = UNSET
    _category: str | object = UNSET
    _color: str | None | object = UNSET
    _series_index: int | None | object = UNSET

    def __post_init__(self) -> None:
        if not isinstance(self.id, str) or not self.id.strip():
            raise ValueError("annotation id must be a non-empty string")
        if not isinstance(self.label, str) or not self.label.strip():
            raise ValueError("annotation label must be a non-empty string")
        if self.target not in {"point", "x_value", "y_value", "category"}:
            raise ValueError(f"unsupported annotation target {self.target!r}")

    @classmethod
    def point(cls, id: str, label: str, x: float, y: float) -> "Annotation":
        return cls(id, label, "point", _x=_finite("x", x), _y=_finite("y", y))

    @classmethod
    def x_value(cls, id: str, label: str, x: float) -> "Annotation":
        return cls(id, label, "x_value", _x=_finite("x", x))

    @classmethod
    def y_value(cls, id: str, label: str, y: float) -> "Annotation":
        return cls(id, label, "y_value", _y=_finite("y", y))

    @classmethod
    def category(cls, id: str, label: str, category: str) -> "Annotation":
        if not isinstance(category, str) or not category:
            raise ValueError("annotation category must be a non-empty string")
        return cls(id, label, "category", _category=category)

    def color(self, value: str | None) -> "Annotation":
        if value is not None:
            normalized = value.removeprefix("#")
            if len(normalized) != 6 or any(character not in "0123456789abcdefABCDEF" for character in normalized):
                raise ValueError("annotation color must be #RRGGBB or None")
            value = f"#{normalized.lower()}"
        return replace(self, _color=value)

    def series_index(self, value: int | None) -> "Annotation":
        if value is not None and (not isinstance(value, int) or isinstance(value, bool) or value < 0):
            raise ValueError("annotation series index must be a non-negative integer or None")
        return replace(self, _series_index=value)

    def to_spec(self) -> dict[str, Any]:
        spec: dict[str, Any] = {"id": self.id, "label": self.label, "target": self.target}
        for name, value in (
            ("x", self._x), ("y", self._y), ("category", self._category),
            ("color", self._color), ("series_index", self._series_index),
        ):
            if value is not UNSET:
                spec[name] = value
        return spec


def _finite(name: str, value: float) -> float:
    result = float(value)
    if not isfinite(result):
        raise ValueError(f"annotation {name} must be finite")
    return result


def _chart_range(name: str, minimum: float, maximum: float) -> tuple[float, float]:
    lower = float(minimum)
    upper = float(maximum)
    if not isfinite(lower) or not isfinite(upper) or lower >= upper:
        raise ValueError(f"chart {name} range must be finite and increasing")
    return lower, upper


def _finite_levels(name: str, values: tuple[float, ...] | list[float]) -> tuple[float, ...]:
    result = tuple(float(value) for value in values)
    if not result or any(not isfinite(value) for value in result):
        raise ValueError(f"chart {name} must contain finite values")
    if any(left >= right for left, right in zip(result, result[1:])):
        raise ValueError(f"chart {name} must be strictly increasing")
    return result


@dataclass(frozen=True)
class ChartBuilder:
    chart: str
    id: str
    _data: Dataset | DatasetView | ArrayData | None = None
    _binding: DataBinding | None = None
    _title: str | None | object = UNSET
    _lod: Lod = Lod.AUTO
    _selection_action: str | None | object = UNSET
    _viewport_action: str | None | object = UNSET
    _accessibility_description: str | None | object = UNSET
    _legend_position: LegendPosition | object = UNSET
    _annotations: tuple[Annotation, ...] = ()
    _tiling_method: TilingMethod | object = UNSET
    _padding: float | object = UNSET
    _color_scale: ColorScale | object = UNSET
    _point_radius: float | object = UNSET
    _x_log: bool | object = UNSET
    _y_log: bool | object = UNSET
    _x_label: str | None | object = UNSET
    _y_label: str | None | object = UNSET
    _y2_label: str | None | object = UNSET
    _z_label: str | None | object = UNSET
    _x_range: tuple[float, float] | object = UNSET
    _y_range: tuple[float, float] | object = UNSET
    _y2_range: tuple[float, float] | object = UNSET
    _z_range: tuple[float, float] | object = UNSET
    _stroke_width: float | object = UNSET
    _opacity: float | object = UNSET
    _bar_gap: float | object = UNSET
    _border_radius: float | object = UNSET
    _box_color: str | object = UNSET
    _median_color: str | object = UNSET
    _whisker_color: str | object = UNSET
    _outlier_color: str | object = UNSET
    _box_opacity: float | object = UNSET
    _box_width: float | object = UNSET
    _outlier_radius: float | object = UNSET
    _bins: int | object = UNSET
    _wireframe: bool | object = UNSET
    _width: float | object = UNSET
    _height: float | object = UNSET
    _fill: bool | object = UNSET
    _min_width: float | object = UNSET
    _min_height: float | object = UNSET
    _aspect_ratio: float | object = UNSET
    _thresholds: tuple[float, ...] | object = UNSET
    _levels: tuple[float, ...] | object = UNSET
    _hole: float | object = UNSET
    _colors: tuple[str, ...] | object = UNSET
    _hover: bool | object = UNSET
    _renderer_2d: Renderer2D | object = UNSET
    _vello_backend: VelloBackend | object = UNSET
    _graph_ratio: float | object = UNSET
    _hidden_series: tuple[int, ...] | object = UNSET
    _legend_action: str | None | object = UNSET
    _fill_color: str | object = UNSET
    _primary_color: str | object = UNSET
    _stroke_color: str | object = UNSET
    _pad_angle: float | object = UNSET
    _corner_radius: float | object = UNSET
    _sort: bool | object = UNSET
    _curve: CurveType | object = UNSET
    _dash_style: str | object = UNSET
    _show_points: bool | object = UNSET
    _contour_upsample_factor: int | object = UNSET
    _smooth_strokes: bool | object = UNSET
    _smoothing_iterations: int | object = UNSET
    _smoothing_max_deviation_px: float | object = UNSET

    def __post_init__(self) -> None:
        if not isinstance(self.id, str) or not self.id.strip():
            raise ValueError("chart id must be a non-empty string")
        if self.chart not in {
            "scatter", "line", "area", "box_plot", "bar", "pie", "donut",
            "heatmap", "contour", "isoline", "surface",
            "treemap",
        }:
            raise ValueError(f"unsupported resource chart kind {self.chart!r}")

    def data(self, source: Dataset | DatasetView | ArrayData) -> "ChartBuilder":
        if not isinstance(source, (Dataset, DatasetView, ArrayData)):
            raise TypeError("chart data must be Dataset, DatasetView, or ArrayData")
        return replace(self, _data=source)

    def bind(self, binding: DataBinding) -> "ChartBuilder":
        return replace(self, _binding=binding, _data=binding.source)

    def x(self, field: str) -> "ChartBuilder":
        return self._role("x", field)

    def y(self, field: str) -> "ChartBuilder":
        return self._role("y", field)

    def y2(self, field: str) -> "ChartBuilder":
        if self.chart != "line":
            raise ValueError("y2 role is only supported by line charts")
        return self._role("y2", field)

    def y0(self, field: str) -> "ChartBuilder":
        if self.chart != "area":
            raise ValueError("y0 role is only supported by area charts")
        return self._role("y0", field)

    def color(self, field: str) -> "ChartBuilder":
        return self._role("color", field)

    def size(self, field: str) -> "ChartBuilder":
        return self._role("size", field)

    def label(self, field: str) -> "ChartBuilder":
        return self._role("label", field)

    def series(self, field: str) -> "ChartBuilder":
        return self._role("series", field)

    def dash(self, field: str) -> "ChartBuilder":
        if self.chart != "line":
            raise ValueError("dash role is only supported by line charts")
        return self._role("dash", field)

    def row_id(self, field: str) -> "ChartBuilder":
        return self._role("row_id", field)

    def parent(self, field: str) -> "ChartBuilder":
        return self._role("parent", field)

    def title(self, value: str | None) -> "ChartBuilder":
        return replace(self, _title=value)

    def lod(self, policy: Lod) -> "ChartBuilder":
        return replace(self, _lod=Lod(policy))

    def on_selection_change(self, action: str | None) -> "ChartBuilder":
        if action is not None and not action:
            raise ValueError("selection action must be non-empty or None")
        if action is not None and self.chart not in {"treemap", "scatter", "line"}:
            raise ValueError(f"{self.chart} chart does not support keyed selection events")
        return replace(self, _selection_action=action)

    def on_viewport_change(self, action: str | None) -> "ChartBuilder":
        if action is not None and not action:
            raise ValueError("viewport action must be non-empty or None")
        if action is not None and self.chart not in {"scatter", "line", "surface"}:
            raise ValueError(f"{self.chart} chart does not support viewport events")
        return replace(self, _viewport_action=action)

    def accessibility_description(self, value: str | None) -> "ChartBuilder":
        return replace(self, _accessibility_description=value)

    def legend_position(self, position: LegendPosition) -> "ChartBuilder":
        if self.chart not in {"scatter", "line", "bar"}:
            raise ValueError(f"{self.chart} chart does not support a legend")
        return replace(self, _legend_position=LegendPosition(position))

    def annotation(self, value: Annotation) -> "ChartBuilder":
        if self.chart not in {"scatter", "line", "bar"}:
            raise ValueError(f"{self.chart} chart does not support annotations")
        if not isinstance(value, Annotation):
            raise TypeError("chart annotation must be an Annotation")
        return replace(self, _annotations=(*self._annotations, value))

    def annotations(self, values: tuple[Annotation, ...] | list[Annotation]) -> "ChartBuilder":
        result = replace(self, _annotations=())
        for value in values:
            result = result.annotation(value)
        return result

    def tiling_method(self, method: TilingMethod) -> "ChartBuilder":
        if self.chart != "treemap":
            raise ValueError("tiling_method is only supported by treemap charts")
        return replace(self, _tiling_method=TilingMethod(method))

    def padding(self, value: float) -> "ChartBuilder":
        if self.chart != "treemap":
            raise ValueError("padding is only supported by treemap charts")
        result = float(value)
        if not isfinite(result) or result < 0.0:
            raise ValueError("treemap padding must be finite and non-negative")
        return replace(self, _padding=result)

    def color_scale(self, value: ColorScale | str) -> "ChartBuilder":
        if self.chart not in {"heatmap", "contour", "surface"}:
            raise ValueError(f"{self.chart} chart does not support a color scale")
        scale = ColorScale(value)
        if self.chart == "surface" and scale not in {
            ColorScale.VIRIDIS,
            ColorScale.PLASMA,
            ColorScale.INFERNO,
            ColorScale.COOLWARM,
        }:
            raise ValueError(f"surface chart does not support {scale.value} color scale")
        return replace(self, _color_scale=scale)

    def point_radius(self, value: float) -> "ChartBuilder":
        if self.chart != "scatter":
            raise ValueError("point_radius is only supported by scatter charts")
        radius = float(value)
        if not isfinite(radius) or radius <= 0.0:
            raise ValueError("scatter point radius must be finite and positive")
        return replace(self, _point_radius=radius)

    def x_log(self, enabled: bool = True) -> "ChartBuilder":
        if self.chart not in {"scatter", "line", "area", "surface", "heatmap", "contour", "isoline"}:
            raise ValueError("x_log is only supported by cartesian and dense-grid charts")
        if not isinstance(enabled, bool):
            raise TypeError("x_log must be bool")
        return replace(self, _x_log=enabled)

    def y_log(self, enabled: bool = True) -> "ChartBuilder":
        if self.chart not in {"scatter", "line", "area", "bar", "surface", "heatmap", "contour", "isoline"}:
            raise ValueError("y_log is only supported by cartesian and dense-grid charts")
        if not isinstance(enabled, bool):
            raise TypeError("y_log must be bool")
        return replace(self, _y_log=enabled)

    def x_label(self, value: str | None) -> "ChartBuilder":
        if self.chart not in {"line", "surface"}:
            raise ValueError("x_label is only supported by line and surface charts")
        if value is not None and not isinstance(value, str):
            raise TypeError("x_label must be str or None")
        return replace(self, _x_label=value)

    def y_label(self, value: str | None) -> "ChartBuilder":
        if self.chart not in {"line", "surface"}:
            raise ValueError("y_label is only supported by line and surface charts")
        if value is not None and not isinstance(value, str):
            raise TypeError("y_label must be str or None")
        return replace(self, _y_label=value)

    def y2_label(self, value: str | None) -> "ChartBuilder":
        if self.chart != "line":
            raise ValueError("y2_label is only supported by line charts")
        return replace(self, _y2_label=value)

    def z_label(self, value: str | None) -> "ChartBuilder":
        if self.chart != "surface":
            raise ValueError("z_label is only supported by surface charts")
        if value is not None and not isinstance(value, str):
            raise TypeError("z_label must be str or None")
        return replace(self, _z_label=value)

    def x_range(self, minimum: float, maximum: float) -> "ChartBuilder":
        if self.chart not in {"scatter", "line", "heatmap", "contour", "isoline"}:
            raise ValueError("x_range is only supported by cartesian and dense-grid charts")
        return replace(self, _x_range=_chart_range("x", minimum, maximum))

    def y_range(self, minimum: float, maximum: float) -> "ChartBuilder":
        if self.chart not in {"scatter", "line", "bar", "heatmap", "contour", "isoline"}:
            raise ValueError("y_range is only supported by cartesian and dense-grid charts")
        return replace(self, _y_range=_chart_range("y", minimum, maximum))

    def y2_range(self, minimum: float, maximum: float) -> "ChartBuilder":
        if self.chart != "line":
            raise ValueError("y2_range is only supported by line charts")
        return replace(self, _y2_range=_chart_range("y2", minimum, maximum))

    def z_range(self, minimum: float, maximum: float) -> "ChartBuilder":
        if self.chart != "surface":
            raise ValueError("z_range is only supported by surface charts")
        return replace(self, _z_range=_chart_range("z", minimum, maximum))

    def stroke_width(self, value: float) -> "ChartBuilder":
        if self.chart not in {"line", "isoline", "box_plot"}:
            raise ValueError("stroke_width is unsupported by this chart kind")
        width = float(value)
        if not isfinite(width) or width <= 0.0:
            raise ValueError("stroke width must be finite and positive")
        return replace(self, _stroke_width=width)

    def opacity(self, value: float) -> "ChartBuilder":
        if self.chart not in {
            "scatter",
            "line",
            "area",
            "bar",
            "heatmap",
            "contour",
            "isoline",
        }:
            raise ValueError("opacity is unsupported by this chart kind")
        opacity = float(value)
        if not isfinite(opacity) or not 0.0 <= opacity <= 1.0:
            raise ValueError("chart opacity must be finite between zero and one")
        return replace(self, _opacity=opacity)

    def bar_gap(self, value: float) -> "ChartBuilder":
        if self.chart != "bar":
            raise ValueError("bar_gap is only supported by bar charts")
        gap = float(value)
        if not isfinite(gap) or gap < 0.0:
            raise ValueError("bar gap must be finite and non-negative")
        return replace(self, _bar_gap=gap)

    def border_radius(self, value: float) -> "ChartBuilder":
        if self.chart != "bar":
            raise ValueError("border_radius is only supported by bar charts")
        radius = float(value)
        if not isfinite(radius) or radius < 0.0:
            raise ValueError("bar border radius must be finite and non-negative")
        return replace(self, _border_radius=radius)

    def box_color(self, value: str) -> "ChartBuilder":
        return self._boxplot_color("box_color", value)

    def median_color(self, value: str) -> "ChartBuilder":
        return self._boxplot_color("median_color", value)

    def whisker_color(self, value: str) -> "ChartBuilder":
        return self._boxplot_color("whisker_color", value)

    def outlier_color(self, value: str) -> "ChartBuilder":
        return self._boxplot_color("outlier_color", value)

    def box_opacity(self, value: float) -> "ChartBuilder":
        if self.chart != "box_plot":
            raise ValueError("box_opacity is only supported by box plots")
        opacity = float(value)
        if not isfinite(opacity) or not 0.0 <= opacity <= 1.0:
            raise ValueError("box opacity must be finite between zero and one")
        return replace(self, _box_opacity=opacity)

    def box_width(self, value: float) -> "ChartBuilder":
        if self.chart != "box_plot":
            raise ValueError("box_width is only supported by box plots")
        width = float(value)
        if not isfinite(width) or width <= 0.0:
            raise ValueError("box width must be finite and positive")
        return replace(self, _box_width=width)

    def outlier_radius(self, value: float) -> "ChartBuilder":
        if self.chart != "box_plot":
            raise ValueError("outlier_radius is only supported by box plots")
        radius = float(value)
        if not isfinite(radius) or radius <= 0.0:
            raise ValueError("outlier radius must be finite and positive")
        return replace(self, _outlier_radius=radius)

    def bins(self, value: int) -> "ChartBuilder":
        if self.chart != "box_plot":
            raise ValueError("bins is only supported by box plots")
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
            raise ValueError("box plot bins must be a positive integer")
        return replace(self, _bins=value)

    def wireframe(self, enabled: bool = True) -> "ChartBuilder":
        if self.chart != "surface":
            raise ValueError("wireframe is only supported by surface charts")
        if not isinstance(enabled, bool):
            raise TypeError("wireframe must be bool")
        return replace(self, _wireframe=enabled)

    def aspect_ratio(self, value: float) -> "ChartBuilder":
        ratio = float(value)
        if not isfinite(ratio) or ratio <= 0.0:
            raise ValueError("chart aspect ratio must be finite and positive")
        return replace(self, _aspect_ratio=ratio)

    def dimensions(self, width: float, height: float) -> "ChartBuilder":
        """Set fixed live dimensions where ``size`` names a semantic role."""

        resolved_width, resolved_height = float(width), float(height)
        if (
            not isfinite(resolved_width)
            or resolved_width <= 0.0
            or not isfinite(resolved_height)
            or resolved_height <= 0.0
        ):
            raise ValueError("chart dimensions must be finite and positive")
        return replace(
            self,
            _width=resolved_width,
            _height=resolved_height,
            _fill=UNSET,
        )

    def fill(self) -> "ChartBuilder":
        return replace(self, _fill=True, _width=UNSET, _height=UNSET)

    def min_size(self, width: float, height: float) -> "ChartBuilder":
        resolved_width, resolved_height = float(width), float(height)
        if (
            not isfinite(resolved_width)
            or resolved_width <= 0.0
            or not isfinite(resolved_height)
            or resolved_height <= 0.0
        ):
            raise ValueError("chart minimum size must be finite and positive")
        return replace(self, _min_width=resolved_width, _min_height=resolved_height)

    def thresholds(self, values: tuple[float, ...] | list[float]) -> "ChartBuilder":
        if self.chart != "contour":
            raise ValueError("thresholds are only supported by contour charts")
        return replace(self, _thresholds=_finite_levels("thresholds", values))

    def levels(self, values: tuple[float, ...] | list[float]) -> "ChartBuilder":
        if self.chart != "isoline":
            raise ValueError("levels are only supported by isoline charts")
        return replace(self, _levels=_finite_levels("levels", values))

    def hole(self, fraction: float) -> "ChartBuilder":
        if self.chart not in {"pie", "donut"}:
            raise ValueError("hole is only supported by pie and donut charts")
        value = float(fraction)
        if not isfinite(value) or not 0.0 <= value < 1.0:
            raise ValueError("pie hole must be finite in [0, 1)")
        return replace(self, _hole=value)

    def colors(self, values: tuple[str, ...] | list[str]) -> "ChartBuilder":
        if self.chart not in {"pie", "donut", "treemap"}:
            raise ValueError("colors are only supported by pie, donut, and treemap charts")
        normalized = tuple(_normalize_hex_color(value) for value in values)
        if not normalized:
            raise ValueError("chart colors must not be empty")
        return replace(self, _colors=normalized)

    def hover(self, enabled: bool = True) -> "ChartBuilder":
        if self.chart != "treemap":
            raise ValueError("hover is only supported by treemap charts")
        if not isinstance(enabled, bool):
            raise TypeError("treemap hover must be bool")
        return replace(self, _hover=enabled)

    def renderer_2d(self, renderer: Renderer2D | str) -> "ChartBuilder":
        if self.chart == "surface":
            raise ValueError("renderer_2d is only supported by 2D charts")
        return replace(self, _renderer_2d=Renderer2D(renderer))

    def vello_backend(self, backend: VelloBackend | str) -> "ChartBuilder":
        if self.chart == "surface":
            raise ValueError("vello_backend is only supported by 2D charts")
        return replace(self, _vello_backend=VelloBackend(backend))

    def graph_ratio(self, value: float) -> "ChartBuilder":
        if self.chart not in {"scatter", "line", "bar"}:
            raise ValueError("graph_ratio is only supported by scatter, line, and bar charts")
        ratio = float(value)
        if not isfinite(ratio) or ratio <= 0.0:
            raise ValueError("graph ratio must be finite and positive")
        return replace(self, _graph_ratio=ratio)

    def hidden_series(self, indices: tuple[int, ...] | list[int]) -> "ChartBuilder":
        if self.chart != "line":
            raise ValueError("hidden_series is only supported by line charts")
        normalized = tuple(indices)
        if any(
            not isinstance(index, int) or isinstance(index, bool) or index < 0
            for index in normalized
        ):
            raise ValueError("hidden series indices must be non-negative integers")
        if len(set(normalized)) != len(normalized):
            raise ValueError("hidden series indices must not contain duplicates")
        return replace(self, _hidden_series=normalized)

    def on_legend_click(self, action: str | None) -> "ChartBuilder":
        if self.chart != "line":
            raise ValueError("legend click events are only supported by line charts")
        if action is not None and (not isinstance(action, str) or not action.strip()):
            raise ValueError("legend action must be non-empty or None")
        return replace(self, _legend_action=action)

    def fill_color(self, value: str) -> "ChartBuilder":
        if self.chart != "area":
            raise ValueError("fill_color is only supported by area charts")
        return replace(self, _fill_color=_normalize_hex_color(value))

    def primary_color(self, value: str) -> "ChartBuilder":
        if self.chart not in {"scatter", "line", "bar"}:
            raise ValueError("primary_color is only supported by scatter, line, and bar charts")
        return replace(self, _primary_color=_normalize_hex_color(value))

    def stroke_color(self, value: str) -> "ChartBuilder":
        if self.chart != "isoline":
            raise ValueError("stroke_color is only supported by isoline charts")
        return replace(self, _stroke_color=_normalize_hex_color(value))

    def pad_angle(self, angle: float) -> "ChartBuilder":
        if self.chart not in {"pie", "donut"}:
            raise ValueError("pad_angle is only supported by pie and donut charts")
        value = float(angle)
        if not isfinite(value) or value < 0.0:
            raise ValueError("pie pad angle must be finite and non-negative")
        return replace(self, _pad_angle=value)

    def corner_radius(self, radius: float) -> "ChartBuilder":
        if self.chart not in {"pie", "donut"}:
            raise ValueError("corner_radius is only supported by pie and donut charts")
        value = float(radius)
        if not isfinite(value) or value < 0.0:
            raise ValueError("pie corner radius must be finite and non-negative")
        return replace(self, _corner_radius=value)

    def sort(self, enabled: bool = True) -> "ChartBuilder":
        if self.chart not in {"pie", "donut"}:
            raise ValueError("sort is only supported by pie and donut charts")
        if not isinstance(enabled, bool):
            raise TypeError("sort must be bool")
        return replace(self, _sort=enabled)

    def curve(self, value: CurveType | str) -> "ChartBuilder":
        if self.chart not in {"line", "area"}:
            raise ValueError("curve is only supported by line and area charts")
        return replace(self, _curve=CurveType(value))

    def dash_style(self, value: str) -> "ChartBuilder":
        if self.chart != "line":
            raise ValueError("dash_style is only supported by line charts")
        if value not in {"solid", "dashed", "dotted", "dash_dot"}:
            raise ValueError("line dash_style must be solid, dashed, dotted, or dash_dot")
        return replace(self, _dash_style=value)

    def show_points(self, enabled: bool = True) -> "ChartBuilder":
        if self.chart != "line":
            raise ValueError("show_points is only supported by line charts")
        if not isinstance(enabled, bool):
            raise TypeError("show_points must be bool")
        return replace(self, _show_points=enabled)

    def contour_upsample_factor(self, factor: int) -> "ChartBuilder":
        if self.chart not in {"contour", "isoline"}:
            raise ValueError(
                "contour_upsample_factor is only supported by contour and isoline charts"
            )
        if not isinstance(factor, int) or isinstance(factor, bool) or not 1 <= factor <= 8:
            raise ValueError("contour upsample factor must be an integer in [1, 8]")
        return replace(self, _contour_upsample_factor=factor)

    def smooth_strokes(self, enabled: bool = True) -> "ChartBuilder":
        if self.chart != "isoline":
            raise ValueError("smooth_strokes is only supported by isoline charts")
        if not isinstance(enabled, bool):
            raise TypeError("smooth_strokes must be bool")
        return replace(self, _smooth_strokes=enabled)

    def smoothing_iterations(self, iterations: int) -> "ChartBuilder":
        if self.chart != "isoline":
            raise ValueError("smoothing_iterations is only supported by isoline charts")
        if (
            not isinstance(iterations, int)
            or isinstance(iterations, bool)
            or not 0 <= iterations <= 4
        ):
            raise ValueError("isoline smoothing iterations must be an integer in [0, 4]")
        return replace(self, _smoothing_iterations=iterations)

    def smoothing_max_deviation_px(self, deviation: float) -> "ChartBuilder":
        if self.chart != "isoline":
            raise ValueError(
                "smoothing_max_deviation_px is only supported by isoline charts"
            )
        value = float(deviation)
        if not isfinite(value) or value < 0.0:
            raise ValueError("isoline smoothing deviation must be finite and non-negative")
        return replace(self, _smoothing_max_deviation_px=value)

    def request_svg_export(
        self,
        context: "SessionContext",
        request_id: str,
        *,
        options: StaticSvgOptions | None = None,
        width: float = 800.0,
        height: float = 600.0,
    ) -> None:
        if not request_id.strip():
            raise ValueError("SVG export request id must be non-empty")
        if options is not None and not isinstance(options, StaticSvgOptions):
            raise TypeError("SVG export options must be StaticSvgOptions or None")
        if options is not None and (width != 800.0 or height != 600.0):
            raise ValueError("SVG export accepts options or width/height, not both")
        resolved = options or StaticSvgOptions.new(width, height)
        context.command(
            request_id,
            "px.export_svg",
            chart=self.to_spec(),
            options=resolved.to_spec(),
        )

    def request_accessibility_summary(
        self, context: "SessionContext", request_id: str
    ) -> None:
        """Request gpui-px's native summary for this resource chart."""
        if not isinstance(request_id, str) or not request_id.strip():
            raise ValueError("chart accessibility request id must be non-empty")
        context.command(
            request_id,
            "px.chart_accessibility_summary",
            chart=self.to_spec(),
        )

    def request_metadata(self, context: "SessionContext", request_id: str) -> None:
        """Request native accessibility, legend, and annotation results."""
        if not isinstance(request_id, str) or not request_id.strip():
            raise ValueError("chart metadata request id must be non-empty")
        if self.chart not in {"scatter", "line", "bar"}:
            raise ValueError(
                "native legend and annotation summaries are only available for scatter, line, and bar charts"
            )
        context.command(
            request_id,
            "px.chart_metadata",
            chart=self.to_spec(),
        )

    def request_viewport(self, context: "SessionContext", request_id: str) -> None:
        """Query the retained host viewport for an interactive cartesian chart."""
        if not request_id.strip():
            raise ValueError("viewport query request id must be non-empty")
        if self.chart not in {"scatter", "line"}:
            raise ValueError("viewport queries are only supported by scatter and line charts")
        if self._viewport_action in {UNSET, None} and self._selection_action in {UNSET, None}:
            raise ValueError(
                "viewport queries require on_viewport_change or on_selection_change so the host retains interaction state"
            )
        context.command(request_id, "px.query_viewport", chart_id=self.id)

    def request_camera(self, context: "SessionContext", request_id: str) -> None:
        """Query the retained viewport camera for a rendered Surface3D chart."""
        self._request_surface_camera(context, request_id, "px.query_surface_camera")

    def reset_camera(self, context: "SessionContext", request_id: str) -> None:
        """Reset a rendered Surface3D chart and return its native camera state."""
        self._request_surface_camera(context, request_id, "px.reset_surface_camera")

    def _request_surface_camera(
        self, context: "SessionContext", request_id: str, command: str
    ) -> None:
        if self.chart != "surface":
            raise ValueError("camera operations are only supported by surface charts")
        if not isinstance(request_id, str) or not request_id.strip():
            raise ValueError("surface camera request id must be non-empty")
        context.command(request_id, command, chart_id=self.id)

    def to_spec(self) -> dict[str, Any]:
        if self._data is None:
            raise ValueError("chart requires .data(...) before serialization")
        if self._selection_action is not UNSET and self._selection_action is not None and isinstance(self._data, (Dataset, DatasetView)):
            dataset = self._data.dataset if isinstance(self._data, DatasetView) else self._data
            if dataset.key is None:
                raise ValueError("chart selection requires a dataset primary key")
        binding = self._binding or DataBinding(self._data)
        if self.chart == "treemap":
            if not isinstance(self._data, (Dataset, DatasetView)):
                raise ValueError("treemap chart requires a Dataset or DatasetView source")
            roles = binding.roles
            missing = {"row_id", "parent", "size"} - roles.keys()
            if missing:
                raise ValueError(f"treemap chart missing roles: {', '.join(sorted(missing))}")
            if self._selection_action is not UNSET and self._selection_action is not None:
                dataset = (
                    self._data.dataset
                    if isinstance(self._data, DatasetView)
                    else self._data
                )
                if dataset.key != roles["row_id"]:
                    raise ValueError("treemap selection requires row_id to match the dataset primary key")
        if isinstance(self._data, (Dataset, DatasetView)):
            roles = binding.roles
            if isinstance(self._data, DatasetView):
                aggregated = any(
                    operation.get("op") == "aggregate"
                    for operation in self._data.operations
                )
                sort = next(
                    (
                        operation
                        for operation in self._data.operations
                        if operation.get("op") == "sort"
                    ),
                    None,
                )
                if sort is not None:
                    if not aggregated and self.chart not in {"scatter", "line"}:
                        raise ValueError(
                            f"{self.chart} chart does not support DatasetView.sort yet"
                        )
                    if aggregated and sort["field"] not in set(roles.values()):
                        raise ValueError(
                            "aggregate chart DatasetView.sort field must match a bound role"
                        )
                    if not aggregated and sort["field"] not in {roles.get("x"), roles.get("y")}:
                        raise ValueError(
                            "chart DatasetView.sort field must match the x or y role"
                        )
                    if not aggregated and any(
                        operation.get("op") == "range"
                        for operation in self._data.operations
                    ):
                        raise ValueError(
                            "chart DatasetView.sort plus range is not supported yet"
                        )
            if (
                self._selection_action is not UNSET
                and self._selection_action is not None
                and self.chart in {"scatter", "line"}
            ):
                dataset = self._data.dataset if isinstance(self._data, DatasetView) else self._data
                if "row_id" not in roles:
                    raise ValueError(f"{self.chart} selection requires a row_id field")
                if roles["row_id"] != dataset.key:
                    raise ValueError(
                        f"{self.chart} selection requires row_id to match the dataset primary key"
                    )
            if ("series" in roles or "color" in roles) and self.chart not in {
                "scatter",
                "line",
                "bar",
            }:
                raise ValueError(f"{self.chart} chart does not support series or color roles yet")
            if "dash" in roles and self.chart != "line":
                raise ValueError("dash role is only supported by line charts")
            if "y0" in roles and self.chart != "area":
                raise ValueError("y0 role is only supported by area charts")
            if "y2" in roles and self.chart != "line":
                raise ValueError("y2 role is only supported by line charts")
            if self.chart in {"bar", "pie", "donut"}:
                if "label" not in roles and "x" not in roles:
                    raise ValueError(f"{self.chart} chart requires a label or x field")
                if "y" not in roles:
                    raise ValueError(f"{self.chart} chart requires a y field")
            elif self.chart not in {"heatmap", "contour", "isoline", "surface", "treemap"}:
                if "x" not in roles or "y" not in roles:
                    raise ValueError(f"{self.chart} chart requires x and y fields")
        spec = {"kind": "px_chart_v2", "chart": self.chart, "id": self.id, "data": binding.to_spec(), "lod": self._lod.value}
        if self._title is not UNSET:
            spec["title"] = self._title
        if self._selection_action is not UNSET:
            spec["selection_action"] = self._selection_action
        if self._viewport_action is not UNSET:
            spec["viewport_action"] = self._viewport_action
        if self._accessibility_description is not UNSET:
            spec["accessibility_description"] = self._accessibility_description
        if self._legend_position is not UNSET:
            spec["legend_position"] = self._legend_position.value
        if self._annotations:
            spec["annotations"] = [annotation.to_spec() for annotation in self._annotations]
        if self._tiling_method is not UNSET:
            spec["tiling_method"] = self._tiling_method.value
        if self._padding is not UNSET:
            spec["padding"] = self._padding
        if self._color_scale is not UNSET:
            spec["color_scale"] = self._color_scale.value
        if self._point_radius is not UNSET:
            spec["point_radius"] = self._point_radius
        for name, value in (
            ("x_log", self._x_log),
            ("y_log", self._y_log),
            ("x_label", self._x_label),
            ("y_label", self._y_label),
            ("y2_label", self._y2_label),
            ("z_label", self._z_label),
            ("x_range", self._x_range),
            ("y_range", self._y_range),
            ("y2_range", self._y2_range),
            ("z_range", self._z_range),
            ("stroke_width", self._stroke_width),
            ("opacity", self._opacity),
            ("bar_gap", self._bar_gap),
            ("border_radius", self._border_radius),
            ("box_color", self._box_color),
            ("median_color", self._median_color),
            ("whisker_color", self._whisker_color),
            ("outlier_color", self._outlier_color),
            ("box_opacity", self._box_opacity),
            ("box_width", self._box_width),
            ("outlier_radius", self._outlier_radius),
            ("bins", self._bins),
            ("wireframe", self._wireframe),
            ("width", self._width),
            ("height", self._height),
            ("fill", self._fill),
            ("min_width", self._min_width),
            ("min_height", self._min_height),
            ("aspect_ratio", self._aspect_ratio),
            ("thresholds", self._thresholds),
            ("levels", self._levels),
            ("hole", self._hole),
            ("colors", self._colors),
            ("hover", self._hover),
            ("renderer_2d", self._renderer_2d),
            ("vello_backend", self._vello_backend),
            ("graph_ratio", self._graph_ratio),
            ("hidden_series", self._hidden_series),
            ("legend_action", self._legend_action),
            ("fill_color", self._fill_color),
            ("primary_color", self._primary_color),
            ("stroke_color", self._stroke_color),
            ("pad_angle", self._pad_angle),
            ("corner_radius", self._corner_radius),
            ("sort", self._sort),
            ("dash_style", self._dash_style),
            ("show_points", self._show_points),
            ("contour_upsample_factor", self._contour_upsample_factor),
            ("smooth_strokes", self._smooth_strokes),
            ("smoothing_iterations", self._smoothing_iterations),
            ("smoothing_max_deviation_px", self._smoothing_max_deviation_px),
        ):
            if value is not UNSET:
                if isinstance(value, Enum):
                    spec[name] = value.value
                else:
                    spec[name] = (
                        list(value)
                        if name.endswith("_range")
                        or name in {"thresholds", "levels", "colors", "hidden_series"}
                        else value
                    )
        if self._curve is not UNSET:
            spec["curve"] = self._curve.value
        return spec

    def _role(self, name: str, field: str) -> "ChartBuilder":
        if self._data is None:
            raise ValueError(f".{name}(...) requires .data(...) first")
        binding = self._binding or DataBinding(self._data)
        return replace(self, _binding=binding.role(name, field))

    def _boxplot_color(self, name: str, value: str) -> "ChartBuilder":
        if self.chart != "box_plot":
            raise ValueError(f"{name} is only supported by box plots")
        return replace(self, **{f"_{name}": _normalize_hex_color(value)})


@dataclass(frozen=True)
class MeshPlotBuilder:
    """Immutable resource-backed builder for native ``gpui-px`` mesh plots."""

    id: str = "mesh_plot"
    _geometry: MeshGeometry | None = None
    _field: MeshScalarField | None = None
    _revision: int = 0
    _view: str = "planar"
    _mode: str = "mesh"
    _color_scale: str = "viridis"
    _color_range: str | tuple[float, float] | dict[str, Any] = "auto"
    _missing_value_policy: str = "reject"
    _wireframe: bool = True
    _title: str | None = None
    _width: float | None = None
    _height: float | None = None
    _fill: bool = False
    _min_width: float | None = None
    _min_height: float | None = None
    _aspect_ratio: float | None = None
    _selection: dict[str, Any] | None = None
    _camera: dict[str, Any] | None = None
    _viewport: dict[str, Any] | None = None
    _contour_levels: dict[str, Any] | None = None
    _equal_aspect: bool = True
    _axes: dict[str, Any] | None = None
    _interactions: tuple[str, ...] = ("pan", "zoom", "inspect", "select", "reset", "fit")
    _toolbar: bool = True
    _hidden_toolbar_actions: tuple[str, ...] = ()
    _colorbar: MeshColorbar | None = None
    _renderer_backend: MeshPlotBackend = MeshPlotBackend.AUTO
    _revolve: MeshRevolve | None = None
    _selection_action: str | None = None
    _export_action: str | None = None

    def __post_init__(self) -> None:
        if not isinstance(self.id, str) or not self.id.strip():
            raise ValueError("mesh plot id must be a non-empty string")

    @property
    def selection_action(self) -> str | None:
        """Action consumed by :func:`gpui_toolkit.ui.mesh_plot`."""
        return self._selection_action

    @property
    def export_action(self) -> str | None:
        """Toolbar-export action consumed by :func:`gpui_toolkit.ui.mesh_plot`."""

        return self._export_action

    def geometry(self, value: MeshGeometry) -> "MeshPlotBuilder":
        if not isinstance(value, MeshGeometry):
            raise TypeError("mesh geometry must be MeshGeometry")
        return replace(self, _geometry=value)

    def field(self, value: MeshScalarField | None) -> "MeshPlotBuilder":
        if value is not None and not isinstance(value, MeshScalarField):
            raise TypeError("mesh field must be MeshScalarField or None")
        return replace(self, _field=value)

    def revision(self, value: int) -> "MeshPlotBuilder":
        return replace(self, _revision=value)

    def view(self, value: str) -> "MeshPlotBuilder":
        return replace(self, _view=value)

    def mode(self, value: str) -> "MeshPlotBuilder":
        return replace(self, _mode=value)

    def color_scale(self, value: ColorScale | str) -> "MeshPlotBuilder":
        return replace(self, _color_scale=ColorScale(value).value)

    def color_range(
        self, value: ColorRange | str | tuple[float, float] | dict[str, Any]
    ) -> "MeshPlotBuilder":
        return replace(
            self,
            _color_range=_mesh_color_range_spec(value) if isinstance(value, ColorRange) else value,
        )

    def missing_value_policy(self, value: str) -> "MeshPlotBuilder":
        return replace(self, _missing_value_policy=value)

    def wireframe(self, value: bool) -> "MeshPlotBuilder":
        if not isinstance(value, bool):
            raise TypeError("mesh wireframe must be bool")
        return replace(self, _wireframe=value)

    def title(self, value: str | None) -> "MeshPlotBuilder":
        return replace(self, _title=value)

    def size(self, width: float | None, height: float | None) -> "MeshPlotBuilder":
        if width is None and height is None:
            return replace(self, _width=None, _height=None, _fill=False)
        if width is None or height is None:
            raise ValueError("mesh fixed size requires both width and height")
        resolved_width, resolved_height = float(width), float(height)
        if not isfinite(resolved_width) or resolved_width <= 0.0 or not isfinite(resolved_height) or resolved_height <= 0.0:
            raise ValueError("mesh fixed size must be finite and positive")
        return replace(self, _width=resolved_width, _height=resolved_height, _fill=False)

    def fill(self) -> "MeshPlotBuilder":
        return replace(self, _fill=True, _width=None, _height=None)

    def min_size(self, width: float, height: float) -> "MeshPlotBuilder":
        resolved_width, resolved_height = float(width), float(height)
        if not isfinite(resolved_width) or resolved_width <= 0.0 or not isfinite(resolved_height) or resolved_height <= 0.0:
            raise ValueError("mesh minimum size must be finite and positive")
        return replace(self, _min_width=resolved_width, _min_height=resolved_height)

    def aspect_ratio(self, value: float) -> "MeshPlotBuilder":
        ratio = float(value)
        if not isfinite(ratio) or ratio <= 0.0:
            raise ValueError("mesh aspect ratio must be finite and positive")
        return replace(self, _aspect_ratio=ratio)

    def selection(self, value: dict[str, Any] | None) -> "MeshPlotBuilder":
        return replace(self, _selection=value)

    def camera(self, value: dict[str, Any] | None) -> "MeshPlotBuilder":
        return replace(self, _camera=value)

    def viewport(self, value: dict[str, Any] | None) -> "MeshPlotBuilder":
        return replace(self, _viewport=value)

    def contour_levels(self, value: dict[str, Any] | None) -> "MeshPlotBuilder":
        return replace(self, _contour_levels=value)

    def equal_aspect(self, value: bool) -> "MeshPlotBuilder":
        if not isinstance(value, bool):
            raise TypeError("mesh equal_aspect must be bool")
        return replace(self, _equal_aspect=value)

    def axes(self, value: dict[str, Any] | None) -> "MeshPlotBuilder":
        return replace(self, _axes=value)

    def interactions(self, values: tuple[str, ...] | list[str]) -> "MeshPlotBuilder":
        return replace(self, _interactions=tuple(values))

    def toolbar(self, enabled: bool) -> "MeshPlotBuilder":
        if not isinstance(enabled, bool):
            raise TypeError("mesh toolbar must be bool")
        return replace(self, _toolbar=enabled)

    def toolbar_action_hidden(self, action: str, hidden: bool = True) -> "MeshPlotBuilder":
        if not isinstance(hidden, bool):
            raise TypeError("mesh toolbar hidden flag must be bool")
        actions = set(self._hidden_toolbar_actions)
        if hidden:
            actions.add(action)
        else:
            actions.discard(action)
        return replace(self, _hidden_toolbar_actions=tuple(sorted(actions)))

    def colorbar(self, value: MeshColorbar | None) -> "MeshPlotBuilder":
        if value is not None and not isinstance(value, MeshColorbar):
            raise TypeError("mesh colorbar must be MeshColorbar or None")
        return replace(self, _colorbar=value)

    def renderer_backend(self, value: MeshPlotBackend | str) -> "MeshPlotBuilder":
        """Select the retained renderer for live native presentation."""

        return replace(self, _renderer_backend=MeshPlotBackend(value))

    def revolve(self, value: MeshRevolve | None) -> "MeshPlotBuilder":
        if value is not None and not isinstance(value, MeshRevolve):
            raise TypeError("mesh revolve must be MeshRevolve or None")
        return replace(self, _revolve=value)

    def on_selection_change(self, action: str | None) -> "MeshPlotBuilder":
        if action is not None and (not isinstance(action, str) or not action.strip()):
            raise ValueError("mesh selection action must be non-empty or None")
        return replace(self, _selection_action=action)

    def on_export(self, action: str | None) -> "MeshPlotBuilder":
        """Emit the native toolbar SVG export result as a typed host event."""

        if action is not None and (not isinstance(action, str) or not action.strip()):
            raise ValueError("mesh export action must be non-empty or None")
        return replace(self, _export_action=action)

    def to_plot(self) -> MeshPlotSpec:
        if self._geometry is None:
            raise ValueError("mesh plot requires .geometry(...) before serialization")
        return MeshPlotSpec(
            geometry=self._geometry, id=self.id, revision=self._revision,
            field=self._field, view=self._view, mode=self._mode,
            color_scale=self._color_scale, color_range=self._color_range,
            missing_value_policy=self._missing_value_policy,
            wireframe=self._wireframe, title=self._title, width=self._width,
            height=self._height, selection=self._selection, camera=self._camera,
            fill=self._fill, min_width=self._min_width, min_height=self._min_height,
            aspect_ratio=self._aspect_ratio,
            viewport=self._viewport, contour_levels=self._contour_levels,
            equal_aspect=self._equal_aspect, axes=self._axes,
            interactions=self._interactions, toolbar=self._toolbar,
            hidden_toolbar_actions=self._hidden_toolbar_actions,
            colorbar=None if self._colorbar is None else self._colorbar.to_spec(),
            renderer_backend=self._renderer_backend.value,
            revolve=self._revolve,
        )

    def to_spec(self) -> dict[str, Any]:
        return self.to_plot().to_spec()

    def request_svg_export(
        self, context: "SessionContext", request_id: str, *,
        options: StaticSvgOptions | None = None,
        width: float = 800.0, height: float = 600.0,
    ) -> None:
        request_mesh_svg_export(
            self.to_plot(), context, request_id,
            options=options, width=width, height=height,
        )


    def request_accessibility_summary(
        self, context: "SessionContext", request_id: str
    ) -> None:
        """Request the native MeshPlot accessibility result without inline data."""
        if not isinstance(request_id, str) or not request_id.strip():
            raise ValueError("mesh accessibility request id must be non-empty")
        context.command(
            request_id,
            "px.mesh_accessibility_summary",
            plot=self.to_spec(),
        )


def scatter(id: str = "scatter") -> ChartBuilder:
    return ChartBuilder("scatter", id)


def line(id: str = "line") -> ChartBuilder:
    return ChartBuilder("line", id)


def area(id: str = "area") -> ChartBuilder:
    """Create a resource-backed area chart declaration."""
    return ChartBuilder("area", id)


def boxplot(id: str = "boxplot") -> ChartBuilder:
    """Create a resource-backed box-plot declaration."""
    return ChartBuilder("box_plot", id)


def heatmap(id: str = "heatmap") -> ChartBuilder:
    return ChartBuilder("heatmap", id)


def contour(id: str = "contour") -> ChartBuilder:
    return ChartBuilder("contour", id)


def isoline(id: str = "isoline") -> ChartBuilder:
    return ChartBuilder("isoline", id)


def surface(id: str = "surface") -> ChartBuilder:
    return ChartBuilder("surface", id)


def pie(id: str = "pie") -> ChartBuilder:
    return ChartBuilder("pie", id)


def donut(id: str = "donut") -> ChartBuilder:
    return ChartBuilder("donut", id)


def bar(id: str = "bar") -> ChartBuilder:
    return ChartBuilder("bar", id)


def treemap(id: str = "treemap") -> ChartBuilder:
    return ChartBuilder("treemap", id)


def mesh(id: str = "mesh_plot") -> MeshPlotBuilder:
    """Create an immutable resource-backed mesh plot builder."""
    return MeshPlotBuilder(id=id)


def mesh_geometry(
    positions: Resource | ArrayData,
    triangles: Resource | ArrayData,
    *,
    id: str = "mesh",
    vertex_ids: Resource | ArrayData | None = None,
    cell_ids: Resource | ArrayData | None = None,
) -> MeshGeometry:
    """Bind mesh topology to revisioned binary resources."""
    resources = (positions, triangles, vertex_ids, cell_ids)
    if any(value is not None and not isinstance(value, (Resource, ArrayData)) for value in resources):
        raise TypeError("mesh_geometry requires Resource or ArrayData handles")
    if isinstance(positions, Resource) and isinstance(triangles, Resource):
        if vertex_ids is not None and not isinstance(vertex_ids, Resource):
            raise TypeError("legacy mesh resources cannot mix with ArrayData ids")
        if cell_ids is not None and not isinstance(cell_ids, Resource):
            raise TypeError("legacy mesh resources cannot mix with ArrayData ids")
        return _resource_mesh_geometry(
            positions, triangles, id=id,
            vertex_ids_resource=vertex_ids, cell_ids_resource=cell_ids,
        )
    if not isinstance(positions, ArrayData) or not isinstance(triangles, ArrayData):
        raise TypeError("ArrayData mesh positions and triangles must use the same resource model")
    if len(positions.shape) != 2 or positions.shape[1] != 3:
        raise ValueError("mesh positions ArrayData shape must be [vertices, 3]")
    if len(triangles.shape) != 2 or triangles.shape[1] != 3:
        raise ValueError("mesh triangles ArrayData shape must be [triangles, 3]")
    if positions.dtype.lower() not in {"f32", "float32", "f64", "float64"}:
        raise ValueError("mesh positions ArrayData dtype must be f32 or f64")
    if triangles.dtype.lower() not in {"u8", "uint8", "u16", "uint16", "u32", "uint32"}:
        raise ValueError("mesh triangles ArrayData dtype must be unsigned integer up to u32")
    for name, value, expected in (
        ("vertex_ids", vertex_ids, positions.shape[0]),
        ("cell_ids", cell_ids, triangles.shape[0]),
    ):
        if value is not None:
            if not isinstance(value, ArrayData):
                raise TypeError(f"ArrayData mesh {name} must also be ArrayData")
            if tuple(value.shape) != (expected,):
                raise ValueError(f"mesh {name} ArrayData shape must be [{expected}]")
            if value.dtype.lower() not in {"u8", "uint8", "u16", "uint16", "u32", "uint32", "u64", "uint64"}:
                raise ValueError(f"mesh {name} ArrayData dtype must be unsigned integer")
    return MeshGeometry(
        positions=(), triangles=(), id=id,
        positions_resource_id=positions.id, positions_generation=positions.generation,
        positions_shape=tuple(positions.shape), positions_dtype=positions.dtype,
        triangles_resource_id=triangles.id, triangles_generation=triangles.generation,
        triangles_shape=tuple(triangles.shape), triangles_dtype=triangles.dtype,
        vertex_ids_resource_id=None if vertex_ids is None else vertex_ids.id,
        vertex_ids_generation=None if vertex_ids is None else vertex_ids.generation,
        vertex_ids_shape=None if vertex_ids is None else tuple(vertex_ids.shape),
        vertex_ids_dtype=None if vertex_ids is None else vertex_ids.dtype,
        cell_ids_resource_id=None if cell_ids is None else cell_ids.id,
        cell_ids_generation=None if cell_ids is None else cell_ids.generation,
        cell_ids_shape=None if cell_ids is None else tuple(cell_ids.shape),
        cell_ids_dtype=None if cell_ids is None else cell_ids.dtype,
    )


def mesh_field(
    values: Resource | ArrayData,
    *,
    association: str = "vertex",
    id: str = "field",
    label: str | None = None,
    unit: str | None = None,
    valid: Resource | ArrayData | None = None,
) -> MeshScalarField:
    """Bind a scalar field and optional validity mask to binary resources."""
    if not isinstance(values, (Resource, ArrayData)):
        raise TypeError("mesh_field requires a Resource or ArrayData handle")
    if isinstance(values, ArrayData):
        if len(values.shape) != 1:
            raise ValueError("mesh field ArrayData must be one-dimensional")
        if values.dtype.lower() not in {"f32", "float32", "f64", "float64"}:
            raise ValueError("mesh field ArrayData dtype must be f32 or f64")
        if valid is not None:
            if not isinstance(valid, ArrayData):
                raise TypeError("ArrayData mesh field validity must also be ArrayData")
            if tuple(valid.shape) != tuple(values.shape) or valid.dtype.lower() not in {"bool", "u8", "uint8"}:
                raise ValueError("mesh validity ArrayData must match field shape and use bool/u8 dtype")
        return MeshScalarField(
            values=(), association=association, id=id, label=label, unit=unit,
            resource_id=values.id, generation=values.generation,
            shape=tuple(values.shape), dtype=values.dtype,
            valid_resource_id=None if valid is None else valid.id,
            valid_generation=None if valid is None else valid.generation,
            valid_shape=None if valid is None else tuple(valid.shape),
            valid_dtype=None if valid is None else valid.dtype,
        )
    if valid is not None and not isinstance(valid, Resource):
        raise TypeError("legacy mesh field resources cannot mix with ArrayData validity")
    return _resource_mesh_field(
        values.id,
        values.generation,
        association=association,
        id=id,
        label=label,
        unit=unit,
        valid_resource_id=None if valid is None else valid.id,
        valid_generation=None if valid is None else valid.generation,
    )


def mesh_plot(
    geometry: MeshGeometry,
    field: MeshScalarField | None = None,
    *,
    id: str = "mesh_plot",
    revision: int = 0,
    view: str = "planar",
    mode: str = "mesh",
    color_scale: str = "viridis",
    color_range: str | tuple[float, float] | dict[str, Any] = "auto",
    missing_value_policy: str = "reject",
    wireframe: bool = True,
    title: str | None = None,
    width: float | None = None,
    height: float | None = None,
    selection: dict[str, Any] | None = None,
    camera: dict[str, Any] | None = None,
    viewport: dict[str, Any] | None = None,
    contour_levels: dict[str, Any] | None = None,
    equal_aspect: bool = True,
    axes: dict[str, Any] | None = None,
    interactions: tuple[str, ...] = ("pan", "zoom", "inspect", "select", "reset", "fit"),
    revolve: MeshRevolve | None = None,
) -> MeshPlotSpec:
    """Create a strict gpui-px mesh plot over revisioned mesh resources."""
    if not isinstance(geometry, MeshGeometry):
        raise TypeError("mesh_plot requires MeshGeometry")
    if field is not None and not isinstance(field, MeshScalarField):
        raise TypeError("mesh_plot field must be MeshScalarField or None")
    return MeshPlotSpec(
        geometry=geometry,
        id=id,
        revision=revision,
        field=field,
        view=view,
        mode=mode,
        color_scale=color_scale,
        color_range=color_range,
        missing_value_policy=missing_value_policy,
        wireframe=wireframe,
        title=title,
        width=width,
        height=height,
        selection=selection,
        camera=camera,
        viewport=viewport,
        contour_levels=contour_levels,
        equal_aspect=equal_aspect,
        axes=axes,
        interactions=interactions,
        revolve=revolve,
    )


def request_mesh_svg_export(
    plot: MeshPlotSpec,
    context: "SessionContext",
    request_id: str,
    *,
    options: StaticSvgOptions | None = None,
    width: float = 800.0,
    height: float = 600.0,
) -> None:
    """Request native static SVG export of a resource-backed mesh plot."""
    if not isinstance(plot, MeshPlotSpec):
        raise TypeError("mesh SVG export requires MeshPlotSpec")
    if not isinstance(request_id, str) or not request_id.strip():
        raise ValueError("mesh SVG export request id must be non-empty")
    if options is not None and not isinstance(options, StaticSvgOptions):
        raise TypeError("mesh SVG export options must be StaticSvgOptions or None")
    if options is not None and (width != 800.0 or height != 600.0):
        raise ValueError("mesh SVG export accepts options or width/height, not both")
    resolved = options or StaticSvgOptions.new(width, height)
    context.command(
        request_id,
        "px.export_mesh_svg",
        plot=plot.to_spec(),
        options=resolved.to_spec(),
    )
