"""Typed declarations for the host-owned :mod:`gpui-design` surface.

These values describe design-token input and reports returned by the native
host.  They deliberately do not reproduce Rust's layout or conformance
algorithms in Python.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
import json
from math import isfinite
from typing import Any, Mapping
from .commands import CommandResult, CommandStatus

if False:
    from .app import SessionContext


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


class DesignTokenFormat(str, Enum):
    STYLE_DICTIONARY_JSON = "style-dictionary-json"


@dataclass(frozen=True)
class DesignTokenPreset:
    preset_id: str
    tokens: tuple[DesignToken, ...]

    def __post_init__(self) -> None:
        if not self.preset_id or not self.tokens:
            raise ValueError("design token preset cannot be empty")


@dataclass(frozen=True)
class DesignTokenExport:
    presets: tuple[DesignTokenPreset, ...]

    @classmethod
    def from_json(cls, source: str) -> "DesignTokenExport":
        wire = json.loads(source)
        return cls(tuple(
            DesignTokenPreset(str(preset["preset_id"]), tuple(DesignToken.from_spec(token) for token in preset["tokens"]))
            for preset in wire["presets"]
        ))

    @classmethod
    def from_spec(cls, wire: Mapping[str, Any]) -> "DesignTokenExport":
        return cls.from_json(json.dumps(wire))


@dataclass(frozen=True)
class DesignConformanceCase:
    preset_id: str
    reduced_motion: bool
    report: DesignConformanceReport
    motion: MotionSpec
    token_count: int

    @property
    def passed(self) -> bool:
        return self.report.passed


@dataclass(frozen=True)
class DesignConformanceMatrix:
    cases: tuple[DesignConformanceCase, ...]

    @property
    def passed(self) -> bool:
        return all(case.passed for case in self.cases)


@dataclass(frozen=True)
class DesignDocumentationReport:
    schema_version: int
    report_type: str
    presets: tuple[DesignPresetDocumentation, ...]
    conformance: DesignConformanceMatrix
    markdown: str

    @property
    def passed(self) -> bool:
        return self.conformance.passed


@dataclass(frozen=True)
class DesignReleaseAsset:
    id: str
    title: str
    kind: str
    path: str
    status: str
    release_note_use: str

    @property
    def is_release_blocking(self) -> bool:
        return self.status == "CaptureRequired"


@dataclass(frozen=True)
class DesignReleasePresentation:
    schema_version: int
    report_type: str
    documentation_report_type: str
    documentation_report: DesignDocumentationReport
    assets: tuple[DesignReleaseAsset, ...]
    release_notes_markdown: str

    @property
    def blocking_assets(self) -> tuple[DesignReleaseAsset, ...]:
        return tuple(asset for asset in self.assets if asset.is_release_blocking)


@dataclass(frozen=True)
class DesignReports:
    tokens: DesignTokenExport
    documentation: DesignDocumentationReport
    release: DesignReleasePresentation


@dataclass(frozen=True)
class ImportedDesignTokens:
    preset_count: int
    token_count: int
    raw: Mapping[str, object]


@dataclass(frozen=True)
class DesignTokenValidationReport:
    schema_version: int
    report_type: str
    passed: bool
    findings: tuple[str, ...]
    preset_count: int
    token_count: int
    conformance_markdown: str


@dataclass(frozen=True)
class DesignToolingHandoffItem:
    id: str
    title: str
    artifact_type: str
    path_or_command: str
    status: str
    release_evidence: str
    remaining_gap: str

    @property
    def is_release_blocking(self) -> bool:
        return self.status not in ("implemented", "documented")


@dataclass(frozen=True)
class DesignToolingHandoffReport:
    schema_version: int
    report_type: str
    crate_name: str
    crate_version: str
    items: tuple[DesignToolingHandoffItem, ...]

    @property
    def blocking_entries(self) -> tuple[DesignToolingHandoffItem, ...]:
        return tuple(item for item in self.items if item.is_release_blocking)

    def item(self, item_id: str) -> DesignToolingHandoffItem:
        for item in self.items:
            if item.id == item_id:
                return item
        raise KeyError(item_id)


def _request(context: "SessionContext", request_id: str, operation: str, *, source: str | None = None, render_markdown: bool = False, format: DesignTokenFormat = DesignTokenFormat.STYLE_DICTIONARY_JSON) -> None:
    arguments: dict[str, object] = {"operation": operation, "format": format.value, "render_markdown": render_markdown}
    if source is not None:
        arguments["input"] = source
    context.command(request_id, "design.tokens", **arguments)


def request_token_export(context: "SessionContext", request_id: str, *, format: DesignTokenFormat = DesignTokenFormat.STYLE_DICTIONARY_JSON) -> None:
    _request(context, request_id, "export", format=format)


def request_token_import(context: "SessionContext", request_id: str, source: str, *, format: DesignTokenFormat = DesignTokenFormat.STYLE_DICTIONARY_JSON) -> None:
    _request(context, request_id, "import", source=source, format=format)


def request_token_validation(context: "SessionContext", request_id: str, source: str, *, render_markdown: bool = True, format: DesignTokenFormat = DesignTokenFormat.STYLE_DICTIONARY_JSON) -> None:
    _request(context, request_id, "validate", source=source, render_markdown=render_markdown, format=format)


def request_handoff_report(context: "SessionContext", request_id: str) -> None:
    _request(context, request_id, "handoff")


def _data(result: CommandResult) -> Mapping[str, Any]:
    if result.status is not CommandStatus.SUCCEEDED:
        raise RuntimeError(result.error or "design token command failed")
    return result.data


def export_from_command(result: CommandResult) -> DesignTokenExport:
    return DesignTokenExport.from_json(str(_data(result)["output"]))


def import_from_command(result: CommandResult) -> ImportedDesignTokens:
    data = _data(result)
    return ImportedDesignTokens(int(data["preset_count"]), int(data["token_count"]), data["raw"])


def validation_from_command(result: CommandResult) -> DesignTokenValidationReport:
    report = _data(result)["report"]
    return DesignTokenValidationReport(int(report["schema_version"]), str(report["report_type"]), bool(report["passed"]), tuple(str(value) for value in report["findings"]), int(report["preset_count"]), int(report["token_count"]), str(report["conformance_markdown"]))


def handoff_from_command(result: CommandResult) -> DesignToolingHandoffReport:
    report = _data(result)["report"]
    return DesignToolingHandoffReport(int(report["schema_version"]), str(report["report_type"]), str(report["crate_name"]), str(report["crate_version"]), tuple(
        DesignToolingHandoffItem(str(item["id"]), str(item["title"]), str(item["artifact_type"]), str(item["path_or_command"]), str(item["status"]), str(item["release_evidence"]), str(item["remaining_gap"]))
        for item in report["items"]
    ))


def request_reports(context: "SessionContext", request_id: str) -> None:
    context.command(request_id, "design.reports")


def _language(value: str) -> DesignLanguage:
    return {
        "AppleHig": DesignLanguage.APPLE_HIG, "Material3": DesignLanguage.MATERIAL3,
        "Fluent": DesignLanguage.FLUENT, "Adwaita": DesignLanguage.ADWAITA,
        "Breeze": DesignLanguage.BREEZE, "Carbon": DesignLanguage.CARBON,
        "Neutral": DesignLanguage.NEUTRAL,
    }[value]


def _documentation_from_spec(report: Mapping[str, Any]) -> DesignDocumentationReport:
    presets = tuple(DesignPresetDocumentation(
        str(preset["preset_id"]), str(preset["label"]), _language(str(preset["language"])), int(preset["token_count"]),
        float(preset["grid_unit"]), float(preset["min_touch_target"]), float(preset["base_size"]),
        CornerRadiusStyle(str(preset["corner_style"]).lower()), int(preset["motion_duration_ms"]), int(preset["reduced_motion_duration_ms"]),
    ) for preset in report["presets"])
    cases = tuple(DesignConformanceCase(
        str(case["preset_id"]), bool(case["reduced_motion"]),
        DesignConformanceReport(tuple(ConformanceFinding(str(value["id"]), str(value["message"])) for value in case["report"]["findings"])),
        MotionSpec(int(case["motion"]["duration_ms"]), int(case["motion"]["fast_ms"]), int(case["motion"]["slow_ms"]), bool(case["motion"]["prefer_spring"]), bool(case["reduced_motion"])),
        int(case["token_count"]),
    ) for case in report["conformance"]["cases"])
    return DesignDocumentationReport(int(report["schema_version"]), str(report["report_type"]), presets, DesignConformanceMatrix(cases), str(report["markdown"]))


def reports_from_command(result: CommandResult) -> DesignReports:
    data = _data(result)
    documentation = _documentation_from_spec(data["documentation"])
    release_wire = data["release"]
    release = DesignReleasePresentation(
        int(release_wire["schema_version"]), str(release_wire["report_type"]), str(release_wire["documentation_report_type"]),
        _documentation_from_spec(release_wire["documentation_report"]),
        tuple(DesignReleaseAsset(str(asset["id"]), str(asset["title"]), str(asset["kind"]), str(asset["path"]), str(asset["status"]), str(asset["release_note_use"])) for asset in release_wire["assets"]),
        str(release_wire["release_notes_markdown"]),
    )
    return DesignReports(DesignTokenExport.from_spec(data["tokens"]), documentation, release)
