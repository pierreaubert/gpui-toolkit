"""Othello (Reversi) rules engine, AI, and board UI for the classic-games demo.

Loaded by ``games_demo.py`` via ``games_demo_othello``; the application
shell lives in that entry file.
"""

from __future__ import annotations

import random
from dataclasses import dataclass, field
from typing import Any

from games_demo_common import (
    _cell_button,
    _PaletteAwareGame,
    _replace,
    _scene_cell,
    _set,
    _status_badge,
)
from gpui_toolkit import ui
from gpui_toolkit.scene2d import (
    GridSpec,
    Scene2D,
    Scene2DInputConfig,
    Scene2DSemantic,
    Scene2DTransition,
    SceneCircle,
    SceneRoundedRect,
    Stroke,
)

# ---------------------------------------------------------------------------
# Othello: reversi rules and search ported from thegustafson/ai-othello
# ---------------------------------------------------------------------------
#
# Rules (bitboard semantics, pass-as-a-move, game-over detection), the
# phase-aware evaluation (material, mobility, potential mobility, corners,
# corner danger, frontier discs), and the alpha-beta search with ordered
# moves and exact endgame solving follow thegustafson/ai-othello
# (https://github.com/thegustafson/ai-othello, crate `ai-othello`, MIT).
# Bitboards are replaced with a small 64-square mailbox board so the Python
# demo needs no native dependency.

OTHELLO_CELL_ACTION = "othello_cell"
OTHELLO_CONTROL_ACTION = "othello_control"
OTHELLO_MODE_ACTION = "othello_mode"
OTHELLO_DIFFICULTY_ACTION = "othello_difficulty"

OTHELLO_MODES = (("two", "Two players"), ("ai", "You (Black) vs AI (White)"))
OTHELLO_DIFFICULTIES = (("beginner", "Beginner (depth 1)"),
                        ("easy", "Easy (depth 2)"),
                        ("medium", "Medium (depth 3)"),
                        ("hard", "Hard (depth 4 + endgame)"))
_OTHELLO_DEPTH = {"beginner": 1, "easy": 2, "medium": 3, "hard": 4}
_OTHELLO_NODE_CAP = {"beginner": 3_000, "easy": 12_000, "medium": 40_000,
                     "hard": 120_000}
_OTHELLO_EXACT_EMPTIES = {"beginner": 0, "easy": 0, "medium": 0, "hard": 10}
_OTHELLO_INF = 40_000
_OTHELLO_TERMINAL = 30_000
_OTHELLO_CORNERS = (0, 7, 56, 63)
_OTHELLO_CORNER_DANGER = frozenset({1, 6, 8, 9, 14, 15,
                                    48, 49, 54, 55, 57, 62})
_OTHELLO_RAYS = ((1, 0), (-1, 0), (0, 1), (0, -1),
                 (1, 1), (1, -1), (-1, 1), (-1, -1))
_OTHELLO_FELT = {"dark": "#2E7D4F", "light": "#43A261"}


def _othello_file(sq: int) -> int:
    return sq & 7


def _othello_rank(sq: int) -> int:
    return sq >> 3


def _othello_sq(file: int, rank: int) -> int:
    return rank * 8 + file


def _othello_name(sq: int) -> str:
    return chr(ord("a") + _othello_file(sq)) + str(_othello_rank(sq) + 1)


def _othello_other(side: str) -> str:
    return "W" if side == "B" else "B"


def _othello_side_name(side: str) -> str:
    return "Black" if side == "B" else "White"


class OthelloPosition:
    """Mailbox board with ai-othello move semantics."""

    def __init__(self, board: list[str | None] | None = None,
                 side: str = "B") -> None:
        if board is None:
            board = [None] * 64
            board[28] = "B"
            board[35] = "B"
            board[27] = "W"
            board[36] = "W"
        self.board: list[str | None] = list(board)
        self.side = side

    def discs(self, side: str) -> int:
        return sum(1 for disc in self.board if disc == side)

    def empties(self) -> int:
        return sum(1 for disc in self.board if disc is None)

    def bracket(self, sq: int, side: str) -> list[int]:
        """Discs that placing ``side`` on ``sq`` would flip (may be empty)."""
        if self.board[sq] is not None:
            return []
        enemy = _othello_other(side)
        file, rank = _othello_file(sq), _othello_rank(sq)
        flips: list[int] = []
        for delta_file, delta_rank in _OTHELLO_RAYS:
            run: list[int] = []
            other, target = file + delta_file, rank + delta_rank
            while (0 <= other < 8 and 0 <= target < 8
                   and self.board[_othello_sq(other, target)] == enemy):
                run.append(_othello_sq(other, target))
                other += delta_file
                target += delta_rank
            if (run and 0 <= other < 8 and 0 <= target < 8
                    and self.board[_othello_sq(other, target)] == side):
                flips.extend(run)
        return flips

    def placements_for(self, side: str) -> list[int]:
        return [sq for sq in range(64)
                if self.board[sq] is None and self.bracket(sq, side)]

    def placements(self) -> list[int]:
        return self.placements_for(self.side)

    def apply(self, sq: int) -> dict[str, Any]:
        flips = self.bracket(sq, self.side)
        if not flips:
            raise ValueError(f"placing at {_othello_name(sq)} flips no discs")
        undo = {"sq": sq, "flipped": flips, "side": self.side}
        self.board[sq] = self.side
        for target in flips:
            self.board[target] = self.side
        self.side = _othello_other(self.side)
        return undo

    def apply_pass(self) -> dict[str, Any]:
        if self.placements():
            raise ValueError("pass is only legal when no placement is legal")
        undo = {"sq": None, "flipped": [], "side": self.side}
        self.side = _othello_other(self.side)
        return undo

    def undo(self, undo: dict[str, Any]) -> None:
        self.side = undo["side"]
        sq = undo["sq"]
        if sq is not None:
            self.board[sq] = None
            enemy = _othello_other(self.side)
            for target in undo["flipped"]:
                self.board[target] = enemy

    def result(self) -> tuple[str, str | None, int, int]:
        """Return (kind, winner, black, white); kind is ongoing/draw/win."""
        black = self.discs("B")
        white = self.discs("W")
        if self.placements_for("B") or self.placements_for("W"):
            return ("ongoing", None, black, white)
        if black == white:
            return ("draw", None, black, white)
        winner = "B" if black > white else "W"
        return ("win", winner, black, white)


def _othello_interpolate(opening: int, endgame: int, phase: int) -> int:
    phase = min(max(phase, 0), 60)
    return (opening * (60 - phase) + endgame * phase) // 60


def _othello_empty_neighbours(board: list[str | None], side: str) -> int:
    """Empty squares adjacent to ``side`` discs (potential mobility)."""
    seen: set[int] = set()
    for sq, disc in enumerate(board):
        if disc != side:
            continue
        file, rank = _othello_file(sq), _othello_rank(sq)
        for delta_file, delta_rank in _OTHELLO_RAYS:
            other, target = file + delta_file, rank + delta_rank
            if 0 <= other < 8 and 0 <= target < 8:
                neighbour = _othello_sq(other, target)
                if board[neighbour] is None:
                    seen.add(neighbour)
    return len(seen)


def _othello_frontier(board: list[str | None], side: str) -> int:
    """Own discs adjacent to an empty square."""
    count = 0
    for sq, disc in enumerate(board):
        if disc != side:
            continue
        file, rank = _othello_file(sq), _othello_rank(sq)
        for delta_file, delta_rank in _OTHELLO_RAYS:
            other, target = file + delta_file, rank + delta_rank
            if (0 <= other < 8 and 0 <= target < 8
                    and board[_othello_sq(other, target)] is None):
                count += 1
                break
    return count


def othello_evaluate(pos: OthelloPosition) -> int:
    """Phase-profile evaluation from the side-to-move perspective."""
    us, them = pos.side, _othello_other(pos.side)
    phase = 64 - pos.empties() - 4
    material = pos.discs(us) - pos.discs(them)
    mobility = len(pos.placements_for(us)) - len(pos.placements_for(them))
    potential = (_othello_empty_neighbours(pos.board, them)
                 - _othello_empty_neighbours(pos.board, us))
    corners = (sum(1 for sq in _OTHELLO_CORNERS if pos.board[sq] == us)
               - sum(1 for sq in _OTHELLO_CORNERS if pos.board[sq] == them))
    danger = (sum(1 for sq in _OTHELLO_CORNER_DANGER if pos.board[sq] == us)
              - sum(1 for sq in _OTHELLO_CORNER_DANGER if pos.board[sq] == them))
    frontier = (_othello_frontier(pos.board, us)
                - _othello_frontier(pos.board, them))
    return (material * _othello_interpolate(1, 14, phase)
            + mobility * _othello_interpolate(24, 5, phase)
            + potential * _othello_interpolate(8, 2, phase)
            + corners * 160
            + danger * _othello_interpolate(-45, -5, phase)
            + frontier * _othello_interpolate(-12, -3, phase))


class _OthelloNodeCap(Exception):
    pass


def _othello_ordered(placements: list[int]) -> list[int]:
    corners = [sq for sq in _OTHELLO_CORNERS if sq in placements]
    rest = [sq for sq in placements if sq not in _OTHELLO_CORNERS]
    return corners + rest


def _othello_search(pos: OthelloPosition, depth: int, alpha: int, beta: int,
                    exact_empties: int, budget: list[int], cap: int) -> int:
    budget[0] += 1
    if budget[0] > cap:
        raise _OthelloNodeCap
    kind, winner, black, white = pos.result()
    if kind != "ongoing":
        if kind == "draw":
            return 0
        margin = abs(black - white)
        if winner == pos.side:
            return _OTHELLO_TERMINAL + margin
        return -_OTHELLO_TERMINAL - margin
    exact = 0 < exact_empties and pos.empties() <= exact_empties
    if depth <= 0 and not exact:
        return othello_evaluate(pos)
    placements = _othello_ordered(pos.placements())
    if not placements:
        undo = pos.apply_pass()
        try:
            return -_othello_search(pos, depth, -beta, -alpha,
                                    exact_empties, budget, cap)
        finally:
            pos.undo(undo)
    best = -_OTHELLO_INF
    for sq in placements:
        undo = pos.apply(sq)
        try:
            score = -_othello_search(pos, depth - 1, -beta, -alpha,
                                     exact_empties, budget, cap)
        finally:
            pos.undo(undo)
        if score > best:
            best = score
        if best > alpha:
            alpha = best
        if alpha >= beta:
            break
    return best


def othello_best_move(match: OthelloMatch, difficulty: str,
                       rng: random.Random) -> tuple[int | None, int, int]:
    """Best placement for the side to move; ``None`` means pass."""
    placements = _othello_ordered(match.position.placements())
    if not placements:
        return (None, 0, 0)
    if difficulty not in _OTHELLO_DEPTH:
        raise ValueError(f"unknown othello difficulty {difficulty!r}")
    if difficulty == "beginner" and rng.random() < 0.35:
        # Beginners occasionally drift away from the book move.
        return (rng.choice(placements), 0, 0)
    depth = _OTHELLO_DEPTH[difficulty]
    cap = _OTHELLO_NODE_CAP[difficulty]
    exact_empties = _OTHELLO_EXACT_EMPTIES[difficulty]
    pos = match.position
    budget = [0]
    best_move: int | None = placements[0]
    best_score = -_OTHELLO_INF
    alpha = -_OTHELLO_INF
    try:
        for sq in placements:
            undo = pos.apply(sq)
            try:
                score = -_othello_search(pos, depth - 1, -_OTHELLO_INF, -alpha,
                                         exact_empties, budget, cap)
            finally:
                pos.undo(undo)
            if best_move is None or score > best_score:
                best_score = score
                best_move = sq
            if score > alpha:
                alpha = score
    except _OthelloNodeCap:
        pass
    return (best_move, best_score, budget[0])


class OthelloMatch:
    """Stateful game controller: history, undo, passes, and results."""

    def __init__(self) -> None:
        self.position = OthelloPosition()
        self.history: list[tuple[int | None, str]] = []
        self.undo_stack: list[dict[str, Any]] = []
        self.status = "active"
        self.winner: str | None = None
        self._refresh_status()

    def is_game_over(self) -> bool:
        return self.status == "finished"

    def counts(self) -> tuple[int, int]:
        return (self.position.discs("B"), self.position.discs("W"))

    def place(self, sq: int) -> int:
        if self.is_game_over():
            raise ValueError("game is over")
        undo = self.position.apply(sq)
        self.undo_stack.append(undo)
        self.history.append((sq, undo["side"]))
        self._refresh_status()
        return len(undo["flipped"])

    def do_pass(self) -> str:
        if self.is_game_over():
            raise ValueError("game is over")
        undo = self.position.apply_pass()
        self.undo_stack.append(undo)
        self.history.append((None, undo["side"]))
        self._refresh_status()
        return undo["side"]

    def undo_move(self) -> tuple[int | None, str]:
        if not self.history:
            raise ValueError("nothing to undo")
        entry = self.history.pop()
        undo = self.undo_stack.pop()
        self.position.undo(undo)
        self._refresh_status()
        return entry

    def _refresh_status(self) -> None:
        kind, winner, _black, _white = self.position.result()
        if kind == "ongoing":
            self.status = "active"
            self.winner = None
        else:
            self.status = "finished"
            self.winner = winner

    def moves_text(self) -> str:
        if not self.history:
            return "No moves yet — Black opens in the center."
        parts: list[str] = []
        number = 0
        for sq, side in self.history[-12:]:
            label = _othello_name(sq) if sq is not None else "pass"
            if side == "B":
                number += 1
                parts.append(f"{number}. {label}")
            else:
                parts.append(label)
        text = " ".join(parts)
        return ("… " if len(self.history) > 12 else "") + text


@dataclass
class OthelloGame(_PaletteAwareGame):
    match: OthelloMatch = field(default_factory=OthelloMatch)
    cursor: tuple[int, int] = (2, 3)
    mode: str = "ai"
    difficulty: str = "easy"
    last_move: int | None = None
    hint_move: int | None = None
    moves_made: int = 0
    palette: str = "dark"

    # -- coordinates ----------------------------------------------------

    def _display_to_sq(self, row: int, col: int) -> int:
        return _othello_sq(col, 7 - row)

    def _sq_to_display(self, sq: int) -> tuple[int, int]:
        return (7 - _othello_rank(sq), _othello_file(sq))

    @staticmethod
    def cell_id(row: int, col: int) -> str:
        return f"othello-cell-{row}-{col}"

    # -- rendering ------------------------------------------------------

    def board_node(self, *, revision: int = 1) -> Scene2D:
        pos = self.match.position
        felt = _OTHELLO_FELT[self.palette]
        cell, gap, padding = 66.0, 2.0, 26.0
        grid = GridSpec(8, 8, padding, padding, cell, cell, gap)
        nodes: list[Any] = [SceneRoundedRect(
            "othello-well", 7.0, 7.0,
            padding * 2 + 8 * cell + 7 * gap - 14.0,
            padding * 2 + 8 * cell + 7 * gap - 14.0,
            fill=self.colors["board"], radius=22.0,
        )]
        legal = set(pos.placements()) if not self.match.is_game_over() else set()
        for row in range(8):
            for col in range(8):
                sq = self._display_to_sq(row, col)
                disc = pos.board[sq]
                name = _othello_name(sq)
                if disc == "B":
                    value = "black disc"
                elif disc == "W":
                    value = "white disc"
                elif sq in legal:
                    value = "legal move"
                else:
                    value = "empty"
                stroke = self.colors["grid"]
                stroke_width = 1.0
                if (row, col) == self.cursor:
                    stroke, stroke_width = self.colors["paper"], 2.2
                nodes.append(_scene_cell(
                    "othello", grid, row, col,
                    fill=felt,
                    label=f"Square {name}", value=value,
                    selected=sq == self.last_move,
                    stroke=stroke, stroke_width=stroke_width, radius=6.0,
                ))
                x, y, width, height = grid.cell_rect(row, col)
                center_x, center_y = grid.cell_center(row, col)
                radius = min(width, height) / 2 - 6.0
                if sq == self.last_move:
                    nodes.append(SceneCircle(
                        f"othello-last-{row}-{col}", center_x, center_y,
                        radius + 3.0, fill=None,
                        stroke=Stroke(self.colors["gold"], 3.2),
                    ))
                if sq == self.hint_move:
                    nodes.append(SceneCircle(
                        f"othello-hint-{row}-{col}", center_x, center_y,
                        radius + 3.0, fill=None,
                        stroke=Stroke(self.colors["gold"], 2.4),
                    ))
                    nodes.append(SceneCircle(
                        f"othello-hint-dot-{row}-{col}", center_x, center_y, 8.0,
                        fill=self.colors["gold"],
                    ))
                elif sq in legal:
                    nodes.append(SceneCircle(
                        f"othello-target-{row}-{col}", center_x, center_y, 8.0,
                        fill=self.colors["paper"], opacity=0.75,
                    ))
                if disc is not None:
                    nodes.append(SceneCircle(
                        f"othello-disc-{row}-{col}", center_x, center_y, radius,
                        fill="#161616" if disc == "B" else "#F2F2F2",
                        stroke=Stroke(self.colors["paper"] if disc == "B"
                                      else self.colors["grid"], 1.6),
                        transition=Scene2DTransition(160, "ease_out_quad"),
                    ))
        width = padding * 2 + 8 * cell + 7 * gap
        return Scene2D(
            "othello-board", width, width, nodes, grid,
            Scene2DInputConfig(pointer=True, continuous=False, capture=True,
                               keyboard=True),
            revision=revision,
            semantic=Scene2DSemantic(
                "grid", "Othello board",
                description="Outflank the rival discs. Black moves first."),
        )

    def section_node(self) -> ui.Node:
        return ui.vstack([
            ui.section_header(
                "Othello",
                "Outflank rows of rival discs to flip them. Black moves first; "
                "the AI replies as White.",
            ),
            ui.wrap([
                self.board_node(),
                ui.vstack([
                    ui.card([
                        ui.heading("How to play", level=3),
                        ui.text("Place on a dotted square to bracket rival discs."),
                        ui.text("Arrow keys move the cursor; Enter places. U undoes, P passes."),
                        ui.text("Corners are gold — squares next to them are traps."),
                    ], title="Rules"),
                    ui.select(
                        id="othello-mode", label="Opponent", value=self.mode,
                        options=list(OTHELLO_MODES), action=OTHELLO_MODE_ACTION,
                    ),
                    ui.select(
                        id="othello-difficulty", label="AI strength", value=self.difficulty,
                        options=list(OTHELLO_DIFFICULTIES),
                        action=OTHELLO_DIFFICULTY_ACTION,
                    ),
                    ui.hstack([
                        _cell_button("● New game", "othello-btn-new", OTHELLO_CONTROL_ACTION),
                        _cell_button("↩ Undo", "othello-btn-undo", OTHELLO_CONTROL_ACTION),
                        _cell_button("⏭ Pass", "othello-btn-pass", OTHELLO_CONTROL_ACTION),
                    ], gap=8.0),
                    ui.hstack([
                        _cell_button("💡 Hint", "othello-btn-hint", OTHELLO_CONTROL_ACTION),
                        _cell_button("🤖 AI move", "othello-btn-ai", OTHELLO_CONTROL_ACTION),
                    ], gap=8.0),
                ], gap=12.0),
            ], gap=20.0),
            ui.wrap([
                ui.metric("To move", "Black", id="othello-to-move"),
                ui.metric("Black", "2", id="othello-black"),
                ui.metric("White", "2", id="othello-white"),
                _status_badge("othello-badge", "Ready", "neutral"),
            ], gap=12.0),
            ui.progress(0.0, label="Discs placed", id="othello-progress"),
            ui.text("You play Black; the AI replies as White.",
                    tone="secondary", id="othello-status"),
            ui.text("No moves yet — Black opens in the center.",
                    tone="secondary", id="othello-moves"),
        ], gap=16.0)

    # -- interaction ----------------------------------------------------

    def _game_over_text(self) -> str:
        black, white = self.match.counts()
        if self.match.winner == "B":
            return f"Black wins {black}–{white}."
        if self.match.winner == "W":
            return f"White wins {white}–{black}."
        return f"Draw {black}–{white}."

    def _side_to_move_text(self) -> str:
        return f"{_othello_side_name(self.match.position.side)} to move."

    def status_ops(self, message: str | None = None) -> list[dict[str, Any]]:
        match = self.match
        black, white = match.counts()
        if match.is_game_over():
            if match.winner is None:
                badge, tone = f"Draw {black}–{white}", "neutral"
            else:
                winner = _othello_side_name(match.winner)
                badge, tone = f"{winner} wins {max(black, white)}–{min(black, white)}", "success"
        elif match.history:
            badge, tone = f"{self._side_to_move_text()}", "accent"
        else:
            badge, tone = "Ready", "neutral"
        ops = [
            _set("othello-to-move", "value",
                 _othello_side_name(match.position.side)),
            _set("othello-black", "value", str(black)),
            _set("othello-white", "value", str(white)),
            _set("othello-badge", "label", badge),
            _set("othello-badge", "tone", tone),
            _set("othello-progress", "value", (black + white) / 64.0),
            _set("othello-moves", "text", match.moves_text()),
        ]
        if message is not None:
            ops.append(_set("othello-status", "text", message))
        return ops

    def _settle_passes(self, notes: list[str]) -> None:
        while (not self.match.is_game_over()
               and not self.match.position.placements()):
            side = self.match.do_pass()
            notes.append(f"{_othello_side_name(side)} has no move and passes.")

    def _ai_replies(self, notes: list[str], rng: random.Random,
                    ai_side: str = "W") -> None:
        while (not self.match.is_game_over()
               and self.match.position.side == ai_side):
            move, _score, _nodes = othello_best_move(self.match, self.difficulty,
                                                     rng)
            if move is None:
                self._settle_passes(notes)
                continue
            flips = self.match.place(move)
            self.moves_made += 1
            self.last_move = move
            notes.append(f"AI played {_othello_name(move)} "
                         f"(flipping {flips}).")
            self._settle_passes(notes)

    def click(self, row: int, col: int,
              rng: random.Random | None = None) -> list[dict[str, Any]]:
        match = self.match
        if match.is_game_over():
            return self.status_ops("Game over — press New game to play again.")
        if not 0 <= row < 8 or not 0 <= col < 8:
            return self.status_ops()
        if self.mode == "ai" and match.position.side != "B":
            return self.status_ops("The AI plays White — switch to Two players "
                                   "to move both sides.")
        rng = rng or random.Random()
        sq = self._display_to_sq(row, col)
        if sq not in match.position.placements():
            if match.position.board[sq] is not None:
                return self.status_ops(f"{_othello_name(sq)} is occupied — "
                                       "pick a dotted square.")
            return self.status_ops(f"{_othello_name(sq)} brackets nothing — "
                                   "pick a dotted square.")
        flips = match.place(sq)
        self.moves_made += 1
        self.last_move = sq
        self.hint_move = None
        notes = [f"Placed {_othello_name(sq)} (flipping {flips})."]
        self._settle_passes(notes)
        if match.is_game_over():
            return self.status_ops(" ".join(notes) + f" {self._game_over_text()}")
        if self.mode == "ai":
            self._ai_replies(notes, rng)
            if match.is_game_over():
                return self.status_ops(" ".join(notes) + f" {self._game_over_text()}")
        return self.status_ops(" ".join(notes) + f" {self._side_to_move_text()}")

    def control(self, name: str, rng: random.Random) -> list[dict[str, Any]]:
        if name == "new":
            self.match = OthelloMatch()
            self.last_move = None
            self.hint_move = None
            self.moves_made = 0
            self.cursor = self._sq_to_display(19)
            return [_replace("othello-board", self.board_node()),
                    *self.status_ops("New game — Black to move.")]
        if name == "undo":
            return self._undo()
        if name == "pass":
            if self.match.is_game_over():
                return self.status_ops("Game over — press New game to play again.")
            if self.match.position.placements():
                count = len(self.match.position.placements())
                return self.status_ops(f"{count} legal move(s) available — "
                                       "pass is not allowed.")
            side = self.match.do_pass()
            self.hint_move = None
            notes = [f"{_othello_side_name(side)} passes."]
            self._settle_passes(notes)
            if self.match.is_game_over():
                return self.status_ops(" ".join(notes) + f" {self._game_over_text()}")
            if self.mode == "ai":
                self._ai_replies(notes, rng)
                if self.match.is_game_over():
                    return self.status_ops(" ".join(notes)
                                           + f" {self._game_over_text()}")
            return self.status_ops(" ".join(notes) + f" {self._side_to_move_text()}")
        if name == "hint":
            if self.match.is_game_over():
                return self.status_ops("Game over — press New game to play again.")
            move, _score, _nodes = othello_best_move(self.match, self.difficulty,
                                                     rng)
            if move is None:
                return self.status_ops("Hint: pass — no placement is legal.")
            self.hint_move = move
            flips = len(self.match.position.bracket(move, self.match.position.side))
            return self.status_ops(f"Hint: {_othello_name(move)} "
                                   f"(flipping {flips}).")
        if name == "ai":
            if self.match.is_game_over():
                return self.status_ops("Game over — press New game to play again.")
            move, _score, _nodes = othello_best_move(self.match, self.difficulty,
                                                     rng)
            notes: list[str] = []
            if move is None:
                self._settle_passes(notes)
            else:
                flips = self.match.place(move)
                self.moves_made += 1
                self.last_move = move
                self.hint_move = None
                notes.append(f"AI played {_othello_name(move)} "
                             f"(flipping {flips}).")
                self._settle_passes(notes)
            if self.match.is_game_over():
                return self.status_ops(" ".join(notes) + f" {self._game_over_text()}")
            return self.status_ops(" ".join(notes) + f" {self._side_to_move_text()}")
        raise ValueError(f"unknown othello control {name!r}")

    def _undo(self) -> list[dict[str, Any]]:
        if not self.match.history:
            return self.status_ops("Nothing to undo.")
        undone = 0
        while self.match.history:
            self.match.undo_move()
            undone += 1
            self.moves_made = max(0, self.moves_made - 1)
            if self.mode != "ai" or self.match.position.side == "B":
                break
            if undone > 66:
                break
        self.hint_move = None
        if self.match.history:
            last_sq = self.match.history[-1][0]
            self.last_move = last_sq
        else:
            self.last_move = None
        plural = "move" if undone == 1 else "moves"
        return self.status_ops(f"Undid {undone} {plural}. "
                               f"{self._side_to_move_text()}")

    def select_mode(self, value: str) -> list[dict[str, Any]]:
        if value not in ("two", "ai"):
            raise ValueError(f"unknown othello mode {value!r}")
        self.mode = value
        self.hint_move = None
        if value == "ai" and not self.match.is_game_over():
            notes: list[str] = []
            self._settle_passes(notes)
            self._ai_replies(notes, random.Random())
            if notes:
                if self.match.is_game_over():
                    return self.status_ops(" ".join(notes)
                                           + f" {self._game_over_text()}")
                return self.status_ops(" ".join(notes)
                                       + f" {self._side_to_move_text()}")
            return self.status_ops("You play Black; the AI replies as White.")
        return self.status_ops("Two players — move both sides.")

    def select_difficulty(self, value: str) -> list[dict[str, Any]]:
        if value not in _OTHELLO_DEPTH:
            raise ValueError(f"unknown othello difficulty {value!r}")
        self.difficulty = value
        label = next(label for key, label in OTHELLO_DIFFICULTIES if key == value)
        return self.status_ops(f"AI strength: {label}.")
