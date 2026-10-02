"""Four classic games in one native GPUI window: Zip, Queens, Sudoku, Tetris.

Run it (opens a native window)::

    python crates/gpui-python-runtime/python/examples/games_demo.py

Rendering model: Python owns game state and sends declarative UI IR plus
Scene2D drawing objects. The native GPUI host paints retained geometry;
Python sends stable-ID diffs after input or bounded elapsed-time steps. No
game loop sleeps in a Python worker thread.
"""

from __future__ import annotations

import json
import os
import random
import threading
from dataclasses import dataclass, field
from typing import Any

from gpui_toolkit import App, Event, Scene2DEvent, Scene2DTick, SessionContext, section, ui
from gpui_toolkit.events import (
    Scene2DActivateInput,
    Scene2DKeyInput,
    Scene2DLifecycleInput,
    Scene2DPointerInput,
)
from gpui_toolkit.game_audio import GameCueAdapter
from gpui_toolkit.miniapp import MiniAppConfig
from gpui_toolkit.scene2d import (
    GridSpec,
    PathCommand,
    Scene2D,
    Scene2DInputConfig,
    Scene2DSemantic,
    Scene2DTransition,
    SceneCircle,
    SceneLine,
    ScenePath,
    SceneRect,
    SceneRoundedRect,
    SceneText,
    Stroke,
    patch_op,
)


# ---------------------------------------------------------------------------
# Small patch/IR helpers
# ---------------------------------------------------------------------------

def _set(node_id: str, prop: str, value: Any) -> dict[str, Any]:
    """Build a ``set`` patch op (the property must exist in the live IR)."""
    return {"op": "set", "id": node_id, "property": prop, "value": value}


def _replace(node_id: str, node: ui.Node) -> dict[str, Any]:
    """Build a ``replace`` patch op for whole-board rebuilds."""
    return {"op": "replace", "id": node_id, "node": node.to_spec()}


def _cell_button(label: str, cell_id: str, action: str, *, selected: bool = False,
                 disabled: bool = False) -> ui.Node:
    """Board button with every patchable property declared up front."""
    return ui.button(label, id=cell_id, action=action, selected=selected,
                     disabled=disabled)


def _status_badge(badge_id: str, label: str, tone: str) -> ui.Node:
    return ui.badge(label, tone=tone, id=badge_id)


DARK_GAME_COLORS = {
    "board": "#172235", "cell": "#202D42", "cell_alt": "#25344B",
    "grid": "#364963", "text": "#F2F6FC", "muted": "#AAB8CC",
    "mint": "#71E0BF", "mint_dim": "#2F786E", "gold": "#FFD36F",
    "danger": "#FF6D7D", "paper": "#F4F1E8", "ink": "#253044",
    "fixed_text": "#F4F1E8", "major_grid": "#D8E1EC",
    "selection": "#314658", "match": "#29424C",
}
LIGHT_GAME_COLORS = {
    "board": "#E8EEF4", "cell": "#FFFFFF", "cell_alt": "#F3F6F9",
    "grid": "#A9B8C7", "text": "#172638", "muted": "#526579",
    "mint": "#087C68", "mint_dim": "#B9E7DA", "gold": "#B87A00",
    "danger": "#B3263B", "paper": "#FFFFFF", "ink": "#172638",
    "fixed_text": "#172638", "major_grid": "#64798B",
    "selection": "#C8ECE3", "match": "#DAEFE9",
}
GAME_PALETTES = {"dark": DARK_GAME_COLORS, "light": LIGHT_GAME_COLORS}
# Kept as the default palette for callers that imported the original constant.
GAME_COLORS = DARK_GAME_COLORS

QUEENS_REGION_COLORS = ("#F1A5A0", "#F7C49A", "#E8D184", "#9ED0B1",
                        "#91C3DB", "#B7A4D8", "#D29BBE", "#B8C7D4")
TETRIS_COLORS = {"I": "#69D7E8", "O": "#F2D56C", "T": "#BA9BFF",
                 "S": "#81DBA1", "Z": "#F47E8A", "J": "#8EABFF", "L": "#FFAE70"}
GAME_CUE_IDS = ("place", "error", "rotate", "lock", "clear", "win")


class _PaletteAwareGame:
    """Per-game board colors, independent of the native window theme."""

    palette: str

    @property
    def colors(self) -> dict[str, str]:
        return GAME_PALETTES[self.palette]


def _scene_cell(prefix: str, grid: GridSpec, row: int, column: int, *,
                fill: str, label: str, value: str = "", selected: bool = False,
                disabled: bool = False, stroke: str | None = None,
                stroke_width: float = 1.0, radius: float = 7.0) -> SceneRoundedRect:
    x, y, width, height = grid.cell_rect(row, column)
    return SceneRoundedRect(
        f"{prefix}-cell-{row}-{column}", x, y, width, height,
        fill=fill,
        stroke=None if stroke is None else Stroke(stroke, stroke_width),
        hit_id=f"{prefix}-cell-{row}-{column}", radius=radius,
        semantic=Scene2DSemantic(
            "grid_cell", label, value_text=value or None,
            selected=selected, disabled=disabled,
        ),
    )


def _scene_text(id: str, center: tuple[float, float], text: str, size: float,
                color: str, *, align: str = "center",
                semantic: Scene2DSemantic | None = None,
                transition: Scene2DTransition | None = None) -> SceneText:
    # Scene2D uses a top-left text origin and a line box of 1.25× the font size.
    return SceneText(id, center[0], center[1] - size * 0.625, text, size, color,
                     align=align, semantic=semantic, transition=transition)


def _grid_surface(surface_id: str, rows: int, columns: int, cell: float,
                  gap: float, padding: float, nodes: list[Any], *,
                  keyboard: bool = False, continuous: bool = False,
                  label: str) -> Scene2D:
    width = padding * 2 + columns * cell + max(0, columns - 1) * gap
    height = padding * 2 + rows * cell + max(0, rows - 1) * gap
    grid = GridSpec(rows, columns, padding, padding, cell, cell, gap)
    return Scene2D(
        surface_id, width, height, nodes, grid,
        Scene2DInputConfig(pointer=True, continuous=continuous, capture=True,
                           keyboard=keyboard),
        semantic=Scene2DSemantic(
            "grid", label, description=f"{rows} rows by {columns} columns"),
    )


# ---------------------------------------------------------------------------
# Zip: visit every cell exactly once, through the checkpoints in order
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class ZipLevel:
    id: str
    label: str
    rows: int
    cols: int
    checkpoints: dict[int, tuple[int, int]]

    @property
    def start(self) -> tuple[int, int]:
        return self.checkpoints[1]

    @property
    def finish(self) -> tuple[int, int]:
        return self.checkpoints[max(self.checkpoints)]

    @property
    def cell_count(self) -> int:
        return self.rows * self.cols


def _snake_path(rows: int, cols: int) -> list[tuple[int, int]]:
    """Boustrophedon walk covering every cell (guaranteed Zip solution)."""
    path: list[tuple[int, int]] = []
    for row in range(rows):
        columns = range(cols) if row % 2 == 0 else range(cols - 1, -1, -1)
        for col in columns:
            path.append((row, col))
    return path


def _zip_level(level_id: str, label: str, rows: int, cols: int,
               marks: list[int]) -> ZipLevel:
    snake = _snake_path(rows, cols)
    checkpoints = {number: snake[position - 1]
                   for number, position in enumerate(marks, start=1)}
    return ZipLevel(level_id, label, rows, cols, checkpoints)


ZIP_LEVELS: tuple[ZipLevel, ...] = (
    _zip_level("zip-4", "4 x 4 Starter", 4, 4, [1, 6, 11, 16]),
    _zip_level("zip-5", "5 x 5 Classic", 5, 5, [1, 9, 17, 25]),
    _zip_level("zip-6", "6 x 6 Challenge", 6, 6, [1, 11, 22, 30, 36]),
)

ZIP_CELL_ACTION = "zip_cell"
ZIP_CONTROL_ACTION = "zip_control"
ZIP_LEVEL_ACTION = "zip_level"


def solve_zip(level: ZipLevel, *, step_cap: int = 500_000) -> list[tuple[int, int]] | None:
    """Depth-first Hamiltonian path through the checkpoints in order.

    Returns the first complete path visiting every cell exactly once and
    ending on the final checkpoint, or ``None`` when the cap is exhausted.
    """
    ordered = sorted(level.checkpoints)
    wanted: dict[tuple[int, int], int] = {cell: number for number, cell in level.checkpoints.items()}
    target = len(ordered)
    deltas = ((1, 0), (-1, 0), (0, 1), (0, -1))
    visited = [[False] * level.cols for _ in range(level.rows)]
    path: list[tuple[int, int]] = []
    steps = 0

    def connected_from(head: tuple[int, int]) -> bool:
        seen = {head}
        stack = [head]
        while stack:
            row, col = stack.pop()
            for dr, dc in deltas:
                nxt = (row + dr, col + dc)
                if (0 <= nxt[0] < level.rows and 0 <= nxt[1] < level.cols
                        and not visited[nxt[0]][nxt[1]] and nxt not in seen):
                    seen.add(nxt)
                    stack.append(nxt)
        unvisited = level.cell_count - len(path)
        return len(seen) == unvisited + 1

    def search(head: tuple[int, int], next_index: int) -> list[tuple[int, int]] | None:
        nonlocal steps
        steps += 1
        if steps > step_cap:
            raise TimeoutError("zip solver step cap exhausted")
        visited[head[0]][head[1]] = True
        path.append(head)
        if len(path) == level.cell_count:
            result = list(path) if head == level.finish else None
            visited[head[0]][head[1]] = False
            path.pop()
            return result
        if connected_from(head):
            for dr, dc in deltas:
                nxt = (head[0] + dr, head[1] + dc)
                if not (0 <= nxt[0] < level.rows and 0 <= nxt[1] < level.cols):
                    continue
                if visited[nxt[0]][nxt[1]]:
                    continue
                checkpoint = wanted.get(nxt)
                if checkpoint is not None and checkpoint != ordered[next_index]:
                    continue
                advanced = next_index + (1 if checkpoint is not None else 0)
                if advanced > target:
                    continue
                found = search(nxt, advanced)
                if found is not None:
                    return found
        visited[head[0]][head[1]] = False
        path.pop()
        return None

    try:
        return search(level.start, 1)
    except TimeoutError:
        return None


@dataclass
class ZipGame(_PaletteAwareGame):
    level_index: int = 0
    path: list[tuple[int, int]] = field(default_factory=list)
    moves: int = 0
    won: bool = False
    palette: str = "dark"

    @property
    def level(self) -> ZipLevel:
        return ZIP_LEVELS[self.level_index]

    @property
    def head(self) -> tuple[int, int] | None:
        return self.path[-1] if self.path else None

    @property
    def next_checkpoint(self) -> int:
        numbers = sorted(self.level.checkpoints)
        visited_numbers = {number for number, cell in self.level.checkpoints.items()
                           if cell in self.path}
        for number in numbers:
            if number not in visited_numbers:
                return number
        return numbers[-1]

    def reset(self, level_index: int | None = None) -> None:
        if level_index is not None:
            self.level_index = level_index
        self.path = []
        self.moves = 0
        self.won = False

    # -- rendering ------------------------------------------------------

    @staticmethod
    def cell_id(row: int, col: int) -> str:
        return f"zip-cell-{row}-{col}"

    def cell_label(self, row: int, col: int) -> str:
        for number, cell in self.level.checkpoints.items():
            if cell == (row, col):
                return str(number)
        return "●" if (row, col) in self.path else "·"

    def cell_selected(self, row: int, col: int) -> bool:
        return (row, col) in self.path

    def board_node(self, *, revision: int = 1) -> Scene2D:
        level = self.level
        cell, gap, padding = 66.0, 3.0, 26.0
        grid = GridSpec(level.rows, level.cols, padding, padding, cell, cell, gap)
        nodes: list[Any] = [SceneRoundedRect(
            "zip-well", 7.0, 7.0,
            padding * 2 + level.cols * cell + (level.cols - 1) * gap - 14.0,
            padding * 2 + level.rows * cell + (level.rows - 1) * gap - 14.0,
            fill=self.colors["board"], radius=24.0,
        )]
        path_cells = set(self.path)
        for row in range(level.rows):
            for col in range(level.cols):
                checkpoint = next((number for number, at in level.checkpoints.items()
                                   if at == (row, col)), None)
                label = f"Row {row + 1}, column {col + 1}"
                value = f"checkpoint {checkpoint}" if checkpoint else "empty"
                nodes.append(_scene_cell(
                    "zip", grid, row, col,
                    fill=self.colors["mint_dim"] if (row, col) in path_cells else self.colors["cell"],
                    label=label, value=value, selected=(row, col) in path_cells,
                    stroke=self.colors["grid"], stroke_width=1.0, radius=11.0,
                ))
        for index, (first, second) in enumerate(zip(self.path, self.path[1:])):
            start, end = grid.cell_center(*first), grid.cell_center(*second)
            nodes.append(SceneLine(
                f"zip-path-{index}", start, end, Stroke(self.colors["mint"], 16.0),
                transition=Scene2DTransition(110, "ease_out_cubic"),
            ))
        for number, (row, col) in level.checkpoints.items():
            x, y = grid.cell_center(row, col)
            nodes.append(SceneCircle(
                f"zip-checkpoint-{number}", x, y, 22.0,
                fill=self.colors["paper"], stroke=Stroke(self.colors["mint"], 2.5),
            ))
            nodes.append(_scene_text(f"zip-number-{number}", (x, y), str(number), 21.0,
                                     self.colors["ink"]))
        if self.head is not None:
            x, y = grid.cell_center(*self.head)
            nodes.append(SceneCircle(
                "zip-head", x, y, 11.0, fill=self.colors["mint"],
                stroke=Stroke(self.colors["paper"], 2.0),
                transition=Scene2DTransition(90, "ease_out_quad"),
            ))
        width = padding * 2 + level.cols * cell + (level.cols - 1) * gap
        height = padding * 2 + level.rows * cell + (level.rows - 1) * gap
        return Scene2D(
            "zip-board", width, height, nodes, grid,
            Scene2DInputConfig(pointer=True, continuous=True, capture=True,
                               keyboard=True),
            revision=revision,
            semantic=Scene2DSemantic(
                "grid", f"Zip puzzle, {level.label}",
                description=f"{level.rows} rows by {level.cols} columns"),
        )

    def section_node(self) -> ui.Node:
        level = self.level
        return ui.vstack([
            ui.section_header(
                "Zip",
                "Draw one path from 1 through every checkpoint in order, "
                "covering all %d cells exactly once." % level.cell_count,
            ),
            ui.wrap([
                self.board_node(),
                ui.vstack([
                    ui.card([
                        ui.heading("How to play", level=3),
                        ui.text("Drag from 1 through the checkpoints in order and cover every cell."),
                        ui.text("Arrow keys extend from the glowing head. Backspace steps back."),
                    ], title="Rules"),
                    ui.select(
                        id="zip-level", label="Puzzle", value=level.id,
                        options=[(candidate.id, candidate.label) for candidate in ZIP_LEVELS],
                        action=ZIP_LEVEL_ACTION,
                    ),
                    ui.hstack([
                        _cell_button("↩ Undo", "zip-btn-undo", ZIP_CONTROL_ACTION),
                        _cell_button("↺ Reset", "zip-btn-reset", ZIP_CONTROL_ACTION),
                    ], gap=8.0),
                ], gap=12.0),
            ], gap=20.0),
            ui.wrap([
                ui.metric("Moves", str(self.moves), id="zip-moves"),
                ui.metric("Covered", f"{len(self.path)}/{level.cell_count}", id="zip-covered"),
                ui.metric("Next", str(self.next_checkpoint), id="zip-next"),
                _status_badge("zip-badge", "Ready", "neutral"),
            ], gap=12.0),
            ui.progress(0.0, label="Coverage", id="zip-progress"),
            ui.text("Tip: follow the checkpoint numbers and leave room for the final corner.",
                    tone="secondary", id="zip-status"),
        ], gap=16.0)

    # -- interaction ----------------------------------------------------

    def status_ops(self, message: str | None = None) -> list[dict[str, Any]]:
        level = self.level
        covered = len(self.path)
        if self.won:
            badge, tone = "Solved!", "success"
        elif covered:
            badge, tone = "Drawing…", "accent"
        else:
            badge, tone = "Ready", "neutral"
        ops = [
            _set("zip-moves", "value", str(self.moves)),
            _set("zip-covered", "value", f"{covered}/{level.cell_count}"),
            _set("zip-next", "value", str(self.next_checkpoint)),
            _set("zip-badge", "label", badge),
            _set("zip-badge", "tone", tone),
            _set("zip-progress", "value", covered / level.cell_count),
        ]
        if message is not None:
            ops.append(_set("zip-status", "text", message))
        return ops

    def click(self, row: int, col: int) -> list[dict[str, Any]]:
        level = self.level
        if self.won:
            return self.status_ops("Already solved — pick another puzzle or press Reset to draw again.")
        cell = (row, col)
        if self.head == cell:
            removed = self.path.pop()
            self.moves += 1
            message = f"Stepped back from {removed}." if self.path else "Path cleared."
            return [
                _set(self.cell_id(*removed), "label", self.cell_label(*removed)),
                _set(self.cell_id(*removed), "selected", False),
                *self.status_ops(message),
            ]
        if not self.path:
            if cell != level.start:
                return self.status_ops(f"Start the path on cell 1 at {level.start}.")
            self.path.append(cell)
            self.moves += 1
            if level.cell_count == 1:
                self.won = True
            return [
                _set(self.cell_id(*cell), "label", self.cell_label(*cell)),
                _set(self.cell_id(*cell), "selected", True),
                *self.status_ops("Path started. Extend it into a neighbouring cell."),
            ]
        head = self.head
        assert head is not None
        adjacent = abs(head[0] - row) + abs(head[1] - col) == 1
        if not adjacent or cell in self.path:
            return self.status_ops("Click a free cell next to the glowing head.")
        checkpoint = next((number for number, at in level.checkpoints.items() if at == cell), None)
        if checkpoint is not None and checkpoint != self.next_checkpoint:
            return self.status_ops(f"Checkpoint {checkpoint} is out of order — find {self.next_checkpoint} first.")
        self.path.append(cell)
        self.moves += 1
        ops = [
            _set(self.cell_id(*cell), "label", self.cell_label(*cell)),
            _set(self.cell_id(*cell), "selected", True),
        ]
        if len(self.path) == level.cell_count:
            if cell == level.finish:
                self.won = True
                return [*ops, *self.status_ops(f"Solved in {self.moves} moves!")]
            self.path.pop()
            return [
                _set(self.cell_id(*cell), "label", self.cell_label(*cell)),
                _set(self.cell_id(*cell), "selected", False),
                *self.status_ops("That fills the grid but strands the finish — step back and reroute."),
            ]
        if checkpoint is not None:
            return [*ops, *self.status_ops(f"Checkpoint {checkpoint} reached — next is {self.next_checkpoint}.")]
        return [*ops, *self.status_ops()]

    def drag_to(self, row: int, col: int) -> list[dict[str, Any]]:
        """Extend through a newly crossed cell or retract when dragged backward."""
        cell = (row, col)
        if cell in self.path:
            index = self.path.index(cell)
            if index == len(self.path) - 1:
                return []
            removed = self.path[index + 1:]
            del self.path[index + 1:]
            self.moves += len(removed)
            message = f"Retracted {len(removed)} cell(s)."
            ops: list[dict[str, Any]] = []
            for removed_cell in removed:
                ops.extend([
                    _set(self.cell_id(*removed_cell), "label", self.cell_label(*removed_cell)),
                    _set(self.cell_id(*removed_cell), "selected", False),
                ])
            return [*ops, *self.status_ops(message)]
        return self.click(row, col)

    def control(self, name: str) -> list[dict[str, Any]]:
        if name == "undo":
            if self.path and not self.won:
                removed = self.path.pop()
                self.moves += 1
                message = f"Stepped back from {removed}." if self.path else "Path cleared."
                return [
                    _set(self.cell_id(*removed), "label", self.cell_label(*removed)),
                    _set(self.cell_id(*removed), "selected", False),
                    *self.status_ops(message),
                ]
            return self.status_ops("Nothing to undo.")
        if name == "reset":
            self.reset()
            return [_replace("zip-board", self.board_node()),
                    *self.status_ops("Fresh grid — click cell 1 to begin.")]
        raise ValueError(f"unknown zip control {name!r}")

    def select_level(self, level_id: str) -> list[dict[str, Any]]:
        for index, candidate in enumerate(ZIP_LEVELS):
            if candidate.id == level_id:
                self.reset(index)
                return [_replace("zip-board", self.board_node()),
                        *self.status_ops(f"{candidate.label}: cover {candidate.cell_count} cells in order.")]
        raise ValueError(f"unknown zip level {level_id!r}")


# ---------------------------------------------------------------------------
# Queens: one crown per row, column and colour — and no touching neighbours
# ---------------------------------------------------------------------------

QUEENS_CELL_ACTION = "queens_cell"
QUEENS_CONTROL_ACTION = "queens_control"
QUEENS_SIZE_ACTION = "queens_size"

QUEENS_SIZES = (6, 7, 8)
QUEENS_REGION_GLYPHS = ("🔴", "🟠", "🟡", "🟢", "🔵", "🟣", "🟤", "⚪")
QUEENS_QUEEN_GLYPH = "♛"
QUEENS_MARK_GLYPH = "✕"

QUEENS_EMPTY = 0
QUEENS_QUEEN = 1
QUEENS_MARK = 2


def queens_solution_candidates(size: int, rng: random.Random) -> tuple[int, ...] | None:
    """Random crown placement with no shared row/column and no diagonal touch.

    Diagonal adjacency is only possible between neighbouring rows, so the
    search constrains consecutive rows to differ by anything but one.
    """
    columns = list(range(size))
    placement: list[int] = []

    def search(row: int) -> tuple[int, ...] | None:
        if row == size:
            return tuple(placement)
        order = columns[:]
        rng.shuffle(order)
        for column in order:
            if column in placement:
                continue
            if placement and abs(column - placement[-1]) == 1:
                continue
            placement.append(column)
            found = search(row + 1)
            if found is not None:
                return found
            placement.pop()
        return None

    return search(0)


def queens_region_map(size: int, solution: tuple[int, ...], rng: random.Random) -> list[list[int]]:
    """Grow one connected colour region around each crown until covered."""
    regions = [[-1] * size for _ in range(size)]
    for row, column in enumerate(solution):
        regions[row][column] = row
    frontier: set[tuple[int, int]] = set()
    for row in range(size):
        for col in range(size):
            if regions[row][col] == -1:
                for dr, dc in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    nr, nc = row + dr, col + dc
                    if 0 <= nr < size and 0 <= nc < size and regions[nr][nc] != -1:
                        frontier.add((row, col))
                        break
    while frontier:
        row, col = rng.choice(tuple(frontier))
        frontier.discard((row, col))
        if regions[row][col] != -1:
            continue
        neighbours = [regions[row + dr][col + dc]
                      for dr, dc in ((1, 0), (-1, 0), (0, 1), (0, -1))
                      if 0 <= row + dr < size and 0 <= col + dc < size
                      and regions[row + dr][col + dc] != -1]
        regions[row][col] = rng.choice(neighbours)
        for dr, dc in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            nr, nc = row + dr, col + dc
            if 0 <= nr < size and 0 <= nc < size and regions[nr][nc] == -1:
                frontier.add((nr, nc))
    return regions


def generate_queens_puzzle(size: int, rng: random.Random) -> tuple[set[tuple[int, int]], list[list[int]]]:
    """Build a solvable puzzle: crown solution first, regions grown around it."""
    solution = queens_solution_candidates(size, rng)
    if solution is None:
        raise RuntimeError(f"could not place {size} non-touching queens")
    regions = queens_region_map(size, solution, rng)
    return ({(row, solution[row]) for row in range(size)}, regions)


def queens_conflicts(size: int, queens: set[tuple[int, int]],
                     regions: list[list[int]]) -> dict[str, Any]:
    """Count row/column/region duplicates and touching pairs."""
    rows = [0] * size
    cols = [0] * size
    areas = [0] * size
    for row, col in queens:
        rows[row] += 1
        cols[col] += 1
        areas[regions[row][col]] += 1
    touching = 0
    for row, col in queens:
        for dr in (-1, 0, 1):
            for dc in (-1, 0, 1):
                if dr == 0 and dc == 0:
                    continue
                if (row + dr, col + dc) in queens:
                    touching += 1
    touching //= 2
    return {
        "rows": sum(1 for count in rows if count > 1),
        "cols": sum(1 for count in cols if count > 1),
        "regions": sum(1 for count in areas if count > 1),
        "touching": touching,
        "total": (sum(1 for count in rows if count > 1)
                  + sum(1 for count in cols if count > 1)
                  + sum(1 for count in areas if count > 1)
                  + touching),
    }


def queens_conflict_cells(size: int, queens: set[tuple[int, int]],
                          regions: list[list[int]]) -> set[tuple[int, int]]:
    """Return every crown participating in a row, column, region, or touch clash."""
    conflicts: set[tuple[int, int]] = set()
    for key_fn in (lambda cell: cell[0], lambda cell: cell[1],
                   lambda cell: regions[cell[0]][cell[1]]):
        groups: dict[int, list[tuple[int, int]]] = {}
        for cell in queens:
            groups.setdefault(key_fn(cell), []).append(cell)
        for cells in groups.values():
            if len(cells) > 1:
                conflicts.update(cells)
    for row, column in queens:
        for other_row, other_column in queens:
            if (row, column) != (other_row, other_column) and max(
                abs(row - other_row), abs(column - other_column)
            ) == 1:
                conflicts.add((row, column))
                conflicts.add((other_row, other_column))
    return conflicts


@dataclass
class QueensGame(_PaletteAwareGame):
    size: int = 7
    regions: list[list[int]] = field(default_factory=list)
    solution: set[tuple[int, int]] = field(default_factory=set)
    marks: list[list[int]] = field(default_factory=list)
    moves: int = 0
    won: bool = False
    selected: tuple[int, int] = (0, 0)
    palette: str = "dark"

    def new_puzzle(self, size: int, rng: random.Random) -> None:
        solution, regions = generate_queens_puzzle(size, rng)
        self.size = size
        self.regions = regions
        self.solution = solution
        self.marks = [[QUEENS_EMPTY] * size for _ in range(size)]
        self.moves = 0
        self.won = False
        self.selected = (0, 0)

    def clear(self) -> None:
        self.marks = [[QUEENS_EMPTY] * self.size for _ in range(self.size)]
        self.moves = 0
        self.won = False

    @property
    def queens(self) -> set[tuple[int, int]]:
        return {(row, col)
                for row in range(self.size)
                for col in range(self.size)
                if self.marks[row][col] == QUEENS_QUEEN}

    # -- rendering ------------------------------------------------------

    @staticmethod
    def cell_id(row: int, col: int) -> str:
        return f"queens-cell-{row}-{col}"

    def cell_label(self, row: int, col: int) -> str:
        state = self.marks[row][col]
        if state == QUEENS_QUEEN:
            return QUEENS_QUEEN_GLYPH
        if state == QUEENS_MARK:
            return QUEENS_MARK_GLYPH
        return QUEENS_REGION_GLYPHS[self.regions[row][col] % len(QUEENS_REGION_GLYPHS)]

    def board_node(self, *, revision: int = 1) -> Scene2D:
        cell, gap, padding = 70.0, 2.0, 24.0
        grid = GridSpec(self.size, self.size, padding, padding, cell, cell, gap)
        conflicts = queens_conflict_cells(self.size, self.queens, self.regions)
        nodes: list[Any] = [SceneRoundedRect(
            "queens-well", 7.0, 7.0,
            padding * 2 + self.size * cell + (self.size - 1) * gap - 14.0,
            padding * 2 + self.size * cell + (self.size - 1) * gap - 14.0,
            fill=self.colors["board"], radius=22.0,
        )]
        for row in range(self.size):
            for column in range(self.size):
                region = self.regions[row][column] % len(QUEENS_REGION_COLORS)
                state = self.marks[row][column]
                value = "crown" if state == QUEENS_QUEEN else "excluded" if state == QUEENS_MARK else "open"
                selected = self.selected == (row, column)
                nodes.append(_scene_cell(
                    "queens", grid, row, column,
                    fill=QUEENS_REGION_COLORS[region],
                    label=f"Row {row + 1}, column {column + 1}, region {region + 1}",
                    value=value, selected=selected,
                    stroke=self.colors["paper"] if selected else self.colors["grid"],
                    stroke_width=3.2 if selected else 1.0, radius=8.0,
                ))
                x, y = grid.cell_center(row, column)
                if state == QUEENS_QUEEN:
                    width, height = 30.0, 34.0
                    points = ((x - width / 2, y + height / 2),
                              (x - width * .40, y - height * .08),
                              (x - width * .18, y + height * .14),
                              (x - width * .32, y - height / 2),
                              (x, y - height * .04),
                              (x + width * .32, y - height / 2),
                              (x + width * .18, y + height * .14),
                              (x + width * .40, y - height * .08),
                              (x + width / 2, y + height / 2))
                    commands = [PathCommand("move_to", (points[0],))]
                    commands.extend(PathCommand("line_to", (point,)) for point in points[1:])
                    commands.append(PathCommand("close"))
                    nodes.append(ScenePath(
                        f"queens-crown-{row}-{column}", commands,
                        fill=self.colors["gold"], stroke=Stroke("#875E23", 1.3),
                        transition=Scene2DTransition(170, "ease_out_back"),
                    ))
                    nodes.append(SceneLine(f"queens-crown-base-{row}-{column}",
                                           (x - width / 2, y + height / 2 - 2),
                                           (x + width / 2, y + height / 2 - 2),
                                           Stroke("#875E23", 2.0)))
                elif state == QUEENS_MARK:
                    nodes.append(SceneLine(f"queens-mark-a-{row}-{column}",
                                           (x - 10, y - 10), (x + 10, y + 10),
                                           Stroke(self.colors["muted"], 3.0)))
                    nodes.append(SceneLine(f"queens-mark-b-{row}-{column}",
                                           (x + 10, y - 10), (x - 10, y + 10),
                                           Stroke(self.colors["muted"], 3.0)))
                if (row, column) in conflicts:
                    x0, y0, width, height = grid.cell_rect(row, column)
                    nodes.append(SceneRoundedRect(
                        f"queens-conflict-{row}-{column}", x0 + 2, y0 + 2,
                        width - 4, height - 4, fill=None,
                        stroke=Stroke(self.colors["danger"], 3.4), radius=8.0,
                    ))
        width = padding * 2 + self.size * cell + (self.size - 1) * gap
        return Scene2D(
            "queens-board", width, width, nodes, grid,
            Scene2DInputConfig(pointer=True, continuous=False, capture=True,
                               keyboard=True),
            revision=revision,
            semantic=Scene2DSemantic(
                "grid", f"Queens puzzle, {self.size} by {self.size}",
                description="Place one crown in each row, column, and colored region."),
        )

    def section_node(self) -> ui.Node:
        return ui.vstack([
            ui.section_header(
                "Queens",
                "Place one %s per row, column and colour. Crowns must not touch, not even diagonally."
                % QUEENS_QUEEN_GLYPH,
            ),
            ui.wrap([
                self.board_node(),
                ui.vstack([
                    ui.card([
                        ui.heading("How to play", level=3),
                        ui.text("Select a colored region cell and press Enter to place a crown."),
                        ui.text("Press M to mark a cell as ruled out. Arrows move the selection."),
                        ui.text("Every puzzle has at least one solution."),
                    ], title="Rules"),
                    ui.select(
                        id="queens-size", label="Board size", value=str(self.size),
                        options=[(str(size), f"{size} x {size}") for size in QUEENS_SIZES],
                        action=QUEENS_SIZE_ACTION,
                    ),
                    ui.hstack([
                        _cell_button("🎲 New puzzle", "queens-btn-new", QUEENS_CONTROL_ACTION),
                        _cell_button("⌫ Clear", "queens-btn-clear", QUEENS_CONTROL_ACTION),
                    ], gap=8.0),
                ], gap=12.0),
            ], gap=20.0),
            ui.wrap([
                ui.metric("Queens", f"{len(self.queens)}/{self.size}", id="queens-count"),
                ui.metric("Conflicts", "0", id="queens-conflicts"),
                ui.metric("Moves", str(self.moves), id="queens-moves"),
                _status_badge("queens-badge", "Ready", "neutral"),
            ], gap=12.0),
            ui.progress(0.0, label="Crowns placed", id="queens-progress"),
            ui.text("Arrow keys move · Enter cycles · M marks a cell.", tone="secondary", id="queens-status"),
        ], gap=16.0)

    # -- interaction ----------------------------------------------------

    def status_ops(self, message: str | None = None) -> list[dict[str, Any]]:
        queens = self.queens
        conflicts = queens_conflicts(self.size, queens, self.regions)
        if self.won:
            badge, tone = "Solved!", "success"
        elif conflicts["total"]:
            badge, tone = "Conflict", "warning"
        elif queens:
            badge, tone = "Playing", "accent"
        else:
            badge, tone = "Ready", "neutral"
        ops = [
            _set("queens-count", "value", f"{len(queens)}/{self.size}"),
            _set("queens-conflicts", "value", str(conflicts["total"])),
            _set("queens-moves", "value", str(self.moves)),
            _set("queens-badge", "label", badge),
            _set("queens-badge", "tone", tone),
            _set("queens-progress", "value", len(queens) / self.size),
        ]
        if message is not None:
            ops.append(_set("queens-status", "text", message))
        return ops

    def _refresh_after_edit(self) -> list[dict[str, Any]]:
        queens = self.queens
        conflicts = queens_conflicts(self.size, queens, self.regions)
        if len(queens) == self.size and conflicts["total"] == 0:
            self.won = True
            return self.status_ops(f"Perfect board in {self.moves} moves!")
        if self.won:
            self.won = False
        parts: list[str] = []
        if conflicts["rows"]:
            parts.append(f"{conflicts['rows']} row(s)")
        if conflicts["cols"]:
            parts.append(f"{conflicts['cols']} column(s)")
        if conflicts["regions"]:
            parts.append(f"{conflicts['regions']} colour(s)")
        if conflicts["touching"]:
            parts.append(f"{conflicts['touching']} touching pair(s)")
        if parts:
            return self.status_ops("Conflict in " + ", ".join(parts) + ".")
        return self.status_ops(f"{len(queens)}/{self.size} crowns placed.")

    def click(self, row: int, col: int) -> list[dict[str, Any]]:
        if self.won:
            return self.status_ops("Already solved — deal a new puzzle to play again.")
        self.selected = (row, col)
        self.marks[row][col] = (self.marks[row][col] + 1) % 3
        self.moves += 1
        return [
            _set(self.cell_id(row, col), "label", self.cell_label(row, col)),
            _set(self.cell_id(row, col), "selected",
                 self.marks[row][col] == QUEENS_QUEEN),
            *self._refresh_after_edit(),
        ]

    def control(self, name: str, rng: random.Random) -> list[dict[str, Any]]:
        if name == "new":
            self.new_puzzle(self.size, rng)
            return [_replace("queens-board", self.board_node()),
                    *self.status_ops(f"Fresh {self.size} x {self.size} puzzle dealt.")]
        if name == "clear":
            self.clear()
            return [_replace("queens-board", self.board_node()),
                    *self.status_ops("Board cleared.")]
        raise ValueError(f"unknown queens control {name!r}")

    def select_size(self, value: str, rng: random.Random) -> list[dict[str, Any]]:
        size = int(value)
        if size not in QUEENS_SIZES:
            raise ValueError(f"unknown queens size {value!r}")
        self.new_puzzle(size, rng)
        return [_replace("queens-board", self.board_node()),
                *self.status_ops(f"Fresh {size} x {size} puzzle dealt.")]


# ---------------------------------------------------------------------------
# Sudoku: the classic 9x9 — every row, column and box holds 1..9
# ---------------------------------------------------------------------------

SUDOKU_CELL_ACTION = "sudoku_cell"
SUDOKU_PAD_ACTION = "sudoku_pad"
SUDOKU_CONTROL_ACTION = "sudoku_control"
SUDOKU_DIFFICULTY_ACTION = "sudoku_difficulty"

SUDOKU_SOLUTION = (
    "534678912",
    "672195348",
    "198342567",
    "859761423",
    "426853791",
    "713924856",
    "961537284",
    "287419635",
    "345286179",
)

# Givens per difficulty ('0' is a hole). Each mask is verified by the test
# suite to admit exactly one solution. New games apply random Sudoku
# symmetries (digit/row/column/band/stack permutations), which preserve
# uniqueness, so every dealt puzzle is fresh yet guaranteed solvable.
SUDOKU_MASKS: dict[str, tuple[str, ...]] = {
    "easy": (
        "530070000",
        "600195000",
        "098000060",
        "800060003",
        "400803001",
        "700020006",
        "060000280",
        "000419005",
        "000080079",
    ),
    "medium": (
        "004600910",
        "072000000",
        "000342000",
        "009060020",
        "400803090",
        "700020050",
        "901500080",
        "000400005",
        "005080070",
    ),
    "hard": (
        "000008912",
        "000005000",
        "100000060",
        "800060003",
        "026050700",
        "003024000",
        "000000080",
        "207410000",
        "345000000",
    ),
}

SUDOKU_DIFFICULTIES = (("easy", "Easy"), ("medium", "Medium"), ("hard", "Hard"))


def _sudoku_grid(rows: tuple[str, ...] | list[str]) -> list[list[int]]:
    return [[int(char) for char in row] for row in rows]


def _sudoku_candidates(grid: list[list[int]], row: int, col: int) -> list[int]:
    if grid[row][col] != 0:
        return []
    used = set(grid[row])
    used.update(grid[r][col] for r in range(9))
    br, bc = 3 * (row // 3), 3 * (col // 3)
    used.update(grid[r][c] for r in range(br, br + 3) for c in range(bc, bc + 3))
    return [digit for digit in range(1, 10) if digit not in used]


def _sudoku_search(grid: list[list[int]], limit: int, counter: list[int]) -> bool:
    """MRV backtracking; stops early once ``counter[0]`` reaches ``limit``."""
    best: tuple[int, int] | None = None
    best_options: list[int] = []
    for row in range(9):
        for col in range(9):
            if grid[row][col] == 0:
                options = _sudoku_candidates(grid, row, col)
                if not options:
                    return False
                if best is None or len(options) < len(best_options):
                    best, best_options = (row, col), options
    if best is None:
        counter[0] += 1
        return counter[0] >= limit
    row, col = best
    for digit in best_options:
        grid[row][col] = digit
        if _sudoku_search(grid, limit, counter):
            if counter[0] >= limit:
                return True
        grid[row][col] = 0
    return counter[0] >= limit


def count_sudoku_solutions(grid: list[list[int]], limit: int = 2) -> int:
    working = [row[:] for row in grid]
    counter = [0]
    _sudoku_search(working, limit, counter)
    return counter[0]


def solve_sudoku(grid: list[list[int]]) -> list[list[int]] | None:
    working = [row[:] for row in grid]
    solved: list[list[int]] | None = None

    def search() -> bool:
        nonlocal solved
        best: tuple[int, int] | None = None
        best_options: list[int] = []
        for row in range(9):
            for col in range(9):
                if working[row][col] == 0:
                    options = _sudoku_candidates(working, row, col)
                    if not options:
                        return False
                    if best is None or len(options) < len(best_options):
                        best, best_options = (row, col), options
        if best is None:
            solved = [row[:] for row in working]
            return True
        row, col = best
        for digit in best_options:
            working[row][col] = digit
            if search():
                return True
            working[row][col] = 0
        return False

    return solved if search() else None


def is_valid_sudoku_solution(grid: list[list[int]]) -> bool:
    digits = set(range(1, 10))
    for row in range(9):
        if set(grid[row]) != digits:
            return False
        if {grid[r][row] for r in range(9)} != digits:
            return False
    for br in range(0, 9, 3):
        for bc in range(0, 9, 3):
            if {grid[r][c] for r in range(br, br + 3) for c in range(bc, bc + 3)} != digits:
                return False
    return True


def _shuffled_band_order(rng: random.Random) -> list[int]:
    bands = [0, 1, 2]
    rng.shuffle(bands)
    order: list[int] = []
    for band in bands:
        rows = [3 * band, 3 * band + 1, 3 * band + 2]
        rng.shuffle(rows)
        order.extend(rows)
    return order


def transform_sudoku(solution: list[list[int]], mask: tuple[str, ...],
                     rng: random.Random) -> tuple[list[list[int]], set[tuple[int, int]]]:
    """Apply random Sudoku symmetries; uniqueness of the puzzle is preserved."""
    digits = list(range(1, 10))
    rng.shuffle(digits)
    remap = {old: new for old, new in zip(range(1, 10), digits)}
    row_map = _shuffled_band_order(rng)
    col_map = _shuffled_band_order(rng)
    new_solution = [[0] * 9 for _ in range(9)]
    givens: set[tuple[int, int]] = set()
    for new_row in range(9):
        for new_col in range(9):
            old_row, old_col = row_map[new_row], col_map[new_col]
            new_solution[new_row][new_col] = remap[solution[old_row][old_col]]
            if mask[old_row][old_col] != "0":
                givens.add((new_row, new_col))
    return new_solution, givens


def dig_sudoku_mask(solution: list[list[int]], target_givens: int,
                    rng: random.Random) -> tuple[str, ...]:
    """Remove givens while the puzzle keeps a unique solution (dev-time tool)."""
    givens = {(row, col) for row in range(9) for col in range(9)}
    order = list(givens)
    rng.shuffle(order)
    for cell in order:
        if len(givens) <= target_givens:
            break
        givens.discard(cell)
        grid = [[solution[row][col] if (row, col) in givens else 0
                 for col in range(9)] for row in range(9)]
        if count_sudoku_solutions(grid, 2) != 1:
            givens.add(cell)
    rows: list[str] = []
    for row in range(9):
        rows.append("".join(str(solution[row][col]) if (row, col) in givens else "0"
                            for col in range(9)))
    return tuple(rows)


@dataclass
class SudokuGame(_PaletteAwareGame):
    difficulty: str = "easy"
    solution: list[list[int]] = field(default_factory=list)
    givens: set[tuple[int, int]] = field(default_factory=set)
    entries: dict[tuple[int, int], int] = field(default_factory=dict)
    selected: tuple[int, int] | None = None
    hints_used: int = 0
    moves: int = 0
    won: bool = False
    solving: bool = False
    palette: str = "dark"

    def new_puzzle(self, difficulty: str, rng: random.Random) -> None:
        solution, givens = transform_sudoku(_sudoku_grid(SUDOKU_SOLUTION),
                                           SUDOKU_MASKS[difficulty], rng)
        self.difficulty = difficulty
        self.solution = solution
        self.givens = givens
        self.entries = {}
        self.selected = None
        self.hints_used = 0
        self.moves = 0
        self.won = False
        self.solving = False

    @property
    def filled(self) -> int:
        return len(self.givens) + len(self.entries)

    def value_at(self, row: int, col: int) -> int:
        if (row, col) in self.givens:
            return self.solution[row][col]
        return self.entries.get((row, col), 0)

    def mistakes(self) -> list[tuple[int, int]]:
        return sorted(cell for cell, digit in self.entries.items()
                      if digit != self.solution[cell[0]][cell[1]])

    def is_complete(self) -> bool:
        if len(self.entries) + len(self.givens) != 81:
            return False
        return not self.mistakes()

    # -- rendering ------------------------------------------------------

    @staticmethod
    def cell_id(row: int, col: int) -> str:
        return f"sudoku-cell-{row}-{col}"

    def cell_label(self, row: int, col: int) -> str:
        value = self.value_at(row, col)
        return str(value) if value else "·"

    def board_node(self, *, revision: int = 1) -> Scene2D:
        cell, padding = 58.0, 20.0
        grid = GridSpec(9, 9, padding, padding, cell, cell, 0.0)
        values = [[self.value_at(row, col) for col in range(9)] for row in range(9)]
        mistakes = set(self.mistakes())
        nodes: list[Any] = [SceneRoundedRect(
            "sudoku-well", 6.0, 6.0, 9 * cell + 2 * padding - 12,
            9 * cell + 2 * padding - 12, fill=self.colors["board"], radius=16.0,
        )]
        for row in range(9):
            for column in range(9):
                selected = self.selected == (row, column)
                value = values[row][column]
                same_value = value != 0 and self.selected is not None and value == self.value_at(*self.selected)
                peer = self.selected is not None and (
                    row == self.selected[0] or column == self.selected[1]
                    or (row // 3, column // 3) == (self.selected[0] // 3, self.selected[1] // 3)
                )
                given = (row, column) in self.givens
                fill = (self.colors["selection"] if selected
                        else self.colors["match"] if same_value
                        else self.colors["cell_alt"] if peer else self.colors["cell"])
                value_label = (f"fixed {value}" if given else str(value)) if value else "empty"
                nodes.append(_scene_cell(
                    "sudoku", grid, row, column, fill=fill,
                    label=f"Row {row + 1}, column {column + 1}", value=value_label,
                    selected=selected, disabled=given,
                    stroke=self.colors["mint"] if selected else self.colors["grid"],
                    stroke_width=3.0 if selected else 1.0, radius=1.0,
                ))
                x, y = grid.cell_center(row, column)
                if value:
                    ink = (self.colors["danger"] if (row, column) in mistakes
                           else self.colors["fixed_text"] if given else self.colors["mint"])
                    nodes.append(_scene_text(f"sudoku-value-{row}-{column}", (x, y),
                                             str(value), 31.0, ink,
                                             semantic=Scene2DSemantic(
                                                 "image", "Digit", value_text=str(value))))
                else:
                    candidates = _sudoku_candidates(values, row, column)
                    for digit in candidates:
                        candidate_row, candidate_col = divmod(digit - 1, 3)
                        px = x - 16.0 + candidate_col * 16.0
                        py = y - 16.0 + candidate_row * 16.0
                        nodes.append(_scene_text(f"sudoku-candidate-{row}-{column}-{digit}",
                                                 (px, py), str(digit), 10.5,
                                                 self.colors["muted"]))
        total = 9 * cell
        for boundary in range(10):
            weight = 3.6 if boundary % 3 == 0 else 1.0
            color = self.colors["major_grid"] if boundary % 3 == 0 else self.colors["grid"]
            offset = padding + boundary * cell
            nodes.append(SceneLine(f"sudoku-grid-v-{boundary}",
                                   (offset, padding), (offset, padding + total),
                                   Stroke(color, weight)))
            nodes.append(SceneLine(f"sudoku-grid-h-{boundary}",
                                   (padding, offset), (padding + total, offset),
                                   Stroke(color, weight)))
        size = total + 2 * padding
        return Scene2D(
            "sudoku-board", size, size, nodes, grid,
            Scene2DInputConfig(pointer=True, continuous=False, capture=True,
                               keyboard=True),
            revision=revision,
            semantic=Scene2DSemantic(
                "grid", "Sudoku board",
                description="9 rows by 9 columns. Fixed clues and editable digits use contrasting colors."),
        )

    def selected_preview(self) -> tuple[str, str]:
        if self.selected is None:
            return "Select a cell", "Choose a cell, then type a digit or use the keypad."
        row, column = self.selected
        value = self.value_at(row, column)
        if (row, column) in self.givens:
            return str(value), f"R{row + 1} C{column + 1} · fixed clue"
        if value:
            return str(value), f"R{row + 1} C{column + 1} · editable digit"
        grid = [[self.value_at(r, c) for c in range(9)] for r in range(9)]
        candidates = _sudoku_candidates(grid, row, column)
        return "—", f"R{row + 1} C{column + 1} · candidates {'  '.join(map(str, candidates)) or 'none'}"

    def _pad_node(self) -> ui.Node:
        rows: list[ui.Node] = []
        for base in (1, 4, 7):
            rows.append(ui.hstack(
                [_cell_button(str(digit), f"sudoku-pad-{digit}", SUDOKU_PAD_ACTION)
                 for digit in (base, base + 1, base + 2)],
                gap=4.0,
            ))
        rows.append(ui.hstack(
            [_cell_button("⌫ Erase", "sudoku-pad-erase", SUDOKU_PAD_ACTION)],
            gap=4.0,
        ))
        return ui.vstack(rows, gap=4.0, id="sudoku-pad")

    def section_node(self) -> ui.Node:
        preview, candidates = self.selected_preview()
        return ui.vstack([
            ui.section_header(
                "Sudoku",
                "Fill every row, column and 3×3 box with the digits 1–9.",
            ),
            ui.wrap([
                self.board_node(),
                ui.vstack([
                    ui.card([
                        ui.heading("How to play", level=3),
                        ui.text("Click a cell, then press a digit. Grey cells are fixed givens."),
                        ui.text("Check highlights mistakes; Solve animates the full solution."),
                    ], title="Rules"),
                    ui.card([
                        ui.heading("Selected cell", level=3),
                        ui.heading(preview, level=1, id="sudoku-preview"),
                        ui.text(candidates, tone="secondary", id="sudoku-candidates"),
                    ], title="Input"),
                    ui.select(
                        id="sudoku-difficulty", label="Difficulty", value=self.difficulty,
                        options=list(SUDOKU_DIFFICULTIES),
                        action=SUDOKU_DIFFICULTY_ACTION,
                    ),
                    self._pad_node(),
                    ui.wrap([
                        _cell_button("💡 Hint", "sudoku-btn-hint", SUDOKU_CONTROL_ACTION),
                        _cell_button("✓ Check", "sudoku-btn-check", SUDOKU_CONTROL_ACTION),
                        _cell_button("★ Solve", "sudoku-btn-solve", SUDOKU_CONTROL_ACTION),
                        _cell_button("↺ New", "sudoku-btn-new", SUDOKU_CONTROL_ACTION),
                    ], gap=8.0),
                ], gap=12.0),
            ], gap=20.0),
            ui.wrap([
                ui.metric("Filled", f"{self.filled}/81", id="sudoku-filled"),
                ui.metric("Hints", str(self.hints_used), id="sudoku-hints"),
                ui.metric("Moves", str(self.moves), id="sudoku-moves"),
                _status_badge("sudoku-badge", "Ready", "neutral"),
            ], gap=12.0),
            ui.progress(self.filled / 81, label="Board filled", id="sudoku-progress"),
            ui.text("Select a cell to see its candidates.", tone="secondary", id="sudoku-status"),
        ], gap=16.0)

    # -- interaction ----------------------------------------------------

    def status_ops(self, message: str | None = None) -> list[dict[str, Any]]:
        if self.won:
            badge, tone = "Solved!", "success"
        elif self.solving:
            badge, tone = "Solving…", "accent"
        elif self.mistakes():
            badge, tone = "Mistakes", "warning"
        elif self.entries:
            badge, tone = "Playing", "accent"
        else:
            badge, tone = "Ready", "neutral"
        ops = [
            _set("sudoku-filled", "value", f"{self.filled}/81"),
            _set("sudoku-hints", "value", str(self.hints_used)),
            _set("sudoku-moves", "value", str(self.moves)),
            _set("sudoku-badge", "label", badge),
            _set("sudoku-badge", "tone", tone),
            _set("sudoku-progress", "value", self.filled / 81),
        ]
        preview, candidates = self.selected_preview()
        ops.extend([
            _set("sudoku-preview", "text", preview),
            _set("sudoku-candidates", "text", candidates),
        ])
        if message is not None:
            ops.append(_set("sudoku-status", "text", message))
        return ops

    def _candidates_message(self, row: int, col: int) -> str:
        if (row, col) in self.givens:
            return f"R{row + 1}C{col + 1} is a given."
        if (row, col) in self.entries:
            return f"R{row + 1}C{col + 1} holds {self.entries[(row, col)]}."
        grid = [[self.value_at(r, c) for c in range(9)] for r in range(9)]
        options = _sudoku_candidates(grid, row, col)
        if not options:
            return f"R{row + 1}C{col + 1} has no legal digit — check for mistakes."
        return f"R{row + 1}C{col + 1} candidates: {' '.join(map(str, options))}"

    def click(self, row: int, col: int) -> list[dict[str, Any]]:
        if self.solving:
            return self.status_ops("The solver is running — wait for it to finish.")
        if self.won:
            return self.status_ops("Already solved — deal a new puzzle to play again.")
        previous = self.selected
        self.selected = (row, col)
        ops = []
        if previous is not None and previous != (row, col):
            ops.append(_set(self.cell_id(*previous), "selected", False))
        ops.append(_set(self.cell_id(row, col), "selected", True))
        return [*ops, *self.status_ops(self._candidates_message(row, col))]

    def _check_win(self) -> list[dict[str, Any]] | None:
        if self.is_complete():
            self.won = True
            return self.status_ops(f"Perfect grid in {self.moves} moves with {self.hints_used} hints!")
        return None

    def enter_digit(self, digit: int) -> list[dict[str, Any]]:
        if self.solving:
            return self.status_ops("The solver is running — wait for it to finish.")
        if self.won:
            return self.status_ops("Already solved — deal a new puzzle to play again.")
        if self.selected is None:
            return self.status_ops("Select a cell first, then press a digit.")
        row, col = self.selected
        if (row, col) in self.givens:
            return self.status_ops("That cell is a fixed given.")
        self.entries[(row, col)] = digit
        self.moves += 1
        ops = [_set(self.cell_id(row, col), "label", str(digit))]
        win = self._check_win()
        if win is not None:
            return [*ops, *win]
        if digit != self.solution[row][col]:
            others = [cell for cell in self.mistakes() if cell != (row, col)]
            hint = f" Plus {len(others)} more mistake(s)." if others else ""
            return [*ops, *self.status_ops(
                f"R{row + 1}C{col + 1} conflicts with the solution.{hint}")]
        return [*ops, *self.status_ops(self._candidates_message(row, col))]

    def erase(self) -> list[dict[str, Any]]:
        if self.solving:
            return self.status_ops("The solver is running — wait for it to finish.")
        if self.selected is None:
            return self.status_ops("Select a cell first.")
        if self.selected in self.givens:
            return self.status_ops("Givens cannot be erased.")
        removed = self.entries.pop(self.selected, None)
        if removed is None:
            return self.status_ops("That cell is already empty.")
        self.moves += 1
        row, col = self.selected
        return [_set(self.cell_id(row, col), "label", "·"),
                *self.status_ops(self._candidates_message(row, col))]

    def hint(self) -> list[dict[str, Any]]:
        if self.solving:
            return self.status_ops("The solver is running — wait for it to finish.")
        if self.won:
            return self.status_ops("Already solved — deal a new puzzle to play again.")
        target = self.selected
        if target is None or target in self.givens or self.value_at(*target) == self.solution[target[0]][target[1]]:
            target = None
            for cell in self.mistakes():
                target = cell
                break
            if target is None:
                for row in range(9):
                    for col in range(9):
                        if self.value_at(row, col) == 0:
                            target = (row, col)
                            break
                    if target is not None:
                        break
        if target is None:
            return self.status_ops("Nothing left to reveal.")
        row, col = target
        self.entries[(row, col)] = self.solution[row][col]
        self.selected = (row, col)
        self.hints_used += 1
        self.moves += 1
        ops = [
            _set(self.cell_id(row, col), "label", str(self.solution[row][col])),
            _set(self.cell_id(row, col), "selected", True),
        ]
        win = self._check_win()
        if win is not None:
            return [*ops, *win]
        return [*ops, *self.status_ops(f"Hint: R{row + 1}C{col + 1} is {self.solution[row][col]}.")]

    def check(self) -> list[dict[str, Any]]:
        if self.solving:
            return self.status_ops("The solver is running — wait for it to finish.")
        wrong = self.mistakes()
        if not wrong and self.is_complete():
            self.won = True
            return self.status_ops(f"Perfect grid in {self.moves} moves with {self.hints_used} hints!")
        if not wrong:
            return self.status_ops("No mistakes so far — keep going.")
        spots = ", ".join(f"R{row + 1}C{col + 1}" for row, col in wrong[:6])
        more = f" (+{len(wrong) - 6} more)" if len(wrong) > 6 else ""
        return self.status_ops(f"{len(wrong)} mistake(s) at {spots}{more}.")

    def control(self, name: str, rng: random.Random) -> list[dict[str, Any]]:
        if name == "hint":
            return self.hint()
        if name == "check":
            return self.check()
        if name == "new":
            self.new_puzzle(self.difficulty, rng)
            return [_replace("sudoku-board", self.board_node()),
                    *self.status_ops(f"Fresh {self.difficulty} puzzle dealt.")]
        if name == "solve":
            # The serial reducer advances the solver on bounded elapsed ticks.
            if self.solving:
                return self.status_ops("The solver is already running.")
            if self.won:
                return self.status_ops("Already solved.")
            self.solving = True
            return self.status_ops("Solving…")
        raise ValueError(f"unknown sudoku control {name!r}")

    def select_difficulty(self, difficulty: str, rng: random.Random) -> list[dict[str, Any]]:
        if difficulty not in SUDOKU_MASKS:
            raise ValueError(f"unknown sudoku difficulty {difficulty!r}")
        self.new_puzzle(difficulty, rng)
        return [_replace("sudoku-board", self.board_node()),
                *self.status_ops(f"Fresh {difficulty} puzzle dealt.")]

    def solve_step_cells(self) -> list[tuple[int, int, int]]:
        """Cells the animated solver still has to fix, in scan order."""
        steps: list[tuple[int, int, int]] = []
        for row in range(9):
            for col in range(9):
                if (row, col) in self.givens:
                    continue
                if self.value_at(row, col) != self.solution[row][col]:
                    steps.append((row, col, self.solution[row][col]))
        return steps

    def apply_solve_step(self, row: int, col: int, digit: int) -> dict[str, Any]:
        self.entries[(row, col)] = digit
        return _set(self.cell_id(row, col), "label", str(digit))

    def finish_solving(self) -> list[dict[str, Any]]:
        self.solving = False
        self.selected = None
        win = self._check_win()
        if win is not None:
            return win
        return self.status_ops("Solver finished.")


# ---------------------------------------------------------------------------
# Tetris: gravity, 7-bag pieces, line clears — plus an autopilot demo mode
# ---------------------------------------------------------------------------

TETRIS_CONTROL_ACTION = "tetris_control"
TETRIS_ROWS = 20
TETRIS_COLS = 10
TETRIS_EMPTY = "⬛"

# Cells are (row, col) offsets in spawn orientation; glyphs are the colour.
_TETROMINOES: dict[str, tuple[list[tuple[int, int]], str, str]] = {
    "I": ([(0, 0), (0, 1), (0, 2), (0, 3)], "🟦", "I"),
    "O": ([(0, 1), (0, 2), (1, 1), (1, 2)], "🟨", "O"),
    "T": ([(0, 1), (1, 0), (1, 1), (1, 2)], "🟪", "T"),
    "S": ([(0, 1), (0, 2), (1, 0), (1, 1)], "🟩", "S"),
    "Z": ([(0, 0), (0, 1), (1, 1), (1, 2)], "🟥", "Z"),
    "J": ([(0, 0), (1, 0), (1, 1), (1, 2)], "🟫", "J"),
    "L": ([(0, 2), (1, 0), (1, 1), (1, 2)], "🟧", "L"),
}
_TETRIS_KICKS = ((0, 0), (0, -1), (0, 1), (-1, 0), (0, -2), (0, 2), (-2, 0), (1, 0))
_TETRIS_LINE_SCORE = (0, 100, 300, 500, 800)


def _rotate_cells(cells: list[tuple[int, int]]) -> list[tuple[int, int]]:
    rotated = [(-col, row) for row, col in cells]
    top = min(row for row, _ in rotated)
    left = min(col for _, col in rotated)
    return [(row - top, col - left) for row, col in rotated]


@dataclass
class TetrisGame(_PaletteAwareGame):
    grid: list[list[str | None]] = field(default_factory=list)
    bag: list[str] = field(default_factory=list)
    current: str | None = None
    cells: list[tuple[int, int]] = field(default_factory=list)
    piece_row: int = 0
    piece_col: int = 0
    next_shape: str | None = None
    score: int = 0
    lines: int = 0
    pieces: int = 0
    running: bool = False
    over: bool = False
    auto: bool = False
    auto_rotations: int = 0
    last_rows: list[str] = field(default_factory=list)
    last_lock_cells: list[tuple[int, int]] = field(default_factory=list)
    last_clear_rows: list[int] = field(default_factory=list)
    palette: str = "dark"

    def __post_init__(self) -> None:
        if not self.grid:
            self.grid = [[None] * TETRIS_COLS for _ in range(TETRIS_ROWS)]

    @property
    def level(self) -> int:
        return 1 + self.lines // 10

    @property
    def tick_interval(self) -> float:
        if self.auto:
            return 0.08
        return max(0.06, 0.7 - 0.06 * (self.level - 1))

    def reset(self) -> None:
        self.grid = [[None] * TETRIS_COLS for _ in range(TETRIS_ROWS)]
        self.bag = []
        self.current = None
        self.cells = []
        self.next_shape = None
        self.score = 0
        self.lines = 0
        self.pieces = 0
        self.running = False
        self.over = False
        self.auto_rotations = 0
        self.last_rows = []
        self.last_lock_cells = []
        self.last_clear_rows = []

    # -- engine ---------------------------------------------------------

    def fits(self, cells: list[tuple[int, int]], row: int, col: int) -> bool:
        for dr, dc in cells:
            r, c = row + dr, col + dc
            if c < 0 or c >= TETRIS_COLS or r >= TETRIS_ROWS:
                return False
            if r >= 0 and self.grid[r][c] is not None:
                return False
        return True

    def _draw(self, rng: random.Random) -> str:
        if not self.bag:
            self.bag = list(_TETROMINOES)
            rng.shuffle(self.bag)
        return self.bag.pop()

    def _spawn(self, rng: random.Random) -> bool:
        shape = self.next_shape or self._draw(rng)
        self.next_shape = self._draw(rng)
        cells = list(_TETROMINOES[shape][0])
        row, col = 0, 3
        if not self.fits(cells, row, col):
            self.current = None
            self.cells = []
            self.over = True
            self.running = False
            return False
        self.current = shape
        self.cells = cells
        self.piece_row = row
        self.piece_col = col
        self.pieces += 1
        self.auto_rotations = 0
        return True

    def _lock(self, rng: random.Random) -> int:
        assert self.current is not None
        glyph = _TETROMINOES[self.current][1]
        self.last_lock_cells = []
        for dr, dc in self.cells:
            r, c = self.piece_row + dr, self.piece_col + dc
            if 0 <= r < TETRIS_ROWS and 0 <= c < TETRIS_COLS:
                self.grid[r][c] = glyph
                self.last_lock_cells.append((r, c))
        self.current = None
        self.cells = []
        cleared = self._clear_lines()
        self.lines += cleared
        self.score += _TETRIS_LINE_SCORE[min(cleared, 4)] * self.level
        if cleared == 0:
            self.score += 1
        if not self.over:
            self._spawn(rng)
        return cleared

    def _clear_lines(self) -> int:
        self.last_clear_rows = [row for row, cells in enumerate(self.grid)
                                if all(cell is not None for cell in cells)]
        kept = [row for row in self.grid if any(cell is None for cell in row)]
        cleared = TETRIS_ROWS - len(kept)
        while len(kept) < TETRIS_ROWS:
            kept.insert(0, [None] * TETRIS_COLS)
        self.grid = kept
        return cleared

    def start(self, rng: random.Random) -> None:
        if self.over or self.current is None:
            keep_auto = self.auto
            self.reset()
            self.auto = keep_auto
        if self.current is None:
            self._spawn(rng)
        self.running = True

    def tick(self, rng: random.Random) -> list[dict[str, Any]]:
        """Advance gravity by one step from the host's monotonic tick lane."""
        if not self.running or self.over or self.current is None:
            return self.status_ops()
        if self.fits(self.cells, self.piece_row + 1, self.piece_col):
            self.piece_row += 1
            return self.delta_ops()
        cleared = self._lock(rng)
        if self.over:
            return self.delta_ops("Game over — press Restart.")
        if cleared:
            names = {1: "Single", 2: "Double", 3: "Triple", 4: "Tetris!"}
            return self.delta_ops(f"{names.get(cleared, f'{cleared} lines')} — {self.score} points.")
        return self.delta_ops()

    # -- autopilot ------------------------------------------------------

    def _placement_score(self, cells: list[tuple[int, int]], row: int, col: int) -> float | None:
        if not self.fits(cells, row, col):
            return None
        heights = [0] * TETRIS_COLS
        holes = 0
        for c in range(TETRIS_COLS):
            filled = {r for r in range(TETRIS_ROWS) if self.grid[r][c] is not None}
            filled.update(row + dr for dr, dc in cells if col + dc == c)
            if not filled:
                continue
            top = min(filled)
            heights[c] = TETRIS_ROWS - top
            holes += sum(1 for r in range(top, TETRIS_ROWS) if r not in filled)
        bumpiness = sum(abs(heights[c] - heights[c + 1]) for c in range(TETRIS_COLS - 1))
        full_rows = 0
        for r in range(TETRIS_ROWS):
            occupied = sum(1 for c in range(TETRIS_COLS) if self.grid[r][c] is not None)
            occupied += sum(1 for dr, dc in cells if row + dr == r and 0 <= col + dc < TETRIS_COLS)
            if occupied >= TETRIS_COLS:
                full_rows += 1
        return full_rows * 30.0 - sum(heights) * 0.6 - holes * 8.0 - bumpiness * 1.5

    def _auto_target(self) -> tuple[list[tuple[int, int]], int, int] | None:
        if self.current is None:
            return None
        best: tuple[list[tuple[int, int]], int, int] | None = None
        best_score = float("-inf")
        cells = list(_TETROMINOES[self.current][0])
        for _ in range(4):
            left = min(c for _, c in cells)
            right = max(c for _, c in cells)
            for col in range(-left, TETRIS_COLS - right):
                row = 0
                while self.fits(cells, row + 1, col):
                    row += 1
                if not self.fits(cells, row, col):
                    continue
                score = self._placement_score(cells, row, col)
                if score is not None and score > best_score:
                    best_score = score
                    best = (list(cells), row, col)
            cells = _rotate_cells(cells)
        return best

    def auto_step(self, rng: random.Random) -> list[dict[str, Any]]:
        if not self.running or self.over or self.current is None:
            return self.status_ops()
        target = self._auto_target()
        if target is None:
            return self.tick(rng)
        cells, _, col = target
        if sorted(self.cells) != sorted(cells) and self.auto_rotations < 4:
            self.auto_rotations += 1
            return self._rotate_inner()
        if self.piece_col < col and self.fits(self.cells, self.piece_row, self.piece_col + 1):
            self.piece_col += 1
            return self.delta_ops()
        if self.piece_col > col and self.fits(self.cells, self.piece_row, self.piece_col - 1):
            self.piece_col -= 1
            return self.delta_ops()
        cleared = self._hard_drop_inner(rng)
        if self.over:
            return self.delta_ops(f"Autopilot topped out at {self.score} points.")
        if cleared >= 4:
            return self.delta_ops(f"Autopilot scores a Tetris! ({self.score})")
        return self.delta_ops()

    # -- manual moves ---------------------------------------------------

    def _rotate_inner(self) -> list[dict[str, Any]]:
        if self.current in (None, "O"):
            return self.delta_ops()
        rotated = _rotate_cells(self.cells)
        for dr, dc in _TETRIS_KICKS:
            row, col = self.piece_row + dr, self.piece_col + dc
            if self.fits(rotated, row, col):
                self.cells = rotated
                self.piece_row = row
                self.piece_col = col
                return self.delta_ops()
        return self.delta_ops()

    def _hard_drop_inner(self, rng: random.Random) -> int:
        while self.fits(self.cells, self.piece_row + 1, self.piece_col):
            self.piece_row += 1
        self.score += 2
        return self._lock(rng)

    def _manual(self) -> list[dict[str, Any]] | None:
        """Gate manual moves; returns ops when the move is rejected."""
        if self.over:
            return self.status_ops("Game over — press Restart.")
        if not self.running:
            return self.status_ops("Press Start first.")
        if self.auto:
            return self.status_ops("Autopilot is flying — disable Auto to steer.")
        if self.current is None:
            return self.status_ops("No active piece.")
        return None

    def move(self, direction: int) -> list[dict[str, Any]]:
        rejected = self._manual()
        if rejected is not None:
            return rejected
        if self.fits(self.cells, self.piece_row, self.piece_col + direction):
            self.piece_col += direction
        return self.delta_ops()

    def rotate(self) -> list[dict[str, Any]]:
        rejected = self._manual()
        if rejected is not None:
            return rejected
        return self._rotate_inner()

    def soft_drop(self, rng: random.Random) -> list[dict[str, Any]]:
        rejected = self._manual()
        if rejected is not None:
            return rejected
        if self.fits(self.cells, self.piece_row + 1, self.piece_col):
            self.piece_row += 1
            self.score += 1
            return self.delta_ops()
        cleared = self._lock(rng)
        if self.over:
            return self.delta_ops("Game over — press Restart.")
        if cleared:
            return self.delta_ops(f"Cleared {cleared} — {self.score} points.")
        return self.delta_ops()

    def hard_drop(self, rng: random.Random) -> list[dict[str, Any]]:
        rejected = self._manual()
        if rejected is not None:
            return rejected
        cleared = self._hard_drop_inner(rng)
        if self.over:
            return self.delta_ops("Game over — press Restart.")
        if cleared:
            return self.delta_ops(f"Cleared {cleared} — {self.score} points.")
        return self.delta_ops()

    def control(self, name: str, rng: random.Random) -> list[dict[str, Any]]:
        if name == "left":
            return self.move(-1)
        if name == "right":
            return self.move(1)
        if name == "down":
            return self.soft_drop(rng)
        if name == "rotate":
            return self.rotate()
        if name == "drop":
            return self.hard_drop(rng)
        if name == "start":
            if self.running:
                self.running = False
                return self.delta_ops("Paused.")
            self.start(rng)
            if self.over:
                return self.delta_ops("Game over — press Restart.")
            return self.delta_ops("Playing — good luck!")
        if name == "restart":
            keep_auto = self.auto
            self.reset()
            self.auto = keep_auto
            self.start(rng)
            return self.delta_ops("Fresh well — good luck!")
        if name == "auto":
            self.auto = not self.auto
            ops = [_set("tetris-btn-auto", "selected", self.auto)]
            if self.auto and not self.running and not self.over:
                self.start(rng)
            return [*ops, *self.delta_ops(
                "Autopilot engaged." if self.auto else "Autopilot off — your move.")]
        raise ValueError(f"unknown tetris control {name!r}")

    # -- rendering ------------------------------------------------------

    @staticmethod
    def row_id(row: int) -> str:
        return f"tetris-row-{row}"

    def render_rows(self) -> list[str]:
        glyphs = [[cell if cell is not None else TETRIS_EMPTY for cell in row]
                  for row in self.grid]
        if self.current is not None:
            piece_glyph = _TETROMINOES[self.current][1]
            for dr, dc in self.cells:
                r, c = self.piece_row + dr, self.piece_col + dc
                if 0 <= r < TETRIS_ROWS and 0 <= c < TETRIS_COLS:
                    glyphs[r][c] = piece_glyph
        return ["".join(row) for row in glyphs]

    def next_label(self) -> str:
        if self.next_shape is None:
            return "—"
        return f"{self.next_shape} piece"

    def ghost_row(self) -> int | None:
        if self.current is None:
            return None
        row = self.piece_row
        while self.fits(self.cells, row + 1, self.piece_col):
            row += 1
        return row

    def board_node(self, *, revision: int = 1) -> Scene2D:
        cell, gap, padding = 23.0, 2.0, 16.0
        grid = GridSpec(TETRIS_ROWS, TETRIS_COLS, padding, padding, cell, cell, gap)
        ghost_row = self.ghost_row()
        ghost = set() if ghost_row is None else {
            (ghost_row + dr, self.piece_col + dc) for dr, dc in self.cells
        }
        current = set() if self.current is None else {
            (self.piece_row + dr, self.piece_col + dc) for dr, dc in self.cells
        }
        active_color = TETRIS_COLORS.get(self.current or "", self.colors["mint"])
        nodes: list[Any] = [SceneRoundedRect(
            "tetris-well", 5, 5,
            padding * 2 + TETRIS_COLS * cell + (TETRIS_COLS - 1) * gap - 10,
            padding * 2 + TETRIS_ROWS * cell + (TETRIS_ROWS - 1) * gap - 10,
            fill=self.colors["board"], radius=18.0,
        )]
        for row in range(TETRIS_ROWS):
            for column in range(TETRIS_COLS):
                settled = self.grid[row][column]
                is_active = (row, column) in current
                is_ghost = (row, column) in ghost and not is_active and settled is None
                fill = (TETRIS_COLORS[settled] if settled in TETRIS_COLORS else
                        active_color if is_active else
                        f"{active_color}66" if is_ghost else self.colors["cell"])
                value = (f"locked {settled}" if settled else
                         f"falling {self.current}" if is_active else
                         f"ghost {self.current}" if is_ghost else "empty")
                nodes.append(_scene_cell(
                    "tetris", grid, row, column, fill=fill,
                    label=f"Row {row + 1}, column {column + 1}", value=value,
                    selected=is_active,
                    stroke=self.colors["grid"], stroke_width=0.8, radius=4.0,
                ))
        for row, column in self.last_lock_cells:
            x, y, width, height = grid.cell_rect(row, column)
            nodes.append(SceneRoundedRect(
                f"tetris-lock-{row}-{column}", x + 1, y + 1, width - 2, height - 2,
                fill=self.colors["gold"], opacity=0.34,
                transition=Scene2DTransition(270, "ease_out_quad"), radius=4.0,
            ))
        for row in self.last_clear_rows:
            _, y, _, height = grid.cell_rect(row, 0)
            nodes.append(SceneRoundedRect(
                f"tetris-clear-{row}", padding, y + 1,
                TETRIS_COLS * cell + (TETRIS_COLS - 1) * gap, height - 2,
                fill=self.colors["gold"], opacity=0.4,
                transition=Scene2DTransition(320, "ease_out_cubic"), radius=4.0,
            ))
        width = padding * 2 + TETRIS_COLS * cell + (TETRIS_COLS - 1) * gap
        height = padding * 2 + TETRIS_ROWS * cell + (TETRIS_ROWS - 1) * gap
        return Scene2D(
            "tetris-board", width, height, nodes, grid,
            Scene2DInputConfig(pointer=True, continuous=False, capture=True,
                               keyboard=True),
            revision=revision,
            semantic=Scene2DSemantic(
                "grid", "Tetris playfield",
                description=(f"{TETRIS_ROWS} rows by {TETRIS_COLS} columns. "
                             "Falling piece is bright; the ghost marks where it will land.")),
        )

    def preview_node(self, *, revision: int = 1) -> Scene2D:
        cell, gap, padding = 34.0, 4.0, 14.0
        grid = GridSpec(4, 4, padding, padding, cell, cell, gap)
        nodes: list[Any] = [SceneRoundedRect(
            "tetris-preview-well", 5, 5, 4 * cell + 3 * gap + 18,
            4 * cell + 3 * gap + 18, fill=self.colors["board"], radius=16.0,
        )]
        if self.next_shape is not None:
            cells = _TETROMINOES[self.next_shape][0]
            top = min(row for row, _ in cells)
            left = min(column for _, column in cells)
            for row, column in cells:
                row, column = row - top, column - left
                x, y, _, _ = grid.cell_rect(row, column)
                nodes.append(SceneRoundedRect(
                    f"tetris-preview-{row}-{column}", x, y, cell, cell,
                    fill=TETRIS_COLORS[self.next_shape], radius=8.0,
                ))
        return Scene2D("tetris-preview", 4 * cell + 3 * gap + 2 * padding,
                       4 * cell + 3 * gap + 2 * padding, nodes,
                       input=Scene2DInputConfig(pointer=False, capture=False, keyboard=False),
                       revision=revision,
                       semantic=Scene2DSemantic(
                           "group", f"Next piece {self.next_shape or 'empty'}"))

    def controls_node(self, *, revision: int = 1) -> Scene2D:
        labels = (("hold-left", "◀", "Move left"),
                  ("hold-right", "▶", "Move right"),
                  ("hold-down", "▼", "Soft drop"),
                  ("hold-rotate", "↻", "Rotate"))
        nodes: list[Any] = []
        width, height, gap = 74.0, 70.0, 10.0
        for index, (action, symbol, accessible) in enumerate(labels):
            x = 6.0 + index * (width + gap)
            nodes.append(SceneRoundedRect(
                f"tetris-control-{action}", x, 6.0, width, height,
                fill=self.colors["cell_alt"], stroke=Stroke(self.colors["grid"], 1.4),
                hit_id=action,
                semantic=Scene2DSemantic("button", accessible,
                                         description="Hold to repeat" if action != "hold-rotate" else "Tap to rotate"),
                radius=16.0,
            ))
            nodes.append(_scene_text(f"tetris-control-icon-{action}",
                                     (x + width / 2, 34.0), symbol, 27.0,
                                     self.colors["text"]))
        return Scene2D(
            "tetris-controls", 4 * width + 3 * gap + 12.0, height + 12.0,
            nodes, input=Scene2DInputConfig(pointer=True, continuous=False, capture=True),
            revision=revision,
            semantic=Scene2DSemantic("group", "Touch controls"),
        )

    def section_node(self) -> ui.Node:
        return ui.vstack([
            ui.section_header(
                "Tetris",
                "Guide the falling tetrominoes; complete rows vanish and score.",
            ),
            ui.wrap([
                ui.card([self.board_node()], title="Well"),
                ui.vstack([
                    ui.card([
                        ui.heading("How to play", level=3),
                        ui.text("Focused keys: ← → move · ↓ soft drop · ↑ rotate · Space drop · P pause."),
                        ui.text("Every 10 lines raises the level and the fall speed."),
                        ui.text("Touch controls support two fingers at once."),
                    ], title="Rules"),
                    ui.card([self.preview_node()], title="Next piece"),
                    ui.hstack([
                        ui.thinking_orb("working", id="tetris-orb", size=48.0,
                                        speed=1.0, paused=True,
                                        aria_label="Tetris running indicator"),
                        ui.vstack([
                            ui.text("Complete rows to score", tone="secondary"),
                            ui.text("Fall speed rises every 10 lines", tone="secondary"),
                        ], gap=2.0),
                    ], gap=12.0),
                    self.controls_node(),
                    ui.wrap([
                        _cell_button("▶ Start", "tetris-btn-start", TETRIS_CONTROL_ACTION),
                        _cell_button("↺ Restart", "tetris-btn-restart", TETRIS_CONTROL_ACTION),
                        _cell_button("🤖 Auto", "tetris-btn-auto", TETRIS_CONTROL_ACTION,
                                     selected=self.auto),
                    ], gap=8.0),
                ], gap=12.0),
            ], gap=20.0),
            ui.wrap([
                ui.metric("Score", str(self.score), id="tetris-score"),
                ui.metric("Lines", str(self.lines), id="tetris-lines"),
                ui.metric("Level", str(self.level), id="tetris-level"),
                ui.metric("Next", self.next_label(), id="tetris-next"),
                _status_badge("tetris-badge", "Ready", "neutral"),
            ], gap=12.0),
            ui.progress(0.0, label="Next level", id="tetris-progress"),
            ui.text("Press Start — or engage Auto and watch.", tone="secondary", id="tetris-status"),
        ], gap=16.0)

    def status_ops(self, message: str | None = None) -> list[dict[str, Any]]:
        if self.over:
            badge, tone = "Game over", "danger"
        elif self.running and self.auto:
            badge, tone = "Autopilot", "success"
        elif self.running:
            badge, tone = "Playing", "accent"
        elif self.current is not None:
            badge, tone = "Paused", "warning"
        else:
            badge, tone = "Ready", "neutral"
        ops = [
            _set("tetris-score", "value", str(self.score)),
            _set("tetris-lines", "value", str(self.lines)),
            _set("tetris-level", "value", str(self.level)),
            _set("tetris-next", "value", self.next_label()),
            _set("tetris-badge", "label", badge),
            _set("tetris-badge", "tone", tone),
            _set("tetris-progress", "value", (self.lines % 10) / 10),
            _set("tetris-orb", "paused", not self.running),
            _set("tetris-btn-start", "label", "⏸ Pause" if self.running else "▶ Start"),
        ]
        if message is not None:
            ops.append(_set("tetris-status", "text", message))
        return ops

    def delta_ops(self, message: str | None = None) -> list[dict[str, Any]]:
        """Patch only the rows whose glyphs changed since the last render."""
        rows = self.render_rows()
        ops: list[dict[str, Any]] = []
        for index, line in enumerate(rows):
            previous = self.last_rows[index] if index < len(self.last_rows) else None
            if line != previous:
                ops.append(_set(self.row_id(index), "text", line))
        self.last_rows = rows
        return [*ops, *self.status_ops(message)]


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
            if node_id.startswith(("zip-cell-", "queens-cell-", "sudoku-cell-", "tetris-row-")):
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
        for game in (self.zip_game, self.queens_game, self.sudoku_game, self.tetris_game):
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
                "tetris" if surface_id in {"tetris-board", "tetris-controls"} else "")

    def _arm_tetris_lock_effect(self, before: tuple[int, int, str | None]) -> None:
        if self.tetris_game.pieces > before[0] and before[2] is not None:
            self.tetris_effect_remaining = 0.32

    @staticmethod
    def _cell_from_pointer(event: Scene2DPointerInput) -> tuple[int, int] | None:
        if event.cell is not None:
            return event.cell.row, event.cell.column
        if event.hit_id is not None:
            for prefix in ("zip-cell", "queens-cell", "sudoku-cell", "tetris-cell"):
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
            for prefix in ("zip-cell", "queens-cell", "sudoku-cell", "tetris-cell"):
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
        sidebar_subtitle="four games, one native window",
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
    )
    app.sections = [
        section("overview", "Overview", _overview_section(app.palette)),
        section("zip", "Zip", app.zip_game.section_node()),
        section("queens", "Queens", app.queens_game.section_node()),
        section("sudoku", "Sudoku", app.sudoku_game.section_node()),
        section("tetris", "Tetris", app.tetris_game.section_node()),
    ]
    # Seed retained state with the exact revision-1 scenes embedded in the
    # initial App IR. Later input publishes consecutive scene revisions in
    # the same context.patch transaction as HUD updates.
    surface_ids = ("zip-board", "queens-board", "sudoku-board", "tetris-board",
                   "tetris-preview", "tetris-controls")
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
