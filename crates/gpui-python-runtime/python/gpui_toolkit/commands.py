"""Typed outcomes for host-owned batch commands."""
from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from types import MappingProxyType
from typing import Any, Mapping


class CommandStatus(str, Enum):
    SUCCEEDED = "succeeded"
    UNSUPPORTED = "unsupported"
    FAILED = "failed"


@dataclass(frozen=True)
class CommandResult:
    """An immutable, correlated response from a native toolkit command."""

    request_id: str
    status: CommandStatus
    data: Mapping[str, Any] = field(default_factory=dict)
    error: str | None = None

    @property
    def ok(self) -> bool:
        """Whether the native command completed successfully.

        Effects have exposed this convenience predicate since the first typed
        host API. Keeping commands symmetrical lets application callbacks use
        one success check while retaining the richer ``CommandStatus`` enum.
        """
        return self.status is CommandStatus.SUCCEEDED

    @classmethod
    def from_wire(cls, request_id: str, value: Any) -> "CommandResult":
        payload = value if isinstance(value, dict) else {}
        if payload.get("ok") is True:
            status = CommandStatus.SUCCEEDED
        elif payload.get("unsupported"):
            status = CommandStatus.UNSUPPORTED
        else:
            status = CommandStatus.FAILED
        data = {key: item for key, item in payload.items() if key not in {"ok", "unsupported", "error"}}
        error = payload.get("error")
        return cls(request_id, status, MappingProxyType(data), None if error is None else str(error))
