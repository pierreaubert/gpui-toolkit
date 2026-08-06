"""Typed semantic event values for Python-authored GPUI applications."""
from __future__ import annotations
from dataclasses import dataclass
from typing import Any

@dataclass(frozen=True)
class Event:
    id: str
    sequence: int
    node_id: str
    kind: str
    action: str | None = None
    payload: dict[str, Any] | None = None

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

@dataclass(frozen=True)
class ValueChange(Event):
    @property
    def value(self) -> Any:
        return (self.payload or {}).get("value")

def specialize(message: dict[str, Any]) -> Event:
    base = Event.from_message(message)
    event_type = {"click": Click, "select": Selection, "change": ValueChange, "commit": ValueChange}.get(base.kind, Event)
    return event_type(**base.__dict__)
