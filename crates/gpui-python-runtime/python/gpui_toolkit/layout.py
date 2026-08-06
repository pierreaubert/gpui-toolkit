"""Declarative ``gpui-builder`` trees; native host performs solving."""
from __future__ import annotations
from dataclasses import dataclass, field
from enum import Enum
import math
from typing import Any

class Axis(str, Enum):
    HORIZONTAL = "horizontal"
    VERTICAL = "vertical"
    def cross(self) -> "Axis": return Axis.VERTICAL if self is Axis.HORIZONTAL else Axis.HORIZONTAL

@dataclass(frozen=True)
class Sizing:
    kind: str
    initial: float | None = None
    min: float = 0.0
    max: float | None = None
    weight: float | None = None
    text: str | None = None
    line_height: float | None = None
    def __post_init__(self) -> None:
        if self.kind not in {"fixed", "fractional", "flex", "text"} or self.min < 0:
            raise ValueError("invalid builder sizing")
        if self.kind == "fixed" and (self.initial is None or self.initial < 0): raise ValueError("fixed sizing requires a non-negative size")
        if self.kind == "fractional" and (self.initial is None or self.max is None or self.max < self.min): raise ValueError("fractional sizing requires initial, min, and max")
        if self.kind == "flex" and (self.weight is None or self.weight <= 0): raise ValueError("flex sizing requires positive weight")
        if self.kind == "text" and (self.text is None or self.line_height is None or self.line_height <= 0): raise ValueError("text sizing requires text and positive line_height")
    @classmethod
    def fixed(cls, size: float) -> "Sizing": return cls("fixed", initial=size, min=size)
    @classmethod
    def fractional(cls, initial: float, min: float, max: float = float("inf")) -> "Sizing": return cls("fractional", initial, min, max)
    @classmethod
    def flex(cls, min: float = 0, weight: float = 1) -> "Sizing": return cls("flex", min=min, weight=weight)
    @classmethod
    def text_measured(cls, text: str, line_height: float, min: float = 0) -> "Sizing": return cls("text", min=min, text=text, line_height=line_height)
    def to_spec(self) -> dict[str, Any]:
        result = self.__dict__.copy()
        if result["max"] == float("inf"): result["max"] = None
        return result

@dataclass(frozen=True)
class DisplayTier:
    name: str
    min_size: float
    def __post_init__(self) -> None:
        if not self.name or not math.isfinite(self.min_size) or self.min_size < 0: raise ValueError("invalid display tier")

@dataclass(frozen=True)
class Slot:
    id: str
    sizing: Sizing
    priority: float = 1.0
    collapsible: bool = False
    display_tiers: tuple[DisplayTier, ...] = ()
    collapse_label: str | None = None
    def __post_init__(self) -> None:
        if not self.id or not math.isfinite(self.priority): raise ValueError("invalid slot")
        if self.collapsible and not self.collapse_label: raise ValueError("collapsible slots require a collapse label")

@dataclass(frozen=True)
class Container:
    id: str
    axis: Axis
    sizing: Sizing
    children: tuple["LayoutNode", ...] = ()
    auto_axis: float | None = None
    divider_size: float = 0.0
    def __post_init__(self) -> None:
        if not self.id or self.divider_size < 0 or (self.auto_axis is not None and self.auto_axis <= 0): raise ValueError("invalid layout container")

LayoutNode = Slot | Container

def to_spec(node: LayoutNode) -> dict[str, Any]:
    if isinstance(node, Slot):
        return {"kind": "slot", "id": node.id, "sizing": node.sizing.to_spec(), "priority": node.priority, "collapsible": node.collapsible, "display_tiers": [tier.__dict__.copy() for tier in node.display_tiers], "collapse_label": node.collapse_label}
    return {"kind": "container", "id": node.id, "axis": node.axis.value, "sizing": node.sizing.to_spec(), "children": [to_spec(child) for child in node.children], "auto_axis": node.auto_axis, "divider_size": node.divider_size}
