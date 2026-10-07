"""Tetris falling-block logic for the classic-games demo.

Loaded by ``games_demo.py`` via ``games_demo_tetris``; the application shell lives in that entry file.
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
    SceneRoundedRect,
    Stroke,
)
from games_demo_common import (
    TETRIS_COLORS,
    _PaletteAwareGame,
    _cell_button,
    _scene_cell,
    _scene_text,
    _set,
    _status_badge,
)


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

