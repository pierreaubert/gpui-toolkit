"""Chess rules engine, AI, and board UI for the classic-games demo.

Loaded by ``games_demo.py`` via ``games_demo_chess``; the application shell lives in that entry file.
"""

from __future__ import annotations

import random

from dataclasses import dataclass, field
from typing import Any
from gpui_toolkit import ui
from gpui_toolkit.scene2d import (
    GridSpec,
    Scene2D,
    Scene2DInputConfig,
    Scene2DSemantic,
    Scene2DShadow,
    SceneCircle,
    SceneRoundedRect,
    SceneText,
    Stroke,
)
from games_demo_common import (
    _PaletteAwareGame,
    _cell_button,
    _replace,
    _scene_cell,
    _set,
    _status_badge,
)


# ---------------------------------------------------------------------------
# Chess: full-rules game with an AI opponent, ported from rust-chess
# ---------------------------------------------------------------------------
#
# Rules, the move-generation pipeline (pseudo-legal generation plus a
# make-and-check legality filter), FEN/SAN/PGN handling, draw detection, and
# the AI evaluation (material + piece-square tables + bishop-pair bonus with
# MVV-LVA-ordered negamax search) follow RumenDamyanov/rust-chess
# (https://github.com/RumenDamyanov/rust-chess, crate `rumenx-chess`, MIT).
# The bitboard backend is replaced with a small 64-square mailbox board so
# the Python demo needs no native dependency.

CHESS_CELL_ACTION = "chess_cell"
CHESS_CONTROL_ACTION = "chess_control"
CHESS_MODE_ACTION = "chess_mode"
CHESS_DIFFICULTY_ACTION = "chess_difficulty"
CHESS_PROMOTION_ACTION = "chess_promotion"

CHESS_START_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"

CHESS_GLYPHS = {
    ("w", "K"): "♔", ("w", "Q"): "♕", ("w", "R"): "♖",
    ("w", "B"): "♗", ("w", "N"): "♘", ("w", "P"): "♙",
    ("b", "K"): "♚", ("b", "Q"): "♛", ("b", "R"): "♜",
    ("b", "B"): "♝", ("b", "N"): "♞", ("b", "P"): "♟",
}
CHESS_PIECE_NAMES = {"P": "pawn", "N": "knight", "B": "bishop",
                     "R": "rook", "Q": "queen", "K": "king"}
CHESS_MODES = (("two", "Two players"), ("ai", "You (White) vs AI (Black)"))
CHESS_DIFFICULTIES = (("harmless", "Harmless (random)"),
                      ("easy", "Easy (depth 1)"),
                      ("medium", "Medium (depth 2)"),
                      ("hard", "Hard (depth 3)"))
CHESS_PROMOTIONS = (("Q", "Queen"), ("R", "Rook"),
                    ("B", "Bishop"), ("N", "Knight"))
_CHESS_DEPTH = {"harmless": 0, "easy": 1, "medium": 2, "hard": 3}
_CHESS_NODE_CAP = {"harmless": 0, "easy": 8_000, "medium": 30_000,
                   "hard": 80_000}
_CHESS_MATE = 90_000
_CHESS_INF = 100_000

# Piece-square tables from rust-chess `src/ai/evaluation.rs` (White's
# perspective, LERF order a1=0 .. h8=63).
_CHESS_PAWN_PST = (
    0, 0, 0, 0, 0, 0, 0, 0,
    5, 10, 10, -20, -20, 10, 10, 5,
    5, -5, -10, 0, 0, -10, -5, 5,
    0, 0, 0, 20, 20, 0, 0, 0,
    5, 5, 10, 25, 25, 10, 5, 5,
    10, 10, 20, 30, 30, 20, 10, 10,
    50, 50, 50, 50, 50, 50, 50, 50,
    0, 0, 0, 0, 0, 0, 0, 0,
)
_CHESS_KNIGHT_PST = (
    -50, -40, -30, -30, -30, -30, -40, -50,
    -40, -20, 0, 5, 5, 0, -20, -40,
    -30, 5, 10, 15, 15, 10, 5, -30,
    -30, 0, 15, 20, 20, 15, 0, -30,
    -30, 5, 15, 20, 20, 15, 5, -30,
    -30, 0, 10, 15, 15, 10, 0, -30,
    -40, -20, 0, 0, 0, 0, -20, -40,
    -50, -40, -30, -30, -30, -30, -40, -50,
)
_CHESS_BISHOP_PST = (
    -20, -10, -10, -10, -10, -10, -10, -20,
    -10, 5, 0, 0, 0, 0, 5, -10,
    -10, 10, 10, 10, 10, 10, 10, -10,
    -10, 0, 10, 10, 10, 10, 0, -10,
    -10, 5, 5, 10, 10, 5, 5, -10,
    -10, 0, 5, 10, 10, 5, 0, -10,
    -10, 0, 0, 0, 0, 0, 0, -10,
    -20, -10, -10, -10, -10, -10, -10, -20,
)
_CHESS_ROOK_PST = (
    0, 0, 0, 5, 5, 0, 0, 0,
    -5, 0, 0, 0, 0, 0, 0, -5,
    -5, 0, 0, 0, 0, 0, 0, -5,
    -5, 0, 0, 0, 0, 0, 0, -5,
    -5, 0, 0, 0, 0, 0, 0, -5,
    -5, 0, 0, 0, 0, 0, 0, -5,
    5, 10, 10, 10, 10, 10, 10, 5,
    0, 0, 0, 0, 0, 0, 0, 0,
)
_CHESS_QUEEN_PST = (
    -20, -10, -10, -5, -5, -10, -10, -20,
    -10, 0, 5, 0, 0, 0, 0, -10,
    -10, 5, 5, 5, 5, 5, 0, -10,
    0, 0, 5, 5, 5, 5, 0, -5,
    -5, 0, 5, 5, 5, 5, 0, -5,
    -10, 0, 5, 5, 5, 5, 0, -10,
    -10, 0, 0, 0, 0, 0, 0, -10,
    -20, -10, -10, -5, -5, -10, -10, -20,
)
_CHESS_KING_PST = (
    20, 30, 10, 0, 0, 10, 30, 20,
    20, 20, 0, 0, 0, 0, 20, 20,
    -10, -20, -20, -20, -20, -20, -20, -10,
    -20, -30, -30, -40, -40, -30, -30, -20,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
)
_CHESS_PST = {"P": _CHESS_PAWN_PST, "N": _CHESS_KNIGHT_PST,
              "B": _CHESS_BISHOP_PST, "R": _CHESS_ROOK_PST,
              "Q": _CHESS_QUEEN_PST, "K": _CHESS_KING_PST}
_CHESS_PIECE_VALUE = {"P": 100, "N": 320, "B": 330,
                      "R": 500, "Q": 900, "K": 0}

_CHESS_KNIGHT_STEPS = ((1, 2), (2, 1), (2, -1), (1, -2),
                       (-1, -2), (-2, -1), (-2, 1), (-1, 2))
_CHESS_KING_STEPS = ((1, 0), (1, 1), (0, 1), (-1, 1),
                     (-1, 0), (-1, -1), (0, -1), (1, -1))
_CHESS_BISHOP_RAYS = ((1, 1), (1, -1), (-1, 1), (-1, -1))
_CHESS_ROOK_RAYS = ((1, 0), (-1, 0), (0, 1), (0, -1))
_CHESS_SQUARES = {"dark": ("#E4DCC8", "#4E6E5D"),
                  "light": ("#FFFFFF", "#8CAEA0")}


def _chess_file(sq: int) -> int:
    return sq & 7


def _chess_rank(sq: int) -> int:
    return sq >> 3


def _chess_sq(file: int, rank: int) -> int:
    return rank * 8 + file


def _chess_algebraic(sq: int) -> str:
    return chr(ord("a") + _chess_file(sq)) + str(_chess_rank(sq) + 1)


def _chess_from_algebraic(name: str) -> int | None:
    if len(name) != 2:
        return None
    file = ord(name[0]) - ord("a")
    rank = ord(name[1]) - ord("1")
    if 0 <= file < 8 and 0 <= rank < 8:
        return _chess_sq(file, rank)
    return None


@dataclass(frozen=True)
class ChessMove:
    """From/to squares plus promotion and move flags (mirrors rust-chess)."""

    from_sq: int
    to_sq: int
    promotion: str | None = None
    capture: bool = False
    en_passant: bool = False
    castling: bool = False
    double_push: bool = False

    def uci(self) -> str:
        text = _chess_algebraic(self.from_sq) + _chess_algebraic(self.to_sq)
        if self.promotion:
            text += self.promotion.lower()
        return text


class ChessPosition:
    """Mailbox board with rust-chess move semantics and FEN support."""

    def __init__(self, board: list[tuple[str, str] | None] | None = None,
                 side: str = "w", castling: set[str] | frozenset[str] | None = None,
                 en_passant: int | None = None, halfmove: int = 0,
                 fullmove: int = 1) -> None:
        self.board: list[tuple[str, str] | None] = (
            list(board) if board is not None else [None] * 64)
        self.side = side
        self.castling: set[str] = (
            set(castling) if castling is not None else {"K", "Q", "k", "q"})
        self.en_passant = en_passant
        self.halfmove = halfmove
        self.fullmove = fullmove

    @staticmethod
    def starting() -> ChessPosition:
        pos = ChessPosition()
        back = ("R", "N", "B", "Q", "K", "B", "N", "R")
        for file, kind in enumerate(back):
            pos.board[_chess_sq(file, 0)] = ("w", kind)
            pos.board[_chess_sq(file, 7)] = ("b", kind)
        for file in range(8):
            pos.board[_chess_sq(file, 1)] = ("w", "P")
            pos.board[_chess_sq(file, 6)] = ("b", "P")
        return pos

    @staticmethod
    def from_fen(fen: str) -> ChessPosition:
        parts = fen.split()
        if len(parts) != 6:
            raise ValueError(f"malformed FEN {fen!r}")
        placement, side, castling, ep, half, full = parts
        board: list[tuple[str, str] | None] = [None] * 64
        rank, file = 7, 0
        for char in placement:
            if char == "/":
                if file != 8:
                    raise ValueError(f"malformed FEN rank in {fen!r}")
                rank -= 1
                file = 0
                continue
            if char.isdigit():
                file += int(char)
                if file > 8:
                    raise ValueError(f"malformed FEN rank in {fen!r}")
                continue
            kind = char.upper()
            if kind not in CHESS_PIECE_NAMES:
                raise ValueError(f"malformed FEN piece {char!r}")
            if not (0 <= file < 8 and 0 <= rank < 8):
                raise ValueError(f"malformed FEN placement in {fen!r}")
            board[_chess_sq(file, rank)] = ("w" if char.isupper() else "b", kind)
            file += 1
        if side not in ("w", "b"):
            raise ValueError(f"malformed FEN side {side!r}")
        rights = set() if castling == "-" else set(castling)
        if not rights.issubset({"K", "Q", "k", "q"}):
            raise ValueError(f"malformed FEN castling {castling!r}")
        en_passant = None
        if ep != "-":
            en_passant = _chess_from_algebraic(ep)
            if en_passant is None:
                raise ValueError(f"malformed FEN en passant {ep!r}")
        try:
            halfmove, fullmove = int(half), int(full)
        except ValueError:
            raise ValueError(f"malformed FEN clocks in {fen!r}") from None
        return ChessPosition(board, side, rights, en_passant, halfmove, fullmove)

    def to_fen(self) -> str:
        rows: list[str] = []
        for rank in range(7, -1, -1):
            empty = 0
            text = ""
            for file in range(8):
                piece = self.board[_chess_sq(file, rank)]
                if piece is None:
                    empty += 1
                    continue
                if empty:
                    text += str(empty)
                    empty = 0
                color, kind = piece
                text += kind if color == "w" else kind.lower()
            if empty:
                text += str(empty)
            rows.append(text)
        castling = "".join(flag for flag in "KQkq" if flag in self.castling) or "-"
        ep = "-" if self.en_passant is None else _chess_algebraic(self.en_passant)
        return (f"{'/'.join(rows)} {self.side} {castling} {ep} "
                f"{self.halfmove} {self.fullmove}")

    def king_sq(self, color: str) -> int:
        for sq, piece in enumerate(self.board):
            if piece == (color, "K"):
                return sq
        raise ValueError(f"missing {color} king")

    def is_attacked(self, sq: int, by: str) -> bool:
        file, rank = _chess_file(sq), _chess_rank(sq)
        if by == "w":
            for delta in (-1, 1):
                other, below = file + delta, rank - 1
                if (0 <= other < 8 and 0 <= below < 8
                        and self.board[_chess_sq(other, below)] == ("w", "P")):
                    return True
        else:
            for delta in (-1, 1):
                other, above = file + delta, rank + 1
                if (0 <= other < 8 and 0 <= above < 8
                        and self.board[_chess_sq(other, above)] == ("b", "P")):
                    return True
        for delta_file, delta_rank in _CHESS_KNIGHT_STEPS:
            other, target = file + delta_file, rank + delta_rank
            if (0 <= other < 8 and 0 <= target < 8
                    and self.board[_chess_sq(other, target)] == (by, "N")):
                return True
        for delta_file, delta_rank in _CHESS_KING_STEPS:
            other, target = file + delta_file, rank + delta_rank
            if (0 <= other < 8 and 0 <= target < 8
                    and self.board[_chess_sq(other, target)] == (by, "K")):
                return True
        for rays, kinds in ((_CHESS_BISHOP_RAYS, ("B", "Q")),
                            (_CHESS_ROOK_RAYS, ("R", "Q"))):
            for delta_file, delta_rank in rays:
                other, target = file + delta_file, rank + delta_rank
                while 0 <= other < 8 and 0 <= target < 8:
                    piece = self.board[_chess_sq(other, target)]
                    if piece is not None:
                        if piece[0] == by and piece[1] in kinds:
                            return True
                        break
                    other += delta_file
                    target += delta_rank
        return False

    def is_in_check(self) -> bool:
        return self.is_attacked(self.king_sq(self.side),
                               "b" if self.side == "w" else "w")

    def pseudo_moves(self) -> list[ChessMove]:
        moves: list[ChessMove] = []
        us = self.side
        them = "b" if us == "w" else "w"
        push = 8 if us == "w" else -8
        step = 1 if us == "w" else -1
        start_rank = 1 if us == "w" else 6
        promo_rank = 6 if us == "w" else 1
        for from_sq, piece in enumerate(self.board):
            if piece is None or piece[0] != us:
                continue
            kind = piece[1]
            file, rank = _chess_file(from_sq), _chess_rank(from_sq)
            if kind == "P":
                if not 0 <= rank + step < 8:
                    continue
                target = from_sq + push
                if self.board[target] is None:
                    if rank == promo_rank:
                        for promo in ("Q", "R", "B", "N"):
                            moves.append(ChessMove(from_sq, target, promotion=promo))
                    else:
                        moves.append(ChessMove(from_sq, target))
                    if rank == start_rank and self.board[from_sq + 2 * push] is None:
                        moves.append(ChessMove(from_sq, from_sq + 2 * push,
                                               double_push=True))
                for delta in (-1, 1):
                    other = file + delta
                    if not 0 <= other < 8:
                        continue
                    capture_sq = _chess_sq(other, rank + step)
                    victim = self.board[capture_sq]
                    if victim is not None and victim[0] == them:
                        if rank == promo_rank:
                            for promo in ("Q", "R", "B", "N"):
                                moves.append(ChessMove(from_sq, capture_sq,
                                                       promotion=promo, capture=True))
                        else:
                            moves.append(ChessMove(from_sq, capture_sq, capture=True))
                if self.en_passant is not None:
                    ep_file = _chess_file(self.en_passant)
                    ep_rank = _chess_rank(self.en_passant)
                    if abs(ep_file - file) == 1 and ep_rank == rank + step:
                        moves.append(ChessMove(from_sq, self.en_passant,
                                               capture=True, en_passant=True))
            elif kind in ("N", "K"):
                steps = (_CHESS_KNIGHT_STEPS if kind == "N"
                         else _CHESS_KING_STEPS)
                for delta_file, delta_rank in steps:
                    other, target_rank = file + delta_file, rank + delta_rank
                    if not (0 <= other < 8 and 0 <= target_rank < 8):
                        continue
                    target = _chess_sq(other, target_rank)
                    victim = self.board[target]
                    if victim is None:
                        moves.append(ChessMove(from_sq, target))
                    elif victim[0] == them:
                        moves.append(ChessMove(from_sq, target, capture=True))
            else:
                if kind == "B":
                    rays = _CHESS_BISHOP_RAYS
                elif kind == "R":
                    rays = _CHESS_ROOK_RAYS
                else:
                    rays = _CHESS_BISHOP_RAYS + _CHESS_ROOK_RAYS
                for delta_file, delta_rank in rays:
                    other, target_rank = file + delta_file, rank + delta_rank
                    while 0 <= other < 8 and 0 <= target_rank < 8:
                        target = _chess_sq(other, target_rank)
                        victim = self.board[target]
                        if victim is None:
                            moves.append(ChessMove(from_sq, target))
                        else:
                            if victim[0] == them:
                                moves.append(ChessMove(from_sq, target, capture=True))
                            break
                        other += delta_file
                        target_rank += delta_rank
        self._castling_moves(moves)
        return moves

    def _castling_moves(self, moves: list[ChessMove]) -> None:
        us = self.side
        them = "b" if us == "w" else "w"
        if us == "w":
            if self.board[4] != ("w", "K") or self.is_attacked(4, them):
                return
            if ("K" in self.castling and self.board[7] == ("w", "R")
                    and self.board[5] is None and self.board[6] is None
                    and not self.is_attacked(5, them)
                    and not self.is_attacked(6, them)):
                moves.append(ChessMove(4, 6, castling=True))
            if ("Q" in self.castling and self.board[0] == ("w", "R")
                    and self.board[1] is None and self.board[2] is None
                    and self.board[3] is None
                    and not self.is_attacked(2, them)
                    and not self.is_attacked(3, them)):
                moves.append(ChessMove(4, 2, castling=True))
        else:
            if self.board[60] != ("b", "K") or self.is_attacked(60, them):
                return
            if ("k" in self.castling and self.board[63] == ("b", "R")
                    and self.board[61] is None and self.board[62] is None
                    and not self.is_attacked(61, them)
                    and not self.is_attacked(62, them)):
                moves.append(ChessMove(60, 62, castling=True))
            if ("q" in self.castling and self.board[56] == ("b", "R")
                    and self.board[57] is None and self.board[58] is None
                    and self.board[59] is None
                    and not self.is_attacked(58, them)
                    and not self.is_attacked(59, them)):
                moves.append(ChessMove(60, 58, castling=True))

    def legal_moves(self) -> list[ChessMove]:
        us = self.side
        legal: list[ChessMove] = []
        for move in self.pseudo_moves():
            undo = self.make_move(move)
            if not self.is_attacked(self.king_sq(us), self.side):
                legal.append(move)
            self.undo_move(move, undo)
        return legal

    def moves_from(self, sq: int) -> list[ChessMove]:
        return [move for move in self.legal_moves() if move.from_sq == sq]

    def make_move(self, move: ChessMove) -> dict[str, Any]:
        us = self.side
        them = "b" if us == "w" else "w"
        moving = self.board[move.from_sq]
        if moving is None or moving[0] != us:
            raise ValueError(f"no {us} piece on {_chess_algebraic(move.from_sq)}")
        undo: dict[str, Any] = {
            "moving": moving[1],
            "captured": None,
            "captured_sq": None,
            "castling": set(self.castling),
            "en_passant": self.en_passant,
            "halfmove": self.halfmove,
        }
        if move.en_passant:
            captured_sq = move.to_sq - 8 if us == "w" else move.to_sq + 8
            undo["captured"] = self.board[captured_sq]
            undo["captured_sq"] = captured_sq
            self.board[captured_sq] = None
        elif move.capture:
            undo["captured"] = self.board[move.to_sq]
            undo["captured_sq"] = move.to_sq
        self.board[move.from_sq] = None
        self.board[move.to_sq] = (us, move.promotion or moving[1])
        if move.castling:
            if move.to_sq == 6:
                self.board[7] = None
                self.board[5] = (us, "R")
            elif move.to_sq == 2:
                self.board[0] = None
                self.board[3] = (us, "R")
            elif move.to_sq == 62:
                self.board[63] = None
                self.board[61] = (us, "R")
            elif move.to_sq == 58:
                self.board[56] = None
                self.board[59] = (us, "R")
        if moving[1] == "K":
            self.castling.discard("K" if us == "w" else "k")
            self.castling.discard("Q" if us == "w" else "q")
        for home, right in ((0, "Q"), (7, "K"), (56, "q"), (63, "k")):
            if move.from_sq == home or move.to_sq == home:
                self.castling.discard(right)
        if move.double_push:
            self.en_passant = move.from_sq + 8 if us == "w" else move.from_sq - 8
        else:
            self.en_passant = None
        if moving[1] == "P" or undo["captured"] is not None:
            self.halfmove = 0
        else:
            self.halfmove += 1
        if us == "b":
            self.fullmove += 1
        self.side = them
        return undo

    def undo_move(self, move: ChessMove, undo: dict[str, Any]) -> None:
        us = "b" if self.side == "w" else "w"
        self.side = us
        if us == "b":
            self.fullmove -= 1
        if move.castling:
            if move.to_sq == 6:
                self.board[5] = None
                self.board[7] = (us, "R")
            elif move.to_sq == 2:
                self.board[3] = None
                self.board[0] = (us, "R")
            elif move.to_sq == 62:
                self.board[61] = None
                self.board[63] = (us, "R")
            elif move.to_sq == 58:
                self.board[59] = None
                self.board[56] = (us, "R")
        self.board[move.to_sq] = None
        self.board[move.from_sq] = (us, undo["moving"])
        if undo["captured"] is not None:
            self.board[undo["captured_sq"]] = undo["captured"]
        self.castling = set(undo["castling"])
        self.en_passant = undo["en_passant"]
        self.halfmove = undo["halfmove"]

    def position_key(self) -> tuple[Any, ...]:
        return (tuple(self.board), self.side, tuple(sorted(self.castling)),
                self.en_passant)


def _chess_move_to_san(pos: ChessPosition, move: ChessMove,
                       legal: list[ChessMove]) -> str:
    """Standard Algebraic Notation without the check/mate suffix."""
    if move.castling:
        return ("O-O" if _chess_file(move.to_sq) > _chess_file(move.from_sq)
                else "O-O-O")
    piece = pos.board[move.from_sq]
    if piece is None:
        raise ValueError("SAN: no piece on from square")
    kind = piece[1]
    if kind == "P":
        san = ""
        if move.capture:
            san += chr(ord("a") + _chess_file(move.from_sq)) + "x"
        san += _chess_algebraic(move.to_sq)
        if move.promotion:
            san += "=" + move.promotion
        return san
    san = kind
    others = [candidate for candidate in legal
              if candidate.to_sq == move.to_sq
              and candidate.from_sq != move.from_sq
              and not candidate.castling
              and pos.board[candidate.from_sq] == (pos.side, kind)]
    if others:
        same_file = any(_chess_file(candidate.from_sq) == _chess_file(move.from_sq)
                        for candidate in others)
        same_rank = any(_chess_rank(candidate.from_sq) == _chess_rank(move.from_sq)
                        for candidate in others)
        if not same_file:
            san += chr(ord("a") + _chess_file(move.from_sq))
        elif not same_rank:
            san += str(_chess_rank(move.from_sq) + 1)
        else:
            san += _chess_algebraic(move.from_sq)
    if move.capture:
        san += "x"
    san += _chess_algebraic(move.to_sq)
    return san


def _chess_insufficient_material(pos: ChessPosition) -> bool:
    """KvK, K+minor v K, and KB v KB with same-colour bishops are draws."""
    minors: dict[str, list[tuple[int, str]]] = {"w": [], "b": []}
    for sq, piece in enumerate(pos.board):
        if piece is None:
            continue
        color, kind = piece
        if kind in ("P", "R", "Q"):
            return False
        if kind in ("N", "B"):
            minors[color].append((sq, kind))
    if not minors["w"] and not minors["b"]:
        return True
    if len(minors["w"]) + len(minors["b"]) == 1:
        return True
    if (len(minors["w"]) == 1 and len(minors["b"]) == 1
            and minors["w"][0][1] == "B" and minors["b"][0][1] == "B"):
        white_colour = (_chess_file(minors["w"][0][0])
                        + _chess_rank(minors["w"][0][0])) & 1
        black_colour = (_chess_file(minors["b"][0][0])
                        + _chess_rank(minors["b"][0][0])) & 1
        return white_colour == black_colour
    return False


class ChessMatch:
    """Stateful game controller: history, undo, repetition, and status."""

    def __init__(self, fen: str | None = None) -> None:
        self.position = (ChessPosition.from_fen(fen) if fen is not None
                         else ChessPosition.starting())
        self.history: list[tuple[ChessMove, str]] = []
        self.undo_stack: list[dict[str, Any]] = []
        self.counts: dict[tuple[Any, ...], int] = {}
        self._record_position()
        self.status = "active"
        self.draw_reason = ""
        self._refresh_status()

    def _record_position(self) -> None:
        key = self.position.position_key()
        self.counts[key] = self.counts.get(key, 0) + 1

    def _unrecord_position(self) -> None:
        key = self.position.position_key()
        left = self.counts.get(key, 1) - 1
        if left <= 0:
            self.counts.pop(key, None)
        else:
            self.counts[key] = left

    def legal_moves(self) -> list[ChessMove]:
        return self.position.legal_moves()

    def moves_from(self, sq: int) -> list[ChessMove]:
        return self.position.moves_from(sq)

    def is_game_over(self) -> bool:
        return self.status in ("checkmate", "stalemate", "draw")

    def make_move(self, move: ChessMove) -> str:
        if self.is_game_over():
            raise ValueError(f"game is over: {self.status}")
        legal = self.legal_moves()
        if move not in legal:
            raise ValueError(f"illegal move {move.uci()}")
        san = _chess_move_to_san(self.position, move, legal)
        undo = self.position.make_move(move)
        self.undo_stack.append(undo)
        self._record_position()
        self._refresh_status()
        if self.status == "checkmate":
            san += "#"
        elif self.status == "check":
            san += "+"
        self.history.append((move, san))
        return san

    def undo_move(self) -> ChessMove:
        if not self.history:
            raise ValueError("nothing to undo")
        move, _san = self.history.pop()
        undo = self.undo_stack.pop()
        self._unrecord_position()
        self.position.undo_move(move, undo)
        self._refresh_status()
        return move

    def _refresh_status(self) -> None:
        legal = self.position.legal_moves()
        in_check = self.position.is_in_check()
        if not legal:
            self.status = "checkmate" if in_check else "stalemate"
            self.draw_reason = ""
            return
        if self.position.halfmove >= 100:
            self.status = "draw"
            self.draw_reason = "fifty-move rule"
            return
        if self.counts.get(self.position.position_key(), 0) >= 3:
            self.status = "draw"
            self.draw_reason = "threefold repetition"
            return
        if _chess_insufficient_material(self.position):
            self.status = "draw"
            self.draw_reason = "insufficient material"
            return
        self.status = "check" if in_check else "active"
        self.draw_reason = ""

    def result(self) -> str:
        if self.status == "checkmate":
            return "0-1" if self.position.side == "w" else "1-0"
        if self.status in ("stalemate", "draw"):
            return "1/2-1/2"
        return "*"

    def pgn(self, white: str = "White", black: str = "Black") -> str:
        lines = ['[Event "GPUI Python Demo"]',
                 f'[White "{white}"]',
                 f'[Black "{black}"]',
                 f'[Result "{self.result()}"]',
                 ""]
        moves: list[str] = []
        for index, (_move, san) in enumerate(self.history):
            if index % 2 == 0:
                moves.append(f"{index // 2 + 1}. {san}")
            else:
                moves.append(san)
        moves.append(self.result())
        lines.append(" ".join(moves))
        return "\n".join(lines)

    def material_balance(self) -> int:
        """White-minus-Black material in centipawns (kings excluded)."""
        balance = 0
        for piece in self.position.board:
            if piece is None:
                continue
            color, kind = piece
            value = _CHESS_PIECE_VALUE[kind]
            balance += value if color == "w" else -value
        return balance


class _ChessNodeCap(Exception):
    pass


def _chess_evaluate(pos: ChessPosition) -> int:
    """Centipawn score from White's perspective (mirrors rust-chess)."""
    score = 0
    white_bishops = 0
    black_bishops = 0
    for sq, piece in enumerate(pos.board):
        if piece is None:
            continue
        color, kind = piece
        table = _CHESS_PST[kind][sq if color == "w" else sq ^ 56]
        value = _CHESS_PIECE_VALUE[kind] + table
        if kind == "B":
            if color == "w":
                white_bishops += 1
            else:
                black_bishops += 1
        score += value if color == "w" else -value
    if white_bishops >= 2:
        score += 30
    if black_bishops >= 2:
        score -= 30
    return score


def _chess_evaluate_relative(pos: ChessPosition) -> int:
    score = _chess_evaluate(pos)
    return score if pos.side == "w" else -score


def _chess_order_score(pos: ChessPosition, move: ChessMove) -> int:
    """MVV-LVA capture ordering with a promotion bonus (higher first)."""
    score = 0
    if move.promotion == "Q":
        score += 900
    elif move.promotion is not None:
        score += 500
    if move.capture:
        victim = pos.board[move.to_sq]
        attacker = pos.board[move.from_sq]
        victim_value = _CHESS_PIECE_VALUE[victim[1]] if victim else 100
        attacker_value = _CHESS_PIECE_VALUE[attacker[1]] if attacker else 0
        score += 10_000 + victim_value * 10 - attacker_value
    return score


def _chess_search(pos: ChessPosition, depth: int, alpha: int, beta: int,
                  ply: int, budget: list[int], cap: int) -> int:
    budget[0] += 1
    if budget[0] > cap:
        raise _ChessNodeCap
    if pos.halfmove >= 100:
        return 0
    if depth <= 0:
        return _chess_evaluate_relative(pos)
    moves = pos.legal_moves()
    if not moves:
        if pos.is_in_check():
            return -(_CHESS_MATE - ply)
        return 0
    moves.sort(key=lambda move: _chess_order_score(pos, move), reverse=True)
    best = -_CHESS_INF
    for move in moves:
        undo = pos.make_move(move)
        try:
            score = -_chess_search(pos, depth - 1, -beta, -alpha, ply + 1,
                                   budget, cap)
        finally:
            pos.undo_move(move, undo)
        if score > best:
            best = score
        if best > alpha:
            alpha = best
        if alpha >= beta:
            break
    return best


def chess_best_move(match: ChessMatch, difficulty: str,
                    rng: random.Random) -> tuple[ChessMove, int, int]:
    """Best move for the side to move; returns (move, score_cp, nodes)."""
    legal = match.legal_moves()
    if not legal:
        raise ValueError("no legal moves")
    if difficulty not in _CHESS_DEPTH:
        raise ValueError(f"unknown chess difficulty {difficulty!r}")
    if difficulty == "harmless":
        return (rng.choice(legal), 0, 0)
    depth = _CHESS_DEPTH[difficulty]
    cap = _CHESS_NODE_CAP[difficulty]
    pos = match.position
    ordered = sorted(legal, key=lambda move: _chess_order_score(pos, move),
                     reverse=True)
    budget = [0]
    best_move = ordered[0]
    best_score = -_CHESS_INF
    alpha = -_CHESS_INF
    try:
        for move in ordered:
            undo = pos.make_move(move)
            try:
                score = -_chess_search(pos, depth - 1, -_CHESS_INF, -alpha,
                                       1, budget, cap)
            finally:
                pos.undo_move(move, undo)
            if score > best_score:
                best_score = score
                best_move = move
            if score > alpha:
                alpha = score
    except _ChessNodeCap:
        pass
    return (best_move, best_score, budget[0])


def chess_move_from_uci(match: ChessMatch, uci: str) -> ChessMove:
    """Resolve a UCI string (e.g. ``e2e4`` or ``e7e8q``) to a legal move."""
    if len(uci) not in (4, 5):
        raise ValueError(f"malformed UCI move {uci!r}")
    from_sq = _chess_from_algebraic(uci[:2])
    to_sq = _chess_from_algebraic(uci[2:4])
    promotion = uci[4].upper() if len(uci) == 5 else None
    if from_sq is None or to_sq is None:
        raise ValueError(f"malformed UCI move {uci!r}")
    if promotion is not None and promotion not in ("Q", "R", "B", "N"):
        raise ValueError(f"malformed UCI promotion {uci!r}")
    for move in match.legal_moves():
        if (move.from_sq == from_sq and move.to_sq == to_sq
                and (move.promotion or None) == promotion):
            return move
    raise ValueError(f"illegal move {uci}")


@dataclass
class ChessGame(_PaletteAwareGame):
    match: ChessMatch = field(default_factory=ChessMatch)
    selected: tuple[int, int] | None = None
    cursor: tuple[int, int] = (6, 4)
    flipped: bool = False
    mode: str = "ai"
    difficulty: str = "easy"
    promotion: str = "Q"
    last_move: tuple[int, int] | None = None
    hint_move: tuple[int, int, str | None] | None = None
    moves_made: int = 0
    palette: str = "dark"

    # -- coordinates ----------------------------------------------------

    def _display_to_sq(self, row: int, col: int) -> int:
        if self.flipped:
            return _chess_sq(7 - col, row)
        return _chess_sq(col, 7 - row)

    def _sq_to_display(self, sq: int) -> tuple[int, int]:
        file, rank = _chess_file(sq), _chess_rank(sq)
        if self.flipped:
            return (rank, 7 - file)
        return (7 - rank, file)

    @staticmethod
    def cell_id(row: int, col: int) -> str:
        return f"chess-cell-{row}-{col}"

    # -- rendering ------------------------------------------------------

    def board_node(self, *, revision: int = 1) -> Scene2D:
        pos = self.match.position
        light, dark = _CHESS_SQUARES[self.palette]
        cell, gap, padding = 66.0, 2.0, 26.0
        grid = GridSpec(8, 8, padding, padding, cell, cell, gap)
        nodes: list[Any] = [SceneRoundedRect(
            "chess-well", 7.0, 7.0,
            padding * 2 + 8 * cell + 7 * gap - 14.0,
            padding * 2 + 8 * cell + 7 * gap - 14.0,
            fill=self.colors["board"], radius=22.0,
        )]
        selected_sq = (self._display_to_sq(*self.selected)
                       if self.selected is not None else None)
        targets: dict[int, ChessMove] = {}
        if selected_sq is not None and not self.match.is_game_over():
            for move in self.match.moves_from(selected_sq):
                targets.setdefault(move.to_sq, move)
        last_squares = set(self.last_move) if self.last_move else set()
        hint_squares = set()
        if self.hint_move is not None:
            hint_squares = {self.hint_move[0], self.hint_move[1]}
        check_sq = None
        if self.match.status in ("check", "checkmate"):
            try:
                check_sq = pos.king_sq(pos.side)
            except ValueError:
                check_sq = None
        for row in range(8):
            for col in range(8):
                sq = self._display_to_sq(row, col)
                is_light = (_chess_file(sq) + _chess_rank(sq)) % 2 == 1
                piece = pos.board[sq]
                name = _chess_algebraic(sq)
                if piece is None:
                    value = "empty"
                else:
                    color_name = "white" if piece[0] == "w" else "black"
                    value = f"{color_name} {CHESS_PIECE_NAMES[piece[1]]}"
                stroke = self.colors["grid"]
                stroke_width = 1.0
                if sq == selected_sq:
                    stroke, stroke_width = self.colors["gold"], 3.4
                elif (row, col) == self.cursor:
                    stroke, stroke_width = self.colors["paper"], 2.2
                nodes.append(_scene_cell(
                    "chess", grid, row, col,
                    fill=light if is_light else dark,
                    label=f"Square {name}", value=value,
                    selected=sq == selected_sq,
                    stroke=stroke, stroke_width=stroke_width, radius=6.0,
                ))
                x, y, width, height = grid.cell_rect(row, col)
                if sq in last_squares:
                    nodes.append(SceneRoundedRect(
                        f"chess-last-{row}-{col}", x + 2, y + 2,
                        width - 4, height - 4, fill=self.colors["gold"],
                        radius=6.0, opacity=0.38,
                    ))
                if sq in hint_squares:
                    nodes.append(SceneRoundedRect(
                        f"chess-hint-{row}-{col}", x + 2, y + 2,
                        width - 4, height - 4, fill=None,
                        stroke=Stroke(self.colors["gold"], 3.6), radius=6.0,
                    ))
                if sq == check_sq:
                    nodes.append(SceneCircle(
                        f"chess-check-{row}-{col}", x + width / 2, y + height / 2,
                        min(width, height) / 2 - 3.0, fill=None,
                        stroke=Stroke(self.colors["danger"], 4.0),
                    ))
                if sq in targets:
                    center_x, center_y = grid.cell_center(row, col)
                    if targets[sq].capture:
                        nodes.append(SceneCircle(
                            f"chess-target-{row}-{col}", center_x, center_y,
                            min(width, height) / 2 - 5.0, fill=None,
                            stroke=Stroke(self.colors["danger"], 3.4),
                        ))
                    else:
                        nodes.append(SceneCircle(
                            f"chess-target-{row}-{col}", center_x, center_y, 9.0,
                            fill=self.colors["mint"],
                            stroke=Stroke(self.colors["paper"], 1.4),
                        ))
                if piece is not None:
                    center_x, center_y = grid.cell_center(row, col)
                    is_white = piece[0] == "w"
                    # Drop shadow keeps both armies readable on either square shade.
                    nodes.append(SceneText(
                        f"chess-piece-{row}-{col}", center_x, center_y - 42.0 * 0.625,
                        CHESS_GLYPHS[piece], 42.0,
                        "#F8FAFC" if is_white else "#141C2A",
                        align="center",
                        shadow=Scene2DShadow(0.0, 1.5, 3.0,
                                             (0.0, 0.0, 0.0, 0.65)
                                             if is_white else
                                             (1.0, 1.0, 1.0, 0.55)),
                    ))
        width = padding * 2 + 8 * cell + 7 * gap
        return Scene2D(
            "chess-board", width, width, nodes, grid,
            Scene2DInputConfig(pointer=True, continuous=False, capture=True,
                               keyboard=True),
            revision=revision,
            semantic=Scene2DSemantic(
                "grid", "Chess board",
                description="Full-rules chess. Select a piece, then a highlighted square."),
        )

    def section_node(self) -> ui.Node:
        return ui.vstack([
            ui.section_header(
                "Chess",
                "Full rules — castling, en passant, promotion — with a rust-chess "
                "AI. Select a piece, then a highlighted square.",
            ),
            ui.wrap([
                self.board_node(),
                ui.vstack([
                    ui.card([
                        ui.heading("How to play", level=3),
                        ui.text("Click a piece to see its legal moves, then click a target."),
                        ui.text("Arrow keys move the cursor; Enter selects. U undoes, F flips."),
                        ui.text("Promotion uses the piece picker below (default queen)."),
                    ], title="Rules"),
                    ui.select(
                        id="chess-mode", label="Opponent", value=self.mode,
                        options=list(CHESS_MODES), action=CHESS_MODE_ACTION,
                    ),
                    ui.select(
                        id="chess-difficulty", label="AI strength", value=self.difficulty,
                        options=list(CHESS_DIFFICULTIES),
                        action=CHESS_DIFFICULTY_ACTION,
                    ),
                    ui.select(
                        id="chess-promotion", label="Promote to", value=self.promotion,
                        options=list(CHESS_PROMOTIONS),
                        action=CHESS_PROMOTION_ACTION,
                    ),
                    ui.hstack([
                        _cell_button("♞ New game", "chess-btn-new", CHESS_CONTROL_ACTION),
                        _cell_button("↩ Undo", "chess-btn-undo", CHESS_CONTROL_ACTION),
                        _cell_button("⇄ Flip", "chess-btn-flip", CHESS_CONTROL_ACTION),
                    ], gap=8.0),
                    ui.hstack([
                        _cell_button("💡 Hint", "chess-btn-hint", CHESS_CONTROL_ACTION),
                        _cell_button("🤖 AI move", "chess-btn-ai", CHESS_CONTROL_ACTION),
                    ], gap=8.0),
                ], gap=12.0),
            ], gap=20.0),
            ui.wrap([
                ui.metric("To move", "White", id="chess-to-move"),
                ui.metric("Move", "1", id="chess-move"),
                ui.metric("Material", "Equal", id="chess-material"),
                _status_badge("chess-badge", "Ready", "neutral"),
            ], gap=12.0),
            ui.progress(0.0, label="Game length", id="chess-progress"),
            ui.text("You play White; the AI replies as Black.",
                    tone="secondary", id="chess-status"),
            ui.text(CHESS_START_FEN, tone="secondary", id="chess-fen"),
            ui.text("No moves yet — 1. e4 is a fine start.",
                    tone="secondary", id="chess-moves"),
        ], gap=16.0)

    # -- interaction ----------------------------------------------------

    def material_text(self) -> str:
        balance = self.match.material_balance()
        if balance == 0:
            return "Equal"
        pawns = abs(balance) / 100
        leader = "White" if balance > 0 else "Black"
        return f"{leader} +{pawns:g}"

    def moves_text(self) -> str:
        history = self.match.history
        if not history:
            return "No moves yet — 1. e4 is a fine start."
        sans = [san for _move, san in history]
        start = max(0, len(sans) - 8)
        parts: list[str] = []
        for index in range(start, len(sans)):
            if index % 2 == 0:
                parts.append(f"{index // 2 + 1}. {sans[index]}")
            else:
                parts.append(sans[index])
        return ("… " if start else "") + " ".join(parts)

    def _side_to_move_text(self) -> str:
        pos = self.match.position
        text = "White to move." if pos.side == "w" else "Black to move."
        if self.match.status == "check":
            text += " Check!"
        return text

    def _game_over_text(self) -> str:
        if self.match.status == "checkmate":
            winner = "Black" if self.match.position.side == "w" else "White"
            return f"Checkmate! {winner} wins."
        if self.match.status == "stalemate":
            return "Stalemate — drawn game."
        return f"Draw — {self.match.draw_reason}."

    def status_ops(self, message: str | None = None) -> list[dict[str, Any]]:
        match = self.match
        side_name = "White" if match.position.side == "w" else "Black"
        if match.status == "checkmate":
            winner = "Black" if match.position.side == "w" else "White"
            badge, tone = f"Checkmate · {winner} wins", "success"
        elif match.status == "stalemate":
            badge, tone = "Stalemate · Draw", "warning"
        elif match.status == "draw":
            badge, tone = f"Draw · {match.draw_reason}", "neutral"
        elif match.status == "check":
            badge, tone = f"Check · {side_name} to move", "warning"
        elif match.history:
            badge, tone = f"{side_name} to move", "accent"
        else:
            badge, tone = "Ready", "neutral"
        ops = [
            _set("chess-to-move", "value", side_name),
            _set("chess-move", "value", str(match.position.fullmove)),
            _set("chess-material", "value", self.material_text()),
            _set("chess-badge", "label", badge),
            _set("chess-badge", "tone", tone),
            _set("chess-progress", "value", min(1.0, len(match.history) / 120.0)),
            _set("chess-fen", "text", match.position.to_fen()),
            _set("chess-moves", "text", self.moves_text()),
        ]
        if message is not None:
            ops.append(_set("chess-status", "text", message))
        return ops

    def click(self, row: int, col: int,
              rng: random.Random | None = None) -> list[dict[str, Any]]:
        match = self.match
        if match.is_game_over():
            return self.status_ops("Game over — press New game to play again.")
        if not 0 <= row < 8 or not 0 <= col < 8:
            return self.status_ops()
        if self.mode == "ai" and match.position.side != "w":
            return self.status_ops("The AI plays Black — switch to Two players "
                                   "to move both sides.")
        sq = self._display_to_sq(row, col)
        piece = match.position.board[sq]
        if self.selected is None:
            if piece is None or piece[0] != match.position.side:
                side_name = "White" if match.position.side == "w" else "Black"
                return self.status_ops(f"Select a {side_name} piece to move.")
            self.selected = (row, col)
            targets = match.moves_from(sq)
            return self.status_ops(
                f"Selected {_chess_algebraic(sq)} — {len(targets)} legal "
                "move(s). Pick a highlighted square.")
        if self.selected == (row, col):
            self.selected = None
            return self.status_ops("Selection cleared.")
        from_sq = self._display_to_sq(*self.selected)
        candidates = [move for move in match.moves_from(from_sq)
                      if move.to_sq == sq]
        if not candidates:
            if piece is not None and piece[0] == match.position.side:
                self.selected = (row, col)
                targets = match.moves_from(sq)
                return self.status_ops(
                    f"Selected {_chess_algebraic(sq)} — {len(targets)} legal "
                    "move(s).")
            self.selected = None
            return self.status_ops("Illegal move — pick a highlighted square.")
        move = next((candidate for candidate in candidates
                     if (candidate.promotion or "Q") == self.promotion),
                    candidates[0])
        return self._play_human_move(move, rng or random.Random())

    def _play_human_move(self, move: ChessMove,
                         rng: random.Random) -> list[dict[str, Any]]:
        san = self.match.make_move(move)
        self.moves_made += 1
        self.last_move = (move.from_sq, move.to_sq)
        self.hint_move = None
        self.selected = None
        if self.match.is_game_over():
            return self.status_ops(f"Played {san}. {self._game_over_text()}")
        if self.mode == "ai":
            ai_move, _score, _nodes = chess_best_move(self.match, self.difficulty,
                                                      rng)
            ai_san = self.match.make_move(ai_move)
            self.moves_made += 1
            self.last_move = (ai_move.from_sq, ai_move.to_sq)
            if self.match.is_game_over():
                return self.status_ops(f"Played {san}; AI replied {ai_san}. "
                                       f"{self._game_over_text()}")
            return self.status_ops(f"Played {san}; AI replied {ai_san}. "
                                   f"{self._side_to_move_text()}")
        return self.status_ops(f"Played {san}. {self._side_to_move_text()}")

    def _undo(self) -> list[dict[str, Any]]:
        if not self.match.history:
            return self.status_ops("Nothing to undo.")
        steps = 1
        if (self.mode == "ai" and len(self.match.history) >= 2
                and self.match.position.side == "w"):
            steps = 2
        undone: list[str] = []
        for _ in range(steps):
            if not self.match.history:
                break
            undone.append(self.match.undo_move().uci())
            self.moves_made = max(0, self.moves_made - 1)
        self.selected = None
        self.hint_move = None
        if self.match.history:
            last = self.match.history[-1][0]
            self.last_move = (last.from_sq, last.to_sq)
        else:
            self.last_move = None
        return self.status_ops(f"Undid {' and '.join(reversed(undone))}. "
                               f"{self._side_to_move_text()}")

    def control(self, name: str, rng: random.Random) -> list[dict[str, Any]]:
        if name == "new":
            self.match = ChessMatch()
            self.selected = None
            self.last_move = None
            self.hint_move = None
            self.moves_made = 0
            self.cursor = self._sq_to_display(_chess_from_algebraic("e2") or 12)
            return [_replace("chess-board", self.board_node()),
                    *self.status_ops("New game — White to move.")]
        if name == "undo":
            return self._undo()
        if name == "flip":
            self.flipped = not self.flipped
            self.selected = None
            return [_replace("chess-board", self.board_node()),
                    *self.status_ops("Board flipped.")]
        if name == "hint":
            if self.match.is_game_over():
                return self.status_ops("Game over — press New game to play again.")
            move, _score, _nodes = chess_best_move(self.match, self.difficulty,
                                                   rng)
            self.hint_move = (move.from_sq, move.to_sq, move.promotion)
            san = _chess_move_to_san(self.match.position, move,
                                     self.match.legal_moves())
            return self.status_ops(f"Hint: {san} ({move.uci()}).")
        if name == "ai":
            if self.match.is_game_over():
                return self.status_ops("Game over — press New game to play again.")
            move, _score, _nodes = chess_best_move(self.match, self.difficulty,
                                                   rng)
            san = self.match.make_move(move)
            self.moves_made += 1
            self.last_move = (move.from_sq, move.to_sq)
            self.hint_move = None
            self.selected = None
            if self.match.is_game_over():
                return self.status_ops(f"AI played {san}. {self._game_over_text()}")
            return self.status_ops(f"AI played {san}. {self._side_to_move_text()}")
        raise ValueError(f"unknown chess control {name!r}")

    def select_mode(self, value: str) -> list[dict[str, Any]]:
        if value not in ("two", "ai"):
            raise ValueError(f"unknown chess mode {value!r}")
        self.mode = value
        self.selected = None
        self.hint_move = None
        if (value == "ai" and not self.match.is_game_over()
                and self.match.position.side == "b"):
            move, _score, _nodes = chess_best_move(self.match, self.difficulty,
                                                   random.Random())
            san = self.match.make_move(move)
            self.moves_made += 1
            self.last_move = (move.from_sq, move.to_sq)
            if self.match.is_game_over():
                return self.status_ops(f"AI (Black) played {san}. "
                                       f"{self._game_over_text()}")
            return self.status_ops(f"AI (Black) played {san}. White to move.")
        if value == "two":
            return self.status_ops("Two players — move both sides.")
        return self.status_ops("You play White; the AI replies as Black.")

    def select_difficulty(self, value: str) -> list[dict[str, Any]]:
        if value not in _CHESS_DEPTH:
            raise ValueError(f"unknown chess difficulty {value!r}")
        self.difficulty = value
        label = next(label for key, label in CHESS_DIFFICULTIES if key == value)
        return self.status_ops(f"AI strength: {label}.")

    def select_promotion(self, value: str) -> list[dict[str, Any]]:
        if value not in ("Q", "R", "B", "N"):
            raise ValueError(f"unknown chess promotion {value!r}")
        self.promotion = value
        label = next(label for key, label in CHESS_PROMOTIONS if key == value)
        return self.status_ops(f"Pawns will promote to {label}.")

