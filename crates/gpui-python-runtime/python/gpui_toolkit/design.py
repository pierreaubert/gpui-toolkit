"""Typed declarations for the host-owned :mod:`gpui-design` surface.

These values describe design-token input and reports returned by the native
host.  They deliberately do not reproduce Rust's layout or conformance
algorithms in Python.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Any, Mapping


class DesignLanguage(str, Enum):
    APPLE_HIG = "apple_hig"
    MATERIAL3 = "material3"
    FLUENT = "fluent"
    ADWAITA = "adwaita"
    BREEZE = "breeze"
    CARBON = "carbon"
    NEUTRAL = "neutral"

    @classmethod
    def parse(cls, value: str) -> "DesignLanguage":
        aliases = {"gtk": cls.ADWAITA, "gnome": cls.ADWAITA, "kde": cls.BREEZE}
        alias = aliases.get(value)
        if alias is not None:
            return alias
        try:
            return cls(value)
        except ValueError as error:
            raise ValueError(f"unknown design language: {value}") from error

    @property
    def label(self) -> str:
        return {
            self.APPLE_HIG: "Apple HIG", self.MATERIAL3: "Material 3",
            self.FLUENT: "Fluent", self.ADWAITA: "Adwaita", self.BREEZE: "Breeze",
            self.CARBON: "Carbon", self.NEUTRAL: "Neutral",
        }[self]


class CornerRadiusStyle(str, Enum):
    CONTINUOUS = "continuous"
    CIRCULAR = "circular"


@dataclass(frozen=True)
class DesignToken:
    """One Style Dictionary-compatible token declaration."""

    name: str
    path: tuple[str, ...]
    value: str
    token_type: str

    def __post_init__(self) -> None:
        if not self.path or any(not part for part in self.path):
            raise ValueError("design token paths require non-empty components")
        if self.name != ".".join(self.path):
            raise ValueError("design token name must equal its dotted path")
        if not self.token_type:
            raise ValueError("design token type is required")

    def to_spec(self) -> dict[str, Any]:
        return {"name": self.name, "path": list(self.path), "value": self.value, "token_type": self.token_type}

    @classmethod
    def from_spec(cls, value: Mapping[str, Any]) -> "DesignToken":
        return cls(str(value["name"]), tuple(map(str, value["path"])), str(value["value"]), str(value["token_type"]))


@dataclass(frozen=True)
class MotionSpec:
    duration_ms: int
    fast_ms: int
    slow_ms: int
    prefer_spring: bool
    reduced_motion: bool

    def __post_init__(self) -> None:
        if min(self.duration_ms, self.fast_ms, self.slow_ms) < 0:
            raise ValueError("motion durations cannot be negative")


@dataclass(frozen=True)
class ConformanceFinding:
    id: str
    message: str


@dataclass(frozen=True)
class DesignConformanceReport:
    findings: tuple[ConformanceFinding, ...] = ()

    @property
    def passed(self) -> bool:
        return not self.findings


@dataclass(frozen=True)
class DesignPresetDocumentation:
    preset_id: str
    label: str
    language: DesignLanguage
    token_count: int
    grid_unit: float
    min_touch_target: float
    base_size: float
    corner_style: CornerRadiusStyle
    motion_duration_ms: int
    reduced_motion_duration_ms: int

    def __post_init__(self) -> None:
        if not self.preset_id or self.token_count < 0:
            raise ValueError("preset_id and a non-negative token_count are required")
