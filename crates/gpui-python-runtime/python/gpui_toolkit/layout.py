"""Declarative ``gpui-builder`` trees; native host performs solving."""
from __future__ import annotations
from dataclasses import dataclass, field
from enum import Enum
import math
from typing import Any, TYPE_CHECKING
from .commands import CommandResult, CommandStatus
if TYPE_CHECKING: from .app import SessionContext

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

@dataclass(frozen=True)
class RatioPreference:
    id: str
    axis: Axis
    ratio: float
    def __post_init__(self) -> None:
        if not self.id or not math.isfinite(self.ratio): raise ValueError("invalid layout ratio preference")
    def to_spec(self) -> dict[str, Any]: return {"id": self.id, "axis": self.axis.value, "ratio": self.ratio}

@dataclass(frozen=True)
class CollapsePreference:
    id: str
    collapsed: bool = True
    def __post_init__(self) -> None:
        if not self.id: raise ValueError("layout collapse preference requires an id")
    def to_spec(self) -> dict[str, Any]: return {"id": self.id, "collapsed": self.collapsed}

@dataclass(frozen=True)
class LayoutPreferences:
    ratios: tuple[RatioPreference, ...] = ()
    collapsed: tuple[CollapsePreference, ...] = ()
    def to_spec(self) -> dict[str, Any]:
        return {"ratios": [value.to_spec() for value in self.ratios], "collapsed": [value.to_spec() for value in self.collapsed]}

@dataclass(frozen=True)
class SetRatio:
    id: str
    axis: Axis
    ratio: float

@dataclass(frozen=True)
class ClearRatio:
    id: str
    axis: Axis

@dataclass(frozen=True)
class SetCollapsed:
    id: str
    collapsed: bool

@dataclass(frozen=True)
class ToggleCollapsed:
    id: str

@dataclass(frozen=True)
class ClearCollapsed:
    id: str

@dataclass(frozen=True)
class ResetLayout:
    pass

LayoutAction = SetRatio | ClearRatio | SetCollapsed | ToggleCollapsed | ClearCollapsed | ResetLayout

class LayoutState:
    """Owned interaction state mirroring ``gpui_builder::LayoutState``."""
    def __init__(self) -> None:
        self._ratios: dict[tuple[str, Axis], float] = {}
        self._collapsed: dict[str, None] = {}
    def ratio_for(self, node_id: str, axis: Axis) -> float | None: return self._ratios.get((node_id, axis))
    def is_collapsed(self, node_id: str) -> bool: return node_id in self._collapsed
    def apply(self, action: LayoutAction) -> None:
        if isinstance(action, SetRatio):
            if not action.id or not math.isfinite(action.ratio): raise ValueError("invalid layout ratio action")
            self._ratios[(action.id, action.axis)] = action.ratio
        elif isinstance(action, ClearRatio): self._ratios.pop((action.id, action.axis), None)
        elif isinstance(action, SetCollapsed):
            if action.collapsed: self._collapsed.setdefault(action.id, None)
            else: self._collapsed.pop(action.id, None)
        elif isinstance(action, ToggleCollapsed):
            if action.id in self._collapsed: self._collapsed.pop(action.id)
            else: self._collapsed[action.id] = None
        elif isinstance(action, ClearCollapsed): self._collapsed.pop(action.id, None)
        elif isinstance(action, ResetLayout):
            self._ratios.clear()
            self._collapsed.clear()
        else: raise TypeError("unsupported layout action")
    def preferences(self) -> LayoutPreferences:
        ratios = tuple(RatioPreference(node_id, axis, ratio) for (node_id, axis), ratio in self._ratios.items())
        collapsed = tuple(CollapsePreference(node_id) for node_id in self._collapsed)
        return LayoutPreferences(ratios, collapsed)

@dataclass(frozen=True)
class AccessibilityMetadata:
    id: str
    role: str | None = None
    label: str | None = None
    description: str | None = None
    def __post_init__(self) -> None:
        if not self.id or self.role not in {None, "none", "group", "region", "tab"}: raise ValueError("invalid layout accessibility metadata")
    def to_spec(self) -> dict[str, Any]: return self.__dict__.copy()

@dataclass(frozen=True)
class SolvedLayoutNode:
    id: str
    width: float
    height: float
    visible: bool
    active_tier: str | None
    collapse_label: str | None
    resolved_axis: Axis | None
    children: tuple["SolvedLayoutNode", ...] = ()
    @classmethod
    def from_wire(cls, value: dict[str, Any]) -> "SolvedLayoutNode":
        axis = value.get("resolved_axis")
        return cls(
            str(value["id"]), float(value["width"]), float(value["height"]), bool(value["visible"]),
            None if value.get("active_tier") is None else str(value["active_tier"]),
            None if value.get("collapse_label") is None else str(value["collapse_label"]),
            None if axis is None else Axis(str(axis)),
            tuple(cls.from_wire(child) for child in value.get("children", ())),
        )
    def find(self, node_id: str) -> "SolvedLayoutNode | None":
        if self.id == node_id: return self
        for child in self.children:
            found = child.find(node_id)
            if found is not None: return found
        return None

@dataclass(frozen=True)
class AccessibilityNode:
    id: str
    role: str
    label: str | None
    description: str | None
    visible: bool
    collapsed: bool
    active_tier: str | None
    children: tuple["AccessibilityNode", ...] = ()
    @classmethod
    def from_wire(cls, value: dict[str, Any]) -> "AccessibilityNode":
        return cls(
            str(value["id"]), str(value["role"]),
            None if value.get("label") is None else str(value["label"]),
            None if value.get("description") is None else str(value["description"]),
            bool(value["visible"]), bool(value["collapsed"]),
            None if value.get("active_tier") is None else str(value["active_tier"]),
            tuple(cls.from_wire(child) for child in value.get("children", ())),
        )
    def find(self, node_id: str) -> "AccessibilityNode | None":
        if self.id == node_id: return self
        for child in self.children:
            found = child.find(node_id)
            if found is not None: return found
        return None

@dataclass(frozen=True)
class CollapsedTab:
    id: str
    label: str

@dataclass(frozen=True)
class LayoutValidationIssue:
    severity: str
    kind: str
    node_id: str
    path: str
    message: str

@dataclass(frozen=True)
class LayoutValidation:
    clean: bool
    error_count: int
    warning_count: int
    issues: tuple[LayoutValidationIssue, ...]
    report: str

@dataclass(frozen=True)
class LayoutInspection:
    declaration_report: str
    solved_report: str

@dataclass(frozen=True)
class LayoutDebugWarning:
    code: str
    node_id: str
    message: str
    remediation: str

@dataclass(frozen=True)
class LayoutDebugReport:
    report: str
    warnings: tuple[LayoutDebugWarning, ...]

@dataclass(frozen=True)
class SolvedLayout:
    root: SolvedLayoutNode
    validation: LayoutValidation
    inspection: LayoutInspection
    debug: LayoutDebugReport
    collapsed_tabs: tuple[CollapsedTab, ...] = ()
    accessibility: AccessibilityNode | None = None

def solve(
    context: "SessionContext", request_id: str, root: LayoutNode, width: float, height: float,
    preferences: LayoutPreferences = LayoutPreferences(), char_width: float = 8.0,
    accessibility: tuple[AccessibilityMetadata, ...] = (),
) -> None:
    if not math.isfinite(width) or width < 0 or not math.isfinite(height) or height < 0:
        raise ValueError("layout dimensions must be finite and non-negative")
    if not math.isfinite(char_width) or char_width <= 0: raise ValueError("char_width must be positive and finite")
    context.command(
        request_id, "builder.solve", root=to_spec(root), width=width, height=height,
        preferences=preferences.to_spec(), char_width=char_width,
        accessibility=[value.to_spec() for value in accessibility],
    )

def solved_layout_from_command(result: CommandResult) -> SolvedLayout:
    if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "builder layout solve failed")
    validation = result.data["validation"]
    inspection = result.data["inspection"]
    debug = result.data["debug"]
    return SolvedLayout(
        SolvedLayoutNode.from_wire(result.data["solved"]),
        LayoutValidation(
            bool(validation["clean"]), int(validation["error_count"]), int(validation["warning_count"]),
            tuple(LayoutValidationIssue(str(issue["severity"]), str(issue["kind"]), str(issue["node_id"]), str(issue["path"]), str(issue["message"])) for issue in validation["issues"]),
            str(validation["report"]),
        ),
        LayoutInspection(str(inspection["declaration_report"]), str(inspection["solved_report"])),
        LayoutDebugReport(
            str(debug["report"]),
            tuple(LayoutDebugWarning(str(warning["code"]), str(warning["node_id"]), str(warning["message"]), str(warning["remediation"])) for warning in debug["warnings"]),
        ),
        tuple(CollapsedTab(str(tab["id"]), str(tab["label"])) for tab in result.data.get("collapsed_tabs", ())),
        None if result.data.get("accessibility") is None else AccessibilityNode.from_wire(result.data["accessibility"]),
    )

@dataclass(frozen=True)
class LayoutViewport:
    label: str
    width: float
    height: float
    def __post_init__(self) -> None:
        if not self.label or not math.isfinite(self.width) or self.width < 0 or not math.isfinite(self.height) or self.height < 0: raise ValueError("invalid layout viewport")
    def to_spec(self) -> dict[str, Any]: return self.__dict__.copy()

@dataclass(frozen=True)
class LayoutSnapshot:
    label: str
    width: float
    height: float
    root: SolvedLayoutNode
    visible_ids: tuple[str, ...] = ()
    collapsed_labels: tuple[str, ...] = ()
    active_tiers: tuple[str, ...] = ()
    resolved_axes: tuple[str, ...] = ()
    @classmethod
    def from_wire(cls, value: dict[str, Any]) -> "LayoutSnapshot":
        return cls(
            str(value["label"]), float(value["width"]), float(value["height"]), SolvedLayoutNode.from_wire(value["root"]),
            tuple(str(item) for item in value.get("visible_ids", ())), tuple(str(item) for item in value.get("collapsed_labels", ())),
            tuple(str(item) for item in value.get("active_tiers", ())), tuple(str(item) for item in value.get("resolved_axes", ())),
        )

@dataclass(frozen=True)
class LayoutSnapshotMatrix:
    snapshots: tuple[LayoutSnapshot, ...]
    retained_snapshots: tuple[LayoutSnapshot, ...]
    report: str
    markdown: str

def solve_matrix(
    context: "SessionContext", request_id: str, root: LayoutNode,
    viewports: tuple[LayoutViewport, ...], preferences: LayoutPreferences = LayoutPreferences(),
    char_width: float = 8.0, include_retained: bool = True,
) -> None:
    if not math.isfinite(char_width) or char_width <= 0: raise ValueError("char_width must be positive and finite")
    context.command(
        request_id, "builder.solve_matrix", root=to_spec(root),
        viewports=[viewport.to_spec() for viewport in viewports], preferences=preferences.to_spec(),
        char_width=char_width, include_retained=include_retained,
    )

def snapshot_matrix_from_command(result: CommandResult) -> LayoutSnapshotMatrix:
    if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "builder snapshot matrix failed")
    return LayoutSnapshotMatrix(
        tuple(LayoutSnapshot.from_wire(value) for value in result.data["snapshots"]),
        tuple(LayoutSnapshot.from_wire(value) for value in result.data.get("retained_snapshots", ())),
        str(result.data["report"]), str(result.data["markdown"]),
    )

class KnobSize(str, Enum):
    XS = "xs"
    SM = "sm"
    MD = "md"

@dataclass(frozen=True)
class KnobSlot:
    id: str
    param_idx: int
    label: str
    size: KnobSize = KnobSize.MD
    bipolar: bool = False
    def to_spec(self) -> dict[str, Any]: return {"id": self.id, "param_idx": self.param_idx, "label": self.label, "size": self.size.value, "bipolar": self.bipolar}

@dataclass(frozen=True)
class KnobRow:
    id: str
    knobs: tuple[KnobSlot, ...]
    def to_spec(self) -> dict[str, Any]: return {"kind": "knob_row", "id": self.id, "knobs": [knob.to_spec() for knob in self.knobs]}

@dataclass(frozen=True)
class BandToggleRow:
    id: str
    label: str
    has_toggle: bool = True
    def to_spec(self) -> dict[str, Any]: return {"kind": "band_toggle", **self.__dict__}

@dataclass(frozen=True)
class ReadoutTileRow:
    id: str
    label: str
    def to_spec(self) -> dict[str, Any]: return {"kind": "readout_tile", **self.__dict__}

@dataclass(frozen=True)
class ToggleGroupRow:
    id: str
    label: str
    def to_spec(self) -> dict[str, Any]: return {"kind": "toggle_group", **self.__dict__}

ChassisRow = KnobRow | BandToggleRow | ReadoutTileRow | ToggleGroupRow

@dataclass(frozen=True)
class ChassisHeader:
    brand_mark: str = ""
    title: str = ""
    subtitle: str = ""

@dataclass(frozen=True)
class ChassisFooter:
    ticks: tuple[str, ...] = ()
    serial: str = ""
    def to_spec(self) -> dict[str, Any]: return {"ticks": list(self.ticks), "serial": self.serial}

@dataclass(frozen=True)
class ChassisSection:
    id: str
    min_width: float
    preferred_width: float
    priority: float = 1.0
    eyebrow: str = ""
    title: str = ""
    caption: str | None = None
    rows: tuple[ChassisRow, ...] = ()
    def to_spec(self) -> dict[str, Any]:
        return {"id": self.id, "min_width": self.min_width, "preferred_width": self.preferred_width, "priority": self.priority, "eyebrow": self.eyebrow, "title": self.title, "caption": self.caption, "rows": [row.to_spec() for row in self.rows]}

@dataclass(frozen=True)
class SolvedChassisSection:
    id: str
    width: float
    visible: bool

def solve_chassis(
    context: "SessionContext", request_id: str, width: float, sections: tuple[ChassisSection, ...],
    header: ChassisHeader = ChassisHeader(), footer: ChassisFooter | None = None,
) -> None:
    if not math.isfinite(width) or width < 0: raise ValueError("chassis width must be finite and non-negative")
    context.command(
        request_id, "builder.solve_chassis", width=width, sections=[section.to_spec() for section in sections],
        header=header.__dict__.copy(), footer=None if footer is None else footer.to_spec(),
    )

def solved_chassis_from_command(result: CommandResult) -> tuple[SolvedChassisSection, ...]:
    if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "builder chassis solve failed")
    return tuple(SolvedChassisSection(str(section["id"]), float(section["width"]), bool(section["visible"])) for section in result.data["sections"])
