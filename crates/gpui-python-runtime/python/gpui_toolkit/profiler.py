"""Allocation-profiling snapshots, budgets, and capability metadata.

The portable declaration layer has the exact no-counting-allocator semantics
of :mod:`gpui_profiler`: samples are valid but contain zeroes. Applications
can inspect :func:`telemetry` before interpreting a zero as "no allocations".
"""
from __future__ import annotations
from dataclasses import dataclass
from enum import Enum
from typing import TYPE_CHECKING

from .commands import CommandResult, CommandStatus

if TYPE_CHECKING:
    from .app import SessionContext


class AllocatorTelemetryMode(str, Enum):
    """How the connected host obtains allocation counters."""

    ZERO = "zero"
    COUNTING_ALLOCATOR = "counting_allocator"


@dataclass(frozen=True)
class AllocatorTelemetry:
    """Host allocator-instrumentation state advertised to Python.

    ``ZERO`` is the documented portable behavior of the Rust API when its
    ``global-allocator`` feature is absent. It is explicit so a zero sample
    cannot accidentally be treated as a performance measurement.
    """

    mode: AllocatorTelemetryMode
    capability: str = "gpui-profiler.allocation-snapshots"

    @property
    def is_counting(self) -> bool:
        return self.mode is AllocatorTelemetryMode.COUNTING_ALLOCATOR


def telemetry() -> AllocatorTelemetry:
    """Return allocator telemetry supported by this Python runtime.

    The declaration layer does not inspect Python allocations: that would not
    measure host rendering and would diverge from ``AllocSnapshot`` in Rust.
    """

    return AllocatorTelemetry(AllocatorTelemetryMode.ZERO)


def snapshot_from_command(result: CommandResult) -> tuple[AllocatorTelemetry, "AllocSnapshot"]:
    """Decode one host-owned allocation snapshot command result."""
    if result.status is not CommandStatus.SUCCEEDED:
        raise RuntimeError(result.error or f"profiler command {result.status.value}")
    mode = AllocatorTelemetryMode(str(result.data.get("mode", "zero")))
    snapshot = AllocSnapshot(int(result.data.get("bytes", 0)), int(result.data.get("count", 0)))
    return AllocatorTelemetry(mode), snapshot


def request_snapshot(context: "SessionContext", request_id: str) -> None:
    """Request a host allocation sample; portable hosts report explicit zero mode."""
    context.command(request_id, "profiler.snapshot")


@dataclass(frozen=True)
class ProfilerSample:
    """A sequence-numbered allocation sample emitted by a host subscription."""

    subscription_id: str
    sequence: int
    telemetry: AllocatorTelemetry
    snapshot: "AllocSnapshot"


def sample_from_wire(value: object) -> ProfilerSample:
    """Decode a host telemetry frame without treating zero mode as a measurement."""
    wire = value if isinstance(value, dict) else {}
    raw = wire.get("sample")
    sample = raw if isinstance(raw, dict) else {}
    subscription_id = str(wire.get("subscription_id", ""))
    sequence = int(wire.get("sequence", 0))
    if not subscription_id or sequence < 1:
        raise ValueError("profiler sample requires a subscription ID and positive sequence")
    mode = AllocatorTelemetryMode(str(sample.get("mode", "zero")))
    return ProfilerSample(
        subscription_id, sequence, AllocatorTelemetry(mode),
        AllocSnapshot(int(sample.get("bytes", 0)), int(sample.get("count", 0))),
    )


def subscribe(context: "SessionContext", request_id: str, subscription_id: str, interval_ms: int = 1_000) -> None:
    """Start a bounded host telemetry stream (50 ms through 60 s intervals)."""
    if not subscription_id.strip() or not 50 <= interval_ms <= 60_000:
        raise ValueError("subscription ID must be non-empty and interval_ms must be 50..60000")
    context.subscribe_profiler(request_id, subscription_id, interval_ms)


def unsubscribe(context: "SessionContext", request_id: str, subscription_id: str) -> None:
    """Cancel a host telemetry stream by stable subscription ID."""
    if not subscription_id.strip():
        raise ValueError("subscription ID must be non-empty")
    context.unsubscribe_profiler(request_id, subscription_id)

@dataclass(frozen=True)
class AllocSnapshot:
    bytes: int = 0
    count: int = 0
    def __post_init__(self) -> None:
        if self.bytes < 0 or self.count < 0: raise ValueError("allocation counters cannot be negative")
    @classmethod
    def now(cls) -> "AllocSnapshot": return cls()
    @classmethod
    def delta_since(cls, start: "AllocSnapshot") -> "AllocSnapshot":
        now = cls.now()
        return cls(max(0, now.bytes - start.bytes), max(0, now.count - start.count))

@dataclass(frozen=True)
class AllocationBudget:
    operation: str
    max_count: int
    max_bytes: int
    def __post_init__(self) -> None:
        if not self.operation or self.max_count < 0 or self.max_bytes < 0: raise ValueError("invalid allocation budget")
    @classmethod
    def zero(cls, operation: str) -> "AllocationBudget": return cls(operation, 0, 0)
    def contains(self, measured: AllocSnapshot) -> bool:
        return measured.count <= self.max_count and measured.bytes <= self.max_bytes
    def assert_contains(self, measured: AllocSnapshot) -> None:
        if not self.contains(measured):
            raise AssertionError(f"allocation budget '{self.operation}' exceeded: measured {measured.count} calls/{measured.bytes} bytes, allowed {self.max_count} calls/{self.max_bytes} bytes")

class AllocProbe:
    def __init__(self) -> None:
        self._telemetry = telemetry()
        self._baseline = AllocSnapshot.now()

    @property
    def telemetry(self) -> AllocatorTelemetry:
        """The instrumentation mode used for this probe's samples."""

        return self._telemetry

    def reset(self) -> None: self._baseline = AllocSnapshot.now()
    def sample(self, _label: str) -> AllocSnapshot:
        delta = AllocSnapshot.delta_since(self._baseline)
        self._baseline = AllocSnapshot.now()
        return delta
