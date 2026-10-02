"""Typed semantic event values for Python-authored GPUI applications."""
from __future__ import annotations
from dataclasses import dataclass
from typing import Any

from .scene2d import GridCell

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


@dataclass(frozen=True)
class Scene2DPointerInput:
    """One captured mouse, touch, or pen sample in logical Scene2D space."""

    phase: str
    device: str
    contact_id: int
    timestamp_ns: int
    x: float
    y: float
    buttons: tuple[str, ...] = ()
    modifiers: tuple[str, ...] = ()
    hit_id: str | None = None
    cell: GridCell | None = None

    @classmethod
    def from_wire(cls, value: dict[str, Any]) -> "Scene2DPointerInput":
        phase = str(value.get("phase", ""))
        device = str(value.get("device", ""))
        if phase not in {"down", "move", "up", "cancel"}:
            raise ValueError(f"unsupported Scene2D pointer phase {phase!r}")
        if device not in {"mouse", "touch", "pen"}:
            raise ValueError(f"unsupported Scene2D pointer device {device!r}")
        position = value.get("position") or {}
        cell_value = value.get("cell")
        cell = None
        if isinstance(cell_value, dict):
            row, column, index = (int(cell_value[name]) for name in ("row", "column", "index"))
            cell = GridCell(row, column, index, str(cell_value.get("id", f"r{row}c{column}")))
        return cls(
            phase=phase, device=device, contact_id=int(value.get("contact_id", 0)),
            timestamp_ns=int(value.get("timestamp_ns", 0)),
            x=float(position.get("x", 0.0)), y=float(position.get("y", 0.0)),
            buttons=tuple(str(item) for item in value.get("buttons", ())),
            modifiers=tuple(str(item) for item in value.get("modifiers", ())),
            hit_id=None if value.get("hit_id") is None else str(value["hit_id"]), cell=cell,
        )


@dataclass(frozen=True)
class Scene2DKeyInput:
    """A focused key transition delivered to a Scene2D surface."""

    phase: str
    key: str
    repeat: bool
    modifiers: tuple[str, ...]
    timestamp_ns: int

    @classmethod
    def from_wire(cls, value: dict[str, Any]) -> "Scene2DKeyInput":
        phase = str(value.get("phase", ""))
        if phase not in {"down", "up"}:
            raise ValueError(f"unsupported Scene2D key phase {phase!r}")
        key = str(value.get("key", ""))
        if not key:
            raise ValueError("Scene2D key event requires a key")
        return cls(phase, key, bool(value.get("repeat", False)),
                   tuple(str(item) for item in value.get("modifiers", ())),
                   int(value.get("timestamp_ns", 0)))


@dataclass(frozen=True)
class Scene2DLifecycleInput:
    """Native focus, section, or session transition that clears held input."""

    reason: str
    timestamp_ns: int


@dataclass(frozen=True)
class Scene2DTransitionCompleteInput:
    """A requested visual transition reached its target values once."""

    id: str
    completion_id: str
    timestamp_ns: int


@dataclass(frozen=True)
class Scene2DActivateInput:
    """Accessibility activation of a semantic Scene2D target."""

    id: str
    timestamp_ns: int
    hit_id: str | None = None
    cell: GridCell | None = None


@dataclass(frozen=True)
class Scene2DEvent(Event):
    """Typed surface event; ``input`` is a pointer or focused key transition."""

    surface_id: str = ""
    input: Scene2DPointerInput | Scene2DKeyInput | Scene2DLifecycleInput | Scene2DTransitionCompleteInput | Scene2DActivateInput | None = None

    @classmethod
    def from_event(cls, event: Event) -> "Scene2DEvent":
        payload = dict(event.payload or {})
        value = payload.get("event")
        if not isinstance(value, dict):
            value = payload
        kind = value.get("type")
        if event.event == "scene2d.cancel_all":
            input_value: Scene2DPointerInput | Scene2DKeyInput | Scene2DLifecycleInput = Scene2DLifecycleInput(
                str(payload.get("reason", "outbound_overflow")), int(value.get("timestamp_ns", 0)))
        elif kind == "pointer":
            input_value = Scene2DPointerInput.from_wire(value)
        elif kind == "key":
            input_value = Scene2DKeyInput.from_wire(value)
        elif kind == "lifecycle":
            input_value = Scene2DLifecycleInput(str(value.get("reason", "unknown")),
                                                 int(value.get("timestamp_ns", 0)))
        elif kind == "transition_complete":
            node_id = str(value.get("id", ""))
            completion_id = str(value.get("completion_id", ""))
            if not node_id or not completion_id:
                raise ValueError("Scene2D transition completion requires node and completion IDs")
            input_value = Scene2DTransitionCompleteInput(
                node_id, completion_id, int(value.get("timestamp_ns", 0)))
        elif kind == "activate":
            node_id = str(value.get("id", ""))
            if not node_id:
                raise ValueError("Scene2D activation requires a node ID")
            cell_value = value.get("cell")
            cell = None
            if isinstance(cell_value, dict):
                row, column, index = (int(cell_value[name]) for name in ("row", "column", "index"))
                cell = GridCell(row, column, index, str(cell_value.get("id", f"r{row}c{column}")))
            input_value = Scene2DActivateInput(
                node_id, int(value.get("timestamp_ns", 0)),
                None if value.get("hit_id") is None else str(value["hit_id"]), cell)
        else:
            raise ValueError(f"unsupported Scene2D input type {kind!r}")
        return cls(**event.__dict__, surface_id=event.node_id, input=input_value)


@dataclass(frozen=True)
class Scene2DTick:
    """Elapsed monotonic time delivered to a foreground application reducer."""

    elapsed_ns: int
    frame: int
    surface_id: str = ""

    @classmethod
    def from_event(cls, event: Event) -> "Scene2DTick":
        payload = dict(event.payload or {})
        value = payload.get("tick", payload)
        if not isinstance(value, dict):
            raise ValueError("Scene2D tick is missing its typed payload")
        elapsed_ns = int(value.get("elapsed_ns", 0))
        frame = int(value.get("frame", 0))
        if elapsed_ns < 0 or frame < 0:
            raise ValueError("Scene2D tick time and frame must be non-negative")
        return cls(elapsed_ns, frame, str(value.get("surface_id", event.node_id)))

    @property
    def elapsed_seconds(self) -> float:
        return self.elapsed_ns / 1_000_000_000.0

def specialize(message: dict[str, Any]) -> Event:
    base = Event.from_message(message)
    event_type = {
        "click": Click,
        "select": Selection,
        "selection_change": Selection,
        "viewport_change": Viewport,
        "change": ValueChange,
        "commit": ValueChange,
        "scene2d.event": Scene2DEvent,
        "scene2d.cancel_all": Scene2DEvent,
    }.get(base.event, Event)
    if event_type is Scene2DEvent:
        return Scene2DEvent.from_event(base)
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
