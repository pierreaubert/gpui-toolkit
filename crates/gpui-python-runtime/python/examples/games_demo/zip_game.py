"""Zip path-covering puzzle logic for the classic-games demo.

Loaded by ``games_demo.py`` via ``games_demo_zip_game``; the application shell lives in that entry file.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any
from gpui_toolkit import ui
from gpui_toolkit.scene2d import (
    GridSpec,
    Scene2D,
    Scene2DInputConfig,
    Scene2DSemantic,
    Scene2DTransition,
    SceneCircle,
    SceneLine,
    SceneRoundedRect,
    Stroke,
)
from games_demo_common import (
    _PaletteAwareGame,
    _cell_button,
    _replace,
    _scene_cell,
    _scene_text,
    _set,
    _status_badge,
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
                        _cell_button("Next →", "zip-btn-next", ZIP_CONTROL_ACTION),
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
            return self.status_ops("Already solved — press Next for another puzzle, or Reset to draw again.")
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
        if name == "next":
            self.reset((self.level_index + 1) % len(ZIP_LEVELS))
            level = self.level
            return [_replace("zip-board", self.board_node()),
                    *self.status_ops(f"{level.label}: cover {level.cell_count} cells in order.")]
        raise ValueError(f"unknown zip control {name!r}")

    def select_level(self, level_id: str) -> list[dict[str, Any]]:
        for index, candidate in enumerate(ZIP_LEVELS):
            if candidate.id == level_id:
                self.reset(index)
                return [_replace("zip-board", self.board_node()),
                        *self.status_ops(f"{candidate.label}: cover {candidate.cell_count} cells in order.")]
        raise ValueError(f"unknown zip level {level_id!r}")

