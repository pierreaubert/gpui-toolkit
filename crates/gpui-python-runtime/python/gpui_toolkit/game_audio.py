"""Optional, nonblocking cue boundary for Python-authored games.

The toolkit deliberately does not load audio or own a mixer. A host such as
``sotf-daw`` may provide a :class:`CueBackend` that already owns preloaded
assets and playback. With no backend this adapter stays silent.
"""
from __future__ import annotations

from collections import OrderedDict
from dataclasses import dataclass
from queue import Empty, Full, Queue
from threading import Event, Lock, Thread
from typing import Protocol, Sequence


class CueBackend(Protocol):
    """Methods expected from an already initialized audio host."""

    def preload(self, cue_ids: Sequence[str]) -> None: ...
    def play(self, cue_id: str, command_id: str) -> None: ...
    def pause(self) -> None: ...
    def resume(self) -> None: ...


@dataclass(frozen=True)
class _CueCommand:
    operation: str
    cue_id: str = ""
    command_id: str = ""
    cue_ids: tuple[str, ...] = ()


class GameCueAdapter:
    """Send cue commands to a backend on a bounded daemon queue.

    Calls made by game reducers never wait for audio I/O. Duplicate play
    commands are ignored by ``command_id``; only a bounded recent-ID window
    is retained. A full queue drops a cue, never a game-state update.
    """

    def __init__(self, backend: CueBackend | None = None, *,
                 max_pending: int = 32, dedupe_window: int = 2048) -> None:
        if max_pending <= 0 or dedupe_window <= 0:
            raise ValueError("audio queue and dedupe window must be positive")
        self._backend = backend
        self._queue: Queue[_CueCommand] = Queue(maxsize=max_pending)
        self._dedupe_window = dedupe_window
        self._seen: OrderedDict[str, None] = OrderedDict()
        self._closed = Event()
        self._dedupe_lock = Lock()
        self._worker: Thread | None = None
        self.dropped_cues = 0
        self.backend_errors = 0
        if backend is not None:
            self._worker = Thread(target=self._run, name="gpui-game-cues", daemon=True)
            self._worker.start()

    @property
    def available(self) -> bool:
        """Whether a backend was supplied; this does not guarantee playback."""
        return self._backend is not None and not self._closed.is_set()

    def preload(self, cue_ids: Sequence[str]) -> bool:
        ids = tuple(_cue_id(cue_id) for cue_id in cue_ids)
        return self._submit(_CueCommand("preload", cue_ids=ids))

    def play(self, cue_id: str, command_id: str) -> bool:
        cue_id = _cue_id(cue_id)
        command_id = str(command_id)
        if not command_id:
            raise ValueError("game cue command_id must not be empty")
        if not self.available:
            return False
        with self._dedupe_lock:
            if command_id in self._seen:
                return False
            self._seen[command_id] = None
            self._seen.move_to_end(command_id)
            while len(self._seen) > self._dedupe_window:
                self._seen.popitem(last=False)
        accepted = self._submit(_CueCommand("play", cue_id, command_id))
        if not accepted:
            with self._dedupe_lock:
                self._seen.pop(command_id, None)
        return accepted

    def pause(self) -> bool:
        return self._submit(_CueCommand("pause"))

    def resume(self) -> bool:
        return self._submit(_CueCommand("resume"))

    def close(self) -> None:
        """Stop accepting cues; the daemon drains pending commands then exits."""
        self._closed.set()

    def _submit(self, command: _CueCommand) -> bool:
        if not self.available:
            return False
        try:
            self._queue.put_nowait(command)
            return True
        except Full:
            self.dropped_cues += 1
            return False

    def _run(self) -> None:
        backend = self._backend
        assert backend is not None
        while not self._closed.is_set() or not self._queue.empty():
            try:
                command = self._queue.get(timeout=0.05)
            except Empty:
                continue
            try:
                if command.operation == "preload":
                    backend.preload(command.cue_ids)
                elif command.operation == "play":
                    backend.play(command.cue_id, command.command_id)
                elif command.operation == "pause":
                    backend.pause()
                elif command.operation == "resume":
                    backend.resume()
            except Exception:
                self.backend_errors += 1
            finally:
                self._queue.task_done()


def _cue_id(value: str) -> str:
    cue_id = str(value).strip()
    if not cue_id or len(cue_id) > 128:
        raise ValueError("game cue IDs must contain 1 to 128 characters")
    return cue_id
