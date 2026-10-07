//! Othello (Reversi) rules engine, AI opponent, and native board UI.
//!
//! Rules (pass-as-a-move, game-over detection), the phase-aware evaluation
//! (material, mobility, potential mobility, corners, corner danger, frontier
//! discs), and the alpha-beta search with ordered moves and exact endgame
//! solving follow thegustafson/ai-othello
//! (<https://github.com/thegustafson/ai-othello>, MIT); the bitboard backend
//! is replaced with a 64-square mailbox board shared with the Python demo so
//! both showcases behave identically with zero extra dependencies.

// Rust guideline compliant 2026-02-21

use super::{
    brush, circle_node, color_alpha, new_scene, next_random, rounded_node, semantic, stroke,
};
use gpui_ui_kit::scene2d::{
    Scene2DColor, Scene2DGrid, Scene2DInput, Scene2DInputConfig, Scene2DKeyPhase, Scene2DScene,
    Scene2DSemanticRole, ScenePoint, SceneRect,
};
use std::collections::HashSet;

/// AI strength, matching the Python demo ladder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum OthelloDifficulty {
    Beginner,
    #[default]
    Easy,
    Medium,
    Hard,
}

impl OthelloDifficulty {
    /// Search depth per strength.
    const fn depth(self) -> u32 {
        match self {
            Self::Beginner => 1,
            Self::Easy => 2,
            Self::Medium => 3,
            Self::Hard => 4,
        }
    }

    /// Nodes (positions) the search may visit before settling.
    const fn node_cap(self) -> u64 {
        match self {
            Self::Beginner => 12_000,
            Self::Easy => 48_000,
            Self::Medium => 160_000,
            Self::Hard => 480_000,
        }
    }

    /// Empties at or below which the endgame solves exactly.
    const fn exact_empties(self) -> usize {
        match self {
            Self::Hard => 10,
            Self::Beginner | Self::Easy | Self::Medium => 0,
        }
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Beginner => "Beginner (depth 1)",
            Self::Easy => "Easy (depth 2)",
            Self::Medium => "Medium (depth 3)",
            Self::Hard => "Hard (depth 4 + endgame)",
        }
    }

    pub(super) const fn value(self) -> &'static str {
        match self {
            Self::Beginner => "beginner",
            Self::Easy => "easy",
            Self::Medium => "medium",
            Self::Hard => "hard",
        }
    }

    fn from_value(value: &str) -> Option<Self> {
        match value {
            "beginner" => Some(Self::Beginner),
            "easy" => Some(Self::Easy),
            "medium" => Some(Self::Medium),
            "hard" => Some(Self::Hard),
            _ => None,
        }
    }
}

/// Human opponent: a second player or the built-in AI as White.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum OthelloMode {
    TwoPlayer,
    #[default]
    VsAi,
}

impl OthelloMode {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::TwoPlayer => "Two players",
            Self::VsAi => "You (Black) vs AI (White)",
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

/// Sentinel larger than any reachable evaluation.
const INF_SCORE: i32 = 40_000;
/// Terminal wins score above every heuristic evaluation.
const TERMINAL_SCORE: i32 = 30_000;
/// Corners in LERF order (a1, h1, a8, h8).
const CORNERS: [usize; 4] = [0, 7, 56, 63];
/// Squares adjoining corners (`b1 g1 a2 b2 g2 h2 a7 b7 g7 h7 b8 g8`); weak early.
const CORNER_DANGER: [usize; 12] = [1, 6, 8, 9, 14, 15, 48, 49, 54, 55, 57, 62];
const RAYS: [(i32, i32); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

// Board felt (`#2E7D4F`); low-chroma enough to survive the light-theme remap.
const FELT: Scene2DColor = Scene2DColor::rgb(0.18, 0.49, 0.31);
const GOLD: Scene2DColor = Scene2DColor::rgb(1.0, 0.827, 0.435);
const PAPER: Scene2DColor = Scene2DColor::rgb(0.957, 0.945, 0.91);
const GRID_LINE: Scene2DColor = Scene2DColor::rgb(0.212, 0.286, 0.388);
const BLACK_DISC: Scene2DColor = Scene2DColor::rgb(0.086, 0.086, 0.086);
const WHITE_DISC: Scene2DColor = Scene2DColor::rgb(0.949, 0.949, 0.949);

/// File (0 = a) of a LERF square.
const fn file_of(square: usize) -> usize {
    square % 8
}

/// Rank (0 = first rank) of a LERF square.
const fn rank_of(square: usize) -> usize {
    square / 8
}

/// LERF square from file and rank.
fn square_at(file: usize, rank: usize) -> usize {
    rank * 8 + file
}

/// Board name (`d3`) of a square.
fn square_name(square: usize) -> String {
    format!(
        "{}{}",
        (b'a' + file_of(square) as u8) as char,
        rank_of(square) + 1
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Side {
    Black,
    White,
}

impl Side {
    const fn other(self) -> Self {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Black => "Black",
            Self::White => "White",
        }
    }
}

/// Restorable state captured by placement or pass.
#[derive(Clone, Debug)]
struct OthelloUndo {
    square: Option<usize>,
    flipped: Vec<usize>,
    side: Side,
}

/// Mailbox board with ai-othello move semantics; Black moves first.
#[derive(Clone, Debug, PartialEq, Eq)]
struct OthelloPosition {
    board: [Option<Side>; 64],
    side: Side,
}

impl OthelloPosition {
    /// Standard start: `d4`/`e5` White, `e4`/`d5` Black, Black to move.
    fn start() -> Self {
        let mut board: [Option<Side>; 64] = [None; 64];
        board[28] = Some(Side::Black);
        board[35] = Some(Side::Black);
        board[27] = Some(Side::White);
        board[36] = Some(Side::White);
        Self {
            board,
            side: Side::Black,
        }
    }

    /// Disc count for one side.
    fn discs(&self, side: Side) -> usize {
        self.board
            .iter()
            .filter(|disc| **disc == Some(side))
            .count()
    }

    /// Empty square count.
    fn empties(&self) -> usize {
        self.board.iter().filter(|disc| disc.is_none()).count()
    }

    /// Discs that placing `side` on `square` would flip, possibly none.
    fn bracket(&self, square: usize, side: Side) -> Vec<usize> {
        if self.board[square].is_some() {
            return Vec::new();
        }
        let enemy = side.other();
        let file = file_of(square) as i32;
        let rank = rank_of(square) as i32;
        let mut flips = Vec::new();
        for (delta_file, delta_rank) in RAYS {
            let mut run = Vec::new();
            let mut other = file + delta_file;
            let mut target = rank + delta_rank;
            while (0..8).contains(&other)
                && (0..8).contains(&target)
                && self.board[square_at(other as usize, target as usize)] == Some(enemy)
            {
                run.push(square_at(other as usize, target as usize));
                other += delta_file;
                target += delta_rank;
            }
            if !run.is_empty()
                && (0..8).contains(&other)
                && (0..8).contains(&target)
                && self.board[square_at(other as usize, target as usize)] == Some(side)
            {
                flips.extend(run);
            }
        }
        flips
    }

    /// Legal placements for one side in ascending square order.
    fn placements_for(&self, side: Side) -> Vec<usize> {
        (0..64)
            .filter(|square| {
                self.board[*square].is_none() && !self.bracket(*square, side).is_empty()
            })
            .collect()
    }

    /// Legal placements for the side to move.
    fn placements(&self) -> Vec<usize> {
        self.placements_for(self.side)
    }

    /// Places a disc, returning the flipped count.
    fn apply(&mut self, square: usize) -> Result<OthelloUndo, String> {
        let flips = self.bracket(square, self.side);
        if flips.is_empty() {
            return Err(format!("placing at {} flips no discs", square_name(square)));
        }
        let undo = OthelloUndo {
            square: Some(square),
            flipped: flips.clone(),
            side: self.side,
        };
        self.board[square] = Some(self.side);
        for target in flips {
            self.board[target] = Some(self.side);
        }
        self.side = self.side.other();
        Ok(undo)
    }

    /// Passes; only legal with no placement available.
    fn apply_pass(&mut self) -> Result<OthelloUndo, String> {
        if !self.placements().is_empty() {
            return Err("pass is only legal when no placement is legal".to_owned());
        }
        let undo = OthelloUndo {
            square: None,
            flipped: Vec::new(),
            side: self.side,
        };
        self.side = self.side.other();
        Ok(undo)
    }

    /// Reverses a placement or pass.
    fn undo(&mut self, undo: &OthelloUndo) {
        self.side = undo.side;
        if let Some(square) = undo.square {
            self.board[square] = None;
            let enemy = undo.side.other();
            for target in &undo.flipped {
                self.board[*target] = Some(enemy);
            }
        }
    }

    /// Game result: ongoing, draw, or the winning side with disc counts.
    fn result(&self) -> OthelloResult {
        let black = self.discs(Side::Black);
        let white = self.discs(Side::White);
        if !self.placements_for(Side::Black).is_empty()
            || !self.placements_for(Side::White).is_empty()
        {
            return OthelloResult::Ongoing;
        }
        if black == white {
            OthelloResult::Draw
        } else {
            OthelloResult::Win {
                winner: if black > white {
                    Side::Black
                } else {
                    Side::White
                },
                black,
                white,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OthelloResult {
    Ongoing,
    Draw,
    Win {
        winner: Side,
        black: usize,
        white: usize,
    },
}

/// Blends opening into endgame weights over the 60 post-start plies.
fn interpolate(opening: i32, endgame: i32, phase: i32) -> i32 {
    let phase = phase.clamp(0, 60);
    (opening * (60 - phase) + endgame * phase) / 60
}

/// Empty squares adjacent to `side` discs (potential mobility).
fn empty_neighbours(board: &[Option<Side>; 64], side: Side) -> usize {
    let mut seen = HashSet::new();
    for (square, disc) in board.iter().enumerate() {
        if *disc != Some(side) {
            continue;
        }
        let file = file_of(square) as i32;
        let rank = rank_of(square) as i32;
        for (delta_file, delta_rank) in RAYS {
            let other = file + delta_file;
            let target = rank + delta_rank;
            if (0..8).contains(&other) && (0..8).contains(&target) {
                let neighbour = square_at(other as usize, target as usize);
                if board[neighbour].is_none() {
                    seen.insert(neighbour);
                }
            }
        }
    }
    seen.len()
}

/// Own discs adjacent to an empty square.
fn frontier(board: &[Option<Side>; 64], side: Side) -> usize {
    let mut count = 0usize;
    for (square, disc) in board.iter().enumerate() {
        if *disc != Some(side) {
            continue;
        }
        let file = file_of(square) as i32;
        let rank = rank_of(square) as i32;
        for (delta_file, delta_rank) in RAYS {
            let other = file + delta_file;
            let target = rank + delta_rank;
            if (0..8).contains(&other)
                && (0..8).contains(&target)
                && board[square_at(other as usize, target as usize)].is_none()
            {
                count += 1;
                break;
            }
        }
    }
    count
}

/// Phase-profile evaluation from the side-to-move perspective.
fn evaluate(position: &OthelloPosition) -> i32 {
    let side = position.side;
    let enemy = side.other();
    let phase = 64 - position.empties() as i32 - 4;
    let material = position.discs(side) as i32 - position.discs(enemy) as i32;
    let mobility =
        position.placements_for(side).len() as i32 - position.placements_for(enemy).len() as i32;
    let potential = empty_neighbours(&position.board, enemy) as i32
        - empty_neighbours(&position.board, side) as i32;
    let corners = CORNERS
        .iter()
        .filter(|square| position.board[**square] == Some(side))
        .count() as i32
        - CORNERS
            .iter()
            .filter(|square| position.board[**square] == Some(enemy))
            .count() as i32;
    let danger = CORNER_DANGER
        .iter()
        .filter(|square| position.board[**square] == Some(side))
        .count() as i32
        - CORNER_DANGER
            .iter()
            .filter(|square| position.board[**square] == Some(enemy))
            .count() as i32;
    let frontier_discs =
        frontier(&position.board, side) as i32 - frontier(&position.board, enemy) as i32;
    material * interpolate(1, 14, phase)
        + mobility * interpolate(24, 5, phase)
        + potential * interpolate(8, 2, phase)
        + corners * 160
        + danger * interpolate(-45, -5, phase)
        + frontier_discs * interpolate(-12, -3, phase)
}

/// Corners first; the rest keep ascending order for determinism.
fn ordered(placements: Vec<usize>) -> Vec<usize> {
    let (corners, rest): (Vec<usize>, Vec<usize>) = placements
        .into_iter()
        .partition(|square| CORNERS.contains(square));
    corners.into_iter().chain(rest).collect()
}

/// Negamax search; `Err` escapes the whole tree once the node cap trips.
/// Passes preserve depth and the exact endgame ignores the depth cutoff.
fn search(
    position: &mut OthelloPosition,
    depth: u32,
    mut alpha: i32,
    beta: i32,
    exact_empties: usize,
    budget: &mut u64,
    cap: u64,
) -> Result<i32, ()> {
    *budget += 1;
    if *budget > cap {
        return Err(());
    }
    match position.result() {
        OthelloResult::Draw => return Ok(0),
        OthelloResult::Win {
            winner,
            black,
            white,
        } => {
            let margin = black.abs_diff(white) as i32;
            return Ok(if winner == position.side {
                TERMINAL_SCORE + margin
            } else {
                -TERMINAL_SCORE - margin
            });
        }
        OthelloResult::Ongoing => {}
    }
    let exact = exact_empties > 0 && position.empties() <= exact_empties;
    if depth == 0 && !exact {
        return Ok(evaluate(position));
    }
    let placements = ordered(position.placements());
    if placements.is_empty() {
        let undo = position.apply_pass().map_err(|_| ())?;
        let score =
            search(position, depth, -beta, -alpha, exact_empties, budget, cap).map(|child| -child);
        position.undo(&undo);
        return score;
    }
    let mut best = -INF_SCORE;
    for square in placements {
        let undo = position.apply(square).map_err(|_| ())?;
        let score = search(
            position,
            depth - 1,
            -beta,
            -alpha,
            exact_empties,
            budget,
            cap,
        )
        .map(|child| -child);
        position.undo(&undo);
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

/// Best placement for the side to move; `None` means pass.
fn best_move(
    game: &OthelloMatch,
    difficulty: OthelloDifficulty,
    rng_state: &mut u64,
) -> (Option<usize>, i32, u64) {
    let placements = ordered(game.position.placements());
    if placements.is_empty() {
        return (None, 0, 0);
    }
    if difficulty == OthelloDifficulty::Beginner && next_random(rng_state) % 100 < 35 {
        let pick = next_random(rng_state) as usize % placements.len();
        return (Some(placements[pick]), 0, 0);
    }
    let mut position = game.position.clone();
    let mut budget = 0;
    let (mut best_square, mut best_score) = (placements[0], -INF_SCORE);
    let mut alpha = -INF_SCORE;
    for square in placements {
        let Ok(undo) = position.apply(square) else {
            continue;
        };
        let score = search(
            &mut position,
            difficulty.depth() - 1,
            -INF_SCORE,
            -alpha,
            difficulty.exact_empties(),
            &mut budget,
            difficulty.node_cap(),
        )
        .map(|child| -child);
        position.undo(&undo);
        let Ok(score) = score else { break };
        if score > best_score {
            best_score = score;
            best_square = square;
        }
        if score > alpha {
            alpha = score;
        }
    }
    (Some(best_square), best_score, budget)
}

/// Stateful game: history, undo, passes, and results.
#[derive(Clone, Debug)]
struct OthelloMatch {
    position: OthelloPosition,
    history: Vec<(Option<usize>, Side)>,
    undo_stack: Vec<OthelloUndo>,
    finished: bool,
    winner: Option<Side>,
}

impl OthelloMatch {
    /// Fresh game with the standard start.
    fn new() -> Self {
        let mut game = Self {
            position: OthelloPosition::start(),
            history: Vec::new(),
            undo_stack: Vec::new(),
            finished: false,
            winner: None,
        };
        game.refresh_status();
        game
    }

    /// Disc counts as (black, white).
    fn counts(&self) -> (usize, usize) {
        (
            self.position.discs(Side::Black),
            self.position.discs(Side::White),
        )
    }

    /// Places a disc, returning the flipped count.
    fn place(&mut self, square: usize) -> Result<usize, String> {
        if self.finished {
            return Err("game is over".to_owned());
        }
        let undo = self.position.apply(square)?;
        let flips = undo.flipped.len();
        self.undo_stack.push(undo);
        self.history
            .push((Some(square), self.position.side.other()));
        self.refresh_status();
        Ok(flips)
    }

    /// Passes, returning the passing side.
    fn pass(&mut self) -> Result<Side, String> {
        if self.finished {
            return Err("game is over".to_owned());
        }
        let undo = self.position.apply_pass()?;
        self.undo_stack.push(undo);
        self.history.push((None, self.position.side.other()));
        self.refresh_status();
        Ok(self.position.side.other())
    }

    /// Takes back the last placement or pass.
    fn undo_move(&mut self) -> Result<(Option<usize>, Side), String> {
        let entry = self
            .history
            .pop()
            .ok_or_else(|| "nothing to undo".to_owned())?;
        let undo = self
            .undo_stack
            .pop()
            .expect("history and undo stack stay paired");
        self.position.undo(&undo);
        self.refresh_status();
        Ok(entry)
    }

    fn refresh_status(&mut self) {
        match self.position.result() {
            OthelloResult::Ongoing => {
                self.finished = false;
                self.winner = None;
            }
            OthelloResult::Draw => {
                self.finished = true;
                self.winner = None;
            }
            OthelloResult::Win { winner, .. } => {
                self.finished = true;
                self.winner = Some(winner);
            }
        }
    }

    /// Trailing placements for the details panel.
    fn moves_text(&self) -> String {
        if self.history.is_empty() {
            return "No moves yet — Black opens in the center.".to_owned();
        }
        let skip = self.history.len().saturating_sub(12);
        let mut parts = Vec::new();
        let mut number = 0;
        for (square, side) in self.history.iter().skip(skip) {
            let label = square.map_or_else(|| "pass".to_owned(), square_name);
            if *side == Side::Black {
                number += 1;
                parts.push(format!("{number}. {label}"));
            } else {
                parts.push(label);
            }
        }
        if skip > 0 {
            format!("… {}", parts.join(" "))
        } else {
            parts.join(" ")
        }
    }
}

/// Native Othello board: placements, AI replies, and retained scene painting.
#[derive(Clone, Debug)]
pub(super) struct OthelloGame {
    game: OthelloMatch,
    cursor: (usize, usize),
    pub(super) mode: OthelloMode,
    pub(super) difficulty: OthelloDifficulty,
    last_move: Option<usize>,
    hint_move: Option<usize>,
    notice: String,
    rng_state: u64,
}

impl Default for OthelloGame {
    fn default() -> Self {
        Self {
            game: OthelloMatch::new(),
            cursor: (5, 3),
            mode: OthelloMode::default(),
            difficulty: OthelloDifficulty::default(),
            last_move: None,
            hint_move: None,
            notice: "You play Black; the AI replies as White.".to_owned(),
            // SplitMix64 stream; distinct from the chess stream.
            rng_state: 0x243F_6A88_85A3_08D3,
        }
    }
}

impl OthelloGame {
    /// Display cell to square.
    fn display_to_square(row: usize, col: usize) -> usize {
        square_at(col, 7 - row)
    }

    /// Places on this cell, then settles passes and the AI reply.
    fn click(&mut self, row: usize, col: usize) -> bool {
        if row >= 8 || col >= 8 {
            return false;
        }
        if self.game.finished {
            "Game over — press New game to play again.".clone_into(&mut self.notice);
            return true;
        }
        if self.mode == OthelloMode::VsAi && self.game.position.side != Side::Black {
            "The AI plays White — switch to Two players to move both sides."
                .clone_into(&mut self.notice);
            return true;
        }
        let square = Self::display_to_square(row, col);
        if !self.game.position.placements().contains(&square) {
            self.notice = if self.game.position.board[square].is_some() {
                format!(
                    "{} is occupied — pick a dotted square.",
                    square_name(square)
                )
            } else {
                format!(
                    "{} brackets nothing — pick a dotted square.",
                    square_name(square)
                )
            };
            return true;
        }
        let Ok(flips) = self.game.place(square) else {
            "Illegal move — pick a dotted square.".clone_into(&mut self.notice);
            return true;
        };
        self.last_move = Some(square);
        self.hint_move = None;
        let mut notes = vec![format!(
            "Placed {} (flipping {flips}).",
            square_name(square)
        )];
        self.settle_passes(&mut notes);
        if self.game.finished {
            self.notice = format!("{} {}", notes.join(" "), self.game_over_text());
            return true;
        }
        if self.mode == OthelloMode::VsAi {
            self.ai_replies(&mut notes);
            if self.game.finished {
                self.notice = format!("{} {}", notes.join(" "), self.game_over_text());
                return true;
            }
        }
        self.notice = format!("{} {}", notes.join(" "), self.side_to_move_text());
        true
    }

    /// Records forced passes until someone can move or the game ends.
    fn settle_passes(&mut self, notes: &mut Vec<String>) {
        while !self.game.finished && self.game.position.placements().is_empty() {
            if let Ok(side) = self.game.pass() {
                notes.push(format!("{} has no move and passes.", side.name()));
            } else {
                break;
            }
        }
    }

    /// Plays AI moves while White is to move.
    fn ai_replies(&mut self, notes: &mut Vec<String>) {
        while !self.game.finished && self.game.position.side == Side::White {
            let (choice, _, _) = best_move(&self.game, self.difficulty, &mut self.rng_state);
            let Some(square) = choice else {
                self.settle_passes(notes);
                continue;
            };
            let Ok(flips) = self.game.place(square) else {
                break;
            };
            self.last_move = Some(square);
            notes.push(format!(
                "AI played {} (flipping {flips}).",
                square_name(square)
            ));
            self.settle_passes(notes);
        }
    }

    fn side_to_move_text(&self) -> String {
        format!("{} to move.", self.game.position.side.name())
    }

    fn game_over_text(&self) -> String {
        let (black, white) = self.game.counts();
        match self.game.winner {
            Some(Side::Black) => format!("Black wins {black}–{white}."),
            Some(Side::White) => format!("White wins {white}–{black}."),
            None => format!("Draw {black}–{white}."),
        }
    }

    /// Starts over with a fresh board.
    pub(super) fn reset(&mut self) -> bool {
        *self = Self {
            rng_state: self.rng_state,
            ..Self::default()
        };
        "New game — Black to move.".clone_into(&mut self.notice);
        true
    }

    /// Takes back one round: to Black's turn against the AI, one ply otherwise.
    pub(super) fn undo(&mut self) -> bool {
        if self.game.history.is_empty() {
            "Nothing to undo.".clone_into(&mut self.notice);
            return true;
        }
        let mut undone = 0;
        while !self.game.history.is_empty() {
            if self.game.undo_move().is_err() {
                break;
            }
            undone += 1;
            if self.mode != OthelloMode::VsAi || self.game.position.side == Side::Black {
                break;
            }
            if undone > 66 {
                break;
            }
        }
        self.hint_move = None;
        self.last_move = self.game.history.last().and_then(|(square, _)| *square);
        let noun = if undone == 1 { "move" } else { "moves" };
        self.notice = format!("Undid {undone} {noun}. {}", self.side_to_move_text());
        true
    }

    /// Passes when no placement is legal, then settles the AI reply.
    pub(super) fn pass_move(&mut self) -> bool {
        if self.game.finished {
            "Game over — press New game to play again.".clone_into(&mut self.notice);
            return true;
        }
        let available = self.game.position.placements().len();
        if available > 0 {
            self.notice = format!("{available} legal move(s) available — pass is not allowed.");
            return true;
        }
        let Ok(side) = self.game.pass() else {
            return false;
        };
        self.hint_move = None;
        let mut notes = vec![format!("{} passes.", side.name())];
        self.settle_passes(&mut notes);
        if self.game.finished {
            self.notice = format!("{} {}", notes.join(" "), self.game_over_text());
            return true;
        }
        if self.mode == OthelloMode::VsAi {
            self.ai_replies(&mut notes);
            if self.game.finished {
                self.notice = format!("{} {}", notes.join(" "), self.game_over_text());
                return true;
            }
        }
        self.notice = format!("{} {}", notes.join(" "), self.side_to_move_text());
        true
    }

    /// Highlights the AI suggestion without playing it.
    pub(super) fn hint(&mut self) -> bool {
        if self.game.finished {
            "Game over — press New game to play again.".clone_into(&mut self.notice);
            return true;
        }
        let (choice, _, _) = best_move(&self.game, self.difficulty, &mut self.rng_state);
        let Some(square) = choice else {
            "Hint: pass — no placement is legal.".clone_into(&mut self.notice);
            return true;
        };
        let flips = self
            .game
            .position
            .bracket(square, self.game.position.side)
            .len();
        self.hint_move = Some(square);
        self.notice = format!("Hint: {} (flipping {flips}).", square_name(square));
        true
    }

    /// Plays the AI move for the side to move.
    pub(super) fn ai_move(&mut self) -> bool {
        if self.game.finished {
            "Game over — press New game to play again.".clone_into(&mut self.notice);
            return true;
        }
        let (choice, _, _) = best_move(&self.game, self.difficulty, &mut self.rng_state);
        let mut notes = Vec::new();
        match choice {
            None => self.settle_passes(&mut notes),
            Some(square) => {
                let Ok(flips) = self.game.place(square) else {
                    return false;
                };
                self.last_move = Some(square);
                self.hint_move = None;
                notes.push(format!(
                    "AI played {} (flipping {flips}).",
                    square_name(square)
                ));
                self.settle_passes(&mut notes);
            }
        }
        if self.game.finished {
            self.notice = format!("{} {}", notes.join(" "), self.game_over_text());
        } else {
            self.notice = format!("{} {}", notes.join(" "), self.side_to_move_text());
        }
        true
    }

    /// Switches opponents; the AI catches up at once when it inherits White.
    pub(super) fn select_mode(&mut self, value: &str) -> bool {
        let Some(mode) = OthelloMode::from_value(value) else {
            return false;
        };
        self.mode = mode;
        self.hint_move = None;
        if mode == OthelloMode::VsAi && !self.game.finished {
            let mut notes = Vec::new();
            self.settle_passes(&mut notes);
            self.ai_replies(&mut notes);
            if !notes.is_empty() {
                if self.game.finished {
                    self.notice = format!("{} {}", notes.join(" "), self.game_over_text());
                } else {
                    self.notice = format!("{} {}", notes.join(" "), self.side_to_move_text());
                }
                return true;
            }
            "You play Black; the AI replies as White.".clone_into(&mut self.notice);
            return true;
        }
        "Two players — move both sides.".clone_into(&mut self.notice);
        true
    }

    /// Switches search depth.
    pub(super) fn select_difficulty(&mut self, value: &str) -> bool {
        let Some(difficulty) = OthelloDifficulty::from_value(value) else {
            return false;
        };
        self.difficulty = difficulty;
        self.notice = format!("AI strength: {}.", difficulty.label());
        true
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
                    "p" => self.pass_move(),
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

    /// `Black 12 · White 10` summary for the details panel.
    pub(super) fn details(&self) -> String {
        let (black, white) = self.game.counts();
        format!("Black {black} · White {white}")
    }

    /// Trailing placements for the details panel.
    pub(super) fn moves_text(&self) -> String {
        self.game.moves_text()
    }

    /// Paints felt, discs, and hints as a retained scene.
    pub(super) fn scene(&self, revision: u64) -> Scene2DScene {
        let pad = 18.0;
        let cell = 48.0;
        let gap = 2.0;
        let width = pad * 2.0 + 8.0 * cell + 7.0 * gap;
        let mut scene = new_scene(
            width,
            width,
            "Othello board",
            "Outflank the rival discs. Black moves first.",
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
            "othello-well",
            None,
            SceneRect::new(5.0, 5.0, width - 10.0, width - 10.0),
            18.0,
            super::color(0.075, 0.12, 0.18),
            None,
            None,
        ));
        let legal: HashSet<usize> = if self.game.finished {
            HashSet::new()
        } else {
            self.game.position.placements().into_iter().collect()
        };
        for row in 0..8 {
            for col in 0..8 {
                self.paint_cell(&mut scene, row, col, pad, cell, gap, &legal);
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

    /// Paints one square with its disc and hint overlays.
    fn paint_cell(
        &self,
        scene: &mut Scene2DScene,
        row: usize,
        col: usize,
        pad: f32,
        cell: f32,
        gap: f32,
        legal: &HashSet<usize>,
    ) {
        let square = Self::display_to_square(row, col);
        let disc = self.game.position.board[square];
        let name = square_name(square);
        let value = match disc {
            Some(Side::Black) => "black disc".to_owned(),
            Some(Side::White) => "white disc".to_owned(),
            None if legal.contains(&square) => "legal move".to_owned(),
            None => "empty".to_owned(),
        };
        let x = pad + col as f32 * (cell + gap);
        let y = pad + row as f32 * (cell + gap);
        let rect = SceneRect::new(x, y, cell, cell);
        let center = ScenePoint::new(x + cell * 0.5, y + cell * 0.5);
        let (edge_color, edge_width) = if (row, col) == self.cursor {
            (PAPER, 2.0)
        } else {
            (GRID_LINE, 0.7)
        };
        scene.nodes.push(rounded_node(
            &format!("othello-cell-{row}-{col}"),
            Some(format!("othello-cell-{row}-{col}")),
            rect,
            5.0,
            FELT,
            Some(stroke(edge_width, edge_color)),
            Some(semantic(
                Scene2DSemanticRole::GridCell,
                format!("Square {name}"),
                value,
                Some(square) == self.last_move,
            )),
        ));
        if Some(square) == self.last_move || Some(square) == self.hint_move {
            scene.nodes.push(circle_node(
                &format!("othello-ring-{row}-{col}"),
                None,
                center,
                cell * 0.5 - 3.0,
                None,
                Some(stroke(3.0, GOLD)),
                None,
            ));
        }
        if legal.contains(&square) && disc.is_none() {
            scene.nodes.push(circle_node(
                &format!("othello-target-{row}-{col}"),
                None,
                center,
                7.0,
                Some(brush(color_alpha(0.957, 0.945, 0.91, 0.75))),
                None,
                None,
            ));
        }
        if let Some(side) = disc {
            scene.nodes.push(circle_node(
                &format!("othello-disc-{row}-{col}"),
                None,
                center,
                cell * 0.5 - 6.0,
                Some(brush(if side == Side::Black {
                    BLACK_DISC
                } else {
                    WHITE_DISC
                })),
                Some(stroke(
                    1.4,
                    if side == Side::Black {
                        PAPER
                    } else {
                        GRID_LINE
                    },
                )),
                None,
            ));
            // Flip transitions animate disc color between revisions.
            let index = scene.nodes.len() - 1;
            scene.nodes[index].transition = Some(super::transition(160));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_ui_kit::scene2d::Scene2DState;

    /// White to move with no placement while Black can play a1.
    fn must_pass_match() -> OthelloMatch {
        let mut board: [Option<Side>; 64] = [Some(Side::White); 64];
        board[0] = None;
        board[63] = Some(Side::Black);
        let mut game = OthelloMatch::new();
        game.position = OthelloPosition {
            board,
            side: Side::White,
        };
        game.refresh_status();
        game
    }

    #[test]
    fn start_has_four_placements() {
        let game = OthelloMatch::new();
        let mut names: Vec<String> = game
            .position
            .placements()
            .into_iter()
            .map(square_name)
            .collect();
        names.sort();
        assert_eq!(names, ["c4", "d3", "e6", "f5"]);
        assert_eq!(game.counts(), (2, 2));
        assert_eq!(game.position.side, Side::Black);
    }

    #[test]
    fn d3_flips_one_disc() {
        let mut game = OthelloMatch::new();
        let flips = game.place(19).expect("d3 is legal");
        assert_eq!(flips, 1);
        assert_eq!(game.counts(), (4, 1));
        assert_eq!(game.position.side, Side::White);
    }

    #[test]
    fn illegal_placement_and_early_pass_error() {
        let mut game = OthelloMatch::new();
        game.place(0).unwrap_err();
        game.pass().unwrap_err();
        game.undo_move().unwrap_err();
    }

    #[test]
    fn must_pass_position_passes_to_black() {
        let mut game = must_pass_match();
        assert!(!game.finished);
        assert!(game.position.placements().is_empty());
        assert_eq!(game.position.placements_for(Side::Black), vec![0]);
        let side = game.pass().expect("pass is legal");
        assert_eq!(side, Side::White);
        assert_eq!(game.position.side, Side::Black);
        game.undo_move().expect("pass undoes");
        assert_eq!(game.position.side, Side::White);
    }

    #[test]
    fn full_selfplay_game_finishes() {
        let mut game = OthelloMatch::new();
        let mut rng = 0x1234_5678_9ABC_DEF0;
        let mut plies = 0;
        while !game.finished && plies < 70 {
            let (choice, _, _) = best_move(&game, OthelloDifficulty::Easy, &mut rng);
            match choice {
                None => {
                    game.pass().expect("pass plays");
                }
                Some(square) => {
                    game.place(square).expect("AI move plays");
                }
            }
            plies += 1;
        }
        assert!(game.finished);
        let (black, white) = game.counts();
        match game.winner {
            Some(Side::Black) => assert!(black > white),
            Some(Side::White) => assert!(white > black),
            None => assert_eq!(black, white),
        }
    }

    #[test]
    fn best_move_is_legal_and_start_evaluates_symmetric() {
        let game = OthelloMatch::new();
        let mut rng = 0x0F1E_2D3C_4B5A_6978;
        let (choice, _, _) = best_move(&game, OthelloDifficulty::Easy, &mut rng);
        assert!(choice.is_some_and(|square| game.position.placements().contains(&square)));
        assert_eq!(evaluate(&OthelloPosition::start()), 0);
        assert!(OthelloDifficulty::from_value("grandmaster").is_none());
    }

    #[test]
    fn click_places_and_ai_replies() {
        let mut game = OthelloGame::default();
        assert!(game.click(5, 3));
        assert!(game.game.history.len() >= 2);
        assert_eq!(game.game.position.side, Side::Black);
    }

    #[test]
    fn illegal_clicks_keep_state() {
        let mut game = OthelloGame::default();
        assert!(game.click(0, 0));
        assert!(game.game.history.is_empty());
        assert!(game.click(3, 3));
        assert!(game.game.history.is_empty());
    }

    #[test]
    fn pass_control_flow() {
        let mut game = OthelloGame::default();
        assert!(game.pass_move());
        assert!(game.game.history.is_empty());
        game.game = must_pass_match();
        assert!(game.select_mode("two"));
        assert!(game.pass_move());
        assert_eq!(game.game.position.side, Side::Black);
    }

    #[test]
    fn controls_and_selects() {
        let mut game = OthelloGame::default();
        assert!(game.select_mode("two"));
        assert!(game.click(5, 3));
        assert_eq!(game.game.history.len(), 1);
        assert!(game.undo());
        assert!(game.game.history.is_empty());
        assert!(game.click(5, 3));
        assert!(game.hint());
        assert!(game.hint_move.is_some());
        assert!(game.ai_move());
        assert_eq!(game.game.history.len(), 2);
        assert!(game.reset());
        assert!(game.game.history.is_empty());
        assert!(!game.select_mode("correspondence"));
        assert!(!game.select_difficulty("grandmaster"));
    }

    #[test]
    fn scene_validates_and_paints_all_squares() {
        let game = OthelloGame::default();
        let scene = game.scene(1);
        Scene2DState::new(scene.clone()).expect("othello scene is valid");
        assert_eq!(scene.grid.as_ref().map(|grid| grid.rows), Some(8));
        let cells = scene
            .nodes
            .iter()
            .filter(|node| node.id.starts_with("othello-cell-"));
        assert_eq!(cells.count(), 64);
        let discs = scene
            .nodes
            .iter()
            .filter(|node| node.id.starts_with("othello-disc-"));
        assert_eq!(discs.count(), 4);
        let targets = scene
            .nodes
            .iter()
            .filter(|node| node.id.starts_with("othello-target-"));
        assert_eq!(targets.count(), 4);
    }
}
