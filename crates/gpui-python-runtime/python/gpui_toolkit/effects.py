"""Typed host effects for native overlays, files, clipboard, and URLs."""
from __future__ import annotations
from dataclasses import dataclass, field
from enum import Enum
from types import MappingProxyType
from typing import TYPE_CHECKING, Any, Mapping
if TYPE_CHECKING: from .app import SessionContext


class EffectStatus(str, Enum):
    SUCCEEDED = "succeeded"
    CANCELLED = "cancelled"
    UNSUPPORTED = "unsupported"
    DENIED = "denied"
    FAILED = "failed"


@dataclass(frozen=True)
class EffectResult:
    """A normalized result for a consent-bearing host operation."""

    request_id: str
    status: EffectStatus
    data: Mapping[str, Any] = field(default_factory=dict)
    error: str | None = None

    @property
    def ok(self) -> bool:
        return self.status is EffectStatus.SUCCEEDED

    @classmethod
    def from_wire(cls, request_id: str, value: Any) -> "EffectResult":
        if not isinstance(value, Mapping):
            return cls(request_id, EffectStatus.FAILED, error="host returned a non-object effect result")
        data = {str(key): item for key, item in value.items() if key not in {"ok", "cancelled", "error", "status"}}
        error = value.get("error")
        error_text = error if isinstance(error, str) else None
        explicit = value.get("status")
        if explicit in {status.value for status in EffectStatus}:
            status = EffectStatus(explicit)
        elif value.get("cancelled"):
            status = EffectStatus.CANCELLED
        elif value.get("ok") is True:
            status = EffectStatus.SUCCEEDED
        elif error_text and error_text.startswith("unsupported effect:"):
            status = EffectStatus.UNSUPPORTED
        elif error_text and "denied" in error_text.lower():
            status = EffectStatus.DENIED
        else:
            status = EffectStatus.FAILED
        return cls(request_id, status, MappingProxyType(data), error_text)

@dataclass(frozen=True)
class Notification:
    message: str
    def __post_init__(self) -> None:
        if not self.message.strip(): raise ValueError("notification message cannot be empty")
    def send(self, context: "SessionContext", request_id: str) -> None: context.effect(request_id, "notification", message=self.message)

@dataclass(frozen=True)
class ConfirmDialog:
    title: str
    message: str = ""
    confirm_label: str = "Confirm"
    cancel_label: str = "Cancel"
    def __post_init__(self) -> None:
        if not self.title.strip() or not self.confirm_label.strip() or not self.cancel_label.strip(): raise ValueError("confirmation labels cannot be empty")
    def send(self, context: "SessionContext", request_id: str) -> None:
        context.effect(request_id, "confirm", title=self.title, message=self.message, confirm_label=self.confirm_label, cancel_label=self.cancel_label)

def clipboard_write(context: "SessionContext", request_id: str, text: str) -> None:
    context.effect(request_id, "clipboard_write", text=text)
def clipboard_read(context: "SessionContext", request_id: str) -> None:
    context.effect(request_id, "clipboard_read")
def open_url(context: "SessionContext", request_id: str, url: str) -> None:
    if not url.strip(): raise ValueError("url cannot be empty")
    context.effect(request_id, "open_url", url=url)
def choose_file(context: "SessionContext", request_id: str, *, prompt: str | None = None, filters: tuple[str, ...] = (), multiple: bool = False, initial_directory: str | None = None) -> None:
    context.effect(request_id, "open_file", prompt=prompt, filters=list(filters), multiple=multiple, initial_directory=initial_directory)
def choose_directory(context: "SessionContext", request_id: str, *, prompt: str | None = None, multiple: bool = False, initial_directory: str | None = None) -> None:
    context.effect(request_id, "open_directory", prompt=prompt, multiple=multiple, initial_directory=initial_directory)
def save_file(context: "SessionContext", request_id: str, *, initial_directory: str | None = None, suggested_name: str | None = None) -> None:
    context.effect(request_id, "save_file", initial_directory=initial_directory, suggested_name=suggested_name)
def close_window(context: "SessionContext", request_id: str) -> None:
    context.effect(request_id, "close_window")
