"""Versioned, atomic application state storage for Python GPUI applications."""

from __future__ import annotations

import json
import os
import re
import tempfile
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Any, Callable, Generic, TypeVar


T = TypeVar("T")


class ValidationSeverity(str, Enum):
    INFO = "info"
    WARNING = "warning"
    ERROR = "error"


@dataclass(frozen=True)
class ValidationResult:
    """Typed validation payload shared by fields, rows, and forms."""

    severity: ValidationSeverity
    code: str
    message: str
    details: Any = None
    fix_action: str | None = None

    @property
    def valid(self) -> bool:
        return self.severity is not ValidationSeverity.ERROR

    def to_spec(self) -> dict[str, Any]:
        return {
            "severity": self.severity.value,
            "code": self.code,
            "message": self.message,
            "details": self.details,
            "fix_action": self.fix_action,
        }


class Binding(Generic[T]):
    """Read/write typed view of a :class:`State` value.

    It is JSON-safe at the declaration boundary: UI builders call ``to_spec``
    and receive the current value, never the mutable state object itself.
    """

    def __init__(self, getter: Callable[[], T], setter: Callable[[T], None] | None = None):
        self._getter = getter
        self._setter = setter

    @property
    def value(self) -> T:
        return self._getter()

    def set(self, value: T) -> None:
        if self._setter is None:
            raise TypeError("computed bindings are read-only")
        self._setter(value)

    def to_spec(self) -> T:
        return self.value


class State(Generic[T]):
    """Application-owned reactive value with explicit revision tracking."""

    def __init__(self, value: T):
        self._value = value
        self._revision = 0
        self._subscribers: list[Callable[[T, int], None]] = []

    @property
    def value(self) -> T:
        return self._value

    @property
    def revision(self) -> int:
        return self._revision

    def set(self, value: T) -> None:
        if value == self._value:
            return
        self._value = value
        self._revision += 1
        for subscriber in tuple(self._subscribers):
            subscriber(value, self._revision)

    def update(self, update: Callable[[T], T]) -> T:
        value = update(self._value)
        self.set(value)
        return value

    def bind(self) -> Binding[T]:
        return Binding(lambda: self.value, self.set)

    def subscribe(self, callback: Callable[[T, int], None]) -> Callable[[], None]:
        self._subscribers.append(callback)

        def unsubscribe() -> None:
            try:
                self._subscribers.remove(callback)
            except ValueError:
                pass

        return unsubscribe


class Computed(Generic[T]):
    """Read-only derived value evaluated when a declaration is serialized."""

    def __init__(self, compute: Callable[[], T]):
        self._compute = compute

    @property
    def value(self) -> T:
        return self._compute()

    def bind(self) -> Binding[T]:
        return Binding(lambda: self.value)

    def to_spec(self) -> T:
        return self.value


class StateError(RuntimeError):
    """A state file is unreadable, incompatible, or could not be migrated."""


Migration = Callable[[Any, int, int], Any]


def application_data_dir(application_id: str) -> Path:
    """Return (and create) the private writable data directory for an app.

    ``GPUI_TOOLKIT_DATA_DIR`` is an explicit deployment/test override. The
    default follows platform conventions and deliberately rejects path-shaped
    application IDs so an app cannot escape its assigned data root.
    """

    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", application_id):
        raise ValueError("application_id must contain only letters, digits, '.', '_' or '-'")

    root = os.environ.get("GPUI_TOOLKIT_DATA_DIR")
    if root:
        base = Path(root)
    elif sys_platform() == "darwin":
        base = Path.home() / "Library" / "Application Support"
    elif os.name == "nt":
        base = Path(os.environ.get("APPDATA", Path.home() / "AppData" / "Roaming"))
    else:
        base = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local" / "share"))

    path = base / application_id
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    return path


def sys_platform() -> str:
    # Kept small and independently replaceable for deterministic tests.
    import sys
    return sys.platform


@dataclass(frozen=True)
class StoredState:
    version: int
    state: Any


class StateStore:
    """An application-owned JSON state file with atomic replacement semantics."""

    def __init__(self, application_id: str, filename: str = "state.json") -> None:
        if not filename or Path(filename).name != filename:
            raise ValueError("filename must be a single file name")
        self.directory = application_data_dir(application_id)
        self.path = self.directory / filename

    def save(self, state: Any, *, version: int) -> None:
        if version < 1:
            raise ValueError("state version must be positive")
        envelope = {"version": version, "state": state}
        try:
            encoded = json.dumps(envelope, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
        except (TypeError, ValueError) as error:
            raise StateError(f"state is not JSON serializable: {error}") from error

        descriptor, temporary_name = tempfile.mkstemp(
            prefix=f".{self.path.name}.", suffix=".tmp", dir=self.directory
        )
        temporary = Path(temporary_name)
        try:
            with os.fdopen(descriptor, "wb") as file:
                file.write(encoded)
                file.flush()
                os.fsync(file.fileno())
            os.chmod(temporary, 0o600)
            os.replace(temporary, self.path)
            self._sync_directory()
        except OSError as error:
            raise StateError(f"could not save state: {error}") from error
        finally:
            try:
                temporary.unlink()
            except FileNotFoundError:
                pass

    def load(self, *, version: int, migrate: Migration | None = None, default: Any = None) -> StoredState:
        if version < 1:
            raise ValueError("state version must be positive")
        if not self.path.exists():
            return StoredState(version, default)
        try:
            envelope = json.loads(self.path.read_text(encoding="utf-8"))
            stored_version = envelope["version"]
            state = envelope["state"]
        except (OSError, UnicodeDecodeError, json.JSONDecodeError, KeyError, TypeError) as error:
            raise StateError(f"could not load state {self.path}: {error}") from error
        if not isinstance(stored_version, int) or stored_version < 1:
            raise StateError("state version must be a positive integer")
        if stored_version == version:
            return StoredState(version, state)
        if migrate is None:
            raise StateError(f"state version {stored_version} is incompatible with {version}")
        try:
            migrated = migrate(state, stored_version, version)
        except Exception as error:
            # The old file remains untouched; callers can present recovery UI.
            raise StateError(f"state migration from {stored_version} to {version} failed") from error
        return StoredState(version, migrated)

    def _sync_directory(self) -> None:
        if os.name == "nt":
            return
        try:
            descriptor = os.open(self.directory, os.O_RDONLY)
        except OSError:
            return
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
