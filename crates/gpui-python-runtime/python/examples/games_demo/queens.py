"""Queens crown-placement puzzle logic for the classic-games demo.

Loaded by ``games_demo.py`` via ``games_demo_queens``; the application shell lives in that entry file.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any
from gpui_toolkit import ui
from gpui_toolkit.scene2d import (
    GridSpec,
    PathCommand,
    Scene2D,
    Scene2DInputConfig,
    Scene2DSemantic,
    Scene2DTransition,
    SceneLine,
    ScenePath,
    SceneRoundedRect,
    Stroke,
)
from games_demo_common import (
    QUEENS_REGION_COLORS,
    _PaletteAwareGame,
    _cell_button,
    _replace,
    _scene_cell,
    _set,
    _status_badge,
)


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

