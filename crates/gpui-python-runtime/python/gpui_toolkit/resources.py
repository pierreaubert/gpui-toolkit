"""Typed retained-resource handles with bounded, observable local storage.

The handle model is transport-neutral: the session may later choose inline
frames, files, shared memory, or a native binary channel without changing the
declarations owned by applications.
"""

from __future__ import annotations

import hashlib
from collections import OrderedDict
from dataclasses import dataclass
from enum import Enum


class ResourceKind(str, Enum):
    ARRAY = "array"
    IMAGE = "image"
    MESH = "mesh"
    TABLE_PAGE = "table_page"
    CHART_SERIES = "chart_series"
    AUDIO_FRAME = "audio_frame"


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


class ResourceStore:
    """Reference-counted, LRU-bounded resource bytes owned by Python."""

    def __init__(self, byte_budget: int = 64 * 1024 * 1024) -> None:
        if byte_budget <= 0:
            raise ValueError("resource byte budget must be positive")
        self._budget = byte_budget
        self._entries: OrderedDict[str, tuple[Resource, bytes, int]] = OrderedDict()
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
        generation = self._generations.get(id, 0) + 1
        self._generations[id] = generation
        old = self._entries.pop(id, None)
        if old is not None:
            self._used -= len(old[1])
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
        self._entries[id] = (resource, payload, 0)
        self._used += len(payload)
        self._evict()
        if id not in self._entries:
            raise ResourceError("resource budget exhausted by retained resources")
        return resource

    def read(self, resource: Resource) -> bytes:
        entry = self._entries.get(resource.id)
        if entry is None or entry[0].generation != resource.generation:
            raise StaleResourceError(f"resource {resource.id!r} generation is no longer available")
        self._entries.move_to_end(resource.id)
        return entry[1]

    def retain(self, resource: Resource) -> None:
        entry = self._checked(resource)
        self._entries[resource.id] = (entry[0], entry[1], entry[2] + 1)

    def release(self, resource: Resource) -> None:
        entry = self._checked(resource)
        self._entries[resource.id] = (entry[0], entry[1], max(0, entry[2] - 1))
        self._evict()

    def stats(self) -> ResourceStats:
        return ResourceStats(len(self._entries), self._used, self._budget, self._evictions)

    def _checked(self, resource: Resource) -> tuple[Resource, bytes, int]:
        entry = self._entries.get(resource.id)
        if entry is None or entry[0].generation != resource.generation:
            raise StaleResourceError(f"resource {resource.id!r} generation is no longer available")
        return entry

    def _evict(self) -> None:
        while self._used > self._budget:
            candidate = next(((id, entry) for id, entry in self._entries.items() if entry[2] == 0), None)
            if candidate is None:
                raise ResourceError("resource budget exhausted by retained resources")
            id, entry = candidate
            self._entries.pop(id)
            self._used -= len(entry[1])
            self._evictions += 1
