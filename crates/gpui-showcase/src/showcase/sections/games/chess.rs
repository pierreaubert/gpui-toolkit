//! Chess rules engine, AI opponent, and native board UI.
//!
//! Rules, the move-generation pipeline (pseudo-legal generation plus a
//! make-and-check legality filter), FEN/SAN handling, draw detection, and the
//! AI evaluation (material plus piece-square tables plus bishop-pair bonus
//! with MVV-LVA-ordered negamax search) follow RumenDamyanov/rust-chess
//! (<https://github.com/RumenDamyanov/rust-chess>, MIT); the bitboard backend
//! is replaced with a 64-square mailbox board shared with the Python demo so
//! both showcases behave identically with zero extra dependencies.

// Rust guideline compliant 2026-02-21

use super::{
    brush, circle_node, color, color_alpha, inset_rect, new_scene, next_random, rounded_node,
    semantic, stroke, text_node, transition,
};
use gpui_ui_kit::scene2d::{
    Scene2DColor, Scene2DGrid, Scene2DInput, Scene2DInputConfig, Scene2DKeyPhase, Scene2DScene,
    Scene2DSemanticRole, Scene2DTextAlign, ScenePoint, SceneRect,
};
use std::collections::HashMap;

/// Starting position in Forsyth-Edwards Notation.
#[cfg(test)]
pub(super) const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

// Piece-square tables from rust-chess `src/ai/evaluation.rs`, White's
// perspective in LERF order (a1 = 0 through h8 = 63). Black lookups mirror
// the square with `square ^ 56`, which flips the rank.
#[rustfmt::skip]
const PAWN_TABLE: [i32; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, 5, 10, 10, -20, -20, 10, 10, 5, 5, -5, -10, 0, 0, -10, -5,
    5, 0, 0, 0, 20, 20, 0, 0, 0, 5, 5, 10, 25, 25, 10, 5, 5, 10, 10, 20, 30, 30, 20,
    10, 10, 50, 50, 50, 50, 50, 50, 50, 50, 0, 0, 0, 0, 0, 0, 0, 0,
];
#[rustfmt::skip]
const KNIGHT_TABLE: [i32; 64] = [
    -50, -40, -30, -30, -30, -30, -40, -50, -40, -20, 0, 5, 5, 0, -20, -40, -30, 5,
    10, 15, 15, 10, 5, -30, -30, 0, 15, 20, 20, 15, 0, -30, -30, 5, 15, 20, 20, 15,
    5, -30, -30, 0, 10, 15, 15, 10, 0, -30, -40, -20, 0, 0, 0, 0, -20, -40, -50, -40,
    -30, -30, -30, -30, -40, -50,
];
#[rustfmt::skip]
const BISHOP_TABLE: [i32; 64] = [
    -20, -10, -10, -10, -10, -10, -10, -20, -10, 5, 0, 0, 0, 0, 5, -10, -10, 10, 10,
    10, 10, 10, 10, -10, -10, 0, 10, 10, 10, 10, 0, -10, -10, 5, 5, 10, 10, 5, 5, -10,
    -10, 0, 5, 10, 10, 5, 0, -10, -10, 0, 0, 0, 0, 0, 0, -10, -20, -10, -10, -10, -10,
    -10, -10, -20,
];
#[rustfmt::skip]
const ROOK_TABLE: [i32; 64] = [
    0, 0, 0, 5, 5, 0, 0, 0, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5,
    0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, -5, 0, 0, 0, 0, 0, 0, -5, 5, 10,
    10, 10, 10, 10, 10, 5, 0, 0, 0, 0, 0, 0, 0, 0,
];
#[rustfmt::skip]
const QUEEN_TABLE: [i32; 64] = [
    -20, -10, -10, -5, -5, -10, -10, -20, -10, 0, 5, 0, 0, 0, 0, -10, -10, 5, 5, 5,
    5, 5, 0, -10, 0, 0, 5, 5, 5, 5, 0, -5, -5, 0, 5, 5, 5, 5, 0, -5, -10, 0, 5, 5, 5,
    5, 0, -10, -10, 0, 0, 0, 0, 0, 0, -10, -20, -10, -10, -5, -5, -10, -10, -20,
];
#[rustfmt::skip]
const KING_TABLE: [i32; 64] = [
    20, 30, 10, 0, 0, 10, 30, 20, 20, 20, 0, 0, 0, 0, 20, 20, -10, -20, -20, -20,
    -20, -20, -20, -10, -20, -30, -30, -40, -40, -30, -30, -20, -30, -40, -40, -50,
    -50, -40, -40, -30, -30, -40, -40, -50, -50, -40, -40, -30, -30, -40, -40, -50,
    -50, -40, -40, -30, -30, -40, -40, -50, -50, -40, -40, -30,
];

/// Mate score base; mated-in-N scores as `MATE_SCORE - ply` so faster mates win.
const MATE_SCORE: i32 = 90_000;
/// Sentinel larger than any reachable evaluation.
const INF_SCORE: i32 = 100_000;

/// Number of nodes (positions) the search may visit before settling.
/// Sized so Hard finishes in well under a second on desktop and mobile.
const NODE_CAP: [u64; 4] = [0, 32_000, 120_000, 320_000];

const KNIGHT_STEPS: [(i32, i32); 8] = [
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
];
const KING_STEPS: [(i32, i32); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];
const BISHOP_RAYS: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
const ROOK_RAYS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

// Board square colors (`#E4DCC8` light, `#4E6E5D` dark). The palette remapper
// only rewrites dark low-chroma fills, so the light squares survive both themes.
const LIGHT_SQUARE: Scene2DColor = Scene2DColor::rgb(0.894, 0.863, 0.784);
const DARK_SQUARE: Scene2DColor = Scene2DColor::rgb(0.306, 0.431, 0.365);
const GOLD: Scene2DColor = Scene2DColor::rgb(1.0, 0.827, 0.435);
const MINT: Scene2DColor = Scene2DColor::rgb(0.443, 0.878, 0.749);
const DANGER: Scene2DColor = Scene2DColor::rgb(1.0, 0.427, 0.49);
const PAPER: Scene2DColor = Scene2DColor::rgb(0.957, 0.945, 0.91);
const GRID_LINE: Scene2DColor = Scene2DColor::rgb(0.212, 0.286, 0.388);
const WHITE_PIECE: Scene2DColor = Scene2DColor::rgb(0.973, 0.98, 0.988);
const BLACK_PIECE: Scene2DColor = Scene2DColor::rgb(0.078, 0.11, 0.165);

/// File (0 = a) of a LERF square.
const fn file_of(square: usize) -> usize {
    square % 8
}

/// Rank (0 = first rank) of a LERF square.
const fn rank_of(square: usize) -> usize {
    square / 8
}

/// LERF square from file and rank.
const fn square(file: usize, rank: usize) -> usize {
    rank * 8 + file
}

/// Algebraic name (`e4`) of a square.
fn square_name(square: usize) -> String {
    format!(
        "{}{}",
        (b'a' + file_of(square) as u8) as char,
        rank_of(square) + 1
    )
}

/// Parses an algebraic square name, returning `None` when malformed.
#[cfg(test)]
fn square_from_name(name: &str) -> Option<usize> {
    let bytes = name.as_bytes();
    if bytes.len() != 2 {
        return None;
    }
    let file = bytes[0].wrapping_sub(b'a') as usize;
    let rank = bytes[1].wrapping_sub(b'1') as usize;
    (file < 8 && rank < 8).then(|| square(file, rank))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Color {
    White,
    Black,
}

impl Color {
    const fn other(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::White => "White",
            Self::Black => "Black",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceKind {
    /// Material value in centipawns; kings never count toward material.
    const fn value(self) -> i32 {
        match self {
            Self::Pawn => 100,
            Self::Knight => 320,
            Self::Bishop => 330,
            Self::Rook => 500,
            Self::Queen => 900,
            Self::King => 0,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Pawn => "pawn",
            Self::Knight => "knight",
            Self::Bishop => "bishop",
            Self::Rook => "rook",
            Self::Queen => "queen",
            Self::King => "king",
        }
    }

    /// Board glyph; outline glyphs read as White, filled glyphs as Black.
    const fn glyph(self, color: Color) -> &'static str {
        match (color, self) {
            (Color::White, Self::King) => "♔",
            (Color::White, Self::Queen) => "♕",
            (Color::White, Self::Rook) => "♖",
            (Color::White, Self::Bishop) => "♗",
            (Color::White, Self::Knight) => "♘",
            (Color::White, Self::Pawn) => "\u{2659}",
            (Color::Black, Self::King) => "♚",
            (Color::Black, Self::Queen) => "♛",
            (Color::Black, Self::Rook) => "♜",
            (Color::Black, Self::Bishop) => "♝",
            (Color::Black, Self::Knight) => "♞",
            (Color::Black, Self::Pawn) => "♟",
        }
    }

    const fn table(self) -> &'static [i32; 64] {
        match self {
            Self::Pawn => &PAWN_TABLE,
            Self::Knight => &KNIGHT_TABLE,
            Self::Bishop => &BISHOP_TABLE,
            Self::Rook => &ROOK_TABLE,
            Self::Queen => &QUEEN_TABLE,
            Self::King => &KING_TABLE,
        }
    }

    #[cfg(test)]
    fn from_fen_letter(letter: char) -> Option<(Color, Self)> {
        let color = if letter.is_ascii_uppercase() {
            Color::White
        } else {
            Color::Black
        };
        let kind = match letter.to_ascii_lowercase() {
            'p' => Self::Pawn,
            'n' => Self::Knight,
            'b' => Self::Bishop,
            'r' => Self::Rook,
            'q' => Self::Queen,
            'k' => Self::King,
            _ => return None,
        };
        Some((color, kind))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct Piece {
    color: Color,
    kind: PieceKind,
}

/// A chess move with promotion and behavior flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ChessMove {
    from: usize,
    to: usize,
    promotion: Option<PieceKind>,
    capture: bool,
    en_passant: bool,
    castling: bool,
    double_push: bool,
}

impl ChessMove {
    const fn quiet(from: usize, to: usize) -> Self {
        Self {
            from,
            to,
            promotion: None,
            capture: false,
            en_passant: false,
            castling: false,
            double_push: false,
        }
    }

    /// UCI notation (`e2e4`, `e7e8q`) used by tests and status text.
    fn uci(self) -> String {
        let mut text = format!("{}{}", square_name(self.from), square_name(self.to));
        if let Some(promotion) = self.promotion {
            let letter = match promotion {
                PieceKind::Queen => 'q',
                PieceKind::Rook => 'r',
                PieceKind::Bishop => 'b',
                PieceKind::Knight => 'n',
                PieceKind::Pawn | PieceKind::King => '?',
            };
            text.push(letter);
        }
        text
    }
}

/// Castling availability as four bits: White kingside/queenside, Black kingside/queenside.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
struct CastlingRights(u8);

impl CastlingRights {
    const WHITE_KINGSIDE: u8 = 1;
    const WHITE_QUEENSIDE: u8 = 2;
    const BLACK_KINGSIDE: u8 = 4;
    const BLACK_QUEENSIDE: u8 = 8;

    fn has(self, flag: u8) -> bool {
        self.0 & flag != 0
    }

    fn remove(&mut self, flag: u8) {
        self.0 &= !flag;
    }

    #[cfg(test)]
    fn from_fen(text: &str) -> Option<Self> {
        if text == "-" {
            return Some(Self(0));
        }
        let mut rights = 0;
        for letter in text.chars() {
            match letter {
                'K' => rights |= Self::WHITE_KINGSIDE,
                'Q' => rights |= Self::WHITE_QUEENSIDE,
                'k' => rights |= Self::BLACK_KINGSIDE,
                'q' => rights |= Self::BLACK_QUEENSIDE,
                _ => return None,
            }
        }
        Some(Self(rights))
    }

    #[cfg(test)]
    fn to_fen(self) -> String {
        if self.0 == 0 {
            return "-".to_owned();
        }
        let mut text = String::with_capacity(4);
        for (flag, letter) in [
            (Self::WHITE_KINGSIDE, 'K'),
            (Self::WHITE_QUEENSIDE, 'Q'),
            (Self::BLACK_KINGSIDE, 'k'),
            (Self::BLACK_QUEENSIDE, 'q'),
        ] {
            if self.has(flag) {
                text.push(letter);
            }
        }
        text
    }
}

/// Restorable state captured by [`ChessPosition::make_move`].
#[derive(Clone, Debug)]
struct ChessUndo {
    moving: PieceKind,
    captured: Option<Piece>,
    captured_square: usize,
    castling: CastlingRights,
    en_passant: Option<usize>,
    halfmove: u16,
}

/// Mailbox board with rust-chess move semantics and FEN support.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ChessPosition {
    board: [Option<Piece>; 64],
    side: Color,
    castling: CastlingRights,
    en_passant: Option<usize>,
    halfmove: u16,
    fullmove: u16,
}

impl ChessPosition {
    /// Standard starting array with White to move.
    fn starting() -> Self {
        let mut board: [Option<Piece>; 64] = [None; 64];
        let back = [
            PieceKind::Rook,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Queen,
            PieceKind::King,
            PieceKind::Bishop,
            PieceKind::Knight,
            PieceKind::Rook,
        ];
        for (file, kind) in back.into_iter().enumerate() {
            board[square(file, 0)] = Some(Piece {
                color: Color::White,
                kind,
            });
            board[square(file, 7)] = Some(Piece {
                color: Color::Black,
                kind,
            });
        }
        for file in 0..8 {
            board[square(file, 1)] = Some(Piece {
                color: Color::White,
                kind: PieceKind::Pawn,
            });
            board[square(file, 6)] = Some(Piece {
                color: Color::Black,
                kind: PieceKind::Pawn,
            });
        }
        Self {
            board,
            side: Color::White,
            castling: CastlingRights(0b1111),
            en_passant: None,
            halfmove: 0,
            fullmove: 1,
        }
    }

    /// Parses a position; rejects bad fields and kingless boards.
    #[cfg(test)]
    fn from_fen(fen: &str) -> Result<Self, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() != 6 {
            return Err(format!("malformed FEN: {fen}"));
        }
        let mut board: [Option<Piece>; 64] = [None; 64];
        let mut rank: i32 = 7;
        let mut file: i32 = 0;
        for letter in parts[0].chars() {
            if letter == '/' {
                if file != 8 {
                    return Err(format!("malformed FEN rank: {fen}"));
                }
                rank -= 1;
                file = 0;
                continue;
            }
            if let Some(skip) = letter.to_digit(10) {
                file += skip as i32;
                if file > 8 {
                    return Err(format!("malformed FEN rank: {fen}"));
                }
                continue;
            }
            let Some((color, kind)) = PieceKind::from_fen_letter(letter) else {
                return Err(format!("malformed FEN piece {letter}"));
            };
            if !(0..8).contains(&file) || !(0..8).contains(&rank) {
                return Err(format!("malformed FEN placement: {fen}"));
            }
            board[square(file as usize, rank as usize)] = Some(Piece { color, kind });
            file += 1;
        }
        let side = match parts[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(format!("malformed FEN side: {fen}")),
        };
        let castling =
            CastlingRights::from_fen(parts[2]).ok_or_else(|| format!("malformed FEN: {fen}"))?;
        let en_passant = if parts[3] == "-" {
            None
        } else {
            Some(square_from_name(parts[3]).ok_or_else(|| format!("malformed FEN: {fen}"))?)
        };
        let halfmove = parts[4]
            .parse::<u16>()
            .map_err(|_| format!("malformed FEN clocks: {fen}"))?;
        let fullmove = parts[5]
            .parse::<u16>()
            .map_err(|_| format!("malformed FEN clocks: {fen}"))?;
        let position = Self {
            board,
            side,
            castling,
            en_passant,
            halfmove,
            fullmove,
        };
        if position.king_square(Color::White).is_none()
            || position.king_square(Color::Black).is_none()
        {
            return Err(format!("FEN position is missing a king: {fen}"));
        }
        Ok(position)
    }

    /// Serializes the position, mirroring [`ChessPosition::from_fen`].
    #[cfg(test)]
    fn to_fen(&self) -> String {
        let mut rows = Vec::with_capacity(8);
        for rank in (0..8).rev() {
            let mut text = String::new();
            let mut empty = 0;
            for file in 0..8 {
                match self.board[square(file, rank)] {
                    None => empty += 1,
                    Some(piece) => {
                        if empty > 0 {
                            text.push_str(&empty.to_string());
                            empty = 0;
                        }
                        let letter = match piece.kind {
                            PieceKind::Pawn => 'p',
                            PieceKind::Knight => 'n',
                            PieceKind::Bishop => 'b',
                            PieceKind::Rook => 'r',
                            PieceKind::Queen => 'q',
                            PieceKind::King => 'k',
                        };
                        text.push(if piece.color == Color::White {
                            letter.to_ascii_uppercase()
                        } else {
                            letter
                        });
                    }
                }
            }
            if empty > 0 {
                text.push_str(&empty.to_string());
            }
            rows.push(text);
        }
        let en_passant = self.en_passant.map_or_else(|| "-".to_owned(), square_name);
        format!(
            "{} {} {} {en_passant} {} {}",
            rows.join("/"),
            if self.side == Color::White { "w" } else { "b" },
            self.castling.to_fen(),
            self.halfmove,
            self.fullmove
        )
    }

    /// Locates a king; validated positions always hold both kings.
    fn king_square(&self, color: Color) -> Option<usize> {
        self.board.iter().position(|piece| {
            *piece
                == Some(Piece {
                    color,
                    kind: PieceKind::King,
                })
        })
    }

    /// Reports whether `attacked` is attacked by `by`.
    fn is_attacked(&self, attacked: usize, by: Color) -> bool {
        let file = file_of(attacked) as i32;
        let rank = rank_of(attacked) as i32;
        let pawn_rank = if by == Color::White {
            rank - 1
        } else {
            rank + 1
        };
        for delta in [-1, 1] {
            let other = file + delta;
            if (0..8).contains(&other) && (0..8).contains(&pawn_rank) {
                let attacker = square(other as usize, pawn_rank as usize);
                if self.board[attacker]
                    == Some(Piece {
                        color: by,
                        kind: PieceKind::Pawn,
                    })
                {
                    return true;
                }
            }
        }
        for (steps, kind) in [
            (KNIGHT_STEPS.as_slice(), PieceKind::Knight),
            (KING_STEPS.as_slice(), PieceKind::King),
        ] {
            for (delta_file, delta_rank) in steps {
                let other = file + delta_file;
                let target = rank + delta_rank;
                if (0..8).contains(&other) && (0..8).contains(&target) {
                    let attacker = square(other as usize, target as usize);
                    if self.board[attacker] == Some(Piece { color: by, kind }) {
                        return true;
                    }
                }
            }
        }
        for (rays, kinds) in [
            (
                BISHOP_RAYS.as_slice(),
                [PieceKind::Bishop, PieceKind::Queen],
            ),
            (ROOK_RAYS.as_slice(), [PieceKind::Rook, PieceKind::Queen]),
        ] {
            for (delta_file, delta_rank) in rays {
                let mut other = file + delta_file;
                let mut target = rank + delta_rank;
                while (0..8).contains(&other) && (0..8).contains(&target) {
                    match self.board[square(other as usize, target as usize)] {
                        Some(piece) if piece.color == by && kinds.contains(&piece.kind) => {
                            return true;
                        }
                        Some(_) => break,
                        None => {
                            other += delta_file;
                            target += delta_rank;
                        }
                    }
                }
            }
        }
        false
    }

    /// Reports whether the side to move is in check.
    fn is_in_check(&self) -> bool {
        // Validated by construction: starting positions and `from_fen` hold both kings.
        let king = self
            .king_square(self.side)
            .expect("side to move has a king");
        self.is_attacked(king, self.side.other())
    }

    /// Pseudo-legal moves; callers filter them through make-and-check.
    fn pseudo_moves(&self) -> Vec<ChessMove> {
        let mut moves = Vec::with_capacity(64);
        let side = self.side;
        let enemy = side.other();
        let forward: i32 = if side == Color::White { 1 } else { -1 };
        let start_rank = if side == Color::White { 1 } else { 6 };
        let promo_rank = if side == Color::White { 6 } else { 1 };
        for (from, piece) in self.board.iter().enumerate() {
            let Some(piece) = piece else { continue };
            if piece.color != side {
                continue;
            }
            let file = file_of(from) as i32;
            let rank = rank_of(from) as i32;
            match piece.kind {
                PieceKind::Pawn => self.push_pawn_moves(
                    &mut moves, from, file, rank, forward, start_rank, promo_rank, enemy,
                ),
                PieceKind::Knight => {
                    push_step_moves(
                        &self.board,
                        &mut moves,
                        from,
                        file,
                        rank,
                        &KNIGHT_STEPS,
                        enemy,
                    );
                }
                PieceKind::King => {
                    push_step_moves(
                        &self.board,
                        &mut moves,
                        from,
                        file,
                        rank,
                        &KING_STEPS,
                        enemy,
                    );
                }
                PieceKind::Bishop => {
                    push_ray_moves(
                        &self.board,
                        &mut moves,
                        from,
                        file,
                        rank,
                        &BISHOP_RAYS,
                        enemy,
                    );
                }
                PieceKind::Rook => {
                    push_ray_moves(&self.board, &mut moves, from, file, rank, &ROOK_RAYS, enemy);
                }
                PieceKind::Queen => {
                    push_ray_moves(
                        &self.board,
                        &mut moves,
                        from,
                        file,
                        rank,
                        &BISHOP_RAYS,
                        enemy,
                    );
                    push_ray_moves(&self.board, &mut moves, from, file, rank, &ROOK_RAYS, enemy);
                }
            }
        }
        self.push_castling_moves(&mut moves);
        moves
    }

    /// Pushes, captures, promotions, and en passant for one pawn.
    #[allow(clippy::too_many_arguments)]
    fn push_pawn_moves(
        &self,
        moves: &mut Vec<ChessMove>,
        from: usize,
        file: i32,
        rank: i32,
        forward: i32,
        start_rank: i32,
        promo_rank: i32,
        enemy: Color,
    ) {
        if !(0..8).contains(&(rank + forward)) {
            return;
        }
        let target_rank = (rank + forward) as usize;
        let target = square(file as usize, target_rank);
        if self.board[target].is_none() {
            if rank == promo_rank {
                for promotion in [
                    PieceKind::Queen,
                    PieceKind::Rook,
                    PieceKind::Bishop,
                    PieceKind::Knight,
                ] {
                    moves.push(ChessMove {
                        promotion: Some(promotion),
                        ..ChessMove::quiet(from, target)
                    });
                }
            } else {
                moves.push(ChessMove::quiet(from, target));
            }
            if rank == start_rank {
                let jump = square(file as usize, (rank + 2 * forward) as usize);
                if self.board[jump].is_none() {
                    moves.push(ChessMove {
                        double_push: true,
                        ..ChessMove::quiet(from, jump)
                    });
                }
            }
        }
        for delta in [-1, 1] {
            let other = file + delta;
            if !(0..8).contains(&other) {
                continue;
            }
            let capture_square = square(other as usize, target_rank);
            if let Some(victim) = self.board[capture_square] {
                if victim.color != enemy {
                    continue;
                }
                if rank == promo_rank {
                    for promotion in [
                        PieceKind::Queen,
                        PieceKind::Rook,
                        PieceKind::Bishop,
                        PieceKind::Knight,
                    ] {
                        moves.push(ChessMove {
                            promotion: Some(promotion),
                            capture: true,
                            ..ChessMove::quiet(from, capture_square)
                        });
                    }
                } else {
                    moves.push(ChessMove {
                        capture: true,
                        ..ChessMove::quiet(from, capture_square)
                    });
                }
            }
        }
        if let Some(ep) = self.en_passant
            && (file_of(ep) as i32 - file).abs() == 1
            && rank_of(ep) as i32 == rank + forward
        {
            moves.push(ChessMove {
                capture: true,
                en_passant: true,
                ..ChessMove::quiet(from, ep)
            });
        }
    }

    /// Castling moves; the king may not castle out of, through, or into check.
    fn push_castling_moves(&self, moves: &mut Vec<ChessMove>) {
        let side = self.side;
        let enemy = side.other();
        let home = if side == Color::White { 4 } else { 60 };
        if self.board[home]
            != Some(Piece {
                color: side,
                kind: PieceKind::King,
            })
            || self.is_attacked(home, enemy)
        {
            return;
        }
        let rook = Piece {
            color: side,
            kind: PieceKind::Rook,
        };
        // Each leg needs its right, its home rook, an empty path, and safe
        // transit squares. Queenside knight files must be empty but may be attacked.
        type CastleLeg<'a> = (u8, usize, usize, &'a [usize], &'a [usize]);
        let legs: [CastleLeg<'_>; 2] = if side == Color::White {
            [
                (CastlingRights::WHITE_KINGSIDE, 7, 6, &[5, 6], &[5, 6]),
                (CastlingRights::WHITE_QUEENSIDE, 0, 2, &[1, 2, 3], &[2, 3]),
            ]
        } else {
            [
                (CastlingRights::BLACK_KINGSIDE, 63, 62, &[61, 62], &[61, 62]),
                (
                    CastlingRights::BLACK_QUEENSIDE,
                    56,
                    58,
                    &[57, 58, 59],
                    &[58, 59],
                ),
            ]
        };
        for (flag, rook_square, king_to, empty, transit) in legs {
            if self.castling.has(flag)
                && self.board[rook_square] == Some(rook)
                && empty.iter().all(|square| self.board[*square].is_none())
                && transit
                    .iter()
                    .all(|square| !self.is_attacked(*square, enemy))
            {
                moves.push(ChessMove {
                    castling: true,
                    ..ChessMove::quiet(home, king_to)
                });
            }
        }
    }
}

/// Knight and king steps for one piece.
fn push_step_moves(
    board: &[Option<Piece>; 64],
    moves: &mut Vec<ChessMove>,
    from: usize,
    file: i32,
    rank: i32,
    steps: &[(i32, i32)],
    enemy: Color,
) {
    for (delta_file, delta_rank) in steps {
        let other = file + delta_file;
        let target_rank = rank + delta_rank;
        if !(0..8).contains(&other) || !(0..8).contains(&target_rank) {
            continue;
        }
        let to = square(other as usize, target_rank as usize);
        match board[to] {
            None => moves.push(ChessMove::quiet(from, to)),
            Some(victim) if victim.color == enemy => moves.push(ChessMove {
                capture: true,
                ..ChessMove::quiet(from, to)
            }),
            Some(_) => {}
        }
    }
}

/// Slider rays for one bishop, rook, or queen.
fn push_ray_moves(
    board: &[Option<Piece>; 64],
    moves: &mut Vec<ChessMove>,
    from: usize,
    file: i32,
    rank: i32,
    rays: &[(i32, i32)],
    enemy: Color,
) {
    for (delta_file, delta_rank) in rays {
        let mut other = file + delta_file;
        let mut target_rank = rank + delta_rank;
        while (0..8).contains(&other) && (0..8).contains(&target_rank) {
            let to = square(other as usize, target_rank as usize);
            match board[to] {
                None => moves.push(ChessMove::quiet(from, to)),
                Some(victim) => {
                    if victim.color == enemy {
                        moves.push(ChessMove {
                            capture: true,
                            ..ChessMove::quiet(from, to)
                        });
                    }
                    break;
                }
            }
            other += delta_file;
            target_rank += delta_rank;
        }
    }
}

impl ChessPosition {
    /// Legal moves via make-and-check filtering of pseudo-legal moves.
    fn legal_moves(&self) -> Vec<ChessMove> {
        let side = self.side;
        let mut position = self.clone();
        let mut legal = Vec::new();
        for chess_move in self.pseudo_moves() {
            let undo = position.make_move(chess_move);
            // Validated by construction; see `is_in_check`.
            let king = position
                .king_square(side)
                .expect("moved side keeps its king");
            if !position.is_attacked(king, position.side) {
                legal.push(chess_move);
            }
            position.undo_move(chess_move, &undo);
        }
        legal
    }

    /// Legal moves leaving one square.
    fn moves_from(&self, from: usize) -> Vec<ChessMove> {
        self.legal_moves()
            .into_iter()
            .filter(|chess_move| chess_move.from == from)
            .collect()
    }

    /// Applies a move, returning the state needed to reverse it.
    fn make_move(&mut self, chess_move: ChessMove) -> ChessUndo {
        let side = self.side;
        let enemy = side.other();
        let moving = self.board[chess_move.from]
            .map(|piece| piece.kind)
            .expect("moving piece exists");
        let mut undo = ChessUndo {
            moving,
            captured: None,
            captured_square: chess_move.to,
            castling: self.castling,
            en_passant: self.en_passant,
            halfmove: self.halfmove,
        };
        if chess_move.en_passant {
            let captured_square = if side == Color::White {
                chess_move.to - 8
            } else {
                chess_move.to + 8
            };
            undo.captured = self.board[captured_square];
            undo.captured_square = captured_square;
            self.board[captured_square] = None;
        } else if chess_move.capture {
            undo.captured = self.board[chess_move.to];
            undo.captured_square = chess_move.to;
        }
        self.board[chess_move.from] = None;
        self.board[chess_move.to] = Some(Piece {
            color: side,
            kind: chess_move.promotion.unwrap_or(moving),
        });
        if chess_move.castling {
            let (rook_from, rook_to) = match chess_move.to {
                6 => (7, 5),
                2 => (0, 3),
                62 => (63, 61),
                _ => (56, 59),
            };
            self.board[rook_from] = None;
            self.board[rook_to] = Some(Piece {
                color: side,
                kind: PieceKind::Rook,
            });
        }
        if moving == PieceKind::King {
            if side == Color::White {
                self.castling.remove(CastlingRights::WHITE_KINGSIDE);
                self.castling.remove(CastlingRights::WHITE_QUEENSIDE);
            } else {
                self.castling.remove(CastlingRights::BLACK_KINGSIDE);
                self.castling.remove(CastlingRights::BLACK_QUEENSIDE);
            }
        }
        for (home, right) in [
            (0, CastlingRights::WHITE_QUEENSIDE),
            (7, CastlingRights::WHITE_KINGSIDE),
            (56, CastlingRights::BLACK_QUEENSIDE),
            (63, CastlingRights::BLACK_KINGSIDE),
        ] {
            if chess_move.from == home || chess_move.to == home {
                self.castling.remove(right);
            }
        }
        self.en_passant = chess_move.double_push.then(|| {
            if side == Color::White {
                chess_move.from + 8
            } else {
                chess_move.from - 8
            }
        });
        if moving == PieceKind::Pawn || undo.captured.is_some() {
            self.halfmove = 0;
        } else {
            self.halfmove += 1;
        }
        if side == Color::Black {
            self.fullmove += 1;
        }
        self.side = enemy;
        undo
    }

    /// Reverses a move previously applied with [`ChessPosition::make_move`].
    fn undo_move(&mut self, chess_move: ChessMove, undo: &ChessUndo) {
        let side = self.side.other();
        self.side = side;
        if side == Color::Black {
            self.fullmove -= 1;
        }
        if chess_move.castling {
            let (rook_from, rook_to) = match chess_move.to {
                6 => (7, 5),
                2 => (0, 3),
                62 => (63, 61),
                _ => (56, 59),
            };
            self.board[rook_to] = None;
            self.board[rook_from] = Some(Piece {
                color: side,
                kind: PieceKind::Rook,
            });
        }
        self.board[chess_move.to] = None;
        self.board[chess_move.from] = Some(Piece {
            color: side,
            kind: undo.moving,
        });
        if let Some(captured) = undo.captured {
            self.board[undo.captured_square] = Some(captured);
        }
        self.castling = undo.castling;
        self.en_passant = undo.en_passant;
        self.halfmove = undo.halfmove;
    }

    /// Repetition key for threefold detection.
    fn position_key(&self) -> PositionKey {
        (self.board, self.side, self.castling.0, self.en_passant)
    }

    /// White-minus-Black material in centipawns, kings excluded.
    fn material_balance(&self) -> i32 {
        self.board
            .iter()
            .flatten()
            .map(|piece| {
                let value = piece.kind.value();
                if piece.color == Color::White {
                    value
                } else {
                    -value
                }
            })
            .sum()
    }
}

/// Standard Algebraic Notation without the check or mate suffix.
fn move_to_san(position: &ChessPosition, chess_move: ChessMove, legal: &[ChessMove]) -> String {
    if chess_move.castling {
        return if file_of(chess_move.to) > file_of(chess_move.from) {
            "O-O".to_owned()
        } else {
            "O-O-O".to_owned()
        };
    }
    let piece = position.board[chess_move.from].expect("SAN from-square holds a piece");
    if piece.kind == PieceKind::Pawn {
        let mut san = String::new();
        if chess_move.capture {
            san.push((b'a' + file_of(chess_move.from) as u8) as char);
            san.push('x');
        }
        san.push_str(&square_name(chess_move.to));
        if let Some(promotion) = chess_move.promotion {
            san.push('=');
            san.push(match promotion {
                PieceKind::Queen => 'Q',
                PieceKind::Rook => 'R',
                PieceKind::Bishop => 'B',
                PieceKind::Knight => 'N',
                PieceKind::Pawn | PieceKind::King => '?',
            });
        }
        return san;
    }
    let mut san = String::from(match piece.kind {
        PieceKind::Knight => "N",
        PieceKind::Bishop => "B",
        PieceKind::Rook => "R",
        PieceKind::Queen => "Q",
        PieceKind::King => "K",
        PieceKind::Pawn => "?",
    });
    let others: Vec<ChessMove> = legal
        .iter()
        .copied()
        .filter(|candidate| {
            candidate.to == chess_move.to
                && candidate.from != chess_move.from
                && !candidate.castling
                && position.board[candidate.from] == Some(piece)
        })
        .collect();
    if !others.is_empty() {
        let same_file = others
            .iter()
            .any(|candidate| file_of(candidate.from) == file_of(chess_move.from));
        let same_rank = others
            .iter()
            .any(|candidate| rank_of(candidate.from) == rank_of(chess_move.from));
        if !same_file {
            san.push((b'a' + file_of(chess_move.from) as u8) as char);
        } else if !same_rank {
            san.push_str(&(rank_of(chess_move.from) + 1).to_string());
        } else {
            san.push_str(&square_name(chess_move.from));
        }
    }
    if chess_move.capture {
        san.push('x');
    }
    san.push_str(&square_name(chess_move.to));
    san
}

/// Draws: bare kings, lone minor versus bare king, same-colour bishops.
fn insufficient_material(position: &ChessPosition) -> bool {
    let mut minors: [Vec<(usize, PieceKind)>; 2] = [Vec::new(), Vec::new()];
    for (square, piece) in position.board.iter().enumerate() {
        let Some(piece) = piece else { continue };
        match piece.kind {
            PieceKind::Pawn | PieceKind::Rook | PieceKind::Queen => return false,
            PieceKind::Knight | PieceKind::Bishop => {
                let side = usize::from(piece.color != Color::White);
                minors[side].push((square, piece.kind));
            }
            PieceKind::King => {}
        }
    }
    if minors[0].is_empty() && minors[1].is_empty() {
        return true;
    }
    if minors[0].len() + minors[1].len() == 1 {
        return true;
    }
    if minors[0].len() == 1
        && minors[1].len() == 1
        && minors[0][0].1 == PieceKind::Bishop
        && minors[1][0].1 == PieceKind::Bishop
    {
        let white_colour = (file_of(minors[0][0].0) + rank_of(minors[0][0].0)) % 2;
        let black_colour = (file_of(minors[1][0].0) + rank_of(minors[1][0].0)) % 2;
        return white_colour == black_colour;
    }
    false
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchStatus {
    Active,
    Check,
    Checkmate,
    Stalemate,
    Draw,
}

/// Repetition key: pieces, side, rights, and en passant square.
type PositionKey = ([Option<Piece>; 64], Color, u8, Option<usize>);

/// Stateful game: history, undo, repetition tracking, and status.
#[derive(Clone, Debug)]
pub(super) struct ChessMatch {
    position: ChessPosition,
    history: Vec<(ChessMove, String)>,
    undo_stack: Vec<ChessUndo>,
    counts: HashMap<PositionKey, u32>,
    status: MatchStatus,
    draw_reason: String,
}

impl ChessMatch {
    /// Fresh game from the standard starting array.
    fn new() -> Self {
        let mut game = Self {
            position: ChessPosition::starting(),
            history: Vec::new(),
            undo_stack: Vec::new(),
            counts: HashMap::new(),
            status: MatchStatus::Active,
            draw_reason: String::new(),
        };
        game.record_position();
        game.refresh_status();
        game
    }

    /// Game from a FEN string, used by tests and future setup boards.
    #[cfg(test)]
    fn from_fen(fen: &str) -> Result<Self, String> {
        let mut game = Self {
            position: ChessPosition::from_fen(fen)?,
            history: Vec::new(),
            undo_stack: Vec::new(),
            counts: HashMap::new(),
            status: MatchStatus::Active,
            draw_reason: String::new(),
        };
        game.record_position();
        game.refresh_status();
        Ok(game)
    }

    fn record_position(&mut self) {
        let key = self.position.position_key();
        *self.counts.entry(key).or_insert(0) += 1;
    }

    fn unrecord_position(&mut self) {
        let key = self.position.position_key();
        if let Some(count) = self.counts.get_mut(&key) {
            *count -= 1;
            if *count == 0 {
                self.counts.remove(&key);
            }
        }
    }

    /// All legal moves for the side to move.
    fn legal_moves(&self) -> Vec<ChessMove> {
        self.position.legal_moves()
    }

    /// Legal moves leaving one square.
    fn moves_from(&self, from: usize) -> Vec<ChessMove> {
        self.position.moves_from(from)
    }

    /// Whether checkmate, stalemate, or a draw ended the game.
    fn is_game_over(&self) -> bool {
        matches!(
            self.status,
            MatchStatus::Checkmate | MatchStatus::Stalemate | MatchStatus::Draw
        )
    }

    /// Plays a legal move, returning its SAN.
    fn make_move(&mut self, chess_move: ChessMove) -> Result<String, String> {
        if self.is_game_over() {
            return Err(format!("game is over: {:?}", self.status));
        }
        let legal = self.legal_moves();
        if !legal.contains(&chess_move) {
            return Err(format!("illegal move {}", chess_move.uci()));
        }
        let mut san = move_to_san(&self.position, chess_move, &legal);
        let undo = self.position.make_move(chess_move);
        self.undo_stack.push(undo);
        self.record_position();
        self.refresh_status();
        match self.status {
            MatchStatus::Checkmate => san.push('#'),
            MatchStatus::Check => san.push('+'),
            MatchStatus::Active | MatchStatus::Stalemate | MatchStatus::Draw => {}
        }
        self.history.push((chess_move, san.clone()));
        Ok(san)
    }

    /// Takes back the last move.
    fn undo_move(&mut self) -> Result<ChessMove, String> {
        let (chess_move, _) = self
            .history
            .pop()
            .ok_or_else(|| "nothing to undo".to_owned())?;
        let undo = self
            .undo_stack
            .pop()
            .expect("history and undo stack stay paired");
        self.unrecord_position();
        self.position.undo_move(chess_move, &undo);
        self.refresh_status();
        Ok(chess_move)
    }

    fn refresh_status(&mut self) {
        let legal = self.position.legal_moves();
        let in_check = self.position.is_in_check();
        if legal.is_empty() {
            self.status = if in_check {
                MatchStatus::Checkmate
            } else {
                MatchStatus::Stalemate
            };
            self.draw_reason.clear();
            return;
        }
        if self.position.halfmove >= 100 {
            self.status = MatchStatus::Draw;
            "fifty-move rule".clone_into(&mut self.draw_reason);
            return;
        }
        if self
            .counts
            .get(&self.position.position_key())
            .is_some_and(|count| *count >= 3)
        {
            self.status = MatchStatus::Draw;
            "threefold repetition".clone_into(&mut self.draw_reason);
            return;
        }
        if insufficient_material(&self.position) {
            self.status = MatchStatus::Draw;
            "insufficient material".clone_into(&mut self.draw_reason);
            return;
        }
        self.status = if in_check {
            MatchStatus::Check
        } else {
            MatchStatus::Active
        };
        self.draw_reason.clear();
    }
}

/// AI strength, matching the Python demo ladder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ChessDifficulty {
    Harmless,
    #[default]
    Easy,
    Medium,
    Hard,
}

impl ChessDifficulty {
    /// Search depth; Harmless plays randomly instead of searching.
    const fn depth(self) -> u32 {
        match self {
            Self::Harmless => 0,
            Self::Easy => 1,
            Self::Medium => 2,
            Self::Hard => 3,
        }
    }

    const fn node_cap(self) -> u64 {
        NODE_CAP[self as usize]
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Harmless => "Harmless (random)",
            Self::Easy => "Easy (depth 1)",
            Self::Medium => "Medium (depth 2)",
            Self::Hard => "Hard (depth 3)",
        }
    }

    pub(super) const fn value(self) -> &'static str {
        match self {
            Self::Harmless => "harmless",
            Self::Easy => "easy",
            Self::Medium => "medium",
            Self::Hard => "hard",
        }
    }

    fn from_value(value: &str) -> Option<Self> {
        match value {
            "harmless" => Some(Self::Harmless),
            "easy" => Some(Self::Easy),
            "medium" => Some(Self::Medium),
            "hard" => Some(Self::Hard),
            _ => None,
        }
    }
}

/// Centipawn score from White's perspective.
fn evaluate(position: &ChessPosition) -> i32 {
    let mut score = 0;
    let mut white_bishops = 0;
    let mut black_bishops = 0;
    for (square, piece) in position.board.iter().enumerate() {
        let Some(piece) = piece else { continue };
        // `0b11_1000` mirrors the rank so Black reads White's tables upside down.
        let table = piece.kind.table()[if piece.color == Color::White {
            square
        } else {
            square ^ 0b11_1000
        }];
        let value = piece.kind.value() + table;
        if piece.kind == PieceKind::Bishop {
            if piece.color == Color::White {
                white_bishops += 1;
            } else {
                black_bishops += 1;
            }
        }
        score += if piece.color == Color::White {
            value
        } else {
            -value
        };
    }
    if white_bishops >= 2 {
        score += 30;
    }
    if black_bishops >= 2 {
        score -= 30;
    }
    score
}

/// Centipawn score from the side-to-move perspective.
fn evaluate_relative(position: &ChessPosition) -> i32 {
    let score = evaluate(position);
    if position.side == Color::White {
        score
    } else {
        -score
    }
}

/// MVV-LVA capture ordering with a promotion bonus; higher searches first.
fn order_score(position: &ChessPosition, chess_move: ChessMove) -> i32 {
    let mut score = match chess_move.promotion {
        Some(PieceKind::Queen) => 900,
        Some(_) => 500,
        None => 0,
    };
    if chess_move.capture {
        let victim = position.board[chess_move.to].map_or(100, |piece| piece.kind.value());
        let attacker = position.board[chess_move.from].map_or(0, |piece| piece.kind.value());
        score += 10_000 + victim * 10 - attacker;
    }
    score
}

/// Negamax search; `Err` escapes the whole tree once the node cap trips.
fn search(
    position: &mut ChessPosition,
    depth: u32,
    mut alpha: i32,
    beta: i32,
    ply: u32,
    budget: &mut u64,
    cap: u64,
) -> Result<i32, ()> {
    *budget += 1;
    if *budget > cap {
        return Err(());
    }
    if position.halfmove >= 100 {
        return Ok(0);
    }
    if depth == 0 {
        return Ok(evaluate_relative(position));
    }
    let mut moves = position.legal_moves();
    if moves.is_empty() {
        if position.is_in_check() {
            return Ok(-(MATE_SCORE - ply as i32));
        }
        return Ok(0);
    }
    moves.sort_by_key(|chess_move| -order_score(position, *chess_move));
    let mut best = -INF_SCORE;
    for chess_move in moves {
        let undo = position.make_move(chess_move);
        let score =
            search(position, depth - 1, -beta, -alpha, ply + 1, budget, cap).map(|child| -child);
        position.undo_move(chess_move, &undo);
        let score = score?;
        if score > best {
            best = score;
        }
        if best > alpha {
            alpha = best;
        }
        if alpha >= beta {
            break;
        }
    }
    Ok(best)
}

/// Best move for the side to move, with score and visited node count.
fn best_move(
    game: &ChessMatch,
    difficulty: ChessDifficulty,
    rng_state: &mut u64,
) -> Option<(ChessMove, i32, u64)> {
    let mut legal = game.legal_moves();
    if legal.is_empty() {
        return None;
    }
    if difficulty == ChessDifficulty::Harmless {
        let pick = next_random(rng_state) as usize % legal.len();
        return Some((legal[pick], 0, 0));
    }
    legal.sort_by_key(|chess_move| -order_score(&game.position, *chess_move));
    let mut position = game.position.clone();
    let mut budget = 0;
    let cap = difficulty.node_cap();
    let mut best_move = legal[0];
    let mut best_score = -INF_SCORE;
    let mut alpha = -INF_SCORE;
    for chess_move in legal {
        let undo = position.make_move(chess_move);
        let score = search(
            &mut position,
            difficulty.depth() - 1,
            -INF_SCORE,
            -alpha,
            1,
            &mut budget,
            cap,
        )
        .map(|child| -child);
        position.undo_move(chess_move, &undo);
        let Ok(score) = score else { break };
        if score > best_score {
            best_score = score;
            best_move = chess_move;
        }
        if score > alpha {
            alpha = score;
        }
    }
    Some((best_move, best_score, budget))
}

/// Resolves a UCI string (`e2e4`, `e7e8q`) to a legal move.
#[cfg(test)]
fn move_from_uci(game: &ChessMatch, uci: &str) -> Result<ChessMove, String> {
    if uci.len() != 4 && uci.len() != 5 {
        return Err(format!("malformed UCI move {uci}"));
    }
    let from = square_from_name(&uci[0..2]).ok_or_else(|| format!("malformed UCI move {uci}"))?;
    let to = square_from_name(&uci[2..4]).ok_or_else(|| format!("malformed UCI move {uci}"))?;
    let promotion = if uci.len() == 5 {
        Some(match uci.as_bytes()[4].to_ascii_lowercase() {
            b'q' => PieceKind::Queen,
            b'r' => PieceKind::Rook,
            b'b' => PieceKind::Bishop,
            b'n' => PieceKind::Knight,
            _ => return Err(format!("malformed UCI promotion {uci}")),
        })
    } else {
        None
    };
    game.legal_moves()
        .into_iter()
        .find(|chess_move| {
            chess_move.from == from && chess_move.to == to && chess_move.promotion == promotion
        })
        .ok_or_else(|| format!("illegal move {uci}"))
}

/// Human opponent: a second player or the built-in AI as Black.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ChessMode {
    TwoPlayer,
    #[default]
    VsAi,
}

impl ChessMode {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::TwoPlayer => "Two players",
            Self::VsAi => "You (White) vs AI (Black)",
        }
    }

    pub(super) const fn value(self) -> &'static str {
        match self {
            Self::TwoPlayer => "two",
            Self::VsAi => "ai",
        }
    }

    fn from_value(value: &str) -> Option<Self> {
        match value {
            "two" => Some(Self::TwoPlayer),
            "ai" => Some(Self::VsAi),
            _ => None,
        }
    }
}

/// Native chess board: selection, AI replies, and retained scene painting.
#[derive(Clone, Debug)]
pub(super) struct ChessGame {
    game: ChessMatch,
    selected: Option<(usize, usize)>,
    cursor: (usize, usize),
    flipped: bool,
    pub(super) mode: ChessMode,
    pub(super) difficulty: ChessDifficulty,
    pub(super) promotion: PieceKind,
    last_move: Option<(usize, usize)>,
    hint_move: Option<usize>,
    notice: String,
    rng_state: u64,
}

impl Default for ChessGame {
    fn default() -> Self {
        Self {
            game: ChessMatch::new(),
            selected: None,
            cursor: (6, 4),
            flipped: false,
            mode: ChessMode::default(),
            difficulty: ChessDifficulty::default(),
            promotion: PieceKind::Queen,
            last_move: None,
            hint_move: None,
            notice: "You play White; the AI replies as Black.".to_owned(),
            // SplitMix64 stream; any nonzero seed plays a varied Harmless game.
            rng_state: 0x9E37_79B9_7F4A_7C15,
        }
    }
}

impl ChessGame {
    /// Display cell to square, honouring board orientation.
    fn display_to_square(&self, row: usize, col: usize) -> usize {
        if self.flipped {
            square(7 - col, row)
        } else {
            square(col, 7 - row)
        }
    }

    /// Square to display cell, honouring board orientation.
    fn square_to_display(&self, square: usize) -> (usize, usize) {
        let (file, rank) = (file_of(square), rank_of(square));
        if self.flipped {
            (rank, 7 - file)
        } else {
            (7 - rank, file)
        }
    }

    /// Selects a piece or plays the selected piece to this cell.
    fn click(&mut self, row: usize, col: usize) -> bool {
        if row >= 8 || col >= 8 {
            return false;
        }
        if self.game.is_game_over() {
            "Game over — press New game to play again.".clone_into(&mut self.notice);
            return true;
        }
        if self.mode == ChessMode::VsAi && self.game.position.side != Color::White {
            "The AI plays Black — switch to Two players to move both sides."
                .clone_into(&mut self.notice);
            return true;
        }
        let square = self.display_to_square(row, col);
        let Some(selected) = self.selected else {
            let piece = self.game.position.board[square];
            if piece.is_none_or(|piece| piece.color != self.game.position.side) {
                self.notice = format!("Select a {} piece to move.", self.game.position.side.name());
                return true;
            }
            let targets = self.game.moves_from(square).len();
            self.selected = Some((row, col));
            self.notice = format!(
                "Selected {} — {targets} legal move(s). Pick a highlighted square.",
                square_name(square)
            );
            return true;
        };
        if selected == (row, col) {
            self.selected = None;
            "Selection cleared.".clone_into(&mut self.notice);
            return true;
        }
        let from = self.display_to_square(selected.0, selected.1);
        let candidates: Vec<ChessMove> = self
            .game
            .moves_from(from)
            .into_iter()
            .filter(|chess_move| chess_move.to == square)
            .collect();
        if candidates.is_empty() {
            let piece = self.game.position.board[square];
            if piece.is_some_and(|piece| piece.color == self.game.position.side) {
                let targets = self.game.moves_from(square).len();
                self.selected = Some((row, col));
                self.notice = format!(
                    "Selected {} — {targets} legal move(s).",
                    square_name(square)
                );
                return true;
            }
            self.selected = None;
            "Illegal move — pick a highlighted square.".clone_into(&mut self.notice);
            return true;
        }
        let choice = candidates
            .iter()
            .copied()
            .find(|candidate| candidate.promotion.unwrap_or(PieceKind::Queen) == self.promotion)
            .unwrap_or(candidates[0]);
        self.play_human_move(choice)
    }

    /// Plays a human move, then the AI reply in `VsAi` mode.
    fn play_human_move(&mut self, chess_move: ChessMove) -> bool {
        let Ok(san) = self.game.make_move(chess_move) else {
            "Illegal move — pick a highlighted square.".clone_into(&mut self.notice);
            return true;
        };
        self.last_move = Some((chess_move.from, chess_move.to));
        self.hint_move = None;
        self.selected = None;
        if self.game.is_game_over() {
            self.notice = format!("Played {san}. {}", self.game_over_text());
            return true;
        }
        if self.mode == ChessMode::VsAi {
            let Some((reply, _, _)) = best_move(&self.game, self.difficulty, &mut self.rng_state)
            else {
                self.notice = format!("Played {san}. {}", self.side_to_move_text());
                return true;
            };
            let Ok(reply_san) = self.game.make_move(reply) else {
                self.notice = format!("Played {san}. {}", self.side_to_move_text());
                return true;
            };
            self.last_move = Some((reply.from, reply.to));
            if self.game.is_game_over() {
                self.notice = format!(
                    "Played {san}; AI replied {reply_san}. {}",
                    self.game_over_text()
                );
            } else {
                self.notice = format!(
                    "Played {san}; AI replied {reply_san}. {}",
                    self.side_to_move_text()
                );
            }
            return true;
        }
        self.notice = format!("Played {san}. {}", self.side_to_move_text());
        true
    }

    fn side_to_move_text(&self) -> String {
        let mut text = format!("{} to move.", self.game.position.side.name());
        if self.game.status == MatchStatus::Check {
            text.push_str(" Check!");
        }
        text
    }

    fn game_over_text(&self) -> String {
        match self.game.status {
            MatchStatus::Checkmate => {
                let winner = self.game.position.side.other().name();
                format!("Checkmate! {winner} wins.")
            }
            MatchStatus::Stalemate => "Stalemate — drawn game.".to_owned(),
            MatchStatus::Draw => format!("Draw — {}.", self.game.draw_reason),
            MatchStatus::Active | MatchStatus::Check => "Game over.".to_owned(),
        }
    }

    /// Starts over with a fresh board and cursor.
    pub(super) fn reset(&mut self) -> bool {
        *self = Self {
            rng_state: self.rng_state,
            ..Self::default()
        };
        "New game — White to move.".clone_into(&mut self.notice);
        true
    }

    /// Takes back one round: two plies against the AI, one otherwise.
    pub(super) fn undo(&mut self) -> bool {
        if self.game.history.is_empty() {
            "Nothing to undo.".clone_into(&mut self.notice);
            return true;
        }
        let mut steps = 1;
        if self.mode == ChessMode::VsAi
            && self.game.history.len() >= 2
            && self.game.position.side == Color::White
        {
            steps = 2;
        }
        let mut undone = Vec::new();
        for _ in 0..steps {
            if self.game.history.is_empty() {
                break;
            }
            if let Ok(chess_move) = self.game.undo_move() {
                undone.push(chess_move.uci());
            }
        }
        undone.reverse();
        self.selected = None;
        self.hint_move = None;
        self.last_move = self
            .game
            .history
            .last()
            .map(|(chess_move, _)| (chess_move.from, chess_move.to));
        self.notice = format!(
            "Undid {}. {}",
            undone.join(" and "),
            self.side_to_move_text()
        );
        true
    }

    /// Rotates the board 180 degrees.
    pub(super) fn flip(&mut self) -> bool {
        self.flipped = !self.flipped;
        self.selected = None;
        "Board flipped.".clone_into(&mut self.notice);
        true
    }

    /// Highlights the AI suggestion without playing it.
    pub(super) fn hint(&mut self) -> bool {
        if self.game.is_game_over() {
            "Game over — press New game to play again.".clone_into(&mut self.notice);
            return true;
        }
        let Some((suggestion, _, _)) = best_move(&self.game, self.difficulty, &mut self.rng_state)
        else {
            return false;
        };
        let san = move_to_san(&self.game.position, suggestion, &self.game.legal_moves());
        self.hint_move = Some(suggestion.to);
        self.selected = Some(self.square_to_display(suggestion.from));
        self.notice = format!("Hint: {san} ({}).", suggestion.uci());
        true
    }

    /// Plays the AI move for the side to move.
    pub(super) fn ai_move(&mut self) -> bool {
        if self.game.is_game_over() {
            "Game over — press New game to play again.".clone_into(&mut self.notice);
            return true;
        }
        let Some((suggestion, _, _)) = best_move(&self.game, self.difficulty, &mut self.rng_state)
        else {
            return false;
        };
        let Ok(san) = self.game.make_move(suggestion) else {
            return false;
        };
        self.last_move = Some((suggestion.from, suggestion.to));
        self.hint_move = None;
        self.selected = None;
        if self.game.is_game_over() {
            self.notice = format!("AI played {san}. {}", self.game_over_text());
        } else {
            self.notice = format!("AI played {san}. {}", self.side_to_move_text());
        }
        true
    }

    /// Switches opponents; the AI moves at once when it inherits Black.
    pub(super) fn select_mode(&mut self, value: &str) -> bool {
        let Some(mode) = ChessMode::from_value(value) else {
            return false;
        };
        self.mode = mode;
        self.selected = None;
        self.hint_move = None;
        if mode == ChessMode::VsAi
            && !self.game.is_game_over()
            && self.game.position.side == Color::Black
        {
            return self.ai_move();
        }
        self.notice = if mode == ChessMode::TwoPlayer {
            "Two players — move both sides.".to_owned()
        } else {
            "You play White; the AI replies as Black.".to_owned()
        };
        true
    }

    /// Switches search depth.
    pub(super) fn select_difficulty(&mut self, value: &str) -> bool {
        let Some(difficulty) = ChessDifficulty::from_value(value) else {
            return false;
        };
        self.difficulty = difficulty;
        self.notice = format!("AI strength: {}.", difficulty.label());
        true
    }

    /// Switches the promotion piece used by pointer play.
    pub(super) fn select_promotion(&mut self, value: &str) -> bool {
        let promotion = match value {
            "Q" => PieceKind::Queen,
            "R" => PieceKind::Rook,
            "B" => PieceKind::Bishop,
            "N" => PieceKind::Knight,
            _ => return false,
        };
        self.promotion = promotion;
        self.notice = format!("Pawns will promote to {}.", promotion.name());
        true
    }

    /// Promotion picker value for the details panel.
    pub(super) fn promotion_value(&self) -> &'static str {
        match self.promotion {
            PieceKind::Queen => "Q",
            PieceKind::Rook => "R",
            PieceKind::Bishop => "B",
            PieceKind::Knight => "N",
            // `select_promotion` only admits queen, rook, bishop, and knight.
            PieceKind::Pawn | PieceKind::King => unreachable!("promotion is always promotable"),
        }
    }

    /// Moves the keyboard cursor, clamped to the board.
    fn move_cursor(&mut self, row_delta: i32, col_delta: i32) -> bool {
        let row = (self.cursor.0 as i32 + row_delta).clamp(0, 7) as usize;
        let col = (self.cursor.1 as i32 + col_delta).clamp(0, 7) as usize;
        if (row, col) == self.cursor {
            return false;
        }
        self.cursor = (row, col);
        true
    }

    /// Routes pointer, key, and activation input from the surface.
    pub(super) fn handle(&mut self, event: Scene2DInput) -> bool {
        match event {
            Scene2DInput::Pointer {
                phase: gpui_ui_kit::scene2d::Scene2DPointerPhase::Up,
                cell: Some(cell),
                ..
            } => self.click(cell.row as usize, cell.column as usize),
            Scene2DInput::Key {
                phase: Scene2DKeyPhase::Down,
                key,
                repeat,
                ..
            } if !repeat => {
                let (dr, dc) = match key.as_str() {
                    "ArrowLeft" => (0, -1),
                    "ArrowRight" => (0, 1),
                    "ArrowUp" => (-1, 0),
                    "ArrowDown" => (1, 0),
                    _ => (0, 0),
                };
                if dr != 0 || dc != 0 {
                    return self.move_cursor(dr, dc);
                }
                match key.as_str() {
                    "Enter" | "Space" => self.click(self.cursor.0, self.cursor.1),
                    "u" => self.undo(),
                    "f" => self.flip(),
                    _ => false,
                }
            }
            Scene2DInput::Activate {
                cell: Some(cell), ..
            } => self.click(cell.row as usize, cell.column as usize),
            Scene2DInput::Lifecycle { .. } => false,
            _ => false,
        }
    }

    /// One-line status for the details panel.
    pub(super) fn status(&self) -> String {
        self.notice.clone()
    }

    /// `Move 12 · White to move · White +2` summary for the details panel.
    pub(super) fn details(&self) -> String {
        let balance = self.game.position.material_balance();
        let material = if balance == 0 {
            "Equal".to_owned()
        } else {
            let leader = if balance > 0 { "White" } else { "Black" };
            format!("{leader} +{}", balance.abs() as f32 / 100.0)
        };
        format!(
            "Move {} · {} to move · {material}",
            self.game.position.fullmove,
            self.game.position.side.name()
        )
    }

    /// Trailing moves in SAN for the details panel.
    pub(super) fn moves_text(&self) -> String {
        if self.game.history.is_empty() {
            return "No moves yet — 1. e4 is a fine start.".to_owned();
        }
        let start = self.game.history.len().saturating_sub(8);
        let mut parts = Vec::new();
        for (index, (_, san)) in self.game.history.iter().enumerate().skip(start) {
            if index % 2 == 0 {
                parts.push(format!("{}. {san}", index / 2 + 1));
            } else {
                parts.push(san.clone());
            }
        }
        if start > 0 {
            format!("… {}", parts.join(" "))
        } else {
            parts.join(" ")
        }
    }

    /// Paints squares, highlights, and pieces as a retained scene.
    pub(super) fn scene(&self, revision: u64) -> Scene2DScene {
        let pad = 18.0;
        let cell = 48.0;
        let gap = 2.0;
        let width = pad * 2.0 + 8.0 * cell + 7.0 * gap;
        let mut scene = new_scene(
            width,
            width,
            "Chess board",
            "Full-rules chess. Select a piece, then a highlighted square.",
        );
        scene.grid = Some(Scene2DGrid {
            rows: 8,
            columns: 8,
            x: pad,
            y: pad,
            cell_width: cell,
            cell_height: cell,
            gap,
            row_labels: (1..=8).map(|value| value.to_string()).collect(),
            column_labels: (1..=8).map(|value| value.to_string()).collect(),
        });
        scene.nodes.push(rounded_node(
            "chess-well",
            None,
            SceneRect::new(5.0, 5.0, width - 10.0, width - 10.0),
            18.0,
            color(0.075, 0.12, 0.18),
            None,
            None,
        ));
        let selected_square = self
            .selected
            .map(|(row, col)| self.display_to_square(row, col));
        let mut targets = HashMap::new();
        if let Some(from) = selected_square
            && !self.game.is_game_over()
        {
            for chess_move in self.game.moves_from(from) {
                targets.entry(chess_move.to).or_insert(chess_move);
            }
        }
        let mut last_squares = Vec::new();
        if let Some((from, to)) = self.last_move {
            last_squares.push(from);
            last_squares.push(to);
        }
        let check_square = if matches!(
            self.game.status,
            MatchStatus::Check | MatchStatus::Checkmate
        ) {
            self.game.position.king_square(self.game.position.side)
        } else {
            None
        };
        for row in 0..8 {
            for col in 0..8 {
                self.paint_cell(
                    &mut scene,
                    row,
                    col,
                    pad,
                    cell,
                    gap,
                    selected_square,
                    &targets,
                    &last_squares,
                    check_square,
                );
            }
        }
        scene.revision = revision;
        scene.input = Scene2DInputConfig {
            pointer: true,
            continuous: false,
            capture: true,
            keyboard: true,
        };
        scene
    }

    /// Paints one square with its overlays and optional piece.
    #[allow(clippy::too_many_arguments)]
    fn paint_cell(
        &self,
        scene: &mut Scene2DScene,
        row: usize,
        col: usize,
        pad: f32,
        cell: f32,
        gap: f32,
        selected_square: Option<usize>,
        targets: &HashMap<usize, ChessMove>,
        last_squares: &[usize],
        check_square: Option<usize>,
    ) {
        let square = self.display_to_square(row, col);
        let light = (file_of(square) + rank_of(square)) % 2 == 1;
        let piece = self.game.position.board[square];
        let name = square_name(square);
        let value = piece.map_or_else(
            || "empty".to_owned(),
            |piece| {
                format!(
                    "{} {}",
                    piece.color.name().to_ascii_lowercase(),
                    piece.kind.name()
                )
            },
        );
        let selected = Some(square) == selected_square;
        let x = pad + col as f32 * (cell + gap);
        let y = pad + row as f32 * (cell + gap);
        let rect = SceneRect::new(x, y, cell, cell);
        let (edge_color, edge_width) = if selected {
            (GOLD, 3.0)
        } else if (row, col) == self.cursor {
            (PAPER, 2.0)
        } else {
            (GRID_LINE, 0.7)
        };
        scene.nodes.push(rounded_node(
            &format!("chess-cell-{row}-{col}"),
            Some(format!("chess-cell-{row}-{col}")),
            rect,
            5.0,
            if light { LIGHT_SQUARE } else { DARK_SQUARE },
            Some(stroke(edge_width, edge_color)),
            Some(semantic(
                Scene2DSemanticRole::GridCell,
                format!("Square {name}"),
                value,
                selected,
            )),
        ));
        if last_squares.contains(&square) {
            scene.nodes.push(rounded_node(
                &format!("chess-last-{row}-{col}"),
                None,
                inset_rect(rect, 2.0),
                5.0,
                color_alpha(1.0, 0.827, 0.435, 0.38),
                None,
                None,
            ));
        }
        if Some(square) == self.hint_move {
            scene.nodes.push(rounded_node(
                &format!("chess-hint-{row}-{col}"),
                None,
                inset_rect(rect, 2.0),
                5.0,
                color_alpha(1.0, 0.827, 0.435, 0.0),
                Some(stroke(3.0, GOLD)),
                None,
            ));
        }
        if Some(square) == check_square {
            scene.nodes.push(circle_node(
                &format!("chess-check-{row}-{col}"),
                None,
                ScenePoint::new(x + cell * 0.5, y + cell * 0.5),
                cell * 0.5 - 3.0,
                None,
                Some(stroke(3.5, DANGER)),
                None,
            ));
        }
        if let Some(target) = targets.get(&square) {
            let center = ScenePoint::new(x + cell * 0.5, y + cell * 0.5);
            if target.capture {
                scene.nodes.push(circle_node(
                    &format!("chess-target-{row}-{col}"),
                    None,
                    center,
                    cell * 0.5 - 5.0,
                    None,
                    Some(stroke(3.0, DANGER)),
                    None,
                ));
            } else {
                scene.nodes.push(circle_node(
                    &format!("chess-target-{row}-{col}"),
                    None,
                    center,
                    7.0,
                    Some(brush(MINT)),
                    Some(stroke(1.2, PAPER)),
                    None,
                ));
            }
        }
        if let Some(piece) = piece {
            // Backing discs keep both armies readable on either square shade.
            let (disc_fill, disc_edge) = if piece.color == Color::White {
                (BLACK_PIECE, PAPER)
            } else {
                (WHITE_PIECE, GRID_LINE)
            };
            scene.nodes.push(circle_node(
                &format!("chess-disc-{row}-{col}"),
                None,
                ScenePoint::new(x + cell * 0.5, y + cell * 0.5),
                cell * 0.5 - 4.0,
                Some(brush(disc_fill)),
                Some(stroke(1.4, disc_edge)),
                None,
            ));
            let mut glyph = text_node(
                &format!("chess-piece-{row}-{col}"),
                None,
                ScenePoint::new(x + cell * 0.5, y + cell * 0.5 - 30.0 * 0.625),
                piece.kind.glyph(piece.color),
                30.0,
                if piece.color == Color::White {
                    WHITE_PIECE
                } else {
                    BLACK_PIECE
                },
                Scene2DTextAlign::Center,
                None,
            );
            glyph.transition = Some(transition(120));
            scene.nodes.push(glyph);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_ui_kit::scene2d::{Scene2DNodeKind, Scene2DState};

    fn play_uci(game: &mut ChessMatch, moves: &[&str]) {
        for uci in moves {
            let chess_move = move_from_uci(game, uci).expect("scripted move is legal");
            game.make_move(chess_move).expect("scripted move plays");
        }
    }

    fn perft(position: &ChessPosition, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        let mut nodes = 0;
        let mut position = position.clone();
        for chess_move in position.legal_moves() {
            let undo = position.make_move(chess_move);
            nodes += perft(&position, depth - 1);
            position.undo_move(chess_move, &undo);
        }
        nodes
    }

    #[test]
    fn start_has_twenty_moves_and_round_trips_fen() {
        let game = ChessMatch::new();
        assert_eq!(game.legal_moves().len(), 20);
        assert_eq!(game.position.to_fen(), START_FEN);
        let rebuilt = ChessPosition::from_fen(START_FEN).expect("start FEN parses");
        assert_eq!(rebuilt.to_fen(), START_FEN);
        ChessPosition::from_fen("not a fen").unwrap_err();
        ChessPosition::from_fen("8/8/8/8/8/8/8/8 w - - 0 1").unwrap_err();
    }

    #[test]
    fn perft_matches_known_node_counts() {
        let start = ChessPosition::starting();
        assert_eq!(perft(&start, 2), 400);
        assert_eq!(perft(&start, 3), 8902);
    }

    #[test]
    fn make_and_undo_restores_fen() {
        let mut position = ChessPosition::starting();
        let before = position.to_fen();
        let moves = position.legal_moves();
        for chess_move in moves.into_iter().take(8) {
            let undo = position.make_move(chess_move);
            position.undo_move(chess_move, &undo);
            assert_eq!(position.to_fen(), before);
        }
    }

    #[test]
    fn fools_mate_is_checkmate_with_san() {
        let mut game = ChessMatch::new();
        play_uci(&mut game, &["f2f3", "e7e5", "g2g4", "d8h4"]);
        assert_eq!(game.status, MatchStatus::Checkmate);
        let sans: Vec<&str> = game.history.iter().map(|(_, san)| san.as_str()).collect();
        assert_eq!(sans, ["f3", "e5", "g4", "Qh4#"]);
        assert!(game.is_game_over());
        game.make_move(ChessMove::quiet(0, 1)).unwrap_err();
    }

    #[test]
    fn castling_generates_both_legs_and_updates_rights() {
        let mut game =
            ChessMatch::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").expect("FEN parses");
        let mut castles: Vec<String> = game
            .legal_moves()
            .into_iter()
            .filter(|chess_move| chess_move.castling)
            .map(ChessMove::uci)
            .collect();
        castles.sort();
        assert_eq!(castles, ["e1c1", "e1g1"]);
        let castle = move_from_uci(&game, "e1g1").expect("kingside castles");
        let san = game.make_move(castle).expect("castle plays");
        assert_eq!(san, "O-O");
        assert_eq!(game.position.to_fen().split_whitespace().nth(2), Some("kq"));
    }

    #[test]
    fn en_passant_capture_removes_pawn() {
        let mut game =
            ChessMatch::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").expect("FEN parses");
        let captures: Vec<ChessMove> = game
            .legal_moves()
            .into_iter()
            .filter(|chess_move| chess_move.en_passant)
            .collect();
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].uci(), "e5d6");
        let san = game.make_move(captures[0]).expect("capture plays");
        assert_eq!(san, "exd6");
        assert_eq!(
            game.position.board[square_from_name("d5").expect("d5")],
            None
        );
    }

    #[test]
    fn promotion_generates_four_moves_with_san() {
        let mut game = ChessMatch::from_fen("4k3/1P6/8/8/8/8/8/4K3 w - - 0 1").expect("FEN parses");
        let mut promos: Vec<String> = game
            .legal_moves()
            .into_iter()
            .filter(|chess_move| chess_move.promotion.is_some())
            .map(ChessMove::uci)
            .collect();
        promos.sort();
        assert_eq!(promos, ["b7b8b", "b7b8n", "b7b8q", "b7b8r"]);
        let queen = move_from_uci(&game, "b7b8q").expect("queen promotion resolves");
        let san = game.make_move(queen).expect("promotion plays");
        assert_eq!(san, "b8=Q+");
    }

    #[test]
    fn stalemate_and_draws_detect() {
        let stalemate = ChessMatch::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").expect("FEN parses");
        assert_eq!(stalemate.status, MatchStatus::Stalemate);
        let bare = ChessMatch::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").expect("FEN parses");
        assert_eq!(bare.status, MatchStatus::Draw);
        assert_eq!(bare.draw_reason, "insufficient material");
        let fifty =
            ChessMatch::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 100 1").expect("FEN parses");
        assert_eq!(fifty.status, MatchStatus::Draw);
        assert_eq!(fifty.draw_reason, "fifty-move rule");
        let mut repetition = ChessMatch::new();
        play_uci(
            &mut repetition,
            &[
                "g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1", "f6g8",
            ],
        );
        assert_eq!(repetition.status, MatchStatus::Draw);
        assert_eq!(repetition.draw_reason, "threefold repetition");
    }

    #[test]
    fn illegal_moves_and_uci_errors() {
        let mut game = ChessMatch::new();
        game.make_move(ChessMove::quiet(12, 28)).unwrap_err();
        move_from_uci(&game, "e2e5").unwrap_err();
        move_from_uci(&game, "bogus").unwrap_err();
        move_from_uci(&game, "e7e8x").unwrap_err();
        game.undo_move().unwrap_err();
    }

    #[test]
    fn best_move_returns_legal_moves() {
        let game = ChessMatch::new();
        for difficulty in [ChessDifficulty::Harmless, ChessDifficulty::Easy] {
            let mut rng = 0x1234_5678_9ABC_DEF0;
            let (choice, _, _) = best_move(&game, difficulty, &mut rng).expect("opening has moves");
            assert!(game.legal_moves().contains(&choice));
        }
        assert!(ChessDifficulty::from_value("grandmaster").is_none());
    }

    #[test]
    fn click_selects_and_moves_with_ai_reply() {
        let mut game = ChessGame::default();
        assert!(game.click(6, 4));
        assert_eq!(game.selected, Some((6, 4)));
        assert!(game.click(4, 4));
        assert_eq!(game.selected, None);
        assert_eq!(game.game.history.len(), 2);
        assert_eq!(game.game.position.side, Color::White);
    }

    #[test]
    fn two_player_mode_has_no_ai_reply() {
        let mut game = ChessGame::default();
        assert!(game.select_mode("two"));
        assert!(game.click(6, 4));
        assert!(game.click(4, 4));
        assert_eq!(game.game.history.len(), 1);
        assert_eq!(game.game.position.side, Color::Black);
    }

    #[test]
    fn promotion_picker_is_respected() {
        let mut game = ChessGame {
            game: ChessMatch::from_fen("4k3/1P6/8/8/8/8/8/4K3 w - - 0 1").expect("FEN parses"),
            ..ChessGame::default()
        };
        assert!(game.select_mode("two"));
        assert!(game.select_promotion("N"));
        assert!(game.click(1, 1));
        assert!(game.click(0, 1));
        assert_eq!(
            game.game.history.last().map(|(_, san)| san.as_str()),
            Some("b8=N")
        );
    }

    #[test]
    fn controls_and_selects() {
        let mut game = ChessGame::default();
        assert!(game.select_mode("two"));
        assert!(game.click(6, 4));
        assert!(game.click(4, 4));
        assert!(game.undo());
        assert!(game.game.history.is_empty());
        assert!(game.flip());
        assert!(game.flipped);
        assert!(game.hint());
        assert!(game.hint_move.is_some());
        assert!(game.ai_move());
        assert_eq!(game.game.history.len(), 1);
        assert!(game.reset());
        assert!(game.game.history.is_empty());
        assert!(!game.select_mode("correspondence"));
        assert!(!game.select_difficulty("grandmaster"));
        assert!(!game.select_promotion("K"));
    }

    #[test]
    fn scene_validates_and_paints_all_squares() {
        let game = ChessGame::default();
        let scene = game.scene(1);
        Scene2DState::new(scene.clone()).expect("chess scene is valid");
        assert_eq!(scene.grid.as_ref().map(|grid| grid.rows), Some(8));
        let cells = scene
            .nodes
            .iter()
            .filter(|node| node.id.starts_with("chess-cell-"));
        assert_eq!(cells.count(), 64);
        let pieces = scene
            .nodes
            .iter()
            .filter(|node| node.id.starts_with("chess-piece-"));
        assert_eq!(pieces.count(), 32);
        // Center-aligned glyphs anchor on the square center.
        let grid = scene.grid.as_ref().expect("chess scene has a grid");
        for node in scene
            .nodes
            .iter()
            .filter(|node| node.id.starts_with("chess-piece-"))
        {
            let Scene2DNodeKind::Text { origin, align, .. } = &node.kind else {
                panic!("{} should be a text node", node.id);
            };
            assert_eq!(*align, Scene2DTextAlign::Center);
            let row: usize = node.id["chess-piece-".len()..]
                .split('-')
                .next()
                .expect("piece id carries a row")
                .parse()
                .expect("piece row parses");
            let col: usize = node.id["chess-piece-".len()..]
                .split('-')
                .nth(1)
                .expect("piece id carries a column")
                .parse()
                .expect("piece column parses");
            let expected_x =
                grid.x + col as f32 * (grid.cell_width + grid.gap) + grid.cell_width * 0.5;
            let expected_y =
                grid.y + row as f32 * (grid.cell_height + grid.gap) + grid.cell_height * 0.5
                    - 30.0 * 0.625;
            assert!(
                (origin.x - expected_x).abs() < 0.01,
                "{} anchors at x {expected_x}, got {}",
                node.id,
                origin.x
            );
            assert!(
                (origin.y - expected_y).abs() < 0.01,
                "{} anchors at y {expected_y}, got {}",
                node.id,
                origin.y
            );
        }
    }
}
