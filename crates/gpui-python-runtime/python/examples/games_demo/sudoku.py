"""Sudoku puzzle logic for the classic-games demo.

Loaded by ``games_demo.py`` via ``games_demo_sudoku``; the application shell lives in that entry file.
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

