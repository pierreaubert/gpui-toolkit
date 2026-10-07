"""Six classic games in one native GPUI window: Zip, Queens, Sudoku, Tetris, Chess, Othello.

Game logic lives in one module per game under ``games_demo/``; this entry
file loads those modules and provides the application shell.


Run it (opens a native window)::

    python crates/gpui-python-runtime/python/examples/games_demo.py

Rendering model: Python owns game state and sends declarative UI IR plus
Scene2D drawing objects. The native GPUI host paints retained geometry;
Python sends stable-ID diffs after input or bounded elapsed-time steps. No
game loop sleeps in a Python worker thread.
"""

from __future__ import annotations

import json
import math
import os
import random
import sys
import threading
import importlib.util

from dataclasses import dataclass, field
from typing import Any
from pathlib import Path
from gpui_toolkit import App, section, ui
from gpui_toolkit.events import (
    Scene2DActivateInput,
    Scene2DKeyInput,
    Scene2DLifecycleInput,
    Scene2DPointerInput,
)
from gpui_toolkit.game_audio import GameCueAdapter
from gpui_toolkit.miniapp import MiniAppConfig
from gpui_toolkit.scene2d import Scene2D, patch_op

_GAMES_DIR = Path(__file__).resolve().parent / "games_demo"


def _load_game_module(name: str) -> Any:
    """Load ``games_demo/<name>.py`` under a private top-level alias."""
    full_name = f"games_demo_{name}"
    existing = sys.modules.get(full_name)
    if existing is not None:
        return existing
    path = _GAMES_DIR / f"{name}.py"
    spec = importlib.util.spec_from_file_location(full_name, path)
    if spec is None or spec.loader is None:
        raise ImportError(f"cannot load game module {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[full_name] = module
    spec.loader.exec_module(module)
    return module


# Load order matters: every game module imports shared helpers from
# ``games_demo_common``.
_common = _load_game_module("common")
_zip_game = _load_game_module("zip_game")
_queens = _load_game_module("queens")
_sudoku = _load_game_module("sudoku")
_tetris = _load_game_module("tetris")
_chess = _load_game_module("chess")
_othello = _load_game_module("othello")

# Re-export every game name (including legacy private helpers used by tests
# and tools) so this entry module keeps its original attribute surface.
for _module in (_common, _zip_game, _queens, _sudoku, _tetris, _chess, _othello):
    for _name, _value in vars(_module).items():
        if _name.startswith("__"):
            continue
        globals()[_name] = _value
del _module, _name, _value
del _common, _zip_game, _queens, _sudoku, _tetris, _chess, _othello



# ---------------------------------------------------------------------------
# Application shell: sections, ordered events, elapsed-time reducers
# ---------------------------------------------------------------------------

def _parse_cell(node_id: str, prefix: str) -> tuple[int, int]:
    try:
        _, _, row, col = node_id.split("-", 3)
    except ValueError:
        raise ValueError(f"malformed {prefix} cell id {node_id!r}") from None
    if not node_id.startswith(prefix):
        raise ValueError(f"malformed {prefix} cell id {node_id!r}")
    return int(row), int(col)


def _cells_between(first: tuple[int, int], last: tuple[int, int]) -> list[tuple[int, int]]:
    """List orthogonal cells crossed by a straight segment between cell centers."""
    row, column = first
    target_row, target_column = last
    delta_row, delta_column = target_row - row, target_column - column
    length_sq = delta_row * delta_row + delta_column * delta_column
    if length_sq == 0:
        return []

    def line_error(candidate: tuple[int, int]) -> float:
        candidate_row, candidate_column = candidate
        cross = delta_column * (candidate_row - first[0]) - delta_row * (candidate_column - first[1])
        return abs(cross) / math.sqrt(length_sq)

    result: list[tuple[int, int]] = []
    while (row, column) != last:
        row_step = 0 if row == target_row else (1 if target_row > row else -1)
        column_step = 0 if column == target_column else (1 if target_column > column else -1)
        row_candidate = (row + row_step, column) if row_step else None
        column_candidate = (row, column + column_step) if column_step else None
        if row_candidate is None:
            next_cell = column_candidate
        elif column_candidate is None:
            next_cell = row_candidate
        else:
            next_cell = row_candidate if line_error(row_candidate) <= line_error(column_candidate) else column_candidate
        assert next_cell is not None
        row, column = next_cell
        result.append(next_cell)
    return result


def _parse_control(node_id: str, prefix: str) -> str:
    marker = f"{prefix}-btn-"
    if not node_id.startswith(marker):
        raise ValueError(f"malformed {prefix} control id {node_id!r}")
    return node_id[len(marker):]


@dataclass
class GamesApp(App):
    serial_reducer: bool = True
    tick_interval: float | None = None
    palette: str = "dark"
    required_capabilities: tuple[str, ...] = (
        "scene2d", "scene2d_pointer_input", "scene2d_keyboard_input", "scene2d_tick_schedule",
    )
    zip_game: ZipGame = field(default_factory=ZipGame)
    queens_game: QueensGame = field(default_factory=QueensGame)
    sudoku_game: SudokuGame = field(default_factory=SudokuGame)
    tetris_game: TetrisGame = field(default_factory=TetrisGame)
    chess_game: ChessGame = field(default_factory=ChessGame)
    othello_game: OthelloGame = field(default_factory=OthelloGame)
    rng: random.Random = field(default_factory=random.Random)
    lock: Any = field(default_factory=threading.RLock, repr=False)
    scene_cache: dict[str, Scene2D] = field(default_factory=dict, repr=False)
    zip_contacts: dict[int, tuple[int, int]] = field(default_factory=dict, repr=False)
    held_tetris_controls: dict[int, str] = field(default_factory=dict, repr=False)
    held_tetris_keys: set[str] = field(default_factory=set, repr=False)
    held_repeat: dict[str, float] = field(default_factory=dict, repr=False)
    tetris_elapsed: float = 0.0
    tetris_effect_remaining: float = 0.0
    sudoku_elapsed: float = 0.0
    solver_steps: list[tuple[int, int, int]] = field(default_factory=list, repr=False)
    ticks_enabled: bool = False
    scheduled_interval: float | None = None
    cues_paused: bool = False
    cues: GameCueAdapter = field(default_factory=GameCueAdapter, repr=False)

    def on_session_ready(self, context: SessionContext) -> None:
        self.cues.preload(GAME_CUE_IDS)

    def on_session_shutdown(self, context: SessionContext) -> None:
        self.cues.close()

    # -- retained scene and timing helpers ------------------------------

    def _make_scene(self, surface_id: str, revision: int) -> Scene2D:
        if surface_id == "zip-board":
            return self.zip_game.board_node(revision=revision)
        if surface_id == "queens-board":
            return self.queens_game.board_node(revision=revision)
        if surface_id == "sudoku-board":
            return self.sudoku_game.board_node(revision=revision)
        if surface_id == "tetris-board":
            return self.tetris_game.board_node(revision=revision)
        if surface_id == "tetris-preview":
            return self.tetris_game.preview_node(revision=revision)
        if surface_id == "tetris-controls":
            return self.tetris_game.controls_node(revision=revision)
        if surface_id == "chess-board":
            return self.chess_game.board_node(revision=revision)
        if surface_id == "othello-board":
            return self.othello_game.board_node(revision=revision)
        raise ValueError(f"unknown Scene2D surface {surface_id!r}")

    @staticmethod
    def _scene_surfaces(game: str) -> tuple[str, ...]:
        return ("tetris-board", "tetris-preview", "tetris-controls") if game == "tetris" else (f"{game}-board",)

    def _publish(
        self, game: str, ops: list[dict[str, Any]], context: SessionContext,
        *, request_id: str | None = None,
    ) -> None:
        patch_ops: list[dict[str, Any]] = []
        pending_scenes: dict[str, Scene2D] = {}
        for surface_id in self._scene_surfaces(game):
            previous = self.scene_cache.get(surface_id)
            revision = 1 if previous is None else previous.revision + 1
            current = self._make_scene(surface_id, revision)
            if previous is None:
                patch_ops.append({"op": "scene2d_replace", "id": surface_id,
                                  "scene": current.scene_spec()})
            else:
                patch_ops.append(patch_op(previous, current))
            pending_scenes[surface_id] = current
        for op in ops:
            node_id = str(op.get("id", ""))
            if node_id.startswith(("zip-cell-", "queens-cell-", "sudoku-cell-", "tetris-row-",
                                       "chess-cell-", "othello-cell-")):
                continue
            if op.get("op") == "replace" and node_id.endswith("-board"):
                continue
            patch_ops.append(op)
        if patch_ops:
            context.patch(patch_ops, request_id=request_id)
            # Keep the reducer's base revision aligned with the patch the host
            # accepted. A failed serialization/write must not advance the cache.
            self.scene_cache.update(pending_scenes)

    def _select_palette(self, palette: str) -> tuple[list[dict[str, Any]], dict[str, Scene2D]]:
        if palette not in GAME_PALETTES:
            raise ValueError(f"unknown game board palette {palette!r}")
        if palette == self.palette:
            return [], {}
        self.palette = palette
        for game in (self.zip_game, self.queens_game, self.sudoku_game,
                     self.tetris_game, self.chess_game, self.othello_game):
            game.palette = palette
        patch_ops: list[dict[str, Any]] = [
            {"op": "set", "id": "palette-light", "property": "selected",
             "value": palette == "light"},
            {"op": "set", "id": "palette-dark", "property": "selected",
             "value": palette == "dark"},
        ]
        pending_scenes: dict[str, Scene2D] = {}
        for surface_id, previous in self.scene_cache.items():
            current = self._make_scene(surface_id, previous.revision + 1)
            patch_ops.append(patch_op(previous, current))
            pending_scenes[surface_id] = current
        return patch_ops, pending_scenes

    def _sync_tick_schedule(self, context: SessionContext) -> None:
        intervals: list[float] = []
        if self.tetris_game.running and not self.tetris_game.over:
            if self.held_tetris_controls or self.held_tetris_keys:
                intervals.append(0.05)
            else:
                intervals.append(max(0.01, self.tetris_game.tick_interval - self.tetris_elapsed))
        if self.tetris_effect_remaining > 0.0:
            intervals.append(min(0.05, self.tetris_effect_remaining))
        if self.sudoku_game.solving:
            intervals.append(max(0.01, 0.08 - self.sudoku_elapsed))
        wanted = min(intervals) if intervals else None
        if (wanted is None) != (self.scheduled_interval is None) or (
            wanted is not None and self.scheduled_interval is not None
            and abs(wanted - self.scheduled_interval) > 0.005
        ):
            self.scheduled_interval = wanted
            self.ticks_enabled = wanted is not None
            context.set_tick_interval(wanted)

    def _clear_held_input(self) -> None:
        self.zip_contacts.clear()
        self.held_tetris_controls.clear()
        self.held_tetris_keys.clear()
        self.held_repeat.clear()

    def _audio_snapshot(self, game: str) -> tuple[Any, ...]:
        if game == "zip":
            return len(self.zip_game.path), self.zip_game.won
        if game == "queens":
            return len(self.queens_game.queens), self.queens_game.won, self.queens_game.moves
        if game == "sudoku":
            return len(self.sudoku_game.entries), len(self.sudoku_game.mistakes()), self.sudoku_game.won
        if game == "tetris":
            return (self.tetris_game.pieces, self.tetris_game.lines, self.tetris_game.score,
                    self.tetris_game.current, self.tetris_game.running)
        if game == "chess":
            return (len(self.chess_game.match.history),
                    self.chess_game.match.status, self.chess_game.moves_made)
        if game == "othello":
            return (len(self.othello_game.match.history),
                    self.othello_game.match.status, self.othello_game.moves_made)
        return ()

    def _emit_game_cue(self, game: str, before: tuple[Any, ...], command_id: str,
                       *, action_hint: str = "") -> None:
        after = self._audio_snapshot(game)
        if after == before:
            return
        cue = ""
        won_index = {"zip": 1, "queens": 1, "sudoku": 2}.get(game)
        if won_index is not None and after[won_index] and not before[won_index]:
            cue = "win"
        elif game == "tetris":
            if after[1] > before[1]:
                cue = "clear"
            elif after[0] > before[0] and before[3] is not None:
                cue = "lock"
            elif action_hint == "rotate":
                cue = "rotate"
        elif game == "chess":
            if after[1] == "checkmate" and before[1] != "checkmate":
                cue = "win"
            elif after[0] != before[0]:
                cue = "place"
        elif game == "othello":
            if after[1] == "finished" and before[1] != "finished":
                cue = "win"
            elif after[0] != before[0]:
                cue = "place"
        elif game == "sudoku" and after[1] > before[1]:
            cue = "error"
        elif game in {"zip", "queens", "sudoku"} and after[0] != before[0]:
            cue = "place"
        if cue:
            self.cues.play(cue, command_id)

    @staticmethod
    def _surface_game(surface_id: str) -> str:
        return ("zip" if surface_id == "zip-board" else
                "queens" if surface_id == "queens-board" else
                "sudoku" if surface_id == "sudoku-board" else
                "chess" if surface_id == "chess-board" else
                "othello" if surface_id == "othello-board" else
                "tetris" if surface_id in {"tetris-board", "tetris-controls"} else "")

    def _arm_tetris_lock_effect(self, before: tuple[int, int, str | None]) -> None:
        if self.tetris_game.pieces > before[0] and before[2] is not None:
            self.tetris_effect_remaining = 0.32

    @staticmethod
    def _cell_from_pointer(event: Scene2DPointerInput) -> tuple[int, int] | None:
        if event.cell is not None:
            return event.cell.row, event.cell.column
        if event.hit_id is not None:
            for prefix in ("zip-cell", "queens-cell", "sudoku-cell", "tetris-cell",
                           "chess-cell", "othello-cell"):
                if event.hit_id.startswith(prefix + "-"):
                    try:
                        return _parse_cell(event.hit_id, prefix)
                    except ValueError:
                        return None
        return None

    @staticmethod
    def _cell_from_activation(event: Scene2DActivateInput) -> tuple[int, int] | None:
        if event.cell is not None:
            return event.cell.row, event.cell.column
        for node_id in (event.hit_id, event.id):
            if node_id is None:
                continue
            for prefix in ("zip-cell", "queens-cell", "sudoku-cell", "tetris-cell",
                           "chess-cell", "othello-cell"):
                if node_id.startswith(prefix + "-"):
                    try:
                        return _parse_cell(node_id, prefix)
                    except ValueError:
                        return None
        return None

    def _move_selection(self, size: int, current: tuple[int, int],
                        dr: int, dc: int) -> tuple[int, int]:
        return (min(size - 1, max(0, current[0] + dr)),
                min(size - 1, max(0, current[1] + dc)))

    def on_scene2d_event(self, event: Scene2DEvent, context: SessionContext) -> None:
        user_input = event.input
        if isinstance(user_input, Scene2DLifecycleInput):
            with self.lock:
                self._clear_held_input()
                # The host retains the requested cadence while the section is
                # inactive and resets elapsed time when it resumes. Clear our
                # accumulators too, so a section switch never causes catch-up.
                if user_input.reason in {"section_changed", "suspended", "disconnected", "resumed"}:
                    self.tetris_elapsed = 0.0
                    self.sudoku_elapsed = 0.0
                if user_input.reason == "resumed":
                    self.cues.resume()
                    self.cues_paused = False
                self._sync_tick_schedule(context)
            if user_input.reason in {"section_changed", "suspended", "disconnected"}:
                self.cues.pause()
                self.cues_paused = True
            context.acknowledge(event)
            return
        game = ""
        ops: list[dict[str, Any]] = []
        changed = False
        game = self._surface_game(event.surface_id)
        before = self._audio_snapshot(game)
        tetris_before = (self.tetris_game.pieces, self.tetris_game.lines,
                         self.tetris_game.current)
        try:
            with self.lock:
                if isinstance(user_input, Scene2DPointerInput):
                    game, ops, changed = self._handle_pointer(event.surface_id, user_input)
                elif isinstance(user_input, Scene2DKeyInput):
                    game, ops, changed = self._handle_key(event.surface_id, user_input)
                elif isinstance(user_input, Scene2DActivateInput):
                    game, ops, changed = self._handle_activate(event.surface_id, user_input)
                if game == "tetris":
                    self._arm_tetris_lock_effect(tetris_before)
                self._sync_tick_schedule(context)
        except ValueError as error:
            context.reject(event, "invalid_scene2d_input", str(error))
            return
        context.acknowledge(event)
        if changed and game:
            action_hint = ""
            if isinstance(user_input, Scene2DKeyInput):
                action_hint = self._key_action(user_input.key)
            elif isinstance(user_input, Scene2DPointerInput):
                action_hint = {"hold-rotate": "rotate"}.get(user_input.hit_id or "", "")
            elif isinstance(user_input, Scene2DActivateInput):
                action_hint = {"hold-rotate": "rotate"}.get(
                    user_input.hit_id or user_input.id, "")
            self._emit_game_cue(game, before, event.id, action_hint=action_hint)
            self._publish(game, ops, context, request_id=event.id)

    def _handle_activate(
        self, surface_id: str, activation: Scene2DActivateInput,
    ) -> tuple[str, list[dict[str, Any]], bool]:
        cell = self._cell_from_activation(activation)
        if surface_id == "zip-board" and cell is not None:
            ops = self.zip_game.drag_to(*cell)
            return "zip", ops, bool(ops)
        if surface_id == "queens-board" and cell is not None:
            return "queens", self.queens_game.click(*cell), True
        if surface_id == "sudoku-board" and cell is not None:
            return "sudoku", self.sudoku_game.click(*cell), True
        if surface_id == "chess-board" and cell is not None:
            return "chess", self.chess_game.click(*cell, self.rng), True
        if surface_id == "othello-board" and cell is not None:
            return "othello", self.othello_game.click(*cell, self.rng), True
        if surface_id == "tetris-controls":
            target = activation.hit_id or activation.id
            action = {
                "hold-left": "left",
                "hold-right": "right",
                "hold-down": "down",
                "hold-rotate": "rotate",
            }.get(target)
            if action is not None:
                # Accessibility activation is one action, never a held contact.
                return "tetris", self.tetris_game.control(action, self.rng), True
        return self._surface_game(surface_id), [], False

    def _handle_pointer(
        self, surface_id: str, pointer: Scene2DPointerInput,
    ) -> tuple[str, list[dict[str, Any]], bool]:
        if surface_id == "zip-board":
            game = self.zip_game
            cell = self._cell_from_pointer(pointer)
            if pointer.phase == "down":
                if cell is None:
                    return "zip", [], False
                self.zip_contacts[pointer.contact_id] = cell
                return "zip", game.drag_to(*cell), True
            if pointer.phase == "move" and pointer.contact_id in self.zip_contacts:
                previous = self.zip_contacts[pointer.contact_id]
                if cell is None or cell == previous:
                    return "zip", [], False
                ops: list[dict[str, Any]] = []
                for crossed in _cells_between(previous, cell):
                    ops.extend(game.drag_to(*crossed))
                self.zip_contacts[pointer.contact_id] = cell
                return "zip", ops, bool(ops)
            if pointer.phase in {"up", "cancel"}:
                self.zip_contacts.pop(pointer.contact_id, None)
            return "zip", [], False
        if surface_id == "queens-board" and pointer.phase == "up":
            cell = self._cell_from_pointer(pointer)
            if cell is not None:
                return "queens", self.queens_game.click(*cell), True
        if surface_id == "sudoku-board" and pointer.phase == "up":
            cell = self._cell_from_pointer(pointer)
            if cell is not None:
                return "sudoku", self.sudoku_game.click(*cell), True
        if surface_id == "chess-board" and pointer.phase == "up":
            cell = self._cell_from_pointer(pointer)
            if cell is not None:
                return "chess", self.chess_game.click(*cell, self.rng), True
        if surface_id == "othello-board" and pointer.phase == "up":
            cell = self._cell_from_pointer(pointer)
            if cell is not None:
                return "othello", self.othello_game.click(*cell, self.rng), True
        if surface_id == "tetris-controls":
            action = pointer.hit_id or ""
            mapping = {"hold-left": "left", "hold-right": "right",
                       "hold-down": "down", "hold-rotate": "rotate"}
            if pointer.phase == "down" and action in mapping:
                move = mapping[action]
                self.held_tetris_controls[pointer.contact_id] = move
                self.held_repeat[f"touch:{pointer.contact_id}"] = 0.0
                return "tetris", self.tetris_game.control(move, self.rng), True
            if pointer.phase in {"up", "cancel"}:
                self.held_tetris_controls.pop(pointer.contact_id, None)
                self.held_repeat.pop(f"touch:{pointer.contact_id}", None)
        return "", [], False

    @staticmethod
    def _key_action(key: str) -> str:
        return {"ArrowLeft": "left", "ArrowRight": "right", "ArrowDown": "down",
                "ArrowUp": "rotate", "x": "rotate", "Space": "drop", "p": "pause"}.get(key, "")

    def _handle_key(
        self, surface_id: str, key_event: Scene2DKeyInput,
    ) -> tuple[str, list[dict[str, Any]], bool]:
        key = key_event.key
        if surface_id == "zip-board":
            if key_event.phase != "down":
                return "", [], False
            if key == "Backspace":
                return "zip", self.zip_game.control("undo"), True
            direction = {"ArrowLeft": (0, -1), "ArrowRight": (0, 1),
                         "ArrowUp": (-1, 0), "ArrowDown": (1, 0)}.get(key)
            if direction is None:
                return "", [], False
            game = self.zip_game
            ops: list[dict[str, Any]] = []
            if not game.path:
                ops.extend(game.drag_to(*game.level.start))
            if game.head is not None:
                row, column = game.head
                target = (row + direction[0], column + direction[1])
                if 0 <= target[0] < game.level.rows and 0 <= target[1] < game.level.cols:
                    ops.extend(game.drag_to(*target))
            return "zip", ops, bool(ops)
        if surface_id == "queens-board":
            game = self.queens_game
            if key_event.phase != "down":
                return "", [], False
            direction = {"ArrowLeft": (0, -1), "ArrowRight": (0, 1),
                         "ArrowUp": (-1, 0), "ArrowDown": (1, 0)}.get(key)
            if direction is not None:
                game.selected = self._move_selection(game.size, game.selected, *direction)
                return "queens", game.status_ops(), True
            row, column = game.selected
            if key in {"Enter", "Space"}:
                return "queens", game.click(row, column), True
            if key == "m":
                game.marks[row][column] = (QUEENS_MARK if game.marks[row][column] != QUEENS_MARK
                                            else QUEENS_EMPTY)
                game.moves += 1
                return "queens", game._refresh_after_edit(), True
        if surface_id == "sudoku-board":
            game = self.sudoku_game
            if key_event.phase != "down":
                return "", [], False
            if key in {"ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"}:
                current = game.selected or (0, 0)
                direction = {"ArrowLeft": (0, -1), "ArrowRight": (0, 1),
                             "ArrowUp": (-1, 0), "ArrowDown": (1, 0)}[key]
                return "sudoku", game.click(*self._move_selection(9, current, *direction)), True
            if len(key) == 1 and key in "123456789":
                return "sudoku", game.enter_digit(int(key)), True
            if key in {"Backspace", "Delete"}:
                return "sudoku", game.erase(), True
        if surface_id == "chess-board":
            game = self.chess_game
            if key_event.phase != "down":
                return "", [], False
            direction = {"ArrowLeft": (0, -1), "ArrowRight": (0, 1),
                         "ArrowUp": (-1, 0), "ArrowDown": (1, 0)}.get(key)
            if direction is not None:
                game.cursor = self._move_selection(8, game.cursor, *direction)
                return "chess", game.status_ops(), True
            if key in {"Enter", "Space"}:
                return "chess", game.click(*game.cursor, self.rng), True
            if key == "u":
                return "chess", game.control("undo", self.rng), True
            if key == "f":
                return "chess", game.control("flip", self.rng), True
        if surface_id == "othello-board":
            game = self.othello_game
            if key_event.phase != "down":
                return "", [], False
            direction = {"ArrowLeft": (0, -1), "ArrowRight": (0, 1),
                         "ArrowUp": (-1, 0), "ArrowDown": (1, 0)}.get(key)
            if direction is not None:
                game.cursor = self._move_selection(8, game.cursor, *direction)
                return "othello", game.status_ops(), True
            if key in {"Enter", "Space"}:
                return "othello", game.click(*game.cursor, self.rng), True
            if key == "u":
                return "othello", game.control("undo", self.rng), True
            if key == "p":
                return "othello", game.control("pass", self.rng), True
        if surface_id == "tetris-board":
            action = self._key_action(key)
            if not action:
                return "", [], False
            if key_event.phase == "up":
                self.held_tetris_keys.discard(key)
                self.held_repeat.pop(f"key:{key}", None)
                return "", [], False
            if action in {"left", "right", "down", "rotate"}:
                first_press = key not in self.held_tetris_keys
                self.held_tetris_keys.add(key)
                self.held_repeat.setdefault(f"key:{key}", 0.0)
                if not first_press:
                    return "", [], False
            if action == "pause":
                action = "start"
            return "tetris", self.tetris_game.control(action, self.rng), True
        return "", [], False

    def on_tick(self, tick: Scene2DTick, context: SessionContext) -> None:
        elapsed = min(tick.elapsed_seconds, 0.25)
        if elapsed <= 0.0:
            return
        if self.cues_paused:
            self.cues.resume()
            self.cues_paused = False
        tetris_before = self._tetris_fingerprint()
        tetris_audio_before = self._audio_snapshot("tetris")
        tetris_effect_before = (self.tetris_game.pieces, self.tetris_game.lines,
                                self.tetris_game.current)
        sudoku_before = (self.sudoku_game.solving, len(self.sudoku_game.entries), self.sudoku_game.won)
        sudoku_audio_before = self._audio_snapshot("sudoku")
        tetris_ops: list[dict[str, Any]] = []
        sudoku_ops: list[dict[str, Any]] = []
        # The host tags ticks with the stable surface that requested the
        # cadence. Older/test callers without a surface id retain the prior
        # all-active behavior; current hosts route each board independently.
        tick_for_tetris = tick.surface_id in {"", "tetris-board", "tetris-controls", "tetris-preview"}
        tick_for_sudoku = tick.surface_id in {"", "sudoku-board"}
        with self.lock:
            game = self.tetris_game
            if tick_for_tetris and game.running and not game.over:
                self._advance_holds(elapsed, tetris_ops)
                self.tetris_elapsed += elapsed
                steps = 0
                while self.tetris_elapsed >= game.tick_interval and steps < 4 and game.running and not game.over:
                    self.tetris_elapsed -= game.tick_interval
                    stepper = game.auto_step if game.auto else game.tick
                    tetris_ops.extend(stepper(self.rng))
                    steps += 1
            if tick_for_sudoku and self.sudoku_game.solving:
                self.sudoku_elapsed += elapsed
                steps = 0
                while self.sudoku_elapsed >= 0.08 and steps < 4 and self.sudoku_game.solving:
                    self.sudoku_elapsed -= 0.08
                    if self.solver_steps:
                        row, column, digit = self.solver_steps.pop(0)
                        self.sudoku_game.apply_solve_step(row, column, digit)
                        sudoku_ops.extend(self.sudoku_game.status_ops())
                    else:
                        sudoku_ops.extend(self.sudoku_game.finish_solving())
                    steps += 1
            if tick_for_tetris and self.tetris_game.pieces > tetris_effect_before[0] and tetris_effect_before[2] is not None:
                self.tetris_effect_remaining = 0.32
            elif tick_for_tetris and self.tetris_effect_remaining > 0.0:
                self.tetris_effect_remaining = max(0.0, self.tetris_effect_remaining - elapsed)
                if self.tetris_effect_remaining == 0.0:
                    self.tetris_game.last_lock_cells.clear()
                    self.tetris_game.last_clear_rows.clear()
            self._sync_tick_schedule(context)
        if tick_for_tetris and self._tetris_fingerprint() != tetris_before:
            self._emit_game_cue("tetris", tetris_audio_before, f"tick-{tick.frame}")
            self._publish("tetris", tetris_ops or self.tetris_game.status_ops(), context)
        sudoku_after = (self.sudoku_game.solving, len(self.sudoku_game.entries), self.sudoku_game.won)
        if tick_for_sudoku and sudoku_after != sudoku_before:
            self._emit_game_cue("sudoku", sudoku_audio_before, f"tick-{tick.frame}-sudoku")
            self._publish("sudoku", sudoku_ops or self.sudoku_game.status_ops(), context)

    def _tetris_fingerprint(self) -> tuple[Any, ...]:
        game = self.tetris_game
        return (tuple(tuple(row) for row in game.grid), game.current, tuple(game.cells),
                game.piece_row, game.piece_col, game.next_shape, game.score,
                game.lines, game.running, game.over, game.auto,
                tuple(game.last_lock_cells), tuple(game.last_clear_rows))

    def _advance_holds(self, elapsed: float, ops: list[dict[str, Any]]) -> None:
        actions = {self._key_action(key) for key in self.held_tetris_keys}
        actions.update(self.held_tetris_controls.values())
        actions.discard("")
        for action in actions:
            sources = [f"key:{key}" for key in self.held_tetris_keys
                       if self._key_action(key) == action]
            sources.extend(f"touch:{contact}" for contact, held in self.held_tetris_controls.items()
                           if held == action)
            if not sources:
                continue
            timer_key = sources[0]
            previous = self.held_repeat.get(timer_key, 0.0)
            current = previous + elapsed
            if current >= 0.20:
                repeat_count = min(3, int((current - 0.20) / 0.075) + 1)
                for _ in range(repeat_count):
                    ops.extend(self.tetris_game.control(action, self.rng))
                current = 0.20 + ((current - 0.20) % 0.075)
            self.held_repeat[timer_key] = current

    # -- events -----------------------------------------------------------

    def on_session_shutdown(self, context: SessionContext) -> Any:
        with self.lock:
            self._clear_held_input()
            self.tetris_game.running = False
            self.sudoku_game.solving = False
            self.solver_steps.clear()
            self._sync_tick_schedule(context)
        self.cues.close()
        return None

    def on_action(self, event: Event, context: SessionContext) -> None:
        action = event.action or ""
        node_id = event.node_id or ""
        if action == "game_palette":
            palette = {"palette-light": "light", "palette-dark": "dark"}.get(node_id)
            if palette is None:
                context.reject(event, "invalid_action", "unknown game board palette control")
                return
            try:
                with self.lock:
                    patch_ops, pending_scenes = self._select_palette(palette)
                context.acknowledge(event)
                if patch_ops:
                    context.patch(patch_ops, request_id=event.id)
                    self.scene_cache.update(pending_scenes)
            except ValueError as error:
                context.reject(event, "invalid_action", str(error))
            return
        game = ("zip" if action.startswith("zip_") else
                "queens" if action.startswith("queens_") else
                "sudoku" if action.startswith("sudoku_") else
                "chess" if action.startswith("chess_") else
                "othello" if action.startswith("othello_") else
                "tetris" if action.startswith("tetris_") else "")
        before = self._audio_snapshot(game)
        tetris_before = (self.tetris_game.pieces, self.tetris_game.lines,
                         self.tetris_game.current)
        try:
            with self.lock:
                ops = self._dispatch(action, node_id, event, context)
                if game == "tetris":
                    self._arm_tetris_lock_effect(tetris_before)
        except ValueError as error:
            context.reject(event, "invalid_action", str(error))
            return
        context.acknowledge(event)
        if ops:
            if game:
                self._sync_tick_schedule(context)
                action_hint = "rotate" if node_id == "tetris-btn-rotate" else ""
                self._emit_game_cue(game, before, event.id, action_hint=action_hint)
                if game == "tetris" and self.tetris_game.running:
                    self.cues.resume()
                    self.cues_paused = False
                self._publish(game, ops, context, request_id=event.id)
            else:
                context.patch(ops, request_id=event.id)

    def _dispatch(self, action: str, node_id: str, event: Event,
                 context: SessionContext) -> list[dict[str, Any]]:
        if action == ZIP_CELL_ACTION:
            row, col = _parse_cell(node_id, "zip-cell")
            return self.zip_game.click(row, col)
        if action == ZIP_CONTROL_ACTION:
            return self.zip_game.control(_parse_control(node_id, "zip"))
        if action == ZIP_LEVEL_ACTION:
            value = (event.payload or {}).get("value", "")
            return self.zip_game.select_level(str(value))
        if action == QUEENS_CELL_ACTION:
            row, col = _parse_cell(node_id, "queens-cell")
            return self.queens_game.click(row, col)
        if action == QUEENS_CONTROL_ACTION:
            return self.queens_game.control(_parse_control(node_id, "queens"), self.rng)
        if action == QUEENS_SIZE_ACTION:
            value = (event.payload or {}).get("value", "")
            return self.queens_game.select_size(str(value), self.rng)
        if action == SUDOKU_CELL_ACTION:
            row, col = _parse_cell(node_id, "sudoku-cell")
            return self.sudoku_game.click(row, col)
        if action == SUDOKU_PAD_ACTION:
            if node_id == "sudoku-pad-erase":
                return self.sudoku_game.erase()
            if node_id.startswith("sudoku-pad-"):
                return self.sudoku_game.enter_digit(int(node_id[len("sudoku-pad-"):]))
            raise ValueError(f"malformed sudoku pad id {node_id!r}")
        if action == SUDOKU_CONTROL_ACTION:
            name = _parse_control(node_id, "sudoku")
            ops = self.sudoku_game.control(name, self.rng)
            if name == "solve" and self.sudoku_game.solving:
                self.solver_steps = self.sudoku_game.solve_step_cells()
                self.sudoku_elapsed = 0.0
            elif name in {"new", "solve"}:
                self.solver_steps.clear()
                self.sudoku_elapsed = 0.0
            return ops
        if action == SUDOKU_DIFFICULTY_ACTION:
            value = (event.payload or {}).get("value", "")
            self.solver_steps.clear()
            return self.sudoku_game.select_difficulty(str(value), self.rng)
        if action == TETRIS_CONTROL_ACTION:
            name = _parse_control(node_id, "tetris")
            ops = self.tetris_game.control(name, self.rng)
            if not self.tetris_game.running:
                self.tetris_elapsed = 0.0
            return ops
        if action == CHESS_CELL_ACTION:
            row, col = _parse_cell(node_id, "chess-cell")
            return self.chess_game.click(row, col, self.rng)
        if action == CHESS_CONTROL_ACTION:
            return self.chess_game.control(_parse_control(node_id, "chess"), self.rng)
        if action == CHESS_MODE_ACTION:
            value = (event.payload or {}).get("value", "")
            return self.chess_game.select_mode(str(value))
        if action == CHESS_DIFFICULTY_ACTION:
            value = (event.payload or {}).get("value", "")
            return self.chess_game.select_difficulty(str(value))
        if action == CHESS_PROMOTION_ACTION:
            value = (event.payload or {}).get("value", "")
            return self.chess_game.select_promotion(str(value))
        if action == OTHELLO_CELL_ACTION:
            row, col = _parse_cell(node_id, "othello-cell")
            return self.othello_game.click(row, col, self.rng)
        if action == OTHELLO_CONTROL_ACTION:
            return self.othello_game.control(_parse_control(node_id, "othello"), self.rng)
        if action == OTHELLO_MODE_ACTION:
            value = (event.payload or {}).get("value", "")
            return self.othello_game.select_mode(str(value))
        if action == OTHELLO_DIFFICULTY_ACTION:
            value = (event.payload or {}).get("value", "")
            return self.othello_game.select_difficulty(str(value))
        raise ValueError(f"unknown action {action!r}")


def _overview_section(palette: str = "dark") -> ui.Node:
    return ui.vstack([
        ui.section_header(
            "Classic Games",
            "Choose a board and play. Each game includes keyboard and touch controls.",
        ),
        ui.hstack([
            ui.text("Board palette", tone="secondary"),
            ui.button("Light", id="palette-light", action="game_palette",
                      selected=palette == "light"),
            ui.button("Dark", id="palette-dark", action="game_palette",
                      selected=palette == "dark"),
            ui.text("This setting changes the game boards independently of the app theme.",
                    tone="secondary"),
        ], gap=8.0, align="center"),
        ui.wrap([
            ui.card([
                ui.heading("Zip", level=3),
                ui.text("Drag a continuous path through the numbered checkpoints, then cover every cell."),
                ui.button("Play Zip →", action="select:zip"),
            ], title="➰ Zip"),
            ui.card([
                ui.heading("Queens", level=3),
                ui.text("Place one crown in each row, column, and color region. Crowns cannot touch."),
                ui.button("Play Queens →", action="select:queens"),
            ], title="♛ Queens"),
            ui.card([
                ui.heading("Sudoku", level=3),
                ui.text("Fill the 9×9 grid with keyboard digits or the keypad. Hints and a solver are available."),
                ui.button("Play Sudoku →", action="select:sudoku"),
            ], title="🔢 Sudoku"),
            ui.card([
                ui.heading("Tetris", level=3),
                ui.text("Move, rotate, and drop falling blocks. Touch controls support simultaneous holds."),
                ui.button("Play Tetris →", action="select:tetris"),
            ], title="🧱 Tetris"),
            ui.card([
                ui.heading("Chess", level=3),
                ui.text("Full rules with a rust-chess AI. Select a piece, then a highlighted square."),
                ui.button("Play Chess →", action="select:chess"),
            ], title="♞ Chess"),
            ui.card([
                ui.heading("Othello", level=3),
                ui.text("Outflank rival discs to flip them. Black moves first; the AI replies as White."),
                ui.button("Play Othello →", action="select:othello"),
            ], title="● Othello"),
        ], gap=16.0),
        ui.text("Keyboard controls work while the board has focus. Touch actions are available on the board and control pad.",
                tone="secondary"),
    ], gap=16.0)


def build_app() -> GamesApp:
    """Build the demo with deterministic opening puzzles (fresh ones in play)."""
    seed = random.Random(7)
    queens = QueensGame()
    queens.new_puzzle(7, seed)
    sudoku = SudokuGame()
    sudoku.new_puzzle("easy", seed)
    app = GamesApp(
        title="Classic Games (Python)",
        sidebar_title="Classic Games",
        sidebar_subtitle="six games, one native window",
        miniapp=MiniAppConfig(
            title="Classic Games (Python)",
            width=1280.0,
            height=900.0,
            with_theme=True,
            initial_theme="dark",
        ),
        sections=[],
        zip_game=ZipGame(),
        queens_game=queens,
        sudoku_game=sudoku,
        tetris_game=TetrisGame(),
        chess_game=ChessGame(),
        othello_game=OthelloGame(),
    )
    app.sections = [
        section("overview", "Overview", _overview_section(app.palette)),
        section("zip", "Zip", app.zip_game.section_node()),
        section("queens", "Queens", app.queens_game.section_node()),
        section("sudoku", "Sudoku", app.sudoku_game.section_node()),
        section("tetris", "Tetris", app.tetris_game.section_node()),
        section("chess", "Chess", app.chess_game.section_node()),
        section("othello", "Othello", app.othello_game.section_node()),
    ]
    # Seed retained state with the exact revision-1 scenes embedded in the
    # initial App IR. Later input publishes consecutive scene revisions in
    # the same context.patch transaction as HUD updates.
    surface_ids = ("zip-board", "queens-board", "sudoku-board", "tetris-board",
                   "tetris-preview", "tetris-controls", "chess-board",
                   "othello-board")
    app.scene_cache = {surface_id: app._make_scene(surface_id, 1)
                       for surface_id in surface_ids}
    return app


def main() -> None:
    # Running the file opens a window: App.run() serves the host session
    # when GPUI_TOOLKIT_SESSION=1 and otherwise execs the bundled native
    # host. Only GPUI_TOOLKIT_DUMP_IR=1 prints the JSON spec instead.
    app = build_app()
    if os.environ.get("GPUI_TOOLKIT_DUMP_IR") == "1":
        print(json.dumps(app.to_spec(), indent=2))
    else:
        app.run()


if __name__ == "__main__":
    main()
