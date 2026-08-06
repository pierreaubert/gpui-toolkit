"""Audio-kit declarative scales, ticks, accessibility, and automation contracts.

Native GPUI elements render the controls; these portable values let Python
declare their data and inspect host-provided semantic reports.
"""
from __future__ import annotations
from dataclasses import dataclass
from enum import Enum
import math
from threading import Lock

class ScaleType(str, Enum):
    LINEAR = "linear"
    QUADRATIC = "quadratic"
    LOGARITHMIC = "logarithmic"
    def value_to_position(self, value: float, minimum: float, maximum: float) -> float:
        if maximum <= minimum: return 0.0
        normalized = min(1.0, max(0.0, (value - minimum) / (maximum - minimum)))
        if self is ScaleType.QUADRATIC: return normalized * normalized
        if self is ScaleType.LOGARITHMIC:
            return math.log(max(0.0, value - minimum) + 1.0) / math.log(maximum - minimum + 1.0)
        return normalized
    def position_to_value(self, position: float, minimum: float, maximum: float) -> float:
        if maximum <= minimum: return minimum
        position = min(1.0, max(0.0, position))
        if self is ScaleType.QUADRATIC: position = math.sqrt(position)
        elif self is ScaleType.LOGARITHMIC: position = math.exp(position * math.log(maximum - minimum + 1.0)) - 1.0
        return minimum + position * (maximum - minimum)

@dataclass(frozen=True)
class TickMark:
    position: float
    is_major: bool
    label: str | None = None
    def __post_init__(self) -> None:
        if not 0 <= self.position <= 1: raise ValueError("tick position must be normalized")

@dataclass(frozen=True)
class TickConfig:
    scale: ScaleType = ScaleType.QUADRATIC
    minimum: float = -60.0
    maximum: float = 0.0
    major_values: tuple[float, ...] = (-60.0, -30.0, -10.0, 0.0)
    minor_count: int = 4
    def __post_init__(self) -> None:
        if not math.isfinite(self.minimum) or not math.isfinite(self.maximum) or self.maximum <= self.minimum or self.minor_count < 0: raise ValueError("invalid tick configuration")
    def generate_ticks(self) -> tuple[TickMark, ...]:
        ticks = [TickMark(self.scale.value_to_position(value, self.minimum, self.maximum), True) for value in self.major_values if self.minimum <= value <= self.maximum]
        if self.minor_count:
            for start, end in zip(self.major_values, self.major_values[1:]):
                for index in range(1, self.minor_count + 1):
                    value = start + (end - start) * index / (self.minor_count + 1)
                    if self.minimum < value < self.maximum: ticks.append(TickMark(self.scale.value_to_position(value, self.minimum, self.maximum), False))
        return tuple(ticks)

@dataclass(frozen=True)
class AudioAccessibilitySummary:
    control_type: str
    label: str
    role: str
    value_now: float | None = None
    value_min: float | None = None
    value_max: float | None = None
    value_text: str | None = None
    unit: str | None = None
    normalized: float | None = None
    selected: bool = False
    disabled: bool = False
    muted: bool = False
    peak_value: float | None = None
    description: str = ""

@dataclass(frozen=True)
class AudioAutomationPattern:
    id: str
    parameter_family: str
    recommended_control: str
    scale: str
    automation_sources: tuple[str, ...]
    expected_interactions: tuple[str, ...]
    accessibility_summary_contract: str
    release_evidence: str
    status: str = "implemented"
    def __post_init__(self) -> None:
        if not self.id or self.status != "implemented": raise ValueError("invalid audio automation pattern")

@dataclass(frozen=True)
class AudioAutomationPatternReport:
    schema_version: int
    report_type: str
    patterns: tuple[AudioAutomationPattern, ...]
    def __post_init__(self) -> None:
        if self.schema_version != 1 or len({item.id for item in self.patterns}) != len(self.patterns): raise ValueError("invalid audio automation report")
    def blocking_entries(self) -> tuple[AudioAutomationPattern, ...]: return tuple(item for item in self.patterns if item.status != "implemented")
    def pattern(self, id: str) -> AudioAutomationPattern | None: return next((item for item in self.patterns if item.id == id), None)

@dataclass(frozen=True)
class MeterSample:
    levels: tuple[float, ...]
    peaks: tuple[float, ...] = ()
    sample_rate: float | None = None
    def __post_init__(self) -> None:
        if not self.levels or any(not math.isfinite(value) for value in (*self.levels, *self.peaks)):
            raise ValueError("meter samples require finite channel levels")
        if self.peaks and len(self.peaks) != len(self.levels): raise ValueError("meter peaks must match channel count")
        if self.sample_rate is not None and (not math.isfinite(self.sample_rate) or self.sample_rate <= 0): raise ValueError("sample rate must be positive and finite")

@dataclass(frozen=True)
class SpectrumSample:
    frequencies: tuple[float, ...]
    levels: tuple[float, ...]
    sample_rate: float
    def __post_init__(self) -> None:
        if len(self.frequencies) != len(self.levels) or not self.frequencies: raise ValueError("spectrum frequencies and levels must be non-empty and aligned")
        if any(not math.isfinite(value) for value in (*self.frequencies, *self.levels)) or any(value <= 0 for value in self.frequencies): raise ValueError("spectrum values must be finite and frequencies positive")
        if not math.isfinite(self.sample_rate) or self.sample_rate <= 0: raise ValueError("sample rate must be positive and finite")

class _LatestStream:
    """One-slot transport: producer never blocks and stale samples are dropped."""
    def __init__(self) -> None: self._latest = None; self._dropped = 0; self._lock = Lock()
    def push(self, sample: object) -> None:
        with self._lock:
            if self._latest is not None: self._dropped += 1
            self._latest = sample
    def take_latest(self):
        with self._lock:
            sample, self._latest = self._latest, None
            return sample
    @property
    def dropped_samples(self) -> int:
        with self._lock: return self._dropped

class MeterStream(_LatestStream):
    def push(self, sample: MeterSample) -> None: super().push(sample)
    def take_latest(self) -> MeterSample | None: return super().take_latest()

class SpectrumStream(_LatestStream):
    def push(self, sample: SpectrumSample) -> None: super().push(sample)
    def take_latest(self) -> SpectrumSample | None: return super().take_latest()
