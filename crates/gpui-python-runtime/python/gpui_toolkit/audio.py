"""Audio-kit declarative scales, ticks, accessibility, and automation contracts.

Native GPUI elements render the controls; these portable values let Python
declare their data and inspect host-provided semantic reports.
"""
from __future__ import annotations
from dataclasses import dataclass
from enum import Enum
import math
import struct
from threading import Event as ThreadEvent, Lock, Thread
from typing import Sequence, TYPE_CHECKING
from .ui import Node
from .commands import CommandResult, CommandStatus
if TYPE_CHECKING: from .app import SessionContext

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

class AudioControlSize(str, Enum):
    XS = "xs"
    SM = "sm"
    MD = "md"
    LG = "lg"

class AudioControlScale(str, Enum):
    LINEAR = "linear"
    LOGARITHMIC = "logarithmic"

def potentiometer(
    *, id: str, value: float, minimum: float = 0.0, maximum: float = 1.0,
    label: str = "", unit: str = "", size: AudioControlSize = AudioControlSize.MD,
    scale: AudioControlScale = AudioControlScale.LINEAR, selected: bool = False, disabled: bool = False,
    action: str | None = None, commit_action: str | None = None, aria_label: str | None = None,
) -> Node:
    return Node("audio_potentiometer", {"id": id, "value": float(value), "min": float(minimum), "max": float(maximum), "label": label, "unit": unit, "size": size.value, "scale": scale.value, "selected": selected, "disabled": disabled, "action": action, "commit_action": commit_action, "aria_label": aria_label})

def vertical_slider(
    *, id: str, value: float, minimum: float = 0.0, maximum: float = 1.0,
    label: str = "", unit: str = "", size: AudioControlSize = AudioControlSize.MD,
    scale: AudioControlScale = AudioControlScale.LINEAR, selected: bool = False, disabled: bool = False,
    peak: float | None = None, with_ticks: bool = False, height: float | None = None,
    action: str | None = None, commit_action: str | None = None, aria_label: str | None = None,
) -> Node:
    return Node("audio_vertical_slider", {"id": id, "value": float(value), "min": float(minimum), "max": float(maximum), "label": label, "unit": unit, "size": size.value, "scale": scale.value, "selected": selected, "disabled": disabled, "peak": peak, "with_ticks": with_ticks, "height": height, "action": action, "commit_action": commit_action, "aria_label": aria_label})

def volume_knob(
    *, id: str, value: float, label: str = "Volume", muted: bool = False,
    width: float | None = None, action: str | None = None, commit_action: str | None = None, mute_action: str | None = None,
    aria_label: str | None = None,
) -> Node:
    return Node("audio_volume_knob", {"id": id, "value": float(value), "min": 0.0, "max": 1.0, "label": label, "muted": muted, "width": width, "action": action, "commit_action": commit_action, "mute_action": mute_action, "aria_label": aria_label})

def horizontal_meter(
    *, id: str, levels: Sequence[float] = (), peaks: Sequence[float] = (),
    channel_names: Sequence[str] = (), width: float | None = None, height: float | None = None,
    stream_id: str | None = None,
) -> Node:
    return Node("audio_horizontal_meter", {"id": id, "levels": [float(value) for value in levels], "peaks": [float(value) for value in peaks], "channel_names": [str(value) for value in channel_names], "width": width, "height": height, "stream_id": stream_id})

def level_meter(
    *, id: str, levels: Sequence[float] = (), peaks: Sequence[float] = (),
    channel_names: Sequence[str] = (), width: float | None = None, height: float | None = None,
    stream_id: str | None = None,
) -> Node:
    return Node("audio_level_meter", {"id": id, "levels": [float(value) for value in levels], "peaks": [float(value) for value in peaks], "channel_names": [str(value) for value in channel_names], "width": width, "height": height, "stream_id": stream_id})

def spectrum(
    *, id: str, magnitudes: Sequence[float] = (), previous: Sequence[float] = (),
    minimum_frequency: float = 20.0, maximum_frequency: float = 20_000.0,
    smoothing: float = 0.8, height: float | None = None, bar_gap: float | None = None,
    stream_id: str | None = None,
) -> Node:
    return Node("audio_spectrum", {"id": id, "magnitudes": [float(value) for value in magnitudes], "previous": [float(value) for value in previous], "minimum_frequency": float(minimum_frequency), "maximum_frequency": float(maximum_frequency), "smoothing": float(smoothing), "height": height, "bar_gap": bar_gap, "stream_id": stream_id})

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
    scale: str | None = None
    selected: bool = False
    disabled: bool = False
    muted: bool = False
    peak_value: float | None = None
    description: str = ""

def request_accessibility(context: "SessionContext", request_id: str, node: Node) -> None:
    context.command(request_id, "audio.accessibility", node=node.to_spec())

def accessibility_from_command(result: CommandResult) -> tuple[AudioAccessibilitySummary, ...]:
    if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "audio accessibility failed")
    return tuple(AudioAccessibilitySummary(
        control_type=str(value["control_type"]), label=str(value["label"]), role=str(value["role"]),
        value_now=None if value.get("value_now") is None else float(value["value_now"]),
        value_min=None if value.get("value_min") is None else float(value["value_min"]),
        value_max=None if value.get("value_max") is None else float(value["value_max"]),
        value_text=None if value.get("value_text") is None else str(value["value_text"]),
        unit=None if value.get("unit") is None else str(value["unit"]),
        normalized=None if value.get("normalized") is None else float(value["normalized"]),
        scale=None if value.get("scale") is None else str(value["scale"]),
        selected=bool(value.get("selected",False)), disabled=bool(value.get("disabled",False)),
        muted=bool(value.get("muted",False)),
        peak_value=None if value.get("peak_value") is None else float(value["peak_value"]),
        description=str(value.get("description","")),
    ) for value in result.data["summaries"])

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
class AudioDesignTokens:
    knob_arc_start_deg: float
    knob_arc_sweep_deg: float
    knob_arc_widths: tuple[float, ...]
    knob_arc_track_widths: tuple[float, ...]
    knob_arc_glow: float
    knob_arc_segments: int
    knob_border_width: float
    knob_label_style: int
    knob_indicator_style: int
    slider_track_widths: tuple[float, ...]
    meter_label_style: int
    meter_use_gradient: bool
    meter_corner_radius: float
    meter_glow: float
    toggle_variant: int
    corner_radius: float
    min_touch_target: float
    control_padding_x: float
    control_padding_y: float
    animation_duration_ms: int
    prefer_spring: bool
    spring_stiffness: float
    spring_damping: float

@dataclass(frozen=True)
class AudioVisualRegressionReport:
    schema_version: int
    report_type: str
    crate_name: str
    crate_version: str
    capture_count: int
    expected_capture_count: int
    unique_capture_ids: bool
    components: tuple[str, ...]
    markdown: str

@dataclass(frozen=True)
class AudioReports:
    automation: AudioAutomationPatternReport
    automation_markdown: str
    visual: AudioVisualRegressionReport
    design_tokens: AudioDesignTokens

def request_reports(context: "SessionContext", request_id: str) -> None:
    context.command(request_id, "audio.reports")

def reports_from_command(result: CommandResult) -> AudioReports:
    if result.status is not CommandStatus.SUCCEEDED: raise RuntimeError(result.error or "audio reports failed")
    automation, visual, tokens = result.data["automation"], result.data["visual"], result.data["design_tokens"]
    patterns = tuple(AudioAutomationPattern(
        str(value["id"]), str(value["parameter_family"]), str(value["recommended_control"]), str(value["scale"]),
        tuple(str(item) for item in value["automation_sources"]), tuple(str(item) for item in value["expected_interactions"]),
        str(value["accessibility_summary_contract"]), str(value["release_evidence"]), str(value["status"]),
    ) for value in automation["patterns"])
    return AudioReports(
        AudioAutomationPatternReport(int(automation["schema_version"]), str(automation["report_type"]), patterns),
        str(automation["markdown"]),
        AudioVisualRegressionReport(int(visual["schema_version"]), str(visual["report_type"]), str(visual["crate_name"]), str(visual["crate_version"]), int(visual["capture_count"]), int(visual["expected_capture_count"]), bool(visual["unique_capture_ids"]), tuple(str(value) for value in visual["components"]), str(visual["markdown"])),
        AudioDesignTokens(
            float(tokens["knob_arc_start_deg"]), float(tokens["knob_arc_sweep_deg"]), tuple(float(value) for value in tokens["knob_arc_widths"]), tuple(float(value) for value in tokens["knob_arc_track_widths"]), float(tokens["knob_arc_glow"]), int(tokens["knob_arc_segments"]), float(tokens["knob_border_width"]), int(tokens["knob_label_style"]), int(tokens["knob_indicator_style"]), tuple(float(value) for value in tokens["slider_track_widths"]), int(tokens["meter_label_style"]), bool(tokens["meter_use_gradient"]), float(tokens["meter_corner_radius"]), float(tokens["meter_glow"]), int(tokens["toggle_variant"]), float(tokens["corner_radius"]), float(tokens["min_touch_target"]), float(tokens["control_padding_x"]), float(tokens["control_padding_y"]), int(tokens["animation_duration_ms"]), bool(tokens["prefer_spring"]), float(tokens["spring_stiffness"]), float(tokens["spring_damping"]),
        ),
    )

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
    """One-slot producer queue with an optional binary host sender."""
    def __init__(self, context: "SessionContext | None" = None, resource_id: str | None = None) -> None:
        if (context is None) != (resource_id is None): raise ValueError("native streams require both context and resource_id")
        if resource_id is not None and (not resource_id.strip() or len(resource_id) > 128): raise ValueError("invalid audio resource id")
        self._latest = None; self._dropped = 0; self._lock = Lock()
        self._context=context; self.resource_id=resource_id; self._generation=1; self._sequence=0
        self._wake=ThreadEvent(); self._closed=False; self._worker=None
        if context is not None:
            self._worker=Thread(target=self._send_loop, name=f"gpui-audio-{resource_id}", daemon=True)
            self._worker.start()
    def push(self, sample: object) -> None:
        with self._lock:
            if self._closed: raise RuntimeError("audio stream is closed")
            if self._latest is not None: self._dropped += 1
            self._latest = sample
        if self._context is not None: self._wake.set()
    def take_latest(self):
        with self._lock:
            sample, self._latest = self._latest, None
            return sample
    @property
    def dropped_samples(self) -> int:
        with self._lock: return self._dropped

    def _send_loop(self) -> None:
        while True:
            self._wake.wait(); self._wake.clear()
            while True:
                sample=self.take_latest()
                if sample is None: break
                self._sequence += 1
                header,payload=self._encode(sample)
                self._context.resource_frame({"resource_id":self.resource_id,"generation":self._generation,"sequence":self._sequence,"dtype":"f32","byte_order":"little","finite_policy":"drop_frame","coalesce":"latest",**header},payload)
            with self._lock:
                if self._closed and self._latest is None: return

    def _encode(self, sample: object) -> tuple[dict[str, object],bytes]: raise NotImplementedError

    def close(self) -> None:
        with self._lock: self._closed=True
        self._wake.set()
        if self._worker is not None:
            self._worker.join(timeout=1)
            self._context.drop_resource(self.resource_id,self._generation)

    def __enter__(self): return self
    def __exit__(self, *_): self.close()

class MeterStream(_LatestStream):
    def __init__(self, context: "SessionContext | None" = None, resource_id: str | None = None, *, channel_count: int | None = None, sample_rate: float = 48_000.0, attack_ms: float = 10.0, release_ms: float = 120.0) -> None:
        if channel_count is not None and not 1 <= channel_count <= 128: raise ValueError("meter channel count must be between 1 and 128")
        if not all(math.isfinite(value) and value >= 0 for value in (sample_rate,attack_ms,release_ms)) or sample_rate == 0: raise ValueError("invalid meter stream metadata")
        self.channel_count=channel_count; self.sample_rate=sample_rate; self.attack_ms=attack_ms; self.release_ms=release_ms
        super().__init__(context,resource_id)
    def push(self, sample: MeterSample) -> None:
        if self.channel_count is not None and len(sample.levels) != self.channel_count: raise ValueError("meter sample channel count changed")
        super().push(sample)
    def take_latest(self) -> MeterSample | None: return super().take_latest()
    def _encode(self, sample: MeterSample) -> tuple[dict[str,object],bytes]:
        if self.channel_count is not None and len(sample.levels) != self.channel_count: raise ValueError("meter sample channel count changed")
        values=(*sample.levels,*sample.peaks); payload=struct.pack(f"<{len(values)}f",*values)
        return {"frame_kind":"meter","shape":[len(sample.levels),2 if sample.peaks else 1],"sample_rate":sample.sample_rate or self.sample_rate,"attack_ms":self.attack_ms,"release_ms":self.release_ms},payload

class SpectrumStream(_LatestStream):
    def __init__(self, context: "SessionContext | None" = None, resource_id: str | None = None, *, bin_count: int | None = None, sample_rate: float = 48_000.0) -> None:
        if bin_count is not None and not 1 <= bin_count <= 32*1024: raise ValueError("spectrum bin count is out of range")
        if not math.isfinite(sample_rate) or sample_rate <= 0: raise ValueError("invalid spectrum sample rate")
        self.bin_count=bin_count; self.sample_rate=sample_rate
        super().__init__(context,resource_id)
    def push(self, sample: SpectrumSample) -> None:
        if self.bin_count is not None and len(sample.levels) != self.bin_count: raise ValueError("spectrum bin count changed")
        super().push(sample)
    def take_latest(self) -> SpectrumSample | None: return super().take_latest()
    def _encode(self, sample: SpectrumSample) -> tuple[dict[str,object],bytes]:
        if self.bin_count is not None and len(sample.levels) != self.bin_count: raise ValueError("spectrum bin count changed")
        payload=struct.pack(f"<{len(sample.levels)}f",*sample.levels)
        return {"frame_kind":"spectrum","shape":[len(sample.levels)],"sample_rate":sample.sample_rate or self.sample_rate,"minimum_frequency":min(sample.frequencies),"maximum_frequency":max(sample.frequencies)},payload
