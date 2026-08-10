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
    def selected_id(self) -> str | None:
        return (self.payload or {}).get("row_id") or (self.payload or {}).get("object_id")

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
class ValueChange(Event):
    @property
    def value(self) -> Any:
        return (self.payload or {}).get("value")

def specialize(message: dict[str, Any]) -> Event:
    base = Event.from_message(message)
    event_type = {"click": Click, "select": Selection, "change": ValueChange, "commit": ValueChange}.get(base.event, Event)
    return event_type(**base.__dict__)
