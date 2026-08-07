"""Typed requests and snapshots for host-native ``gpui-pretext`` layout.

Python declares preparation and layout work; the native host remains the sole
implementation of text measurement, segmentation, bidi handling, and line
breaking.  Returned snapshots are safe to retain or serialize in app state.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
import math
from typing import Any, TYPE_CHECKING

from .commands import CommandResult, CommandStatus

if TYPE_CHECKING:
    from .app import SessionContext


class WhiteSpaceMode(str, Enum):
    NORMAL = "normal"
    PRE_WRAP = "pre_wrap"


class LineBreakStrategy(str, Enum):
    GREEDY = "greedy"
    OPTIMAL = "optimal"


@dataclass(frozen=True)
class EngineProfile:
    line_fit_epsilon: float = 0.005
    carry_cjk_after_closing_quote: bool = False
    prefer_prefix_widths_for_breakable_runs: bool = False
    prefer_early_soft_hyphen_break: bool = False

    def __post_init__(self) -> None:
        if not math.isfinite(self.line_fit_epsilon) or self.line_fit_epsilon < 0:
            raise ValueError("line_fit_epsilon must be finite and non-negative")


@dataclass(frozen=True)
class PrepareOptions:
    white_space: WhiteSpaceMode = WhiteSpaceMode.NORMAL


@dataclass(frozen=True)
class TextBudget:
    max_input_bytes: int = 16 * 1024 * 1024
    max_graphemes: int = 4_000_000
    max_segments: int = 1_000_000

    def __post_init__(self) -> None:
        if min(self.max_input_bytes, self.max_graphemes, self.max_segments) < 0:
            raise ValueError("text budgets cannot be negative")

    @classmethod
    def unlimited(cls) -> "TextBudget":
        # Rust uses usize::MAX on the supported 64-bit hosts.
        return cls(2**64 - 1, 2**64 - 1, 2**64 - 1)


@dataclass(frozen=True)
class KnuthPlassParams:
    line_penalty: float = 0.0
    hyphen_penalty: float = 50.0
    flagged_demerits: float = 3000.0
    fitness_demerits: float = 10000.0
    tolerance: float = 2.0
    looseness_recovery: bool = True

    def __post_init__(self) -> None:
        values = (self.line_penalty, self.hyphen_penalty, self.flagged_demerits, self.fitness_demerits, self.tolerance)
        if not all(math.isfinite(value) for value in values) or self.tolerance < 0:
            raise ValueError("Knuth-Plass parameters must be finite and tolerance non-negative")


@dataclass(frozen=True)
class TextPreparationRequest:
    text: str
    profile: EngineProfile = field(default_factory=EngineProfile)
    options: PrepareOptions = field(default_factory=PrepareOptions)
    budget: TextBudget = field(default_factory=TextBudget)
    include_segments: bool = False

    def to_spec(self) -> dict[str, Any]:
        return {
            "text": self.text,
            "profile": self.profile.__dict__.copy(),
            "options": {"white_space": self.options.white_space.value},
            "budget": self.budget.__dict__.copy(),
            "include_segments": self.include_segments,
        }


@dataclass(frozen=True)
class LayoutCursor:
    segment_index: int
    grapheme_index: int

    def __post_init__(self) -> None:
        if min(self.segment_index, self.grapheme_index) < 0:
            raise ValueError("layout cursor indices cannot be negative")


@dataclass(frozen=True)
class LayoutLineRange:
    width: float
    start: LayoutCursor
    end: LayoutCursor


@dataclass(frozen=True)
class LayoutResult:
    line_count: int
    height: float

    def __post_init__(self) -> None:
        if self.line_count < 0 or not math.isfinite(self.height):
            raise ValueError("layout results require a non-negative count and finite height")


@dataclass(frozen=True)
class PrepareProfile:
    analysis_segments: int
    prepared_segments: int
    breakable_segments: int

    def __post_init__(self) -> None:
        if min(self.analysis_segments, self.prepared_segments, self.breakable_segments) < 0:
            raise ValueError("prepare-profile counts cannot be negative")


@dataclass(frozen=True)
class PreparedLayout:
    result: LayoutResult
    lines: tuple[tuple[str, float, LayoutCursor, LayoutCursor], ...]
    segments: tuple[str, ...]


@dataclass(frozen=True)
class VariableFontAxis:
    tag: str
    minimum: float
    default: float
    maximum: float
    value: float | None = None

    def __post_init__(self) -> None:
        if len(self.tag) != 4 or not self.tag.isascii() or not all(math.isfinite(value) for value in (self.minimum, self.default, self.maximum)) or not self.minimum <= self.default <= self.maximum:
            raise ValueError("variable font axis requires a four-byte ASCII tag and ordered finite bounds")
        if self.value is not None and (not math.isfinite(self.value) or not self.minimum <= self.value <= self.maximum):
            raise ValueError("variable font axis value is outside its range")

    def to_spec(self) -> dict[str, object]:
        return {"tag": self.tag, "min": self.minimum, "default": self.default, "max": self.maximum, "value": self.default if self.value is None else self.value}


@dataclass(frozen=True)
class RichTextStyle:
    bold: bool = False
    italic: bool = False
    code: bool = False
    link: str | None = None


@dataclass(frozen=True)
class RichTextSpan:
    text: str
    style: RichTextStyle = field(default_factory=RichTextStyle)


@dataclass(frozen=True)
class AccessibleTextRun:
    byte_start: int
    byte_end: int
    label: str
    role: str


@dataclass(frozen=True)
class RichTextAnalysis:
    spans: tuple[RichTextSpan, ...]
    accessibility_runs: tuple[AccessibleTextRun, ...]
    bidi_levels: tuple[int, ...] | None
    axes: tuple[VariableFontAxis, ...]
    css_settings: str


@dataclass(frozen=True)
class LanguageSupportNote:
    category: str
    level: str
    summary: str
    recommendation: str


@dataclass(frozen=True)
class LanguageSupportReport:
    schema_version: int
    report_type: str
    notes: tuple[LanguageSupportNote, ...]


@dataclass(frozen=True)
class LocaleGoldenCase:
    id: str
    locale: str
    category: str
    text: str
    white_space: str
    max_width: float
    line_height: float
    expected_lines: tuple[str, ...]
    note: str


@dataclass(frozen=True)
class LocaleGoldenReport:
    schema_version: int
    report_type: str
    cases: tuple[LocaleGoldenCase, ...]
    markdown: str


@dataclass(frozen=True)
class BenchmarkBaselineCase:
    id: str
    benchmark_id: str
    focus: str
    baseline_artifact: str
    comparator_artifact: str
    release_requirement: str


@dataclass(frozen=True)
class PlatformTextComparator:
    id: str
    platform: str
    backend: str
    artifact: str
    requirement: str


@dataclass(frozen=True)
class BenchmarkBaselineReport:
    schema_version: int
    report_type: str
    criterion_command: str
    baseline_policy: str
    cases: tuple[BenchmarkBaselineCase, ...]
    comparators: tuple[PlatformTextComparator, ...]
    locale_case_ids: tuple[str, ...]
    markdown: str


@dataclass(frozen=True)
class TextReports:
    language: LanguageSupportReport
    locale: LocaleGoldenReport
    benchmark: BenchmarkBaselineReport


def prepare_layout(context: "SessionContext", request_id: str, text: str, *, max_width: float, line_height: float = 16.0, char_width: float = 8.0, profile: EngineProfile | None = None, options: PrepareOptions | None = None, budget: TextBudget | None = None, strategy: LineBreakStrategy = LineBreakStrategy.GREEDY, knuth_plass: KnuthPlassParams | None = None) -> None:
    """Request host-native preparation and greedy line layout with finite metrics."""
    if not text or not all(math.isfinite(value) and value > 0 for value in (max_width, line_height, char_width)):
        raise ValueError("text and positive finite layout metrics are required")
    profile = profile or EngineProfile()
    options = options or PrepareOptions()
    budget = budget or TextBudget()
    knuth_plass = knuth_plass or KnuthPlassParams()
    context.command(request_id, "text.prepare_layout", text=text, max_width=max_width, line_height=line_height, char_width=char_width, profile=profile.__dict__.copy(), options={"white_space": options.white_space.value}, budget=budget.__dict__.copy(), strategy=strategy.value, knuth_plass=knuth_plass.__dict__.copy())


def analyze_rich_text(context: "SessionContext", request_id: str, text: str, *, axes: tuple[VariableFontAxis, ...] = ()) -> None:
    if not text:
        raise ValueError("rich text cannot be empty")
    context.command(request_id, "text.rich", text=text, axes=[axis.to_spec() for axis in axes])


def rich_text_from_command(result: CommandResult) -> RichTextAnalysis:
    if result.status is not CommandStatus.SUCCEEDED:
        raise RuntimeError(result.error or "rich text analysis failed")
    data = result.data
    spans = tuple(RichTextSpan(str(span["text"]), RichTextStyle(bool(span["style"]["bold"]), bool(span["style"]["italic"]), bool(span["style"]["code"]), None if span["style"].get("link") is None else str(span["style"]["link"]))) for span in data["spans"])
    runs = tuple(AccessibleTextRun(int(run["byte_start"]), int(run["byte_end"]), str(run["label"]), str(run["role"])) for run in data["accessibility_runs"])
    axes = tuple(VariableFontAxis(str(axis["tag"]), float(axis["min"]), float(axis["default"]), float(axis["max"]), float(axis["value"])) for axis in data["axes"])
    levels = None if data.get("bidi_levels") is None else tuple(int(value) for value in data["bidi_levels"])
    return RichTextAnalysis(spans, runs, levels, axes, str(data["css_settings"]))


def request_reports(context: "SessionContext", request_id: str) -> None:
    context.command(request_id, "text.reports")


def reports_from_command(result: CommandResult) -> TextReports:
    if result.status is not CommandStatus.SUCCEEDED:
        raise RuntimeError(result.error or "text reports failed")
    data = result.data
    language = data["language"]
    locale = data["locale"]
    benchmark = data["benchmark"]
    return TextReports(
        LanguageSupportReport(int(language["schema_version"]), str(language["report_type"]), tuple(LanguageSupportNote(str(note["category"]), str(note["level"]), str(note["summary"]), str(note["recommendation"])) for note in language["notes"])),
        LocaleGoldenReport(int(locale["schema_version"]), str(locale["report_type"]), tuple(LocaleGoldenCase(str(case["id"]), str(case["locale"]), str(case["category"]), str(case["text"]), str(case["white_space"]), float(case["max_width"]), float(case["line_height"]), tuple(str(line) for line in case["expected_lines"]), str(case["note"])) for case in locale["cases"]), str(locale["markdown"])),
        BenchmarkBaselineReport(int(benchmark["schema_version"]), str(benchmark["report_type"]), str(benchmark["criterion_command"]), str(benchmark["baseline_policy"]), tuple(BenchmarkBaselineCase(str(case["id"]), str(case["benchmark_id"]), str(case["focus"]), str(case["baseline_artifact"]), str(case["comparator_artifact"]), str(case["release_requirement"])) for case in benchmark["cases"]), tuple(PlatformTextComparator(str(value["id"]), str(value["platform"]), str(value["backend"]), str(value["artifact"]), str(value["requirement"])) for value in benchmark["comparators"]), tuple(str(value) for value in benchmark["locale_case_ids"]), str(benchmark["markdown"])),
    )


def prepared_layout_from_command(result: CommandResult) -> PreparedLayout:
    if result.status is not CommandStatus.SUCCEEDED:
        raise RuntimeError(result.error or f"text layout {result.status.value}")
    try:
        lines = tuple((str(line["text"]), float(line["width"]),
            LayoutCursor(int(line["start"]["segment_index"]), int(line["start"]["grapheme_index"])),
            LayoutCursor(int(line["end"]["segment_index"]), int(line["end"]["grapheme_index"])))
            for line in result.data["lines"])
        return PreparedLayout(LayoutResult(int(result.data["line_count"]), float(result.data["height"])), lines, tuple(str(value) for value in result.data["segments"]))
    except (KeyError, TypeError, ValueError) as error:
        raise ValueError("native text layout result has an invalid shape") from error
