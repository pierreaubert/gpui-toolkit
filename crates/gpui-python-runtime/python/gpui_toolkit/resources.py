"""Typed retained-resource handles with bounded, observable local storage.

The handle model is transport-neutral: the session may later choose inline
frames, files, shared memory, or a native binary channel without changing the
declarations owned by applications.
"""

from __future__ import annotations

import hashlib
import json
import math
import struct
from collections import OrderedDict
from dataclasses import dataclass
from enum import Enum
from typing import Any, Iterator, Mapping, Sequence


MAX_MESH_FRAME_BYTES = 16 * 1024 * 1024
MAX_MESH_RESOURCE_BYTES = 1 << 30
MAX_MESH_CHUNKS = 4096


class ResourceKind(str, Enum):
    ARRAY = "array"
    IMAGE = "image"
    MESH = "mesh"
    TABLE_PAGE = "table_page"
    CHART_SERIES = "chart_series"
    AUDIO_FRAME = "audio_frame"


class MeshFrameKind(str, Enum):
    GEOMETRY = "geometry"
    FIELD = "field"
    MASK = "mask"
    IDS = "ids"


class MeshDtype(str, Enum):
    F32LE = "f32le"
    F64LE = "f64le"
    U32LE = "u32le"
    U64LE = "u64le"
    BOOL_PACKED = "bool_packed"
    BOOL_BYTES = "bool_bytes"


class ResourceError(RuntimeError):
    pass


class StaleResourceError(ResourceError):
    pass


@dataclass(frozen=True)
class Resource:
    id: str
    generation: int
    kind: ResourceKind
    byte_length: int
    checksum: str
    shape: tuple[int, ...] = ()
    dtype: str = "u8"
    byte_order: str = "little"
    finite_policy: str = "allow"

    def to_spec(self) -> dict[str, object]:
        return {
            "id": self.id,
            "generation": self.generation,
            "kind": self.kind.value,
            "byte_length": self.byte_length,
            "checksum": self.checksum,
            "shape": list(self.shape),
            "dtype": self.dtype,
            "byte_order": self.byte_order,
            "finite_policy": self.finite_policy,
        }


@dataclass(frozen=True)
class ResourceStats:
    entries: int
    bytes_used: int
    byte_budget: int
    evictions: int
    referenced_entries: int = 0
    references: int = 0


@dataclass(frozen=True)
class MeshFrame:
    resource_id: str
    generation: int
    sequence: int
    chunk_count: int
    kind: MeshFrameKind
    dtype: MeshDtype
    shape: tuple[int, ...]
    payload: bytes

    def header(self) -> dict[str, object]:
        return {
            "type": "mesh_frame",
            "resource_id": self.resource_id,
            "generation": self.generation,
            "sequence": self.sequence,
            "chunk_count": self.chunk_count,
            "kind": self.kind.value,
            "dtype": self.dtype.value,
            "shape": list(self.shape),
            "byte_length": len(self.payload),
        }

    def expected_bytes(self) -> int:
        return _expected_bytes(math.prod(self.shape), self.dtype)

    def validate(self) -> None:
        if not self.resource_id.strip() or len(self.resource_id) > 128:
            raise ValueError("mesh frame resource_id must be non-empty and at most 128 characters")
        if not isinstance(self.generation, int) or isinstance(self.generation, bool) or self.generation <= 0:
            raise ValueError("mesh frame generation must be positive")
        if not 0 < self.chunk_count <= MAX_MESH_CHUNKS:
            raise ValueError("mesh frame chunk_count is out of range")
        if not 0 <= self.sequence < self.chunk_count:
            raise ValueError("mesh frame sequence is outside chunk_count")
        if not self.shape or any(
            not isinstance(dimension, int) or isinstance(dimension, bool) or dimension <= 0
            for dimension in self.shape
        ):
            raise ValueError("mesh frame shape must contain positive dimensions")
        expected = self.expected_bytes()
        if expected > MAX_MESH_RESOURCE_BYTES:
            raise ValueError("mesh resource exceeds 1 GiB")
        if not self.payload or len(self.payload) > MAX_MESH_FRAME_BYTES:
            raise ValueError("mesh frame payload is empty or exceeds 16 MiB")
        if self.chunk_count == 1 and len(self.payload) != expected:
            raise ValueError(f"mesh frame payload has {len(self.payload)} bytes; expected {expected}")
        if len(self.payload) > expected:
            raise ValueError(f"mesh frame payload has {len(self.payload)} bytes; expected at most {expected}")

    def encode(self) -> bytes:
        self.validate()
        header = json.dumps(self.header(), separators=(",", ":")).encode("utf-8")
        return header + b"\n" + bytes(self.payload) + b"\n"

    @classmethod
    def decode(cls, header: str | bytes | Mapping[str, Any], payload: bytes) -> "MeshFrame":
        decoded = dict(header) if isinstance(header, Mapping) else json.loads(header)
        if decoded.get("type", "mesh_frame") != "mesh_frame":
            raise ValueError("unexpected mesh frame type")
        declared = decoded.get("byte_length")
        if declared is not None and int(declared) != len(payload):
            raise ValueError("mesh frame header byte_length does not match payload")
        frame = cls(
            resource_id=str(decoded["resource_id"]),
            generation=int(decoded["generation"]),
            sequence=int(decoded["sequence"]),
            chunk_count=int(decoded["chunk_count"]),
            kind=MeshFrameKind(decoded["kind"]),
            dtype=MeshDtype(decoded["dtype"]),
            shape=tuple(int(value) for value in decoded["shape"]),
            payload=bytes(payload),
        )
        frame.validate()
        return frame


class ResourceStore:
    """Reference-counted, LRU-bounded resource bytes owned by Python."""

    def __init__(self, byte_budget: int = 64 * 1024 * 1024) -> None:
        if byte_budget <= 0:
            raise ValueError("resource byte budget must be positive")
        self._budget = byte_budget
        # Keep retained older generations alive while a consumer transitions
        # to a replacement.  The latest generation is indexed separately so
        # unretained replacements can still discard their predecessor
        # immediately without making stale handles valid again.
        self._entries: OrderedDict[tuple[str, int], tuple[Resource, bytes, int]] = OrderedDict()
        self._generations: dict[str, int] = {}
        self._used = 0
        self._evictions = 0

    def put(
        self,
        id: str,
        data: bytes | bytearray | memoryview,
        *,
        kind: ResourceKind = ResourceKind.ARRAY,
        shape: tuple[int, ...] = (),
        dtype: str = "u8",
        byte_order: str = "little",
        finite_policy: str = "allow",
    ) -> Resource:
        if not id.strip():
            raise ValueError("resource id must not be empty")
        payload = bytes(data)
        if len(payload) > self._budget:
            raise ResourceError("resource exceeds configured byte budget")
        if kind is ResourceKind.MESH and len(payload) > MAX_MESH_RESOURCE_BYTES:
            raise ResourceError("mesh resource exceeds 1 GiB")
        generation = self._generations.get(id, 0) + 1
        self._generations[id] = generation
        old_key = (id, generation - 1)
        old = self._entries.get(old_key)
        resource = Resource(
            id=id,
            generation=generation,
            kind=kind,
            byte_length=len(payload),
            checksum=hashlib.sha256(payload).hexdigest(),
            shape=shape,
            dtype=dtype,
            byte_order=byte_order,
            finite_policy=finite_policy,
        )
        # Preflight the budget before mutating the store.  This keeps a
        # failed replacement from evicting unrelated resources or leaving an
        # over-budget newest generation behind.
        old_reclaimable = old is not None and old[2] == 0
        projected = self._used - (len(old[1]) if old_reclaimable else 0) + len(payload)
        evictable = sum(
            len(entry[1])
            for key, entry in self._entries.items()
            if entry[2] == 0 and key != old_key
        )
        if projected > self._budget and projected - self._budget > evictable:
            raise ResourceError("resource budget exhausted by retained resources")

        if old_reclaimable:
            self._entries.pop(old_key)
            self._used -= len(old[1])

        self._entries[(id, generation)] = (resource, payload, 0)
        self._used += len(payload)
        self._evict()
        if (id, generation) not in self._entries:
            raise ResourceError("resource budget exhausted by retained resources")
        return resource

    def put_typed_array(
        self,
        id: str,
        values: Any,
        *,
        shape: Sequence[int],
        dtype: str | MeshDtype,
        kind: ResourceKind = ResourceKind.MESH,
    ) -> Resource:
        """Pack a typed array into the portable little-endian resource format."""
        normalized = _normalize_dtype(dtype)
        normalized_shape = _normalize_shape(shape, normalized)
        payload = _pack_typed_array(values, normalized_shape, normalized)
        return self.put(
            id,
            payload,
            kind=kind,
            shape=normalized_shape,
            dtype=normalized.value,
            byte_order="little",
        )

    def put_mesh_array(
        self,
        id: str,
        values: Any,
        *,
        shape: Sequence[int],
        dtype: str | MeshDtype,
    ) -> Resource:
        return self.put_typed_array(id, values, shape=shape, dtype=dtype, kind=ResourceKind.MESH)

    def iter_mesh_frames(
        self,
        resource: Resource,
        kind: str | MeshFrameKind,
        *,
        max_frame_bytes: int = MAX_MESH_FRAME_BYTES,
    ) -> Iterator[tuple[dict[str, object], bytes]]:
        """Yield deterministic, sequence-ordered wire chunks for a mesh resource."""
        if resource.kind is not ResourceKind.MESH:
            raise ResourceError("mesh frames require a ResourceKind.MESH resource")
        if resource.byte_order.lower() not in {"little", "le"}:
            raise ResourceError("mesh frames require little-endian resources")
        if not 0 < max_frame_bytes <= MAX_MESH_FRAME_BYTES:
            raise ValueError("mesh frame size must be between 1 and 16 MiB")
        frame_kind = kind if isinstance(kind, MeshFrameKind) else MeshFrameKind(kind)
        dtype = _normalize_dtype(resource.dtype)
        payload = self.read(resource)
        chunk_count = max(1, math.ceil(len(payload) / max_frame_bytes))
        for sequence in range(chunk_count):
            start = sequence * max_frame_bytes
            chunk = payload[start : start + max_frame_bytes]
            frame = MeshFrame(
                resource_id=resource.id,
                generation=resource.generation,
                sequence=sequence,
                chunk_count=chunk_count,
                kind=frame_kind,
                dtype=dtype,
                shape=resource.shape,
                payload=chunk,
            )
            try:
                frame.validate()
            except ValueError as error:
                raise ResourceError(str(error)) from error
            yield frame.header(), chunk

    def mesh_frames(
        self,
        resource: Resource,
        kind: str | MeshFrameKind,
        *,
        max_frame_bytes: int = MAX_MESH_FRAME_BYTES,
    ) -> Iterator[tuple[dict[str, object], bytes]]:
        """Compatibility alias for :meth:`iter_mesh_frames`."""
        return self.iter_mesh_frames(resource, kind, max_frame_bytes=max_frame_bytes)

    def send_mesh_frames(
        self,
        context: Any,
        resource: Resource,
        kind: str | MeshFrameKind,
        *,
        max_frame_bytes: int = MAX_MESH_FRAME_BYTES,
    ) -> None:
        for header, payload in self.iter_mesh_frames(
            resource, kind, max_frame_bytes=max_frame_bytes
        ):
            context.mesh_frame(header, payload)

    def read(self, resource: Resource) -> bytes:
        key = (resource.id, resource.generation)
        entry = self._entries.get(key)
        if entry is None:
            raise StaleResourceError(f"resource {resource.id!r} generation is no longer available")
        self._entries.move_to_end(key)
        return entry[1]

    def retain(self, resource: Resource) -> None:
        entry = self._checked(resource)
        self._entries[(resource.id, resource.generation)] = (entry[0], entry[1], entry[2] + 1)

    def release(self, resource: Resource) -> None:
        entry = self._checked(resource)
        key = (resource.id, resource.generation)
        remaining = max(0, entry[2] - 1)
        if remaining == 0 and self._generations.get(resource.id) != resource.generation:
            self._entries.pop(key)
            self._used -= len(entry[1])
            return
        self._entries[key] = (entry[0], entry[1], remaining)
        self._evict()

    def drop(self, resource: Resource) -> None:
        """Explicitly remove an unretained resource from local storage.

        Generation history is intentionally kept so a later resource with the
        same id cannot make an old handle valid again.
        """
        entry = self._checked(resource)
        if entry[2] > 0:
            raise ResourceError(f"resource {resource.id!r} is still retained")
        self._entries.pop((resource.id, resource.generation))
        self._used -= len(entry[1])

    def clear(self) -> None:
        """Drop all unconditionally retained bytes while preserving generations."""
        self._entries.clear()
        self._used = 0

    def stats(self) -> ResourceStats:
        return ResourceStats(
            len(self._entries),
            self._used,
            self._budget,
            self._evictions,
            sum(entry[2] > 0 for entry in self._entries.values()),
            sum(entry[2] for entry in self._entries.values()),
        )

    def _checked(self, resource: Resource) -> tuple[Resource, bytes, int]:
        entry = self._entries.get((resource.id, resource.generation))
        if entry is None:
            raise StaleResourceError(f"resource {resource.id!r} generation is no longer available")
        return entry

    def _evict(self) -> None:
        while self._used > self._budget:
            candidate = next(
                ((key, entry) for key, entry in self._entries.items() if entry[2] == 0),
                None,
            )
            if candidate is None:
                raise ResourceError("resource budget exhausted by retained resources")
            key, entry = candidate
            self._entries.pop(key)
            self._used -= len(entry[1])
            self._evictions += 1


def _normalize_shape(shape: Sequence[int], dtype: MeshDtype | None = None) -> tuple[int, ...]:
    normalized = tuple(int(dimension) for dimension in shape)
    if not normalized or any(dimension <= 0 for dimension in normalized):
        raise ValueError("typed array shape must contain positive dimensions")
    elements = 1
    for dimension in normalized:
        elements *= dimension
    if dtype is not None and _expected_bytes(elements, dtype) > MAX_MESH_RESOURCE_BYTES:
        raise ResourceError("typed mesh array exceeds 1 GiB")
    return normalized


def _normalize_dtype(dtype: str | MeshDtype) -> MeshDtype:
    value = dtype.value if isinstance(dtype, MeshDtype) else str(dtype).strip().lower()
    aliases = {
        "f32": MeshDtype.F32LE,
        "f64": MeshDtype.F64LE,
        "u32": MeshDtype.U32LE,
        "u64": MeshDtype.U64LE,
        "bool": MeshDtype.BOOL_BYTES,
        "bool_packed": MeshDtype.BOOL_PACKED,
        "bool_bytes": MeshDtype.BOOL_BYTES,
    }
    try:
        return aliases[value] if value in aliases else MeshDtype(value)
    except ValueError as error:
        raise ValueError(f"unsupported mesh dtype: {dtype!r}") from error


def _pack_typed_array(values: Any, shape: tuple[int, ...], dtype: MeshDtype) -> bytes:
    elements = math.prod(shape)
    if isinstance(values, (bytes, bytearray, memoryview)):
        payload = bytes(values)
    else:
        if hasattr(values, "tolist"):
            values = values.tolist()
        flat = list(_flatten(values))
        if len(flat) != elements:
            raise ValueError(f"typed array has {len(flat)} values; expected {elements}")
        if dtype is MeshDtype.BOOL_PACKED:
            packed = bytearray((elements + 7) // 8)
            for index, value in enumerate(flat):
                if bool(value):
                    packed[index // 8] |= 1 << (index % 8)
            payload = bytes(packed)
        elif dtype is MeshDtype.BOOL_BYTES:
            payload = bytes(1 if bool(value) else 0 for value in flat)
        else:
            format_code = {
                MeshDtype.F32LE: "f",
                MeshDtype.F64LE: "d",
                MeshDtype.U32LE: "I",
                MeshDtype.U64LE: "Q",
            }[dtype]
            try:
                payload = struct.pack("<" + format_code * elements, *flat)
            except (OverflowError, struct.error, TypeError, ValueError) as error:
                raise ValueError(f"values cannot be packed as {dtype.value}") from error
    expected = _expected_bytes(elements, dtype)
    if len(payload) != expected:
        raise ValueError(f"typed array has {len(payload)} bytes; expected {expected}")
    return payload


def _flatten(value: Any) -> Iterator[Any]:
    if isinstance(value, (str, bytes, bytearray, memoryview)):
        raise ValueError("typed array values must be numeric or boolean")
    if isinstance(value, Sequence):
        for item in value:
            yield from _flatten(item)
    else:
        yield value


def _expected_bytes(elements: int, dtype: MeshDtype) -> int:
    if dtype in {MeshDtype.F32LE, MeshDtype.U32LE}:
        return elements * 4
    if dtype in {MeshDtype.F64LE, MeshDtype.U64LE}:
        return elements * 8
    if dtype is MeshDtype.BOOL_PACKED:
        return (elements + 7) // 8
    return elements
