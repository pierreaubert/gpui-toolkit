"""Typed ARIA and focus-navigation declarations applied by the native host."""
from __future__ import annotations
from dataclasses import dataclass
from enum import Enum
import math
from typing import Any

class AriaRole(str, Enum):
    BUTTON="button"; CHECKBOX="checkbox"; DIALOG="dialog"; HEADING="heading"; MENU="menu"; MENU_ITEM="menuitem"; PROGRESS_BAR="progressbar"; SLIDER="slider"; TAB="tab"; TAB_LIST="tablist"; TEXTBOX="textbox"
class AriaLive(str, Enum):
    OFF="off"; POLITE="polite"; ASSERTIVE="assertive"
class FocusDirection(str, Enum):
    HORIZONTAL="horizontal"; VERTICAL="vertical"; BOTH="both"

@dataclass(frozen=True)
class AriaProps:
    role: AriaRole
    description: str | None = None
    states: tuple[str, ...] = ()
    live: AriaLive | None = None
    level: int | None = None
    value_now: float | None = None
    value_min: float | None = None
    value_max: float | None = None
    value_text: str | None = None
    def __post_init__(self) -> None:
        if self.level is not None and not 1 <= self.level <= 6: raise ValueError("ARIA level must be 1 through 6")
        values=(self.value_now,self.value_min,self.value_max)
        if any(value is not None and not math.isfinite(value) for value in values): raise ValueError("ARIA values must be finite")
        if self.value_min is not None and self.value_max is not None and self.value_min > self.value_max: raise ValueError("ARIA minimum exceeds maximum")
    def to_spec(self) -> dict[str, Any]:
        return {"role": self.role.value, "description": self.description, "states": list(self.states), "live": self.live.value if self.live else None, "level": self.level, "value_now": self.value_now, "value_min": self.value_min, "value_max": self.value_max, "value_text": self.value_text}

@dataclass(frozen=True)
class FocusGroup:
    id: str
    direction: FocusDirection = FocusDirection.BOTH
    wraparound: bool = True
    focus_ring: bool = True
    gap: float | None = None
    def __post_init__(self) -> None:
        if not self.id.strip() or self.gap is not None and self.gap < 0: raise ValueError("invalid focus group")
    def to_spec(self) -> dict[str, Any]: return {"id":self.id,"direction":self.direction.value,"wraparound":self.wraparound,"focus_ring":self.focus_ring,"gap":self.gap}
