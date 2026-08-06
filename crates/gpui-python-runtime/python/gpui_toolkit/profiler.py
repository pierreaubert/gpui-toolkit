"""Allocation-profiling snapshots and budgets.

The native host supplies non-zero samples only when built with its counting
allocator; the portable Python declaration layer deliberately reports zero.
"""
from __future__ import annotations
from dataclasses import dataclass

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
    def __init__(self) -> None: self._baseline = AllocSnapshot.now()
    def reset(self) -> None: self._baseline = AllocSnapshot.now()
    def sample(self, _label: str) -> AllocSnapshot:
        delta = AllocSnapshot.delta_since(self._baseline)
        self._baseline = AllocSnapshot.now()
        return delta
