"""Typed semantic event values for Python-authored GPUI applications."""
from __future__ import annotations
from dataclasses import dataclass
from typing import Any

@dataclass(frozen=True)
class Event:
    id: str
    sequence: int
    node_id: str
    event: str
    action: str | None = None
    payload: dict[str, Any] | None = None

    @property
    def kind(self) -> str:
        """Canonical event discriminator; ``event`` remains wire-compatible."""
        return self.event

    @classmethod
    def from_message(cls, message: dict[str, Any]) -> "Event":
        return cls(str(message["id"]), int(message.get("sequence", 0)), str(message["node_id"]), str(message.get("event", "")), message.get("action"), dict(message.get("payload") or {}))

@dataclass(frozen=True)
class Click(Event):
    @property
    def modifiers(self) -> tuple[str, ...]:
        return tuple((self.payload or {}).get("modifiers", ()))

@dataclass(frozen=True)
class Selection(Event):
    @property
    def selected_keys(self) -> tuple[str, ...]:
        return tuple(str(value) for value in (self.payload or {}).get("keys", ()))

    @property
    def selected_id(self) -> str | None:
        return (self.payload or {}).get("row_id") or (self.payload or {}).get("object_id")

    @property
    def keys(self) -> tuple[str, ...]:
        """Stable selected row/object keys (alias for ``selected_keys``)."""
        return self.selected_keys

    @property
    def key(self) -> str | None:
        """The first selected key, useful for single-selection charts."""
        keys = self.selected_keys
        if keys:
            return keys[0]
        value = (self.payload or {}).get("key")
        return None if value is None else str(value)

    @property
    def x(self) -> float | None:
        return _finite_float((self.payload or {}).get("x"))

    @property
    def y(self) -> float | None:
        return _finite_float((self.payload or {}).get("y"))

    @property
    def series(self) -> str | None:
        value = (self.payload or {}).get("series")
        return None if value is None else str(value)

    @property
    def series_index(self) -> int | None:
        value = (self.payload or {}).get("series_index")
        if isinstance(value, bool) or value is None:
            return None
        try:
            return int(value)
        except (TypeError, ValueError):
            return None

    @property
    def point_index(self) -> int | None:
        value = (self.payload or {}).get("point_index")
        if isinstance(value, bool) or value is None:
            return None
        try:
            return int(value)
        except (TypeError, ValueError):
            return None

    @property
    def value(self) -> float | None:
        """Numeric value carried by categorical or treemap selections."""
        return _finite_float((self.payload or {}).get("value"))

    @property
    def plot_id(self) -> str | None:
        return (self.payload or {}).get("plot_id")

    @property
    def mesh_id(self) -> str | None:
        return (self.payload or {}).get("mesh_id")

    @property
    def cell_index(self) -> int | None:
        return (self.payload or {}).get("cell_index")

    @property
    def cell_id(self) -> int | None:
        return (self.payload or {}).get("cell_id")

    @property
    def vertex_id(self) -> int | None:
        return (self.payload or {}).get("vertex_id")

    @property
    def world_position(self) -> tuple[float, float, float] | None:
        position = (self.payload or {}).get("world_position")
        return None if position is None else tuple(float(value) for value in position)

    @property
    def displayed_value(self) -> float | None:
        value = (self.payload or {}).get("displayed_value")
        return None if value is None else float(value)

    @property
    def field_id(self) -> str | None:
        return (self.payload or {}).get("field_id")


@dataclass(frozen=True)
class Viewport(Event):
    @property
    def x_range(self) -> tuple[float, float] | None:
        value = (self.payload or {}).get("x")
        return None if value is None else (float(value[0]), float(value[1]))

    @property
    def y_range(self) -> tuple[float, float] | None:
        value = (self.payload or {}).get("y")
        return None if value is None else (float(value[0]), float(value[1]))

    @property
    def zoom_level(self) -> int | None:
        value = (self.payload or {}).get("zoom_level")
        if isinstance(value, bool) or value is None:
            return None
        try:
            return int(value)
        except (TypeError, ValueError):
            return None

    @property
    def is_zoomed(self) -> bool | None:
        value = (self.payload or {}).get("is_zoomed")
        return value if isinstance(value, bool) else None

    @property
    def camera(self) -> dict[str, Any] | None:
        value = (self.payload or {}).get("camera")
        return None if value is None else dict(value)

    @property
    def camera_distance(self) -> float | None:
        value = (self.camera or {}).get("distance")
        return None if value is None else float(value)

    @property
    def camera_angles(self) -> tuple[float, float] | None:
        """Return ``(azimuth, elevation)`` in radians when present."""
        camera = self.camera or {}
        if "azimuth" not in camera or "elevation" not in camera:
            return None
        return (float(camera["azimuth"]), float(camera["elevation"]))

    @property
    def camera_target(self) -> tuple[float, float, float] | None:
        value = (self.camera or {}).get("target")
        if value is None or len(value) != 3:
            return None
        return (float(value[0]), float(value[1]), float(value[2]))

@dataclass(frozen=True)
class ValueChange(Event):
    @property
    def value(self) -> Any:
        return (self.payload or {}).get("value")

def specialize(message: dict[str, Any]) -> Event:
    base = Event.from_message(message)
    event_type = {
        "click": Click,
        "select": Selection,
        "selection_change": Selection,
        "viewport_change": Viewport,
        "change": ValueChange,
        "commit": ValueChange,
    }.get(base.event, Event)
    return event_type(**base.__dict__)


def _finite_float(value: Any) -> float | None:
    if isinstance(value, bool) or value is None:
        return None
    try:
        result = float(value)
    except (TypeError, ValueError):
        return None
    return result if result == result and abs(result) != float("inf") else None


# Descriptive aliases make callback annotations read naturally while retaining
# the compact wire event names and backwards-compatible classes.
ChartSelection = Selection
ChartViewport = Viewport
