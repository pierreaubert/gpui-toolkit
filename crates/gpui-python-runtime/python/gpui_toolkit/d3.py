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
