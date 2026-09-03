"""Application declarations for Python-authored GPUI apps."""

from __future__ import annotations

import json
import math
import os
import asyncio
import inspect
import shutil
import secrets
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from threading import Event as ThreadEvent, Lock, Semaphore, Thread
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Mapping, Sequence

from .events import Event, specialize as specialize_event
from .effects import EffectResult
from .commands import CommandResult
from .miniapp import MiniAppConfig
from .resources import MAX_MESH_FRAME_BYTES, MAX_MESH_RESOURCE_BYTES, MeshFrame


PYTHON_APP_IR_SCHEMA_VERSION = 1
PYTHON_APP_SESSION_VERSION = 1
MAX_SESSION_MESSAGE_BYTES = 4 * 1024 * 1024
PYTHON_SESSION_CAPABILITIES = frozenset({
    "events", "patches", "jobs", "effects", "commands", "profiler_telemetry",
    "meshplot", "mesh_binary_frames", "datasets", "array_resources",
    "mesh_frame_ack",
    "px_interactions", "px_static_export", "px_chart_results", "resource_frame_ack",
    "resource_mmap_frames",
})


def _negotiate_capabilities(
    host_capabilities: Sequence[Any], required_capabilities: Sequence[str],
) -> list[str]:
    """Return the common session capabilities or fail before the first snapshot.

    This keeps optional native functionality explicit: an installed Python app
    cannot accidentally render into a host that lacks one of its declared
    requirements.
    """
    host = {str(capability) for capability in host_capabilities}
    required = {str(capability) for capability in required_capabilities}
    if "" in required:
        raise RuntimeError("required session capabilities must not contain an empty value")
    unavailable = sorted(required - host)
    if unavailable:
        raise RuntimeError(
            "GPUI native host does not support required session capabilities: "
            + ", ".join(unavailable)
        )
    return sorted(host & PYTHON_SESSION_CAPABILITIES)


def _spec(value: Any) -> Any:
    if hasattr(value, "to_spec"):
        return value.to_spec()
    return value


@dataclass(frozen=True)
class Section:
    id: str
    label: str
    content: Any

    def to_spec(self) -> dict[str, Any]:
        return {"id": self.id, "label": self.label, "content": _spec(self.content)}


@dataclass
class App:
    # Application instances intentionally remain mutable: Python owns domain
    # state and action handlers may update it off the native render thread.
    title: str = "GPUI Python App"
    sections: Sequence[Section] = field(default_factory=list)
    width: float | None = None
    height: float | None = None
    sidebar_title: str = "Python UI"
    sidebar_subtitle: str = "Python declarations, Rust renderers"
    required_capabilities: Sequence[str] = field(default_factory=tuple)
    resources: Sequence[Any] = field(default_factory=tuple)
    miniapp: MiniAppConfig | None = None

    def to_spec(self) -> dict[str, Any]:
        if not self.sections:
            raise ValueError("App requires at least one section")
        miniapp = None if self.miniapp is None else self.miniapp.to_spec()
        title = self.title if miniapp is None else self.miniapp.title
        width = self.width if miniapp is None else self.miniapp.width
        height = self.height if miniapp is None else self.miniapp.height
        for name, value in (("width", width), ("height", height)):
            if value is not None and (not math.isfinite(float(value)) or float(value) <= 0.0):
                raise ValueError(f"App {name} must be finite and positive when specified")
        spec = {
            "schema_version": PYTHON_APP_IR_SCHEMA_VERSION,
            "title": title,
            "sidebar_title": self.sidebar_title,
            "sidebar_subtitle": self.sidebar_subtitle,
            "sections": [section.to_spec() for section in self.sections],
            "miniapp": miniapp,
        }
        if width is not None:
            spec["width"] = float(width)
        if height is not None:
            spec["height"] = float(height)
        return spec

    def run(self) -> None:
        _validate_python_runtime()
        if os.environ.get("GPUI_TOOLKIT_DUMP_IR") == "1":
            print(json.dumps(self.to_spec()))
            return
        if os.environ.get("GPUI_TOOLKIT_SESSION") == "1":
            self.serve()
            return

        host = _host_binary()
        if host is None:
            raise RuntimeError(
                "No GPUI native host was found. Install the application bundle or set "
                "GPUI_TOOLKIT_HOST to its bundled host executable."
            )
        os.execv(host, [host, str(Path(sys.argv[0]).resolve())])

    def on_action(self, event: "Event", context: "SessionContext") -> Any:
        """Override to handle structured host events.

        The method may return normally or be declared ``async``. It executes in
        the Python session process, never on GPUI's render thread.
        """

    def on_session_ready(self, context: "SessionContext") -> Any:
        """Override to publish application state after capability negotiation.

        The hook runs before the initial snapshot, allowing applications to
        restore host-owned job history or emit a bounded recovery notice
        without delaying the native render thread.
        """
        for resource in self.resources:
            spec = resource.to_spec() if hasattr(resource, "to_spec") else None
            kind = spec.get("kind") if isinstance(spec, Mapping) else None
            if kind == "dataset":
                context.bind_dataset(resource)
            elif kind == "array_data":
                context.bind_array(resource)
            else:
                raise TypeError(
                    "App.resources accepts only Dataset and ArrayData resources"
                )

    def on_session_shutdown(self, context: "SessionContext") -> Any:
        """Override to persist safe state before the host closes the session."""

    def on_effect_result(self, request_id: str, result: EffectResult, context: "SessionContext") -> Any:
        """Override to receive a typed result from a native host effect."""

    def on_command_result(self, request_id: str, result: CommandResult, context: "SessionContext") -> Any:
        """Override to receive a typed result from a native host command."""

    def on_profiler_sample(self, sample: Any, context: "SessionContext") -> Any:
        """Override to receive a bounded-rate native allocation sample."""

    def serve(self) -> None:
        """Run the persistent stdio session used by the native host."""
        initialize = _read_message()
        if initialize.get("type") != "initialize":
            raise RuntimeError("expected initialize message from GPUI host")
        if initialize.get("session_version") != PYTHON_APP_SESSION_VERSION:
            raise RuntimeError(
                f"unsupported python_app_session version "
                f"{initialize.get('session_version')}; supported version is "
                f"{PYTHON_APP_SESSION_VERSION}"
            )
        negotiated_capabilities = _negotiate_capabilities(
            initialize.get("capabilities", []), self.required_capabilities,
        )
        context = SessionContext(capabilities=negotiated_capabilities)
        context.send({
            "type": "ready",
            "session_version": PYTHON_APP_SESSION_VERSION,
            "capabilities": negotiated_capabilities,
        })
        try:
            ready_result = self.on_session_ready(context)
            if inspect.isawaitable(ready_result):
                asyncio.run(ready_result)
        except Exception:
            context.error(None, "session_ready_failed", "Python session initialization failed")
        context.snapshot(self.to_spec())

        # Action handlers run away from this control loop. This preserves input,
        # cancellation, shutdown, and heartbeat responsiveness when Python
        # begins long-running local or remote simulation work.
        with ThreadPoolExecutor(max_workers=4, thread_name_prefix="gpui-action") as executor:
            for message in _messages(context):
                message_type = message.get("type")
                if message_type == "shutdown":
                    try:
                        shutdown_result = self.on_session_shutdown(context)
                        if inspect.isawaitable(shutdown_result):
                            asyncio.run(shutdown_result)
                    except Exception:
                        context.error(None, "session_shutdown_failed", "Python session shutdown failed")
                    executor.shutdown(wait=False, cancel_futures=True)
                    return
                if message_type == "heartbeat":
                    context.send({"type": "heartbeat", "id": message.get("id", "")})
                    continue
                if message_type == "cancel":
                    request_id = str(message.get("request_id", ""))
                    context.cancel_job(request_id)
                    executor.submit(
                        self._handle_action,
                        Event(request_id, 0, request_id, "cancel", "cancel", {}),
                        context,
                    )
                    continue
                if message_type == "effect_result":
                    executor.submit(
                        self._handle_effect_result,
                        str(message.get("request_id", "")),
                        message.get("result"),
                        context,
                    )
                    continue
                if message_type == "command_result":
                    executor.submit(
                        self._handle_command_result,
                        str(message.get("request_id", "")),
                        message.get("result"),
                        context,
                    )
                    continue
                if message_type == "resource_frame_result":
                    try:
                        context._acknowledge_resource_frame(message)
                    except ValueError:
                        context.error(
                            str(message.get("resource_id", "")) or None,
                            "resource_frame_ack_failed",
                            "Native host sent an invalid resource frame acknowledgement",
                        )
                    continue
                if message_type == "mesh_frame_result":
                    try:
                        context._acknowledge_mesh_frame(message)
                    except ValueError:
                        context.error(
                            str(message.get("resource_id", "")) or None,
                            "mesh_frame_ack_failed",
                            "Native host sent an invalid mesh frame acknowledgement",
                        )
                    continue
                if message_type == "profiler_sample":
                    executor.submit(self._handle_profiler_sample, message, context)
                    continue
                if message_type != "event":
                    context.error(message.get("id"), "unsupported_message", f"unsupported message: {message_type}")
                    continue
                event = specialize_event(message)
                executor.submit(self._handle_action, event, context)

    def _handle_action(self, event: "Event", context: "SessionContext") -> None:
        try:
            result = self.on_action(event, context)
            if inspect.isawaitable(result):
                asyncio.run(result)
        except Exception:
            # Exception strings can contain secrets supplied by applications;
            # retain correlation without leaking implementation data to UI logs.
            context.error(event.id, "action_failed", "Python action handler failed")

    def _handle_effect_result(self, request_id: str, result: Any, context: "SessionContext") -> None:
        try:
            outcome = self.on_effect_result(request_id, EffectResult.from_wire(request_id, result), context)
            if inspect.isawaitable(outcome):
                asyncio.run(outcome)
        except Exception:
            context.error(request_id, "effect_result_failed", "Python effect-result handler failed")

    def _handle_command_result(self, request_id: str, result: Any, context: "SessionContext") -> None:
        try:
            outcome = self.on_command_result(request_id, CommandResult.from_wire(request_id, result), context)
            if inspect.isawaitable(outcome):
                asyncio.run(outcome)
        except Exception:
            context.error(request_id, "command_result_failed", "Python command-result handler failed")

    def _handle_profiler_sample(self, wire: Mapping[str, Any], context: "SessionContext") -> None:
        try:
            from .profiler import sample_from_wire
            outcome = self.on_profiler_sample(sample_from_wire(wire), context)
            if inspect.isawaitable(outcome):
                asyncio.run(outcome)
        except Exception:
            context.error(None, "profiler_sample_failed", "Python profiler-sample handler failed")


class ResourceBackpressureError(RuntimeError):
    """Raised before publication would exceed the session's in-flight byte limit."""


@dataclass(frozen=True)
class ResourceFrameAcknowledgement:
    """Host acknowledgement for one dataset or array resource frame."""

    resource_id: str
    generation: int
    sequence: int
    byte_length: int
    complete: bool
    accepted: bool
    error: str | None = None

    @classmethod
    def from_wire(cls, wire: Mapping[str, Any]) -> "ResourceFrameAcknowledgement":
        resource_id = wire.get("resource_id")
        generation = wire.get("generation")
        sequence = wire.get("sequence")
        byte_length = wire.get("byte_length")
        complete = wire.get("complete")
        accepted = wire.get("accepted")
        error = wire.get("error")
        if not isinstance(resource_id, str) or not resource_id:
            raise ValueError("resource frame acknowledgement requires resource_id")
        if not isinstance(generation, int) or isinstance(generation, bool) or generation <= 0:
            raise ValueError("resource frame acknowledgement requires positive generation")
        if not isinstance(sequence, int) or isinstance(sequence, bool) or sequence < 0:
            raise ValueError("resource frame acknowledgement requires non-negative sequence")
        if not isinstance(byte_length, int) or isinstance(byte_length, bool) or byte_length < 0:
            raise ValueError("resource frame acknowledgement requires non-negative byte_length")
        if not isinstance(complete, bool) or not isinstance(accepted, bool):
            raise ValueError("resource frame acknowledgement requires boolean complete and accepted")
        if error is not None and not isinstance(error, str):
            raise ValueError("resource frame acknowledgement error must be a string or None")
        if accepted and error is not None:
            raise ValueError("accepted resource frame acknowledgement cannot contain an error")
        if not accepted and (error is None or not error):
            raise ValueError("rejected resource frame acknowledgement requires an error")
        return cls(
            resource_id=resource_id,
            generation=generation,
            sequence=sequence,
            byte_length=byte_length,
            complete=complete,
            accepted=accepted,
            error=error,
        )


@dataclass(frozen=True)
class MeshFrameAcknowledgement(ResourceFrameAcknowledgement):
    """Host acknowledgement for one binary mesh-resource frame."""


class SessionContext:
    """Structured outbound messages for a persistent application session."""

    def __init__(
        self,
        *,
        max_outstanding_resource_bytes: int = 1 << 30,
        capabilities: Sequence[str] = (),
        resource_directory: str | os.PathLike[str] | None = None,
        resource_token: str | None = None,
    ) -> None:
        if (
            not isinstance(max_outstanding_resource_bytes, int)
            or isinstance(max_outstanding_resource_bytes, bool)
            or max_outstanding_resource_bytes < 0
        ):
            raise ValueError("max_outstanding_resource_bytes must be a non-negative integer")
        self._revision = 0
        self._lock = Lock()
        self._jobs: dict[str, CancellationToken] = {}
        self._job_history: dict[str, dict[str, Any]] = {}
        self._resource_limits: dict[str, Semaphore] = {}
        self._max_outstanding_resource_bytes = max_outstanding_resource_bytes
        self._outstanding_resource_bytes = 0
        self._outstanding_resource_frames: dict[tuple[str, int, int], int] = {}
        self._outstanding_mesh_frames: dict[tuple[str, int, int], int] = {}
        self._outstanding_resource_paths: dict[tuple[str, int, int], Path] = {}
        self._resource_frame_rejections: list[ResourceFrameAcknowledgement] = []
        self._mesh_frame_rejections: list[MeshFrameAcknowledgement] = []
        negotiated = frozenset(str(capability) for capability in capabilities)
        directory_value = resource_directory
        token_value = resource_token
        mmap_negotiated = "resource_mmap_frames" in negotiated
        if mmap_negotiated:
            if directory_value is None:
                directory_value = os.environ.get("GPUI_TOOLKIT_RESOURCE_DIR")
            if token_value is None:
                token_value = os.environ.get("GPUI_TOOLKIT_RESOURCE_TOKEN")
            if directory_value is None or token_value is None:
                raise RuntimeError(
                    "native host negotiated resource_mmap_frames without session credentials"
                )
        if directory_value is not None and not mmap_negotiated:
            raise ValueError("resource_directory requires resource_mmap_frames capability")
        self._resource_directory = (
            None if directory_value is None else Path(directory_value).resolve(strict=True)
        )
        self._resource_token = token_value
        if self._resource_directory is not None:
            if not self._resource_directory.is_dir() or not self._resource_token:
                raise ValueError("mmap resource transport requires a directory and session token")

    @property
    def max_outstanding_resource_bytes(self) -> int:
        return self._max_outstanding_resource_bytes

    @property
    def outstanding_resource_bytes(self) -> int:
        with self._lock:
            return self._outstanding_resource_bytes

    @property
    def resource_frame_rejections(self) -> tuple[ResourceFrameAcknowledgement, ...]:
        with self._lock:
            return tuple(self._resource_frame_rejections)

    @property
    def mesh_frame_rejections(self) -> tuple[MeshFrameAcknowledgement, ...]:
        with self._lock:
            return tuple(self._mesh_frame_rejections)

    def _acknowledge_resource_frame(
        self, wire: Mapping[str, Any]
    ) -> ResourceFrameAcknowledgement:
        acknowledgement = ResourceFrameAcknowledgement.from_wire(wire)
        key = (
            acknowledgement.resource_id,
            acknowledgement.generation,
            acknowledgement.sequence,
        )
        with self._lock:
            expected_length = self._outstanding_resource_frames.get(key)
            if expected_length is None:
                raise ValueError(
                    "resource frame acknowledgement does not match an outstanding frame: "
                    f"{acknowledgement.resource_id}@{acknowledgement.generation}"
                    f"[{acknowledgement.sequence}]"
                )
            if acknowledgement.byte_length != expected_length:
                raise ValueError(
                    "resource frame acknowledgement byte_length mismatch: "
                    f"expected {expected_length}, got {acknowledgement.byte_length}"
                )
            del self._outstanding_resource_frames[key]
            path = self._outstanding_resource_paths.pop(key, None)
            self._outstanding_resource_bytes -= expected_length
            if not acknowledgement.accepted:
                self._resource_frame_rejections.append(acknowledgement)
        if path is not None:
            try:
                path.unlink(missing_ok=True)
            except OSError:
                pass
        return acknowledgement

    def _acknowledge_mesh_frame(
        self, wire: Mapping[str, Any]
    ) -> MeshFrameAcknowledgement:
        acknowledgement = MeshFrameAcknowledgement.from_wire(wire)
        key = (
            acknowledgement.resource_id,
            acknowledgement.generation,
            acknowledgement.sequence,
        )
        with self._lock:
            expected_length = self._outstanding_mesh_frames.get(key)
            if expected_length is None:
                raise ValueError(
                    "mesh frame acknowledgement does not match an outstanding frame: "
                    f"{acknowledgement.resource_id}@{acknowledgement.generation}"
                    f"[{acknowledgement.sequence}]"
                )
            if acknowledgement.byte_length != expected_length:
                raise ValueError(
                    "mesh frame acknowledgement byte_length mismatch: "
                    f"expected {expected_length}, got {acknowledgement.byte_length}"
                )
            del self._outstanding_mesh_frames[key]
            self._outstanding_resource_bytes -= expected_length
            if not acknowledgement.accepted:
                self._mesh_frame_rejections.append(acknowledgement)
        return acknowledgement

    def send(self, message: dict[str, Any]) -> None:
        with self._lock:
            self._write_locked(message)

    @staticmethod
    def _encode_message(message: dict[str, Any]) -> str:
        encoded = json.dumps(message, separators=(",", ":"))
        if len(encoded.encode("utf-8")) > MAX_SESSION_MESSAGE_BYTES:
            raise ValueError(
                f"GPUI session message exceeds {MAX_SESSION_MESSAGE_BYTES} byte limit"
            )
        return encoded

    def _write_locked(self, message: dict[str, Any]) -> None:
        print(self._encode_message(message), flush=True)

    def resource_frame(self, header: Mapping[str, Any], payload: bytes) -> None:
        """Write one atomic JSON-header/raw-payload frame to the native host."""
        message = {"type": "resource_frame", **dict(header), "byte_length": len(payload)}
        encoded = self._encode_message(message).encode("utf-8")
        if len(payload) > 256 * 1024:
            raise ValueError("audio resource frame exceeds 256 KiB")
        with self._lock:
            sys.stdout.flush()
            stream = getattr(sys.stdout, "buffer", None)
            if stream is None:
                raise RuntimeError("binary resource frames require a binary stdout stream")
            stream.write(encoded)
            stream.write(b"\n")
            stream.write(payload)
            stream.write(b"\n")
            stream.flush()

    def mesh_frame(self, header: Mapping[str, Any], payload: bytes) -> None:
        """Write one bounded JSON-header/raw-payload mesh frame to the host."""
        try:
            frame = MeshFrame.decode(header, bytes(payload))
        except (KeyError, TypeError, ValueError) as error:
            raise ValueError(f"invalid mesh frame: {error}") from error
        if len(frame.payload) > MAX_MESH_FRAME_BYTES or len(frame.payload) > MAX_MESH_RESOURCE_BYTES:
            raise ValueError("mesh resource frame exceeds the configured size limit")
        message = {**dict(header), "type": "mesh_frame", "byte_length": len(frame.payload)}
        encoded = self._encode_message(message).encode("utf-8")
        with self._lock:
            key = (frame.resource_id, frame.generation, frame.sequence)
            if key in self._outstanding_mesh_frames:
                raise ValueError(
                    "mesh frame is already outstanding: "
                    f"{frame.resource_id}@{frame.generation}[{frame.sequence}]"
                )
            projected = self._outstanding_resource_bytes + len(frame.payload)
            if projected > self._max_outstanding_resource_bytes:
                raise ResourceBackpressureError(
                    "mesh frame would exceed max_outstanding_resource_bytes: "
                    f"{projected} > {self._max_outstanding_resource_bytes}"
                )
            sys.stdout.flush()
            stream = getattr(sys.stdout, "buffer", None)
            if stream is None:
                raise RuntimeError("binary mesh frames require a binary stdout stream")
            stream.write(encoded)
            stream.write(b"\n")
            stream.write(payload)
            stream.write(b"\n")
            stream.flush()
            self._outstanding_mesh_frames[key] = len(frame.payload)
            self._outstanding_resource_bytes = projected

    def dataset_frame(
        self,
        *,
        resource_id: str,
        generation: int,
        sequence: int,
        chunk_count: int,
        schema_fingerprint: str,
        payload: bytes,
    ) -> None:
        """Write one bounded Arrow-IPC dataset payload outside JSON UI IR."""
        if not resource_id or generation <= 0 or chunk_count <= 0 or not 0 <= sequence < chunk_count:
            raise ValueError("invalid dataset frame metadata")
        if len(payload) > 16 * 1024 * 1024:
            raise ValueError("dataset frame exceeds 16 MiB")
        checksum = 0xCBF29CE484222325
        for byte in payload:
            checksum = ((checksum ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
        message = {
            "type": "dataset_frame", "resource_id": resource_id, "generation": generation,
            "sequence": sequence, "chunk_count": chunk_count,
            "byte_length": len(payload), "schema_fingerprint": schema_fingerprint,
            "checksum": checksum,
        }
        encoded = self._encode_message(message).encode("utf-8")
        with self._lock:
            key = (resource_id, generation, sequence)
            if key in self._outstanding_resource_frames:
                raise ValueError(
                    "resource frame is already outstanding: "
                    f"{resource_id}@{generation}[{sequence}]"
                )
            projected = self._outstanding_resource_bytes + len(payload)
            if projected > self._max_outstanding_resource_bytes:
                raise ResourceBackpressureError(
                    "resource frame would exceed max_outstanding_resource_bytes: "
                    f"{projected} > {self._max_outstanding_resource_bytes}"
                )
            sys.stdout.flush()
            stream = getattr(sys.stdout, "buffer", None)
            if stream is None:
                raise RuntimeError("binary dataset frames require binary stdout stream")
            stream.write(encoded)
            stream.write(b"\n")
            stream.write(payload)
            stream.write(b"\n")
            stream.flush()
            self._outstanding_resource_frames[key] = len(payload)
            self._outstanding_resource_bytes = projected

    def mapped_dataset_frame(
        self,
        *,
        resource_id: str,
        generation: int,
        schema_fingerprint: str,
        payloads: Sequence[bytes],
    ) -> None:
        """Publish one complete generation through a session-owned mmap file."""
        if self._resource_directory is None or self._resource_token is None:
            raise RuntimeError("resource_mmap_frames was not negotiated")
        if not resource_id or generation <= 0 or not schema_fingerprint:
            raise ValueError("invalid mapped dataset frame metadata")
        chunks = tuple(memoryview(payload).cast("B") for payload in payloads)
        byte_length = sum(len(payload) for payload in chunks)
        if byte_length <= 0 or byte_length > 1 << 30:
            raise ValueError("mapped dataset resource must contain between 1 byte and 1 GiB")
        checksum = 0xCBF29CE484222325
        for payload in chunks:
            for byte in payload:
                checksum = ((checksum ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
        key = (resource_id, generation, 0)
        filename = f"resource-{secrets.token_hex(24)}.bin"
        path = self._resource_directory / filename
        message = {
            "type": "mapped_dataset_frame",
            "resource_id": resource_id,
            "generation": generation,
            "sequence": 0,
            "chunk_count": 1,
            "byte_length": byte_length,
            "schema_fingerprint": schema_fingerprint,
            "checksum": checksum,
            "filename": filename,
            "session_token": self._resource_token,
        }
        with self._lock:
            if key in self._outstanding_resource_frames:
                raise ValueError(
                    "resource frame is already outstanding: "
                    f"{resource_id}@{generation}[0]"
                )
            projected = self._outstanding_resource_bytes + byte_length
            if projected > self._max_outstanding_resource_bytes:
                raise ResourceBackpressureError(
                    "resource frame would exceed max_outstanding_resource_bytes: "
                    f"{projected} > {self._max_outstanding_resource_bytes}"
                )
            descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            try:
                with os.fdopen(descriptor, "wb") as stream:
                    for payload in chunks:
                        stream.write(payload)
                    stream.flush()
                    os.fsync(stream.fileno())
                self._write_locked(message)
            except BaseException:
                try:
                    path.unlink(missing_ok=True)
                except OSError:
                    pass
                raise
            self._outstanding_resource_frames[key] = byte_length
            self._outstanding_resource_paths[key] = path
            self._outstanding_resource_bytes = projected

    def drop_resource(self, resource_id: str, generation: int) -> None:
        self.send({"type": "drop_resource", "resource_id": resource_id, "generation": generation})

    def publish_resource(self, resource: Any) -> None:
        """Publish only a v2 resource descriptor, never its values, to the host.

        The negotiated data transport will transfer a corresponding Arrow IPC,
        shared-memory, or binary payload separately. This small message lets the
        host reject stale generations before attaching that payload.
        """
        if not hasattr(resource, "to_spec"):
            raise TypeError("publish_resource requires a v2 resource with to_spec()")
        spec = resource.to_spec()
        kind = spec.get("kind")
        if kind == "dataset":
            message = {
                "type": "resource_descriptor",
                "schema_version": 2,
                "resource_id": spec["id"],
                "generation": spec["generation"],
                "resource_kind": "dataset",
                "schema_fingerprint": spec["schema_fingerprint"],
                "schema": spec["schema"],
                "column_types": spec["column_types"],
            }
        elif kind == "array_data":
            message = {
                "type": "resource_descriptor",
                "schema_version": 2,
                "resource_id": spec["id"],
                "generation": spec["generation"],
                "resource_kind": "array_data",
                "schema_fingerprint": spec["schema_fingerprint"],
                "shape": spec["shape"],
                "dtype": spec["dtype"],
                "byte_length": spec["byte_length"],
            }
        else:
            raise ValueError(f"unsupported v2 resource kind {kind!r}")
        self.send(message)

    def bind_resource(self, resource: Any) -> Callable[[], None]:
        """Publish a resource and republish only its descriptor after commits."""
        if not hasattr(resource, "subscribe"):
            raise TypeError("bind_resource requires a subscribable v2 resource")
        self.publish_resource(resource)
        return resource.subscribe(self.publish_resource)

    def publish_dataset(self, dataset: Any, *, max_frame_bytes: int = 16 * 1024 * 1024) -> None:
        """Publish a Dataset descriptor followed by bounded Arrow IPC frames.

        ``publish_resource`` deliberately remains descriptor-only for negotiated
        shared-memory transports. This explicit method is the binary fallback;
        it never serializes a dataset value into a JSON session message.
        """
        if not hasattr(dataset, "arrow_ipc_chunks"):
            raise TypeError("publish_dataset requires a v2 Dataset")
        self.publish_resource(dataset)
        chunks = dataset.arrow_ipc_chunks(max_bytes=max_frame_bytes)
        if self._resource_directory is not None and chunks:
            self.mapped_dataset_frame(
                resource_id=dataset.id,
                generation=dataset.generation,
                schema_fingerprint=dataset.schema_fingerprint,
                payloads=chunks,
            )
            return
        for sequence, payload in enumerate(chunks):
            self.dataset_frame(
                resource_id=dataset.id,
                generation=dataset.generation,
                sequence=sequence,
                chunk_count=len(chunks),
                schema_fingerprint=dataset.schema_fingerprint,
                payload=payload,
            )

    def bind_dataset(self, dataset: Any, *, max_frame_bytes: int = 16 * 1024 * 1024) -> Callable[[], None]:
        """Publish a Dataset generation and all later commits through Arrow IPC.

        Unlike transport-neutral ``bind_resource()``, this deliberately selects
        the binary fallback. A missing optional Arrow adapter therefore raises
        its typed publication error instead of leaving a live table or chart
        silently bound to a descriptor with no batches.
        """
        if not hasattr(dataset, "subscribe"):
            raise TypeError("bind_dataset requires a subscribable v2 Dataset")

        def publish(updated: Any) -> None:
            self.publish_dataset(updated, max_frame_bytes=max_frame_bytes)

        publish(dataset)
        return dataset.subscribe(publish)

    def publish_array(self, array: Any, *, max_frame_bytes: int = 16 * 1024 * 1024) -> None:
        """Publish an ArrayData descriptor followed by bounded raw-buffer frames."""
        if not hasattr(array, "binary_chunks"):
            raise TypeError("publish_array requires v2 ArrayData")
        self.publish_resource(array)
        chunks = array.binary_chunks(max_bytes=max_frame_bytes)
        if self._resource_directory is not None and chunks and sum(map(len, chunks)) > 0:
            self.mapped_dataset_frame(
                resource_id=array.id,
                generation=array.generation,
                schema_fingerprint=array.schema_fingerprint,
                payloads=chunks,
            )
            return
        for sequence, payload in enumerate(chunks):
            self.dataset_frame(
                resource_id=array.id,
                generation=array.generation,
                sequence=sequence,
                chunk_count=len(chunks),
                schema_fingerprint=array.schema_fingerprint,
                payload=payload,
            )

    def bind_array(self, array: Any, *, max_frame_bytes: int = 16 * 1024 * 1024) -> Callable[[], None]:
        """Publish an ArrayData generation and each later replacement."""
        if not hasattr(array, "subscribe"):
            raise TypeError("bind_array requires subscribable v2 ArrayData")

        def publish(updated: Any) -> None:
            self.publish_array(updated, max_frame_bytes=max_frame_bytes)

        publish(array)
        return array.subscribe(publish)

    def snapshot(self, app_ir: dict[str, Any]) -> None:
        self.send({"type": "snapshot", "app_ir": app_ir})

    def patch(self, ops: Sequence[dict[str, Any]], *, request_id: str | None = None) -> None:
        """Apply a revision-ordered update, optionally correlated to an event.

        Pass ``event.id`` when responding to an input event. If that event is
        superseded before this patch arrives, the host discards the mutation
        while still advancing the session revision.
        """
        with self._lock:
            self._revision += 1
            message = {"type": "patch", "revision": self._revision, "ops": list(ops)}
            if request_id is not None:
                message["request_id"] = request_id
            self._write_locked(message)

    @staticmethod
    def _mesh_patch_generation(generation: int) -> int:
        if generation <= 0:
            raise ValueError("mesh patch generation must be positive")
        return int(generation)

    def replace_mesh_geometry(
        self, plot_id: str, generation: int, geometry: Any, *, request_id: str | None = None,
    ) -> None:
        self.patch([{
            "op": "replace_mesh_geometry", "plot_id": plot_id,
            "generation": self._mesh_patch_generation(generation),
            "geometry": _spec(geometry),
        }], request_id=request_id)

    def replace_mesh_field(
        self, plot_id: str, generation: int, field: Any, *, request_id: str | None = None,
    ) -> None:
        self.patch([{
            "op": "replace_mesh_field", "plot_id": plot_id,
            "generation": self._mesh_patch_generation(generation),
            "field": _spec(field),
        }], request_id=request_id)

    def set_mesh_plot_prop(
        self, plot_id: str, generation: int, property: str, value: Any, *, request_id: str | None = None,
    ) -> None:
        if not property.strip():
            raise ValueError("mesh plot property must not be empty")
        self.patch([{
            "op": "set_mesh_plot_prop", "plot_id": plot_id,
            "generation": self._mesh_patch_generation(generation),
            "property": property, "value": _spec(value),
        }], request_id=request_id)

    def set_mesh_plot_selection(
        self, plot_id: str, generation: int, selection: Any, *, request_id: str | None = None,
    ) -> None:
        self.patch([{
            "op": "set_mesh_plot_selection", "plot_id": plot_id,
            "generation": self._mesh_patch_generation(generation),
            "selection": _spec(selection),
        }], request_id=request_id)

    def clear_mesh_plot_selection(
        self, plot_id: str, generation: int, *, request_id: str | None = None,
    ) -> None:
        self.patch([{
            "op": "clear_mesh_plot_selection", "plot_id": plot_id,
            "generation": self._mesh_patch_generation(generation),
        }], request_id=request_id)

    def set_mesh_plot_camera(
        self, plot_id: str, generation: int, camera: Any, *, request_id: str | None = None,
    ) -> None:
        self.patch([{
            "op": "set_mesh_plot_camera", "plot_id": plot_id,
            "generation": self._mesh_patch_generation(generation),
            "camera": _spec(camera),
        }], request_id=request_id)

    def reset_mesh_plot_camera(
        self, plot_id: str, generation: int, *, request_id: str | None = None,
    ) -> None:
        self.patch([{
            "op": "reset_mesh_plot_camera", "plot_id": plot_id,
            "generation": self._mesh_patch_generation(generation),
        }], request_id=request_id)

    def set_mesh_plot_viewport(
        self, plot_id: str, generation: int, viewport: Any, *, request_id: str | None = None,
    ) -> None:
        self.patch([{
            "op": "set_mesh_plot_viewport", "plot_id": plot_id,
            "generation": self._mesh_patch_generation(generation),
            "viewport": _spec(viewport),
        }], request_id=request_id)

    def reset_mesh_plot_viewport(
        self, plot_id: str, generation: int, *, request_id: str | None = None,
    ) -> None:
        self.patch([{
            "op": "reset_mesh_plot_viewport", "plot_id": plot_id,
            "generation": self._mesh_patch_generation(generation),
        }], request_id=request_id)

    def job(self, id: str, state: str, **values: Any) -> None:
        if not id.strip():
            raise ValueError("job id must not be empty")
        record = {"id": id, "state": state, **values}
        with self._lock:
            self._job_history[id] = record.copy()
        self.send({"type": "job", **record})

    def job_history(self) -> list[dict[str, Any]]:
        """Return JSON-serializable job state for application persistence.

        Persist this through :class:`StateStore`; `restore_job_history` can
        then show terminal jobs and externally managed remote jobs after a
        restart without claiming they were newly executed locally.
        """
        with self._lock:
            return [record.copy() for _, record in sorted(self._job_history.items())]

    def restore_job_history(self, records: Sequence[Mapping[str, Any]]) -> None:
        """Re-publish previously saved structured job state to the host."""
        for record in records:
            job_id = str(record.get("id", ""))
            state = str(record.get("state", ""))
            if not job_id or state not in {
                "queued", "running", "cancelling", "cancelled", "succeeded", "failed",
                "reconnecting", "interrupted", "abandoned",
            }:
                raise ValueError("invalid saved job history record")
            values = {key: value for key, value in record.items() if key not in {"id", "state"}}
            self.job(job_id, state, **values)

    def job_log(self, id: str, message: str, severity: str = "info") -> None:
        self.send({
            "type": "job_log",
            "id": id,
            "line": {"severity": severity, "message": message},
        })

    def set_resource_limit(self, tag: str, limit: int) -> None:
        """Limit concurrent jobs using a named application resource.

        Resource tags let an application serialize e.g. GPU work while keeping
        unrelated IO work concurrent. Configure limits before starting jobs.
        """
        if not tag.strip():
            raise ValueError("resource tag must not be empty")
        if limit < 1:
            raise ValueError("resource limit must be at least one")
        with self._lock:
            if tag in self._resource_limits:
                raise ValueError(f"resource limit for {tag!r} is already configured")
            self._resource_limits[tag] = Semaphore(limit)

    def spawn_job(
        self,
        id: str,
        target: Callable[["CancellationToken"], Any],
        *,
        resource_tags: Sequence[str] = (),
    ) -> "CancellationToken":
        """Run a cooperative background task and stream its terminal state.

        Jobs wait in ``queued`` while acquiring configured resource tags.
        Acquiring tags in sorted order prevents deadlocks between multi-resource
        tasks; unconfigured tags impose no limit.
        """
        if not id.strip():
            raise ValueError("job id must not be empty")
        token = CancellationToken()
        with self._lock:
            if id in self._jobs:
                raise ValueError(f"job {id!r} already exists")
            self._jobs[id] = token
        self.job(id, "queued")

        tags = tuple(sorted(set(resource_tags)))
        if any(not tag.strip() for tag in tags):
            raise ValueError("resource tags must not be empty")

        def run() -> None:
            acquired: list[Semaphore] = []
            try:
                if token.cancelled:
                    self.job(id, "cancelled")
                    return
                for tag in tags:
                    with self._lock:
                        semaphore = self._resource_limits.get(tag)
                    if semaphore is None:
                        continue
                    while not token.cancelled:
                        if semaphore.acquire(timeout=0.05):
                            acquired.append(semaphore)
                            break
                    if token.cancelled:
                        self.job(id, "cancelled")
                        return
                self.job(id, "running")
                target(token)
            except Exception:
                self.job(id, "failed", message="Job failed")
            else:
                self.job(id, "cancelled" if token.cancelled else "succeeded")
            finally:
                for semaphore in reversed(acquired):
                    semaphore.release()
                with self._lock:
                    self._jobs.pop(id, None)

        Thread(target=run, name=f"gpui-job-{id}", daemon=True).start()
        return token

    def cancel_job(self, id: str) -> bool:
        """Request cooperative cancellation for a task created by ``spawn_job``."""
        with self._lock:
            token = self._jobs.get(id)
        if token is None:
            return False
        token.cancel()
        self.job(id, "cancelling", message="Cancellation requested")
        return True

    def spawn_process_job(
        self,
        id: str,
        command: Sequence[str],
        *,
        cwd: str | os.PathLike[str] | None = None,
        env: Mapping[str, str] | None = None,
        resource_tags: Sequence[str] = (),
    ) -> "CancellationToken":
        """Run a child process without a shell and stream its output.

        ``command`` is always an argument vector, never a shell fragment.
        Stdout and stderr are drained concurrently so a verbose solver cannot
        block on either pipe. Cancellation terminates the child, then kills it
        if it does not exit promptly.
        """
        args = [str(argument) for argument in command]
        if not args or not args[0].strip():
            raise ValueError("process command requires a program name")
        if any("\x00" in argument for argument in args):
            raise ValueError("process command arguments must not contain NUL")
        process_env = None if env is None else {str(key): str(value) for key, value in env.items()}

        def target(token: CancellationToken) -> None:
            process = subprocess.Popen(
                args,
                cwd=None if cwd is None else os.fspath(cwd),
                env=process_env,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                encoding="utf-8",
                errors="replace",
                bufsize=1,
            )

            def drain(stream: Any, severity: str) -> None:
                if stream is None:
                    return
                for line in stream:
                    self.job_log(id, line.rstrip("\r\n"), severity)

            readers = [
                Thread(target=drain, args=(process.stdout, "info"), daemon=True),
                Thread(target=drain, args=(process.stderr, "error"), daemon=True),
            ]
            for reader in readers:
                reader.start()
            while process.poll() is None:
                if token.cancelled:
                    process.terminate()
                    try:
                        process.wait(timeout=1)
                    except subprocess.TimeoutExpired:
                        process.kill()
                    break
                token._cancelled.wait(0.05)
            return_code = process.wait()
            for reader in readers:
                reader.join(timeout=1)
            if process.stdout is not None:
                process.stdout.close()
            if process.stderr is not None:
                process.stderr.close()
            if not token.cancelled and return_code != 0:
                raise RuntimeError(f"process exited with status {return_code}")

        return self.spawn_job(id, target, resource_tags=resource_tags)

    def effect(self, request_id: str, effect: str, **arguments: Any) -> None:
        self.send({"type": "effect", "request_id": request_id, "effect": effect, "arguments": arguments})

    def command(self, request_id: str, command: str, **arguments: Any) -> None:
        """Request a typed host command using only JSON-safe arguments."""
        if not request_id.strip() or not command.strip():
            raise ValueError("commands require a request ID and command name")
        self.send({"type": "command", "request_id": request_id, "command": command, "arguments": arguments})

    def subscribe_profiler(self, request_id: str, subscription_id: str, interval_ms: int = 1_000) -> None:
        """Ask the host for a bounded allocation-telemetry subscription."""
        self.command(request_id, "profiler.subscribe", subscription_id=subscription_id, interval_ms=interval_ms)

    def unsubscribe_profiler(self, request_id: str, subscription_id: str) -> None:
        """Cancel a previously requested allocation-telemetry subscription."""
        self.command(request_id, "profiler.unsubscribe", subscription_id=subscription_id)

    def credential_store(
        self,
        request_id: str,
        reference: str,
        *,
        secret: str | None = None,
        delete: bool = False,
    ) -> None:
        """Store or delete an opaque platform credential reference.

        On supported platforms the native host writes the secret directly to
        the credential store and returns only ``credential_ref`` in its typed
        effect result. The secret is never rendered or logged by the host.
        """
        if not reference.strip():
            raise ValueError("credential reference must not be empty")
        if delete:
            if secret is not None:
                raise ValueError("credential deletion must not include a secret")
            self.effect(request_id, "credential_store", operation="delete", reference=reference)
        else:
            if secret is None:
                raise ValueError("credential storage requires a secret")
            self.effect(request_id, "credential_store", operation="store", reference=reference, secret=secret)

    def acknowledge(self, event: Event | str) -> None:
        """Confirm that an event was accepted by the application."""
        self.send({"type": "acknowledged", "request_id": event.id if isinstance(event, Event) else event})

    def reject(self, event: Event | str, code: str, message: str) -> None:
        """Reject an event with a user-readable validation error."""
        self.send({
            "type": "rejected",
            "request_id": event.id if isinstance(event, Event) else event,
            "code": code,
            "message": message,
        })

    def supersede(self, event: Event | str, superseded_by: Event | str) -> None:
        """Mark a stale high-frequency event as replaced by a newer event."""
        self.send({
            "type": "superseded",
            "request_id": event.id if isinstance(event, Event) else event,
            "superseded_by": (
                superseded_by.id if isinstance(superseded_by, Event) else superseded_by
            ),
        })

    def error(self, request_id: str | None, code: str, message: str) -> None:
        self.send({"type": "error", "request_id": request_id, "code": code, "message": message})


class CancellationToken:
    """Cooperative cancellation state supplied to a background job."""

    def __init__(self) -> None:
        self._cancelled = ThreadEvent()

    @property
    def cancelled(self) -> bool:
        return self._cancelled.is_set()

    def cancel(self) -> None:
        self._cancelled.set()


def section(id: str, label: str, content: Any) -> Section:
    return Section(id=id, label=label, content=content)


def _host_binary() -> str | None:
    configured = os.environ.get("GPUI_TOOLKIT_HOST")
    if configured:
        path = Path(configured)
        if not path.is_file():
            raise RuntimeError(
                f"GPUI_TOOLKIT_HOST does not point to a native host executable: {path}"
            )
        if not os.access(path, os.X_OK):
            raise RuntimeError(f"GPUI_TOOLKIT_HOST is not executable: {path}")
        return str(path)
    package_bin = Path(__file__).resolve().parent / "bin"
    executable_name = "gpui-python-host.exe" if os.name == "nt" else "gpui-python-host"
    bundled = package_bin / executable_name
    if bundled.is_file():
        if not os.access(bundled, os.X_OK):
            raise RuntimeError(f"bundled GPUI native host is not executable: {bundled}")
        return str(bundled)
    bundled = Path(sys.executable).resolve().parent / "gpui-python-host"
    if bundled.is_file():
        return str(bundled)
    return shutil.which("gpui-python-host") or shutil.which("gpui-python-showcase")


def _validate_python_runtime() -> None:
    """Fail at the Python/native boundary before a child process is launched."""
    if sys.version_info < (3, 10):
        raise RuntimeError(
            "gpui-toolkit requires Python 3.10 or newer; install a supported "
            "Python interpreter before launching the native host."
        )


def _read_message() -> dict[str, Any]:
    line = sys.stdin.readline()
    if not line:
        raise RuntimeError("GPUI host closed the session")
    value = json.loads(line)
    if not isinstance(value, dict):
        raise RuntimeError("GPUI session message must be an object")
    return value


def _messages(context: SessionContext | None = None) -> Any:
    for line in sys.stdin:
        if line.strip():
            try:
                value = json.loads(line)
            except (json.JSONDecodeError, UnicodeDecodeError):
                if context is not None:
                    # Do not echo malformed input back to the host: a broken
                    # or hostile message may contain secrets or unbounded text.
                    context.error(None, "malformed_message", "GPUI session message is not valid JSON")
                continue
            if isinstance(value, dict):
                yield value
            elif context is not None:
                context.error(None, "malformed_message", "GPUI session message must be an object")
