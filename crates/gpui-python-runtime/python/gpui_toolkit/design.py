"""Typed declarations for the host-owned :mod:`gpui-design` surface.

These values describe design-token input and reports returned by the native
host.  They deliberately do not reproduce Rust's layout or conformance
algorithms in Python.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from math import isfinite
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


class DesignPlatform(str, Enum):
    MACOS = "macos"
    IOS = "ios"
    WINDOWS = "windows"
    ANDROID = "android"
    LINUX = "linux"
    OTHER = "other"


class ToggleVariant(str, Enum):
    CAPSULE = "capsule"
    SWITCH = "switch"
    CHECKBOX = "checkbox"


class LabelPosition(str, Enum):
    ABOVE = "above"
    BELOW = "below"
    LEFT = "left"
    RIGHT = "right"


class GroupSeparatorStyle(str, Enum):
    NONE = "none"
    DIVIDER = "divider"
    CARD = "card"


def _finite_nonnegative(*values: float) -> None:
    if any(not isfinite(value) or value < 0.0 for value in values):
        raise ValueError("design dimensions must be finite and non-negative")


@dataclass(frozen=True)
class CornerRadii:
    sm: float
    md: float
    lg: float
    xl: float
    style: CornerRadiusStyle

    def __post_init__(self) -> None:
        _finite_nonnegative(self.sm, self.md, self.lg, self.xl)
        if not isinstance(self.style, CornerRadiusStyle):
            raise ValueError("corner style must be a CornerRadiusStyle")


@dataclass(frozen=True)
class SpacingRules:
    grid_unit: float
    control_padding_x: float
    control_padding_y: float
    control_gap: float
    section_gap: float
    card_padding: float

    def __post_init__(self) -> None:
        _finite_nonnegative(*self.__dict__.values())


@dataclass(frozen=True)
class InteractionRules:
    min_touch_target: float
    border_width: float
    focus_ring_width: float
    focus_ring_offset: float

    def __post_init__(self) -> None:
        _finite_nonnegative(*self.__dict__.values())


@dataclass(frozen=True)
class ElevationRules:
    level_0_blur: float
    level_1_blur: float
    level_2_blur: float
    shadow_opacity: float
    shadow_y_offset: float

    def __post_init__(self) -> None:
        _finite_nonnegative(self.level_0_blur, self.level_1_blur, self.level_2_blur)
        if not isfinite(self.shadow_opacity) or not 0.0 <= self.shadow_opacity <= 1.0:
            raise ValueError("shadow_opacity must be finite and in [0, 1]")
        if not isfinite(self.shadow_y_offset):
            raise ValueError("shadow_y_offset must be finite")


@dataclass(frozen=True)
class TypographyRules:
    font_family: str
    dynamic_sizing: bool
    base_size: float
    small_size: float
    large_size: float

    def __post_init__(self) -> None:
        if not self.font_family.strip() or any(not isfinite(value) or value <= 0.0 for value in (self.base_size, self.small_size, self.large_size)):
            raise ValueError("typography requires a family and positive finite sizes")


@dataclass(frozen=True)
class AnimationRules:
    duration_ms: int
    fast_ms: int
    slow_ms: int
    prefer_spring: bool
    spring_stiffness: float
    spring_damping: float

    def __post_init__(self) -> None:
        if min(self.duration_ms, self.fast_ms, self.slow_ms) <= 0 or not all(isfinite(value) and value > 0.0 for value in (self.spring_stiffness, self.spring_damping)):
            raise ValueError("animation rules require positive durations and spring values")


@dataclass(frozen=True)
class AudioControlRules:
    knob_arc_start_deg: float
    knob_arc_sweep_deg: float
    knob_arc_width: float
    knob_arc_segments: int
    knob_border_width: float
    slider_track_widths: tuple[float, float, float]

    def __post_init__(self) -> None:
        if self.knob_arc_segments <= 0:
            raise ValueError("knob_arc_segments must be positive")
        _finite_nonnegative(self.knob_arc_width, self.knob_border_width, *self.slider_track_widths)


@dataclass(frozen=True)
class LayoutThresholds:
    vertical_threshold: float
    group_stack_threshold: float
    compact_slider_threshold: float
    hide_viz_threshold: float
    compact_knob_threshold: float
    large_knob_threshold: float
    slider_height_normal: float
    slider_height_compact: float

    def __post_init__(self) -> None:
        if any(not isfinite(value) or value <= 0.0 for value in self.__dict__.values()):
            raise ValueError("layout thresholds must be positive and finite")


@dataclass(frozen=True)
class DesignSystemSnapshot:
    language: DesignLanguage
    platform: DesignPlatform
    corners: CornerRadii
    spacing: SpacingRules
    interaction: InteractionRules
    elevation: ElevationRules
    animation: AnimationRules
    typography: TypographyRules
    layout: LayoutThresholds
    audio_controls: AudioControlRules
    toggle_variant: ToggleVariant
    label_position: LabelPosition
    group_separator: GroupSeparatorStyle

    def __post_init__(self) -> None:
        if not isinstance(self.language, DesignLanguage) or not isinstance(self.platform, DesignPlatform):
            raise ValueError("design system language and platform must be typed enums")


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
