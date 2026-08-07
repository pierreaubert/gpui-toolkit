"""Typed requests for native gpui-d3rs algorithms."""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from math import isfinite
from typing import TYPE_CHECKING, Any, Sequence

from .commands import CommandResult, CommandStatus

if TYPE_CHECKING:
    from .app import SessionContext


Domain = tuple[float, float]


def _domain(value: Sequence[float], name: str) -> Domain:
    if len(value) != 2:
        raise ValueError(f"{name} must contain exactly two values")
    minimum, maximum = float(value[0]), float(value[1])
    if not isfinite(minimum) or not isfinite(maximum) or minimum >= maximum:
        raise ValueError(f"{name} must be finite and increasing")
    return minimum, maximum


class ZoomOperationKind(str, Enum):
    ZOOM_TO = "zoom_to"
    RESET = "reset"
    BACK = "back"


@dataclass(frozen=True)
class ZoomOperation:
    kind: ZoomOperationKind
    x: Domain | None = None
    y: Domain | None = None

    @classmethod
    def zoom_to(cls, x: Sequence[float], y: Sequence[float]) -> "ZoomOperation":
        return cls(ZoomOperationKind.ZOOM_TO, _domain(x, "x"), _domain(y, "y"))

    @classmethod
    def reset(cls) -> "ZoomOperation":
        return cls(ZoomOperationKind.RESET)

    @classmethod
    def back(cls) -> "ZoomOperation":
        return cls(ZoomOperationKind.BACK)

    def to_spec(self) -> dict[str, Any]:
        if self.kind is ZoomOperationKind.ZOOM_TO:
            if self.x is None or self.y is None:
                raise ValueError("zoom_to requires x and y domains")
            return {"kind": self.kind.value, "x": list(self.x), "y": list(self.y)}
        if self.x is not None or self.y is not None:
            raise ValueError(f"{self.kind.value} does not accept domains")
        return {"kind": self.kind.value}


@dataclass(frozen=True)
class ZoomRequest:
    original_x: Domain
    original_y: Domain
    operations: Sequence[ZoomOperation] = ()
    log_x: bool = False
    log_y: bool = False

    def __post_init__(self) -> None:
        object.__setattr__(self, "original_x", _domain(self.original_x, "original_x"))
        object.__setattr__(self, "original_y", _domain(self.original_y, "original_y"))

    def to_spec(self) -> dict[str, Any]:
        return {
            "original_x": list(self.original_x), "original_y": list(self.original_y),
            "log_x": self.log_x, "log_y": self.log_y,
            "operations": [operation.to_spec() for operation in self.operations],
        }

    def send(self, context: "SessionContext", request_id: str) -> None:
        """Run this request with Rust's ``ZoomState`` through the host."""
        context.command(request_id, "d3.zoom", **self.to_spec())


@dataclass(frozen=True)
class ZoomResult:
    x: Domain
    y: Domain
    zoomed: bool
    level: int
    back_results: tuple[bool, ...]

    @classmethod
    def from_command(cls, result: CommandResult) -> "ZoomResult":
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or f"d3 zoom command {result.status.value}")
        data = result.data
        return cls(
            _domain(data.get("x", ()), "x"), _domain(data.get("y", ()), "y"),
            bool(data.get("zoomed")), int(data.get("level", 0)),
            tuple(bool(value) for value in data.get("back_results", ())),
        )


class ArrayOperation(str, Enum):
    BISECT_LEFT = "bisect_left"
    BISECT_RIGHT = "bisect_right"
    QUANTILE = "quantile"


@dataclass(frozen=True)
class ArrayRequest:
    """Run one native d3-array search or quantile operation."""

    operation: ArrayOperation
    data: Sequence[float]
    value: float | None = None
    percentile: float | None = None

    def to_spec(self) -> dict[str, Any]:
        data = [float(item) for item in self.data]
        if any(not isfinite(item) for item in data):
            raise ValueError("d3 array data must be finite")
        result: dict[str, Any] = {"operation": self.operation.value, "data": data}
        if self.operation in {ArrayOperation.BISECT_LEFT, ArrayOperation.BISECT_RIGHT}:
            if self.value is None or not isfinite(float(self.value)):
                raise ValueError("bisect requires a finite value")
            result["value"] = float(self.value)
        elif self.percentile is None or not 0 <= float(self.percentile) <= 1:
            raise ValueError("quantile requires a percentile in [0, 1]")
        else:
            result["percentile"] = float(self.percentile)
        return result

    def send(self, context: "SessionContext", request_id: str) -> None:
        context.command(request_id, "d3.array", **self.to_spec())

    @staticmethod
    def value_from_command(result: CommandResult) -> float | int | None:
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or f"d3 array command {result.status.value}")
        value = result.data.get("value")
        if value is None or isinstance(value, (int, float)):
            return value
        raise ValueError("native d3 array result has an unexpected shape")


class StatisticsOperation(str, Enum):
    SUM = "sum"
    MEAN = "mean"
    MEDIAN = "median"
    VARIANCE = "variance"
    DEVIATION = "deviation"
    QUANTILE = "quantile"
    EXTENT = "extent"
    CUMSUM = "cumsum"


@dataclass(frozen=True)
class StatisticsRequest:
    operation: StatisticsOperation
    data: Sequence[float]
    percentile: float | None = None
    def send(self, context: "SessionContext", request_id: str) -> None:
        values = [float(value) for value in self.data]
        if any(not isfinite(value) for value in values): raise ValueError("statistics data must be finite")
        arguments: dict[str, Any] = {"operation": self.operation.value, "data": values}
        if self.operation is StatisticsOperation.QUANTILE:
            if self.percentile is None or not 0 <= self.percentile <= 1: raise ValueError("quantile requires percentile in [0, 1]")
            arguments["percentile"] = self.percentile
        context.command(request_id, "d3.statistics", **arguments)
    @staticmethod
    def value_from_command(result: CommandResult) -> float | list[float] | tuple[float, float] | None:
        if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "d3 statistics failed")
        value = result.data.get("value")
        if isinstance(value, list) and len(value) == 2: return (float(value[0]), float(value[1]))
        if isinstance(value, list): return [float(item) for item in value]
        if value is None or isinstance(value, (int, float)): return None if value is None else float(value)
        raise ValueError("native d3 statistics result has an unexpected shape")


class TickOperation(str, Enum):
    TICKS = "ticks"
    STEP = "tick_step"
    INCREMENT = "tick_increment"
    NICE = "nice"
    TIME = "time_ticks"
    INTERVAL = "interval"
    LOG = "log"


@dataclass(frozen=True)
class TickRequest:
    operation: TickOperation
    start: float
    stop: float
    count: int = 10
    interval: float | None = None
    base: float = 10.0
    subdivisions: bool = True
    def send(self, context: "SessionContext", request_id: str) -> None:
        if not isfinite(self.start) or not isfinite(self.stop) or self.count < 0: raise ValueError("invalid tick range")
        context.command(request_id, "d3.ticks", operation=self.operation.value, start=self.start, stop=self.stop, count=self.count, interval=self.interval, base=self.base, subdivisions=self.subdivisions)
    @staticmethod
    def value_from_command(result: CommandResult) -> float | tuple[float, float] | list[float]:
        if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "d3 ticks failed")
        value = result.data["value"]
        if isinstance(value, list) and len(value) == 2 and all(isinstance(item, (int, float)) for item in value): return (float(value[0]), float(value[1]))
        if isinstance(value, list): return [float(item) for item in value]
        return float(value)


class ScaleKind(str, Enum):
    LINEAR = "linear"
    LOG = "log"
    POWER = "power"
    SQRT = "sqrt"
    SYMLOG = "symlog"
    QUANTIZE = "quantize"
    QUANTILE = "quantile"
    THRESHOLD = "threshold"
    ORDINAL = "ordinal"
    BAND = "band"
    POINT = "point"


@dataclass(frozen=True)
class ScaleRequest:
    kind: ScaleKind
    domain: Sequence[float] | Sequence[str]
    range: Sequence[float] | Sequence[str]
    values: Sequence[float] | Sequence[str]
    clamp: bool = False
    base: float = 10.0
    exponent: float = 1.0
    constant: float = 1.0
    tick_count: int = 10
    padding_inner: float = 0.0
    padding_outer: float = 0.0
    align: float = 0.5
    round: bool = False
    def send(self, context: "SessionContext", request_id: str) -> None:
        categorical = self.kind in {ScaleKind.ORDINAL, ScaleKind.BAND, ScaleKind.POINT}
        discrete_range = self.kind in {ScaleKind.QUANTIZE, ScaleKind.QUANTILE, ScaleKind.THRESHOLD, ScaleKind.ORDINAL}
        domain = [str(value) for value in self.domain] if categorical else [float(value) for value in self.domain]
        values = [str(value) for value in self.values] if categorical else [float(value) for value in self.values]
        scale_range = [str(value) for value in self.range] if discrete_range else [float(value) for value in self.range]
        context.command(request_id, "d3.scale", kind=self.kind.value, domain=domain, range=scale_range, values=values, clamp=self.clamp, base=self.base, exponent=self.exponent, constant=self.constant, tick_count=self.tick_count, padding_inner=self.padding_inner, padding_outer=self.padding_outer, align=self.align, round=self.round)


@dataclass(frozen=True)
class ScaleOutput:
    values: tuple[float | str | None, ...]
    ticks: tuple[float, ...] = ()
    thresholds: tuple[float, ...] = ()
    bandwidth: float | None = None
    step: float | None = None
    @classmethod
    def from_command(cls, result: CommandResult) -> "ScaleOutput":
        if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "d3 scale failed")
        output = result.data["output"]
        return cls(tuple(None if value is None else value for value in output["values"]), tuple(float(value) for value in output.get("ticks", ())), tuple(float(value) for value in output.get("thresholds", ())), None if output.get("bandwidth") is None else float(output["bandwidth"]), None if output.get("step") is None else float(output["step"]))


@dataclass(frozen=True)
class D3ParityEntry:
    id: str
    d3_area: str
    gpui_d3rs_modules: str
    status: str
    evidence: str
    release_requirement: str

@dataclass(frozen=True)
class D3BenchmarkCase:
    id: str
    module: str
    bench_target: str
    benchmark_group: str
    benchmark_id: str
    dataset_scale: str
    evidence: str

@dataclass(frozen=True)
class D3Reports:
    parity_entries: tuple[D3ParityEntry, ...]
    parity_markdown: str
    benchmark_cases: tuple[D3BenchmarkCase, ...]
    benchmark_markdown: str


class D3BridgeKind(str, Enum):
    DIRECT_COMMAND = "direct_command"
    CHART_SPEC = "chart_spec"
    SCENE_SPEC = "scene_spec"
    HOST_INTERACTION = "host_interaction"


class AlgorithmOperation(str, Enum):
    COLOR_INTERPOLATE = "color_interpolate"
    COLOR_CONVERT = "color_convert"
    FORMAT = "format"
    FORMAT_PREFIX = "format_prefix"
    TIME_INTERVAL = "time_interval"
    TIME_SCALE = "time_scale"
    CSV_PARSE = "csv_parse"
    DSV_PARSE = "dsv_parse"
    DSV_FORMAT = "dsv_format"
    INTERPOLATE_NUMBER = "interpolate_number"
    INTERPOLATE_ARRAY = "interpolate_array"
    INTERPOLATE_STRING = "interpolate_string"
    INTERPOLATE_TRANSFORM_CSS = "interpolate_transform_css"
    INTERPOLATE_TRANSFORM_SVG = "interpolate_transform_svg"
    INTERPOLATE_ZOOM = "interpolate_zoom"
    EASE = "ease"
    SELECTION_JOIN = "selection_join"
    BRUSH_GESTURE = "brush_gesture"
    DRAG_GESTURE = "drag_gesture"
    TRANSITION_SAMPLE = "transition_sample"
    RANDOM_UNIFORM = "random_uniform"
    RANDOM = "random"
    SHUFFLE = "shuffle"


class EaseKind(str, Enum):
    LINEAR = "linear"
    QUAD_IN = "quad_in"
    QUAD_OUT = "quad_out"
    QUAD_IN_OUT = "quad_in_out"
    CUBIC_IN = "cubic_in"
    CUBIC_OUT = "cubic_out"
    CUBIC_IN_OUT = "cubic_in_out"
    SIN_IN = "sin_in"
    SIN_OUT = "sin_out"
    SIN_IN_OUT = "sin_in_out"
    EXP_IN = "exp_in"
    EXP_OUT = "exp_out"
    EXP_IN_OUT = "exp_in_out"
    CIRCLE_IN = "circle_in"
    CIRCLE_OUT = "circle_out"
    CIRCLE_IN_OUT = "circle_in_out"
    ELASTIC_IN = "elastic_in"
    ELASTIC_OUT = "elastic_out"
    ELASTIC_IN_OUT = "elastic_in_out"
    BACK_IN = "back_in"
    BACK_OUT = "back_out"
    BACK_IN_OUT = "back_in_out"
    BOUNCE_IN = "bounce_in"
    BOUNCE_OUT = "bounce_out"
    BOUNCE_IN_OUT = "bounce_in_out"


class RandomKind(str, Enum):
    UNIFORM = "uniform"
    NORMAL = "normal"
    LOG_NORMAL = "log_normal"
    EXPONENTIAL = "exponential"
    BERNOULLI = "bernoulli"
    POISSON = "poisson"
    IRWIN_HALL = "irwin_hall"
    BATES = "bates"


@dataclass(frozen=True)
class AlgorithmRequest:
    operation: AlgorithmOperation
    arguments: dict[str, Any]

    @classmethod
    def easing(cls, kind: EaseKind, values: Sequence[float]) -> "AlgorithmRequest":
        return cls(AlgorithmOperation.EASE, {"kind": kind.value, "values": list(values)})

    @classmethod
    def random(
        cls,
        kind: RandomKind,
        *,
        count: int,
        seed: int,
        **parameters: float | int,
    ) -> "AlgorithmRequest":
        if count < 0:
            raise ValueError("random sample count must be non-negative")
        return cls(
            AlgorithmOperation.RANDOM,
            {"kind": kind.value, "count": count, "seed": seed, **parameters},
        )

    @classmethod
    def dsv_parse(cls, input: str, delimiter: str = ",") -> "AlgorithmRequest":
        if len(delimiter) != 1:
            raise ValueError("DSV delimiter must be one character")
        return cls(AlgorithmOperation.DSV_PARSE, {"input": input, "delimiter": delimiter})

    @classmethod
    def dsv_format(
        cls,
        rows: Sequence[dict[str, Any]],
        columns: Sequence[str],
        delimiter: str = ",",
    ) -> "AlgorithmRequest":
        if delimiter not in {",", "\t"}:
            raise ValueError("DSV formatting supports comma or tab delimiters")
        return cls(
            AlgorithmOperation.DSV_FORMAT,
            {"rows": list(rows), "columns": list(columns), "delimiter": delimiter},
        )

    @classmethod
    def selection_join(
        cls, old_keys: Sequence[str], new_keys: Sequence[str]
    ) -> "AlgorithmRequest":
        return cls(
            AlgorithmOperation.SELECTION_JOIN,
            {"old_keys": list(old_keys), "new_keys": list(new_keys)},
        )

    @classmethod
    def brush_gesture(
        cls, points: Sequence[tuple[float, float]], *, min_size: float = 0.0
    ) -> "AlgorithmRequest":
        if len(points) < 2:
            raise ValueError("brush gesture requires at least two points")
        return cls(
            AlgorithmOperation.BRUSH_GESTURE,
            {"points": [list(point) for point in points], "min_size": min_size},
        )

    @classmethod
    def drag_gesture(
        cls,
        points: Sequence[tuple[float, float]],
        *,
        pointer_id: int = 1,
        click_distance: float = 0.0,
    ) -> "AlgorithmRequest":
        if len(points) < 2:
            raise ValueError("drag gesture requires at least two points")
        return cls(
            AlgorithmOperation.DRAG_GESTURE,
            {
                "points": [list(point) for point in points],
                "pointer_id": pointer_id,
                "click_distance": click_distance,
            },
        )

    @classmethod
    def transition_sample(
        cls,
        start: float,
        end: float,
        duration_ms: float,
        delta_ms: Sequence[float],
        *,
        delay_ms: float = 0.0,
        easing: EaseKind = EaseKind.LINEAR,
    ) -> "AlgorithmRequest":
        return cls(
            AlgorithmOperation.TRANSITION_SAMPLE,
            {
                "start": start,
                "end": end,
                "duration_ms": duration_ms,
                "delay_ms": delay_ms,
                "delta_ms": list(delta_ms),
                "kind": easing.value,
            },
        )

    def send(self, context: "SessionContext", request_id: str) -> None:
        context.command(
            request_id,
            "d3.algorithms",
            operation=self.operation.value,
            **self.arguments,
        )


@dataclass(frozen=True)
class AlgorithmResult:
    operation: AlgorithmOperation
    value: Any

    @classmethod
    def from_command(cls, result: CommandResult) -> "AlgorithmResult":
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or f"D3 algorithm {result.status.value}")
        return cls(AlgorithmOperation(str(result.data["operation"])), result.data.get("value"))


@dataclass(frozen=True)
class D3ModuleBridge:
    module: str
    bridge: D3BridgeKind
    python_path: str
    evidence: str


@dataclass(frozen=True)
class D3ModuleCatalog:
    modules: tuple[D3ModuleBridge, ...]

    def by_name(self, module: str) -> D3ModuleBridge | None:
        return next((entry for entry in self.modules if entry.module == module), None)


def request_module_catalog(context: "SessionContext", request_id: str) -> None:
    context.command(request_id, "d3.modules")


def module_catalog_from_command(result: CommandResult) -> D3ModuleCatalog:
    if result.status is not CommandStatus.SUCCEEDED:
        raise RuntimeError(result.error or f"D3 module catalog {result.status.value}")
    return D3ModuleCatalog(
        tuple(
            D3ModuleBridge(
                module=str(entry["module"]),
                bridge=D3BridgeKind(str(entry["bridge"])),
                python_path=str(entry["python_path"]),
                evidence=str(entry["evidence"]),
            )
            for entry in result.data.get("modules", ())
        )
    )

def request_reports(context: "SessionContext", request_id: str) -> None:
    context.command(request_id, "d3.reports")

def reports_from_command(result: CommandResult) -> D3Reports:
    if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "d3 reports failed")
    parity, benchmark = result.data["parity"], result.data["benchmark"]
    return D3Reports(
        tuple(D3ParityEntry(str(value["id"]), str(value["d3_area"]), str(value["gpui_d3rs_modules"]), str(value["status"]), str(value["evidence"]), str(value["release_requirement"])) for value in parity["entries"]),
        str(parity["markdown"]),
        tuple(D3BenchmarkCase(str(value["id"]), str(value["module"]), str(value["bench_target"]), str(value["benchmark_group"]), str(value["benchmark_id"]), str(value["dataset_scale"]), str(value["evidence"])) for value in benchmark["cases"]),
        str(benchmark["markdown"]),
    )
