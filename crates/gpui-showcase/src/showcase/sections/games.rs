//! Four small, playable native games built on the retained Scene2D surface.

// Rust guideline compliant 2026-02-21

use super::prelude::*;
use gpui::{AnyElement, Subscription, WeakEntity};
use gpui_ui_kit::scene2d::{
    GameSurface, Scene2DBrush, Scene2DColor, Scene2DEasing, Scene2DGrid, Scene2DInput,
    Scene2DInputConfig, Scene2DKeyPhase, Scene2DNode, Scene2DNodeKind, Scene2DPathCommand,
    Scene2DScene, Scene2DSemantic, Scene2DSemanticRole, Scene2DState, Scene2DStroke,
    Scene2DTextAlign, Scene2DTransform, Scene2DTransition, ScenePoint, SceneRect,
};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::time::Duration;
use web_time::Instant;

const ZIP_ROWS: usize = 5;
const ZIP_COLS: usize = 5;
const ZIP_START: (usize, usize) = (0, 0);
const ZIP_CHECKPOINTS: [(usize, usize); 5] = [(0, 0), (0, 4), (2, 2), (4, 0), (4, 4)];
const ZIP_SOLUTION: [(usize, usize); ZIP_ROWS * ZIP_COLS] = [
    (0, 0),
    (0, 1),
    (0, 2),
    (0, 3),
    (0, 4),
    (1, 4),
    (1, 3),
    (1, 2),
    (1, 1),
    (1, 0),
    (2, 0),
    (2, 1),
    (2, 2),
    (2, 3),
    (2, 4),
    (3, 4),
    (3, 3),
    (3, 2),
    (3, 1),
    (3, 0),
    (4, 0),
    (4, 1),
    (4, 2),
    (4, 3),
    (4, 4),
];

const QUEENS_SIZE: usize = 8;
const QUEENS_SOLUTION: [usize; QUEENS_SIZE] = [0, 4, 7, 5, 2, 6, 1, 3];
// Eight fixed, connected regions. Each solution queen belongs to the region
// matching its row, so the built-in board satisfies all four rules.
const QUEENS_REGIONS: [[usize; QUEENS_SIZE]; QUEENS_SIZE] = [
    [0, 0, 0, 1, 1, 1, 2, 2],
    [0, 0, 4, 1, 1, 1, 2, 2],
    [0, 4, 4, 1, 1, 3, 2, 2],
    [0, 0, 4, 4, 3, 3, 3, 2],
    [6, 4, 4, 5, 3, 3, 3, 2],
    [6, 6, 4, 5, 3, 5, 5, 5],
    [6, 6, 6, 5, 5, 5, 7, 7],
    [6, 6, 7, 7, 7, 7, 7, 7],
];
const QUEENS_COLORS: [Scene2DColor; QUEENS_SIZE] = [
    Scene2DColor::rgb(0.89, 0.43, 0.42),
    Scene2DColor::rgb(0.90, 0.59, 0.34),
    Scene2DColor::rgb(0.83, 0.69, 0.30),
    Scene2DColor::rgb(0.34, 0.68, 0.49),
    Scene2DColor::rgb(0.29, 0.63, 0.76),
    Scene2DColor::rgb(0.53, 0.47, 0.77),
    Scene2DColor::rgb(0.75, 0.43, 0.63),
    Scene2DColor::rgb(0.54, 0.62, 0.70),
];

const SUDOKU_PUZZLE: [&str; 9] = [
    "530070000",
    "600195000",
    "098000060",
    "800060003",
    "400803001",
    "700020006",
    "060000280",
    "000419005",
    "000080079",
];
const SUDOKU_SOLUTION: [[u8; 9]; 9] = [
    [5, 3, 4, 6, 7, 8, 9, 1, 2],
    [6, 7, 2, 1, 9, 5, 3, 4, 8],
    [1, 9, 8, 3, 4, 2, 5, 6, 7],
    [8, 5, 9, 7, 6, 1, 4, 2, 3],
    [4, 2, 6, 8, 5, 3, 7, 9, 1],
    [7, 1, 3, 9, 2, 4, 8, 5, 6],
    [9, 6, 1, 5, 3, 7, 2, 8, 4],
    [2, 8, 7, 4, 1, 9, 6, 3, 5],
    [3, 4, 5, 2, 8, 6, 1, 7, 9],
];

const TETRIS_ROWS: usize = 20;
const TETRIS_COLS: usize = 10;
const TICK_INTERVAL: Duration = Duration::from_millis(50);
const HISTORY_LIMIT: usize = 256;
const TETRIS_PIECE_ORDER: [PieceKind; 7] = [
    PieceKind::T,
    PieceKind::L,
    PieceKind::O,
    PieceKind::S,
    PieceKind::I,
    PieceKind::J,
    PieceKind::Z,
];

fn push_history<T>(history: &mut VecDeque<T>, value: T) {
    while history.len() >= HISTORY_LIMIT {
        history.pop_front();
    }
    history.push_back(value);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameKind {
    Zip,
    Queens,
    Sudoku,
    Tetris,
}

impl GameKind {
    const ALL: [Self; 4] = [Self::Zip, Self::Queens, Self::Sudoku, Self::Tetris];

    const fn label(self) -> &'static str {
        match self {
            Self::Zip => "Zip",
            Self::Queens => "Queens",
            Self::Sudoku => "Sudoku",
            Self::Tetris => "Tetris",
        }
    }

    const fn help(self) -> &'static str {
        match self {
            Self::Zip => {
                "Start at 1 and drag a continuous path through each numbered checkpoint. Cover every tile once. Arrow keys move the path; Backspace undoes a step."
            }
            Self::Queens => {
                "Place one crown in every row, column, and colored region. Crowns cannot touch diagonally. Select a square and press Enter; M marks a square."
            }
            Self::Sudoku => {
                "Fill every row, column, and 3×3 box with the digits 1–9. Select a square, then use the number keys or keypad. Clues are locked."
            }
            Self::Tetris => {
                "Use the arrows to move, Up or X to rotate, Space to drop, and P to pause. Hold the surface controls to move or soft drop; two fingers work at once."
            }
        }
    }

    fn from_name(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "zip" => Some(Self::Zip),
            "queens" => Some(Self::Queens),
            "sudoku" => Some(Self::Sudoku),
            "tetris" => Some(Self::Tetris),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameAction {
    New,
    Undo,
    Hint,
    Check,
    Pause,
    Drop,
    ToggleHelp,
}

/// Persistent game state and the native scenes used by the Games showcase.
pub(crate) struct GamesShowcase {
    active: bool,
    active_game: GameKind,
    help_open: bool,
    status: String,
    zip: ZipGame,
    queens: QueensGame,
    sudoku: SudokuGame,
    tetris: TetrisGame,
    zip_surface: Scene2DState,
    queens_surface: Scene2DState,
    sudoku_surface: Scene2DState,
    tetris_surface: Scene2DState,
    tetris_preview: Scene2DState,
    tetris_controls: Scene2DState,
    light_palette: bool,
    window_active: bool,
    activation_subscription: Option<Subscription>,
    tick_generation: u64,
    tick_active: bool,
    last_tick: Instant,
}

impl GamesShowcase {
    /// Creates fresh deterministic boards and their validated native scenes.
    pub(crate) fn new(_cx: &mut Context<Self>) -> Self {
        let active_game = super::super::initial_game_name()
            .and_then(|value| GameKind::from_name(&value))
            .unwrap_or(GameKind::Zip);
        let zip = ZipGame::default();
        let queens = QueensGame::default();
        let sudoku = SudokuGame::default();
        let tetris = TetrisGame::default();
        Self {
            zip_surface: Scene2DState::new(zip.scene(1)).expect("built-in Zip scene is valid"),
            queens_surface: Scene2DState::new(queens.scene(1))
                .expect("built-in Queens scene is valid"),
            sudoku_surface: Scene2DState::new(sudoku.scene(1))
                .expect("built-in Sudoku scene is valid"),
            tetris_surface: Scene2DState::new(tetris.scene(1))
                .expect("built-in Tetris scene is valid"),
            tetris_preview: Scene2DState::new(tetris.preview_scene(1))
                .expect("built-in Tetris preview is valid"),
            tetris_controls: Scene2DState::new(tetris.controls_scene(1))
                .expect("built-in Tetris controls are valid"),
            active: false,
            active_game,
            help_open: false,
            status: "Ready when you are.".to_owned(),
            zip,
            queens,
            sudoku,
            tetris,
            light_palette: false,
            window_active: false,
            activation_subscription: None,
            tick_generation: 0,
            tick_active: false,
            last_tick: Instant::now(),
        }
    }

    /// Pauses native timers and clears held controls when the showcase hides.
    pub(crate) fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        if self.active == active {
            return;
        }
        self.active = active;
        if !active {
            self.tetris.release_all();
        }
        self.set_scene_suspension();
        self.sync_ticker(cx);
        cx.notify();
    }

    fn set_window_active(&mut self, active: bool, cx: &mut Context<Self>) {
        if self.window_active == active {
            return;
        }
        self.window_active = active;
        if !active {
            self.tetris.release_all();
        }
        self.tetris.reset_clock();
        self.set_scene_suspension();
        self.sync_ticker(cx);
        cx.notify();
    }

    fn set_scene_suspension(&self) {
        let suspended = !(self.active && self.window_active);
        for scene in [
            &self.zip_surface,
            &self.queens_surface,
            &self.sudoku_surface,
            &self.tetris_surface,
            &self.tetris_preview,
            &self.tetris_controls,
        ] {
            scene.set_suspended(suspended);
        }
    }

    fn surface(&self, game: GameKind, handle: WeakEntity<Self>) -> GameSurface {
        let (id, aria_label, state) = match game {
            GameKind::Zip => ("native-zip", "Zip board", self.zip_surface.clone()),
            GameKind::Queens => ("native-queens", "Queens board", self.queens_surface.clone()),
            GameKind::Sudoku => ("native-sudoku", "Sudoku board", self.sudoku_surface.clone()),
            GameKind::Tetris => (
                "native-tetris",
                "Tetris playfield",
                self.tetris_surface.clone(),
            ),
        };
        GameSurface::from_state(id, state.clone())
            .aria_label(aria_label)
            .on_input(move |event, _window, app| {
                let _ = handle.update(app, |this, cx| this.handle_input(game, event, cx));
            })
    }

    fn control_surface(&self, handle: WeakEntity<Self>) -> GameSurface {
        GameSurface::from_state("native-tetris-touch-controls", self.tetris_controls.clone())
            .aria_label("Tetris touch controls. Hold left, right, or soft drop; tap rotate.")
            .on_input(move |event, _window, app| {
                let _ = handle.update(app, |this, cx| {
                    this.handle_input(GameKind::Tetris, event, cx)
                });
            })
    }

    fn set_game(&mut self, game: GameKind, cx: &mut Context<Self>) {
        if self.active_game == game {
            return;
        }
        self.tetris.release_all();
        self.active_game = game;
        self.help_open = false;
        self.status = "Ready when you are.".to_owned();
        self.sync_ticker(cx);
        cx.notify();
    }

    fn sync_ticker(&mut self, cx: &mut Context<Self>) {
        let should_tick = self.active
            && self.window_active
            && self.active_game == GameKind::Tetris
            && self.tetris.needs_ticks();
        if should_tick == self.tick_active {
            return;
        }
        self.tick_active = should_tick;
        self.tick_generation = self.tick_generation.wrapping_add(1);
        if !should_tick {
            self.tetris.reset_clock();
            return;
        }
        let generation = self.tick_generation;
        self.last_tick = Instant::now();
        cx.spawn(async move |this: WeakEntity<Self>, cx| {
            loop {
                cx.background_executor().timer(TICK_INTERVAL).await;
                let result = this.update(cx, |this, cx| {
                    if !this.tick_active || this.tick_generation != generation {
                        return false;
                    }
                    let now = Instant::now();
                    let elapsed = now
                        .duration_since(this.last_tick)
                        .min(Duration::from_millis(250));
                    this.last_tick = now;
                    this.advance_tetris(elapsed, cx);
                    this.tetris.needs_ticks()
                });
                if !matches!(result, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    fn advance_tetris(&mut self, elapsed: Duration, cx: &mut Context<Self>) {
        let scene_changed = self.tetris.advance(elapsed);
        if scene_changed {
            self.refresh_tetris_scenes(cx);
            self.status = self.tetris.status();
        }
        self.sync_ticker(cx);
        cx.notify();
    }

    fn handle_input(&mut self, game: GameKind, event: Scene2DInput, cx: &mut Context<Self>) {
        if game == GameKind::Tetris
            && let Scene2DInput::Lifecycle { reason, .. } = &event
        {
            match reason {
                gpui_ui_kit::scene2d::Scene2DLifecycleReason::Suspended => {
                    self.set_window_active(false, cx);
                }
                gpui_ui_kit::scene2d::Scene2DLifecycleReason::Resumed => {
                    self.set_window_active(true, cx);
                }
                _ => {}
            }
        }
        let changed = match game {
            GameKind::Zip => self.zip.handle(event),
            GameKind::Queens => self.queens.handle(event),
            GameKind::Sudoku => self.sudoku.handle(event),
            GameKind::Tetris => self.tetris.handle(event),
        };
        if !changed {
            self.sync_ticker(cx);
            return;
        }
        match game {
            GameKind::Zip => {
                self.status = self.zip.status();
                self.replace_surface(
                    &self.zip_surface.clone(),
                    self.zip.scene(self.zip_revision()),
                    cx,
                );
            }
            GameKind::Queens => {
                self.status = self.queens.status();
                self.replace_surface(
                    &self.queens_surface.clone(),
                    self.queens.scene(self.queens_revision()),
                    cx,
                );
            }
            GameKind::Sudoku => {
                self.status = self.sudoku.status();
                self.replace_surface(
                    &self.sudoku_surface.clone(),
                    self.sudoku.scene(self.sudoku_revision()),
                    cx,
                );
            }
            GameKind::Tetris => {
                self.status = self.tetris.status();
                self.refresh_tetris_scenes(cx);
            }
        }
        self.sync_ticker(cx);
        cx.notify();
    }

    fn replace_surface(&self, state: &Scene2DState, scene: Scene2DScene, cx: &mut Context<Self>) {
        if state.replace_scene(self.palette_scene(scene)).is_ok() {
            cx.notify();
        }
    }

    fn palette_scene(&self, scene: Scene2DScene) -> Scene2DScene {
        apply_board_palette(scene, self.light_palette)
    }

    fn sync_palette(&mut self, light: bool, cx: &mut Context<Self>) {
        if self.light_palette == light {
            return;
        }
        self.light_palette = light;
        let _ = self.zip_surface.replace_scene(
            self.palette_scene(
                self.zip
                    .scene(self.zip_surface.scene().revision.saturating_add(1)),
            ),
        );
        let _ = self.queens_surface.replace_scene(
            self.palette_scene(
                self.queens
                    .scene(self.queens_surface.scene().revision.saturating_add(1)),
            ),
        );
        let _ = self.sudoku_surface.replace_scene(
            self.palette_scene(
                self.sudoku
                    .scene(self.sudoku_surface.scene().revision.saturating_add(1)),
            ),
        );
        self.refresh_tetris_scenes(cx);
        cx.notify();
    }

    fn zip_revision(&self) -> u64 {
        self.zip_surface.scene().revision.saturating_add(1)
    }

    fn queens_revision(&self) -> u64 {
        self.queens_surface.scene().revision.saturating_add(1)
    }

    fn sudoku_revision(&self) -> u64 {
        self.sudoku_surface.scene().revision.saturating_add(1)
    }

    fn refresh_tetris_scenes(&self, cx: &mut Context<Self>) {
        let _ = self.tetris_surface.replace_scene(
            self.palette_scene(
                self.tetris
                    .scene(self.tetris_surface.scene().revision.saturating_add(1)),
            ),
        );
        let _ = self.tetris_preview.replace_scene(
            self.palette_scene(
                self.tetris
                    .preview_scene(self.tetris_preview.scene().revision.saturating_add(1)),
            ),
        );
        let _ = self.tetris_controls.replace_scene(
            self.palette_scene(
                self.tetris
                    .controls_scene(self.tetris_controls.scene().revision.saturating_add(1)),
            ),
        );
        cx.notify();
    }

    fn apply_action(&mut self, game: GameKind, action: GameAction, cx: &mut Context<Self>) {
        if action == GameAction::ToggleHelp {
            self.help_open = !self.help_open;
            cx.notify();
            return;
        }
        let changed = match (game, action) {
            (GameKind::Zip, GameAction::New) => self.zip.reset(),
            (GameKind::Zip, GameAction::Undo) => self.zip.undo(),
            (GameKind::Zip, GameAction::Hint) => self.zip.hint(),
            (GameKind::Queens, GameAction::New) => self.queens.reset(),
            (GameKind::Queens, GameAction::Undo) => self.queens.undo(),
            (GameKind::Queens, GameAction::Hint) => self.queens.hint(),
            (GameKind::Sudoku, GameAction::New) => self.sudoku.reset(),
            (GameKind::Sudoku, GameAction::Undo) => self.sudoku.undo(),
            (GameKind::Sudoku, GameAction::Hint) => self.sudoku.hint(),
            (GameKind::Sudoku, GameAction::Check) => self.sudoku.check(),
            (GameKind::Tetris, GameAction::New) => self.tetris.new_game(),
            (GameKind::Tetris, GameAction::Pause) => self.tetris.toggle_pause(),
            (GameKind::Tetris, GameAction::Drop) => self.tetris.hard_drop(),
            _ => false,
        };
        if changed {
            match game {
                GameKind::Zip => {
                    self.status = self.zip.status();
                    let scene = self.zip.scene(self.zip_revision());
                    self.replace_surface(&self.zip_surface.clone(), scene, cx);
                }
                GameKind::Queens => {
                    self.status = self.queens.status();
                    let scene = self.queens.scene(self.queens_revision());
                    self.replace_surface(&self.queens_surface.clone(), scene, cx);
                }
                GameKind::Sudoku => {
                    self.status = self.sudoku.status();
                    let scene = self.sudoku.scene(self.sudoku_revision());
                    self.replace_surface(&self.sudoku_surface.clone(), scene, cx);
                }
                GameKind::Tetris => {
                    self.status = self.tetris.status();
                    self.refresh_tetris_scenes(cx);
                }
            }
        }
        self.sync_ticker(cx);
        cx.notify();
    }

    fn action_button(
        &self,
        game: GameKind,
        action: GameAction,
        id: &'static str,
        label: &'static str,
        handle: WeakEntity<Self>,
    ) -> Button {
        Button::new(id, label)
            .variant(ButtonVariant::Secondary)
            .on_click(move |_window, app| {
                let _ = handle.update(app, |this, cx| this.apply_action(game, action, cx));
            })
    }

    fn game_tab(&self, game: GameKind, handle: WeakEntity<Self>) -> Button {
        let id = match game {
            GameKind::Zip => "native-game-tab-zip",
            GameKind::Queens => "native-game-tab-queens",
            GameKind::Sudoku => "native-game-tab-sudoku",
            GameKind::Tetris => "native-game-tab-tetris",
        };
        Button::new(id, game.label())
            .variant(if self.active_game == game {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Secondary
            })
            .selected(self.active_game == game)
            .on_click(move |_window, app| {
                let _ = handle.update(app, |this, cx| this.set_game(game, cx));
            })
    }

    fn surface_element(
        &self,
        game: GameKind,
        viewport_width: f32,
        viewport_height: f32,
        handle: WeakEntity<Self>,
    ) -> impl IntoElement {
        let (id, base_height, base_max_width, scene_ratio): (&'static str, f32, f32, f32) =
            match game {
                GameKind::Zip => ("zip-native-surface", 410.0, 420.0, 1.0),
                GameKind::Queens => ("queens-native-surface", 440.0, 460.0, 1.0),
                GameKind::Sudoku => ("sudoku-native-surface", 430.0, 460.0, 1.0),
                GameKind::Tetris => ("tetris-native-surface", 560.0, 330.0, 262.0 / 502.0),
            };
        let compact_landscape = viewport_width >= 600.0 && viewport_height < 600.0;
        let phone_portrait = viewport_width < 600.0;
        let height = if compact_landscape {
            match game {
                GameKind::Tetris => (viewport_height - 150.0).clamp(190.0, 270.0),
                GameKind::Sudoku => (viewport_height - 170.0).clamp(210.0, 300.0),
                GameKind::Zip | GameKind::Queens => (viewport_height - 150.0).clamp(220.0, 310.0),
            }
        } else if phone_portrait {
            match game {
                GameKind::Tetris => (viewport_height * 0.43).clamp(300.0, 390.0),
                GameKind::Sudoku => (viewport_height * 0.38).clamp(280.0, 340.0),
                GameKind::Zip | GameKind::Queens => {
                    (viewport_height * 0.52).clamp(320.0, base_height)
                }
            }
        } else {
            base_height.min((viewport_height * 0.62).clamp(290.0, 560.0))
        };
        let max_width = base_max_width
            .min((viewport_width - 32.0).max(180.0))
            .min(height * scene_ratio);
        div()
            .id(id)
            // A narrower aspect-ratio surface must not rely on flex stretch to
            // infer its width. In a portrait VStack, `w_full + max_w` can
            // report a zero-width Scene2D canvas even though height is set.
            .w(px(max_width))
            .h(px(height))
            .max_w(px(max_width))
            .overflow_hidden()
            .child(self.surface(game, handle))
    }

    fn sudoku_keypad(&self, handle: WeakEntity<Self>) -> impl IntoElement {
        VStack::new()
            .spacing(StackSpacing::Xs)
            .child(
                HStack::new()
                    .spacing(StackSpacing::Xs)
                    .child(self.sudoku_digit_button(1, handle.clone()))
                    .child(self.sudoku_digit_button(2, handle.clone()))
                    .child(self.sudoku_digit_button(3, handle.clone())),
            )
            .child(
                HStack::new()
                    .spacing(StackSpacing::Xs)
                    .child(self.sudoku_digit_button(4, handle.clone()))
                    .child(self.sudoku_digit_button(5, handle.clone()))
                    .child(self.sudoku_digit_button(6, handle.clone())),
            )
            .child(
                HStack::new()
                    .spacing(StackSpacing::Xs)
                    .child(self.sudoku_digit_button(7, handle.clone()))
                    .child(self.sudoku_digit_button(8, handle.clone()))
                    .child(self.sudoku_digit_button(9, handle.clone())),
            )
            .child(
                Button::new("native-sudoku-erase", "Erase")
                    .size(ButtonSize::Lg)
                    .variant(ButtonVariant::Ghost)
                    .on_click(move |_window, app| {
                        let _ = handle.update(app, |this, cx| {
                            if this.sudoku.erase() {
                                this.status = this.sudoku.status();
                                let scene = this.sudoku.scene(this.sudoku_revision());
                                this.replace_surface(&this.sudoku_surface.clone(), scene, cx);
                                cx.notify();
                            }
                        });
                    }),
            )
    }

    fn sudoku_digit_button(&self, digit: u8, handle: WeakEntity<Self>) -> Button {
        Button::new(format!("native-sudoku-digit-{digit}"), digit.to_string())
            .size(ButtonSize::Lg)
            .variant(ButtonVariant::Secondary)
            .aria_label(format!("Enter {digit}"))
            .on_click(move |_window, app| {
                let _ = handle.update(app, |this, cx| {
                    if this.sudoku.enter(digit) {
                        this.status = this.sudoku.status();
                        let scene = this.sudoku.scene(this.sudoku_revision());
                        this.replace_surface(&this.sudoku_surface.clone(), scene, cx);
                        cx.notify();
                    }
                });
            })
    }

    fn sudoku_selected_cell_preview(&self) -> impl IntoElement {
        let (label, value) = self
            .sudoku
            .selected
            .map(|(row, col)| {
                let value = self.sudoku.values[row][col];
                (
                    format!("Selected cell · Row {}, column {}", row + 1, col + 1),
                    if value == 0 {
                        "·".to_owned()
                    } else {
                        value.to_string()
                    },
                )
            })
            .unwrap_or_else(|| ("Select a cell on the board".to_owned(), "·".to_owned()));
        let preview_fill = if self.light_palette {
            rgba(0xe4edf4ff)
        } else {
            rgba(0x1a2a38ff)
        };
        HStack::new()
            .spacing(StackSpacing::Md)
            .align(StackAlign::Center)
            .child(Text::new(label).weight(TextWeight::Medium))
            .child(
                div()
                    .id("native-sudoku-selected-value")
                    .w(px(72.0))
                    .h(px(72.0))
                    .rounded_lg()
                    .bg(preview_fill)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child(value),
            )
    }

    fn render_body(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        handle: WeakEntity<Self>,
    ) -> impl IntoElement {
        let game = self.active_game;
        let board = self.surface_element(game, viewport_width, viewport_height, handle.clone());
        let help_button = self
            .action_button(
                game,
                GameAction::ToggleHelp,
                "native-game-help",
                "How to play",
                handle.clone(),
            )
            .expanded(self.help_open);
        let mut actions = HStack::new()
            .spacing(StackSpacing::Sm)
            .wrap(true)
            .child(help_button);
        match game {
            GameKind::Zip | GameKind::Queens | GameKind::Sudoku => {
                actions = actions
                    .child(self.action_button(
                        game,
                        GameAction::New,
                        "native-game-new",
                        "Reset",
                        handle.clone(),
                    ))
                    .child(self.action_button(
                        game,
                        GameAction::Undo,
                        "native-game-undo",
                        "Undo",
                        handle.clone(),
                    ))
                    .child(self.action_button(
                        game,
                        GameAction::Hint,
                        "native-game-hint",
                        "Hint",
                        handle.clone(),
                    ));
                if game == GameKind::Sudoku {
                    actions = actions.child(self.action_button(
                        game,
                        GameAction::Check,
                        "native-sudoku-check",
                        "Check",
                        handle.clone(),
                    ));
                }
            }
            GameKind::Tetris => {
                actions = actions
                    .child(self.action_button(
                        game,
                        GameAction::New,
                        "native-tetris-new",
                        "New game",
                        handle.clone(),
                    ))
                    .child(self.action_button(
                        game,
                        GameAction::Pause,
                        "native-tetris-pause",
                        if self.tetris.running && !self.tetris.paused {
                            "Pause"
                        } else {
                            "Start / resume"
                        },
                        handle.clone(),
                    ))
                    .child(self.action_button(
                        game,
                        GameAction::Drop,
                        "native-tetris-drop",
                        "Drop",
                        handle.clone(),
                    ));
            }
        }

        let compact_landscape = viewport_width >= 600.0 && viewport_height < 600.0;
        let phone_portrait = viewport_width < 600.0;
        let details: AnyElement = match game {
            GameKind::Zip => VStack::new()
                .spacing(StackSpacing::Md)
                .child(Text::new(self.zip.status()).weight(TextWeight::Medium))
                .child(Text::new(format!(
                    "Path: {}/{} cells · next checkpoint {}",
                    self.zip.path.len(),
                    ZIP_ROWS * ZIP_COLS,
                    self.zip.next_checkpoint()
                )))
                .into_any_element(),
            GameKind::Queens => VStack::new()
                .spacing(StackSpacing::Md)
                .child(Text::new(self.queens.status()).weight(TextWeight::Medium))
                .child(Text::new(format!(
                    "{} of {} crowns",
                    self.queens.count(),
                    QUEENS_SIZE
                )))
                .into_any_element(),
            GameKind::Sudoku => VStack::new()
                .spacing(StackSpacing::Sm)
                .child(Text::new(self.sudoku.status()).weight(TextWeight::Medium))
                .child(self.sudoku_selected_cell_preview().into_any_element())
                .child(Text::new(self.sudoku.preview()))
                .child(self.sudoku_keypad(handle.clone()).into_any_element())
                .into_any_element(),
            GameKind::Tetris => {
                let preview_size = if compact_landscape {
                    72.0
                } else if phone_portrait {
                    86.0
                } else {
                    112.0
                };
                HStack::new()
                    .spacing(StackSpacing::Sm)
                    .align(StackAlign::Center)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Xs)
                            .child(Text::new(self.tetris.status()).weight(TextWeight::Medium))
                            .child(Text::new(format!(
                                "Score {} · Lines {} · Level {}",
                                self.tetris.score,
                                self.tetris.lines,
                                self.tetris.level()
                            )))
                            .child(Text::new(format!("Next: {}", self.tetris.next.label()))),
                    )
                    .child(
                        div()
                            .w(px(preview_size))
                            .h(px(preview_size))
                            .flex_none()
                            .child(
                                GameSurface::from_state(
                                    "native-tetris-preview",
                                    self.tetris_preview.clone(),
                                )
                                .aria_label("Next Tetris piece preview"),
                            ),
                    )
                    .into_any_element()
            }
        };
        let body = if game == GameKind::Tetris {
            // Touch holds are distinct captured Scene2D contacts. Keep them on
            // the surface hit objects rather than converting them to button taps.
            let controls_width = if compact_landscape {
                (viewport_width * 0.36).clamp(220.0, 300.0)
            } else {
                (viewport_width - 32.0).clamp(240.0, 360.0)
            };
            let controls_height = if compact_landscape { 76.0 } else { 88.0 };
            let controls = div()
                .w(px(controls_width))
                .h(px(controls_height))
                .max_w(px(controls_width))
                .flex_none()
                .child(self.control_surface(handle.clone()));
            if compact_landscape {
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .align(StackAlign::Center)
                    .child(board)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .max_w(px((viewport_width - 190.0).max(220.0)))
                            .child(details)
                            .child(controls)
                            .child(actions),
                    )
                    .into_any_element()
            } else if phone_portrait {
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(details)
                    .child(board)
                    .child(controls)
                    .child(actions)
                    .into_any_element()
            } else {
                HStack::new()
                    .spacing(StackSpacing::Lg)
                    .wrap(true)
                    .scrollable_on_mobile(false)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(board)
                            .child(controls),
                    )
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Md)
                            .max_w(px(420.0))
                            .child(actions)
                            .child(details),
                    )
                    .into_any_element()
            }
        } else {
            let details_panel = if game == GameKind::Sudoku {
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .max_w(px(if viewport_width < 600.0 { 420.0 } else { 500.0 }))
                    .child(details)
                    .child(actions)
            } else {
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .max_w(px(if viewport_width < 600.0 { 420.0 } else { 500.0 }))
                    .child(actions)
                    .child(details)
            };
            HStack::new()
                .spacing(StackSpacing::Lg)
                .wrap(true)
                .scrollable_on_mobile(false)
                .child(board)
                .child(details_panel)
                .into_any_element()
        };
        if self.help_open {
            VStack::new()
                .spacing(StackSpacing::Sm)
                .child(body)
                .child(Text::new(game.help()).size(TextSize::Sm).muted(true))
                .into_any_element()
        } else {
            body
        }
    }
}

impl Render for GamesShowcase {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let background = cx.theme().background;
        self.sync_palette(
            color_luminance(Scene2DColor::rgba(
                background.r,
                background.g,
                background.b,
                background.a,
            )) > 0.52,
            cx,
        );
        self.set_window_active(window.is_window_active(), cx);
        if self.activation_subscription.is_none() {
            self.activation_subscription =
                Some(cx.observe_window_activation(window, |games, window, cx| {
                    games.set_window_active(window.is_window_active(), cx)
                }));
        }
        let handle = cx.entity().downgrade();
        let tabs = HStack::new()
            .spacing(StackSpacing::Sm)
            .wrap(true)
            .scrollable_on_mobile(false);
        let tabs = GameKind::ALL.into_iter().fold(tabs, |tabs, game| {
            tabs.child(self.game_tab(game, handle.clone()))
        });
        VStack::new()
            .spacing(StackSpacing::Md)
            .scrollable_on_mobile(false)
            .child(tabs)
            .child(self.render_body(
                window.viewport_size().width.as_f32(),
                window.viewport_size().height.as_f32(),
                handle,
            ))
    }
}

#[derive(Clone, Debug, Default)]
struct ZipGame {
    path: Vec<(usize, usize)>,
    undo: VecDeque<Vec<(usize, usize)>>,
    won: bool,
}

impl ZipGame {
    fn next_checkpoint(&self) -> usize {
        ZIP_CHECKPOINTS
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, point)| !self.path.contains(point))
            .map(|(index, _)| index + 1)
            .unwrap_or(ZIP_CHECKPOINTS.len())
    }

    fn record(&mut self) {
        push_history(&mut self.undo, self.path.clone());
        self.won = false;
    }

    fn step(&mut self, cell: (usize, usize)) -> bool {
        if cell.0 >= ZIP_ROWS || cell.1 >= ZIP_COLS || self.won {
            return false;
        }
        if self.path.last() == Some(&cell) {
            return false;
        }
        if self.path.len() >= 2 && self.path[self.path.len() - 2] == cell {
            self.record();
            self.path.pop();
            return true;
        }
        if self.path.contains(&cell) {
            return false;
        }
        if self.path.is_empty() {
            if cell != ZIP_START {
                return false;
            }
        } else {
            let current = self.path[self.path.len() - 1];
            if current.0.abs_diff(cell.0) + current.1.abs_diff(cell.1) != 1 {
                return false;
            }
            let checkpoint_index = ZIP_CHECKPOINTS.iter().position(|point| *point == cell);
            if let Some(index) = checkpoint_index {
                if index + 1 != self.next_checkpoint() {
                    return false;
                }
            }
        }
        self.record();
        self.path.push(cell);
        self.won =
            self.path.len() == ZIP_ROWS * ZIP_COLS && self.path.last() == ZIP_CHECKPOINTS.last();
        true
    }

    fn undo(&mut self) -> bool {
        if let Some(path) = self.undo.pop_back() {
            self.path = path;
            self.won = self.path.len() == ZIP_ROWS * ZIP_COLS;
            true
        } else {
            false
        }
    }

    fn reset(&mut self) -> bool {
        let changed = !self.path.is_empty() || self.won;
        *self = Self::default();
        changed
    }

    fn hint(&mut self) -> bool {
        let index = self.path.len();
        if let Some(cell) = ZIP_SOLUTION.get(index).copied() {
            self.step(cell)
        } else {
            false
        }
    }

    fn handle(&mut self, event: Scene2DInput) -> bool {
        match event {
            Scene2DInput::Pointer {
                phase,
                cell: Some(cell),
                ..
            } if matches!(
                phase,
                gpui_ui_kit::scene2d::Scene2DPointerPhase::Down
                    | gpui_ui_kit::scene2d::Scene2DPointerPhase::Move
                    | gpui_ui_kit::scene2d::Scene2DPointerPhase::Up
            ) =>
            {
                self.step((cell.row as usize, cell.column as usize))
            }
            Scene2DInput::Key {
                phase: Scene2DKeyPhase::Down,
                key,
                repeat,
                ..
            } if !repeat => {
                if key == "Backspace" {
                    return self.undo();
                }
                if self.path.is_empty() {
                    return self.step(ZIP_START);
                }
                let (row, col) = self.path[self.path.len() - 1];
                let target = match key.as_str() {
                    "ArrowLeft" if col > 0 => Some((row, col - 1)),
                    "ArrowRight" if col + 1 < ZIP_COLS => Some((row, col + 1)),
                    "ArrowUp" if row > 0 => Some((row - 1, col)),
                    "ArrowDown" if row + 1 < ZIP_ROWS => Some((row + 1, col)),
                    _ => None,
                };
                target.is_some_and(|cell| self.step(cell))
            }
            Scene2DInput::Activate {
                cell: Some(cell), ..
            } => self.step((cell.row as usize, cell.column as usize)),
            _ => false,
        }
    }

    fn status(&self) -> String {
        if self.won {
            "Solved — every tile is covered.".to_owned()
        } else if self.path.is_empty() {
            "Start on checkpoint 1.".to_owned()
        } else {
            format!("Checkpoint {} next.", self.next_checkpoint())
        }
    }

    fn scene(&self, revision: u64) -> Scene2DScene {
        let pad = 18.0;
        let cell = 62.0;
        let gap = 6.0;
        let width = pad * 2.0 + ZIP_COLS as f32 * cell + (ZIP_COLS - 1) as f32 * gap;
        let mut scene = new_scene(
            width,
            width,
            "Zip path puzzle",
            "Cover every cell in checkpoint order.",
        );
        scene.grid = Some(Scene2DGrid {
            rows: ZIP_ROWS as u32,
            columns: ZIP_COLS as u32,
            x: pad,
            y: pad,
            cell_width: cell,
            cell_height: cell,
            gap,
            row_labels: (1..=ZIP_ROWS).map(|value| value.to_string()).collect(),
            column_labels: (1..=ZIP_COLS).map(|value| value.to_string()).collect(),
        });
        scene.nodes.push(rounded_node(
            "zip-board",
            None,
            SceneRect::new(5.0, 5.0, width - 10.0, width - 10.0),
            18.0,
            color(0.075, 0.12, 0.18),
            None,
            None,
        ));
        for row in 0..ZIP_ROWS {
            for col in 0..ZIP_COLS {
                let cell_index = row * ZIP_COLS + col;
                let rect = SceneRect::new(
                    pad + col as f32 * (cell + gap),
                    pad + row as f32 * (cell + gap),
                    cell,
                    cell,
                );
                let value = if self.path.contains(&(row, col)) {
                    "visited"
                } else {
                    "open"
                };
                let mut node = rounded_node(
                    &format!("zip-cell-{row}-{col}"),
                    Some(format!("zip-cell-{row}-{col}")),
                    rect,
                    11.0,
                    if self.path.contains(&(row, col)) {
                        color(0.10, 0.36, 0.32)
                    } else {
                        color(0.13, 0.19, 0.27)
                    },
                    Some(stroke(1.0, color(0.34, 0.47, 0.60))),
                    Some(semantic(
                        Scene2DSemanticRole::GridCell,
                        format!("Row {}, column {}", row + 1, col + 1),
                        value,
                        false,
                    )),
                );
                node.transition = Some(transition(160));
                scene.nodes.push(node);
                let _ = cell_index;
            }
        }
        for (index, pair) in self.path.windows(2).enumerate() {
            scene.nodes.push(line_node(
                &format!("zip-path-{index}"),
                scene_point(cell_center(pair[0], pad, cell, gap)),
                scene_point(cell_center(pair[1], pad, cell, gap)),
                stroke(12.0, color(0.21, 0.82, 0.68)),
            ));
        }
        for (index, path_cell) in self.path.iter().copied().enumerate() {
            let (x, y) = cell_center(path_cell, pad, cell, gap);
            scene.nodes.push(circle_node(
                &format!("zip-path-joint-{index}"),
                None,
                ScenePoint::new(x, y),
                6.0,
                Some(brush(color(0.21, 0.82, 0.68))),
                None,
                None,
            ));
        }
        for (index, (row, col)) in ZIP_CHECKPOINTS.iter().copied().enumerate() {
            let (x, y) = cell_center((row, col), pad, cell, gap);
            scene.nodes.push(circle_node(
                &format!("zip-checkpoint-{}", index + 1),
                None,
                ScenePoint::new(x, y),
                18.0,
                Some(brush(color(0.96, 0.81, 0.46))),
                Some(stroke(2.0, color(0.10, 0.21, 0.28))),
                None,
            ));
            scene.nodes.push(text_node(
                &format!("zip-number-{}", index + 1),
                None,
                ScenePoint::new(x, y - 7.0),
                (index + 1).to_string(),
                18.0,
                color(0.08, 0.15, 0.21),
                Scene2DTextAlign::Center,
                Some(semantic(
                    Scene2DSemanticRole::Status,
                    format!("Checkpoint {}", index + 1),
                    "numbered path marker",
                    false,
                )),
            ));
        }
        let head = self.path.last().copied().unwrap_or(ZIP_START);
        let (head_x, head_y) = cell_center(head, pad, cell, gap);
        let mut head_node = circle_node(
            "zip-head",
            None,
            ScenePoint::new(0.0, 0.0),
            22.0,
            None,
            Some(stroke(3.0, color(0.21, 0.91, 0.76))),
            Some(semantic(
                Scene2DSemanticRole::Status,
                "Path head",
                format!("Row {}, column {}", head.0 + 1, head.1 + 1),
                true,
            )),
        );
        head_node.transform = Scene2DTransform::translation(head_x, head_y);
        head_node.transition = Some(transition(180));
        scene.nodes.push(head_node);
        scene.revision = revision;
        scene.input = Scene2DInputConfig {
            pointer: true,
            continuous: true,
            capture: true,
            keyboard: true,
        };
        scene
    }
}

#[derive(Clone, Debug)]
struct QueensGame {
    marks: [[u8; QUEENS_SIZE]; QUEENS_SIZE],
    selected: (usize, usize),
    undo: VecDeque<([[u8; QUEENS_SIZE]; QUEENS_SIZE], (usize, usize))>,
    won: bool,
}

impl Default for QueensGame {
    fn default() -> Self {
        Self {
            marks: [[0; QUEENS_SIZE]; QUEENS_SIZE],
            selected: (0, 0),
            undo: VecDeque::new(),
            won: false,
        }
    }
}

impl QueensGame {
    fn record(&mut self) {
        push_history(&mut self.undo, (self.marks, self.selected));
        self.won = false;
    }

    fn region(row: usize, col: usize) -> usize {
        QUEENS_REGIONS[row][col]
    }

    fn queens(&self) -> Vec<(usize, usize)> {
        let mut queens = Vec::new();
        for row in 0..QUEENS_SIZE {
            for col in 0..QUEENS_SIZE {
                if self.marks[row][col] == 1 {
                    queens.push((row, col));
                }
            }
        }
        queens
    }

    fn conflicts(&self) -> [[bool; QUEENS_SIZE]; QUEENS_SIZE] {
        let queens = self.queens();
        let mut conflicts = [[false; QUEENS_SIZE]; QUEENS_SIZE];
        for first in 0..queens.len() {
            for second in first + 1..queens.len() {
                let (r1, c1) = queens[first];
                let (r2, c2) = queens[second];
                let same_region = Self::region(r1, c1) == Self::region(r2, c2);
                let touching = r1.abs_diff(r2) <= 1 && c1.abs_diff(c2) <= 1;
                if r1 == r2 || c1 == c2 || same_region || touching {
                    conflicts[r1][c1] = true;
                    conflicts[r2][c2] = true;
                }
            }
        }
        conflicts
    }

    fn is_valid(&self) -> bool {
        let queens = self.queens();
        queens.len() == QUEENS_SIZE && self.conflicts().iter().flatten().all(|conflict| !conflict)
    }

    fn count(&self) -> usize {
        self.queens().len()
    }

    fn place(&mut self, row: usize, col: usize) -> bool {
        if row >= QUEENS_SIZE || col >= QUEENS_SIZE {
            return false;
        }
        self.record();
        self.selected = (row, col);
        self.marks[row][col] = (self.marks[row][col] + 1) % 3;
        self.won = self.is_valid();
        true
    }

    fn move_selection(&mut self, row_delta: i32, col_delta: i32) -> bool {
        let row = (self.selected.0 as i32 + row_delta).clamp(0, (QUEENS_SIZE - 1) as i32) as usize;
        let col = (self.selected.1 as i32 + col_delta).clamp(0, (QUEENS_SIZE - 1) as i32) as usize;
        if (row, col) == self.selected {
            return false;
        }
        self.selected = (row, col);
        true
    }

    fn undo(&mut self) -> bool {
        if let Some((marks, selected)) = self.undo.pop_back() {
            self.marks = marks;
            self.selected = selected;
            self.won = self.is_valid();
            true
        } else {
            false
        }
    }

    fn reset(&mut self) -> bool {
        let changed = self.count() > 0 || self.won;
        *self = Self::default();
        changed
    }

    fn hint(&mut self) -> bool {
        for row in 0..QUEENS_SIZE {
            if !self.queens().iter().any(|(queen_row, _)| *queen_row == row) {
                return self.place(row, QUEENS_SOLUTION[row]);
            }
        }
        false
    }

    fn handle(&mut self, event: Scene2DInput) -> bool {
        match event {
            Scene2DInput::Pointer {
                phase: gpui_ui_kit::scene2d::Scene2DPointerPhase::Up,
                cell: Some(cell),
                ..
            } => self.place(cell.row as usize, cell.column as usize),
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
                    return self.move_selection(dr, dc);
                }
                match key.as_str() {
                    "Enter" | "Space" => self.place(self.selected.0, self.selected.1),
                    "m" => {
                        self.record();
                        let (row, col) = self.selected;
                        self.marks[row][col] = if self.marks[row][col] == 2 { 0 } else { 2 };
                        true
                    }
                    _ => false,
                }
            }
            Scene2DInput::Activate {
                cell: Some(cell), ..
            } => self.place(cell.row as usize, cell.column as usize),
            Scene2DInput::Lifecycle { .. } => false,
            _ => false,
        }
    }

    fn status(&self) -> String {
        if self.won {
            return "Solved — one crown in every row, column, and region.".to_owned();
        }
        let conflicts = self
            .conflicts()
            .iter()
            .flatten()
            .filter(|value| **value)
            .count();
        if conflicts > 0 {
            format!(
                "{} crowns are in conflict. Move or remove a crown.",
                conflicts
            )
        } else {
            format!("{} of {} crowns placed.", self.count(), QUEENS_SIZE)
        }
    }

    fn scene(&self, revision: u64) -> Scene2DScene {
        let size = 8;
        let pad = 18.0;
        let cell = 48.0;
        let gap = 2.0;
        let width = pad * 2.0 + size as f32 * cell + (size - 1) as f32 * gap;
        let mut scene = new_scene(
            width,
            width,
            "Queens puzzle",
            "Place one crown per row, column, and colored region. Crowns cannot touch.",
        );
        scene.grid = Some(Scene2DGrid {
            rows: size as u32,
            columns: size as u32,
            x: pad,
            y: pad,
            cell_width: cell,
            cell_height: cell,
            gap,
            row_labels: (1..=size).map(|value| value.to_string()).collect(),
            column_labels: (1..=size).map(|value| value.to_string()).collect(),
        });
        let conflicts = self.conflicts();
        scene.nodes.push(rounded_node(
            "queens-board",
            None,
            SceneRect::new(5.0, 5.0, width - 10.0, width - 10.0),
            18.0,
            color(0.075, 0.12, 0.18),
            None,
            None,
        ));
        for row in 0..size {
            for col in 0..size {
                let x = pad + col as f32 * (cell + gap);
                let y = pad + row as f32 * (cell + gap);
                let rect = SceneRect::new(x, y, cell, cell);
                let value = match self.marks[row][col] {
                    1 => "crown",
                    2 => "excluded",
                    _ => "open",
                };
                let selected = self.selected == (row, col);
                let region = Self::region(row, col);
                scene.nodes.push(rounded_node(
                    &format!("queens-cell-{row}-{col}"),
                    Some(format!("queens-cell-{row}-{col}")),
                    rect,
                    7.0,
                    QUEENS_COLORS[region],
                    Some(stroke(
                        if selected { 3.0 } else { 0.7 },
                        if selected {
                            color(0.12, 0.93, 0.72)
                        } else {
                            color(0.94, 0.97, 1.0)
                        },
                    )),
                    Some(semantic(
                        Scene2DSemanticRole::GridCell,
                        format!("Row {}, column {}, region {}", row + 1, col + 1, region + 1),
                        if conflicts[row][col] {
                            "conflict"
                        } else {
                            value
                        },
                        selected,
                    )),
                ));
                let (cx, cy) = (x + cell * 0.5, y + cell * 0.5);
                if self.marks[row][col] == 1 {
                    let points = [
                        ScenePoint::new(cx - 14.0, cy + 12.0),
                        ScenePoint::new(cx - 10.0, cy - 6.0),
                        ScenePoint::new(cx - 2.0, cy + 2.0),
                        ScenePoint::new(cx + 1.0, cy - 13.0),
                        ScenePoint::new(cx + 8.0, cy + 2.0),
                        ScenePoint::new(cx + 13.0, cy - 8.0),
                        ScenePoint::new(cx + 14.0, cy + 12.0),
                    ];
                    let mut commands = vec![Scene2DPathCommand::MoveTo { point: points[0] }];
                    commands.extend(
                        points[1..]
                            .iter()
                            .copied()
                            .map(|point| Scene2DPathCommand::LineTo { point }),
                    );
                    commands.push(Scene2DPathCommand::Close);
                    scene.nodes.push(node(
                        format!("queens-crown-{row}-{col}"),
                        None,
                        Some(semantic(
                            Scene2DSemanticRole::Image,
                            "Crown",
                            "Queen piece",
                            false,
                        )),
                        Scene2DNodeKind::Path {
                            commands,
                            fill: Some(brush(color(0.98, 0.78, 0.31))),
                            stroke: Some(stroke(1.2, color(0.32, 0.24, 0.10))),
                        },
                        None,
                    ));
                } else if self.marks[row][col] == 2 {
                    scene.nodes.push(line_node(
                        &format!("queens-mark-{row}-{col}"),
                        ScenePoint::new(cx - 8.0, cy - 8.0),
                        ScenePoint::new(cx + 8.0, cy + 8.0),
                        stroke(2.5, color(0.28, 0.37, 0.46)),
                    ));
                    scene.nodes.push(line_node(
                        &format!("queens-mark-b-{row}-{col}"),
                        ScenePoint::new(cx + 8.0, cy - 8.0),
                        ScenePoint::new(cx - 8.0, cy + 8.0),
                        stroke(2.5, color(0.28, 0.37, 0.46)),
                    ));
                }
                if conflicts[row][col] {
                    scene.nodes.push(rounded_node(
                        &format!("queens-conflict-{row}-{col}"),
                        None,
                        inset_rect(rect, 2.0),
                        6.0,
                        color_alpha(0.86, 0.20, 0.28, 0.18),
                        Some(stroke(3.0, color(1.0, 0.30, 0.36))),
                        None,
                    ));
                }
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
}

#[derive(Clone, Debug)]
struct SudokuGame {
    values: [[u8; 9]; 9],
    givens: [[bool; 9]; 9],
    selected: Option<(usize, usize)>,
    history: VecDeque<([[u8; 9]; 9], Option<(usize, usize)>)>,
    won: bool,
    message: String,
}

impl Default for SudokuGame {
    fn default() -> Self {
        let mut values = [[0; 9]; 9];
        let mut givens = [[false; 9]; 9];
        for (row, text) in SUDOKU_PUZZLE.iter().enumerate() {
            for (col, digit) in text.bytes().enumerate() {
                values[row][col] = digit.saturating_sub(b'0');
                givens[row][col] = values[row][col] != 0;
            }
        }
        Self {
            values,
            givens,
            selected: Some((0, 2)),
            history: VecDeque::new(),
            won: false,
            message: "Cell R1 C3 selected; choose a digit.".to_owned(),
        }
    }
}

impl SudokuGame {
    fn record(&mut self) {
        push_history(&mut self.history, (self.values, self.selected));
        self.won = false;
    }

    fn is_conflict(&self, row: usize, col: usize) -> bool {
        let value = self.values[row][col];
        if value == 0 {
            return false;
        }
        for other in 0..9 {
            if other != col && self.values[row][other] == value {
                return true;
            }
            if other != row && self.values[other][col] == value {
                return true;
            }
        }
        let row_start = row / 3 * 3;
        let col_start = col / 3 * 3;
        (row_start..row_start + 3).any(|r| {
            (col_start..col_start + 3).any(|c| (r, c) != (row, col) && self.values[r][c] == value)
        })
    }

    fn conflicts(&self) -> BTreeSet<(usize, usize)> {
        let mut result = BTreeSet::new();
        for row in 0..9 {
            for col in 0..9 {
                if self.is_conflict(row, col) {
                    result.insert((row, col));
                }
            }
        }
        result
    }

    fn candidates(&self, row: usize, col: usize) -> Vec<u8> {
        if self.values[row][col] != 0 {
            return Vec::new();
        }
        (1..=9)
            .filter(|digit| {
                !self.values[row].contains(digit)
                    && !(0..9).any(|r| self.values[r][col] == *digit)
                    && !(row / 3 * 3..row / 3 * 3 + 3).any(|r| {
                        (col / 3 * 3..col / 3 * 3 + 3).any(|c| self.values[r][c] == *digit)
                    })
            })
            .collect()
    }

    fn select(&mut self, row: usize, col: usize) -> bool {
        if row >= 9 || col >= 9 || self.selected == Some((row, col)) {
            return false;
        }
        self.selected = Some((row, col));
        self.message = self.preview();
        true
    }

    fn enter(&mut self, digit: u8) -> bool {
        if !(1..=9).contains(&digit) {
            return false;
        }
        let Some((row, col)) = self.selected.or_else(|| self.first_empty()) else {
            self.message = "The board is full.".to_owned();
            return true;
        };
        if self.givens[row][col] {
            self.message = "Clues cannot be changed.".to_owned();
            return true;
        }
        self.record();
        self.selected = Some((row, col));
        self.values[row][col] = digit;
        self.won = self.values == SUDOKU_SOLUTION;
        self.message = if self.won {
            "Solved — every row, column, and box is complete.".to_owned()
        } else {
            self.preview()
        };
        true
    }

    fn erase(&mut self) -> bool {
        let Some((row, col)) = self.selected else {
            return false;
        };
        if self.givens[row][col] || self.values[row][col] == 0 {
            return false;
        }
        self.record();
        self.values[row][col] = 0;
        self.message = self.preview();
        true
    }

    fn move_selection(&mut self, row_delta: i32, col_delta: i32) -> bool {
        let current = self.selected.unwrap_or((0, 0));
        let row = (current.0 as i32 + row_delta).clamp(0, 8) as usize;
        let col = (current.1 as i32 + col_delta).clamp(0, 8) as usize;
        self.select(row, col)
    }

    fn hint(&mut self) -> bool {
        let selected = self
            .selected
            .filter(|(r, c)| !self.givens[*r][*c] && self.values[*r][*c] == 0)
            .or_else(|| self.first_empty());
        if let Some((row, col)) = selected {
            self.select(row, col);
            self.enter(SUDOKU_SOLUTION[row][col])
        } else {
            false
        }
    }

    fn check(&mut self) -> bool {
        let conflicts = self.conflicts().len();
        let wrong = (0..9)
            .flat_map(|r| (0..9).map(move |c| (r, c)))
            .filter(|(r, c)| {
                self.values[*r][*c] != 0 && self.values[*r][*c] != SUDOKU_SOLUTION[*r][*c]
            })
            .count();
        self.won = self.values == SUDOKU_SOLUTION;
        self.message = if self.won {
            "Solved — every row, column, and box is complete.".to_owned()
        } else if conflicts > 0 || wrong > 0 {
            format!(
                "Check the highlighted cells: {conflicts} rule conflicts, {wrong} incorrect entries."
            )
        } else {
            "No conflicts so far. Keep going.".to_owned()
        };
        true
    }

    fn undo(&mut self) -> bool {
        if let Some((values, selected)) = self.history.pop_back() {
            self.values = values;
            self.selected = selected;
            self.won = self.values == SUDOKU_SOLUTION;
            self.message = self.preview();
            true
        } else {
            false
        }
    }

    fn reset(&mut self) -> bool {
        *self = Self::default();
        true
    }

    fn first_empty(&self) -> Option<(usize, usize)> {
        (0..9)
            .flat_map(|row| (0..9).map(move |col| (row, col)))
            .find(|(row, col)| !self.givens[*row][*col] && self.values[*row][*col] == 0)
    }

    fn preview(&self) -> String {
        self.selected
            .map(|(row, col)| {
                let value = self.values[row][col];
                let candidates = self
                    .candidates(row, col)
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(" · ");
                if value == 0 {
                    format!(
                        "Selected R{} C{} · Candidates: {}",
                        row + 1,
                        col + 1,
                        if candidates.is_empty() {
                            "none".to_owned()
                        } else {
                            candidates
                        }
                    )
                } else {
                    format!(
                        "Selected R{} C{} · {}{}",
                        row + 1,
                        col + 1,
                        if self.givens[row][col] {
                            "clue "
                        } else {
                            "entry "
                        },
                        value
                    )
                }
            })
            .unwrap_or_else(|| "Select a cell to preview its candidates.".to_owned())
    }

    fn status(&self) -> String {
        if self.won {
            return "Solved — every row, column, and box is complete.".to_owned();
        }
        self.message.clone()
    }

    fn handle(&mut self, event: Scene2DInput) -> bool {
        match event {
            Scene2DInput::Pointer {
                phase: gpui_ui_kit::scene2d::Scene2DPointerPhase::Up,
                cell: Some(cell),
                ..
            } => self.select(cell.row as usize, cell.column as usize),
            Scene2DInput::Key {
                phase: Scene2DKeyPhase::Down,
                key,
                repeat,
                ..
            } if !repeat => match key.as_str() {
                "ArrowLeft" => self.move_selection(0, -1),
                "ArrowRight" => self.move_selection(0, 1),
                "ArrowUp" => self.move_selection(-1, 0),
                "ArrowDown" => self.move_selection(1, 0),
                "Backspace" | "Delete" => self.erase(),
                value if value.len() == 1 && value.as_bytes()[0].is_ascii_digit() => value
                    .parse::<u8>()
                    .ok()
                    .is_some_and(|digit| self.enter(digit)),
                _ => false,
            },
            Scene2DInput::Activate {
                cell: Some(cell), ..
            } => self.select(cell.row as usize, cell.column as usize),
            _ => false,
        }
    }

    fn scene(&self, revision: u64) -> Scene2DScene {
        let pad = 17.0;
        let cell = 39.0;
        let width = pad * 2.0 + cell * 9.0;
        let mut scene = new_scene(
            width,
            width,
            "Sudoku board",
            "Use the number keys or keypad to enter digits. Select a cell to preview candidates.",
        );
        scene.grid = Some(Scene2DGrid {
            rows: 9,
            columns: 9,
            x: pad,
            y: pad,
            cell_width: cell,
            cell_height: cell,
            gap: 0.0,
            row_labels: (1..=9).map(|value| value.to_string()).collect(),
            column_labels: (1..=9).map(|value| value.to_string()).collect(),
        });
        let conflicts = self.conflicts();
        scene.nodes.push(rounded_node(
            "sudoku-well",
            None,
            SceneRect::new(4.0, 4.0, width - 8.0, width - 8.0),
            14.0,
            color(0.07, 0.11, 0.17),
            None,
            None,
        ));
        for row in 0..9 {
            for col in 0..9 {
                let rect =
                    SceneRect::new(pad + col as f32 * cell, pad + row as f32 * cell, cell, cell);
                let selected = self.selected == Some((row, col));
                let related = self.selected.is_some_and(|(sr, sc)| {
                    row == sr || col == sc || (row / 3, col / 3) == (sr / 3, sc / 3)
                });
                let given = self.givens[row][col];
                let fill = if selected {
                    color(0.14, 0.36, 0.43)
                } else if related {
                    color(0.11, 0.19, 0.25)
                } else {
                    color(0.09, 0.15, 0.21)
                };
                scene.nodes.push(rect_node(
                    &format!("sudoku-cell-{row}-{col}"),
                    Some(format!("sudoku-cell-{row}-{col}")),
                    rect,
                    Some(brush(fill)),
                    Some(stroke(0.55, color(0.28, 0.38, 0.48))),
                    Some(semantic(
                        Scene2DSemanticRole::GridCell,
                        format!("Row {}, column {}", row + 1, col + 1),
                        if conflicts.contains(&(row, col)) {
                            format!("conflict {}", self.values[row][col])
                        } else if self.values[row][col] == 0 {
                            "empty".to_owned()
                        } else {
                            format!(
                                "{}{}",
                                if given { "clue " } else { "entry " },
                                self.values[row][col]
                            )
                        },
                        selected,
                    )),
                ));
                if self.values[row][col] != 0 {
                    let text_color = if conflicts.contains(&(row, col)) {
                        color(1.0, 0.37, 0.41)
                    } else if given {
                        color(0.92, 0.95, 0.98)
                    } else {
                        color(0.38, 0.89, 0.76)
                    };
                    scene.nodes.push(text_node(
                        &format!("sudoku-digit-{row}-{col}"),
                        None,
                        ScenePoint::new(rect.x + cell * 0.5, rect.y + (cell - 23.0 * 1.25) * 0.5),
                        self.values[row][col].to_string(),
                        23.0,
                        text_color,
                        Scene2DTextAlign::Center,
                        Some(semantic(
                            Scene2DSemanticRole::Image,
                            format!("Digit {}", self.values[row][col]),
                            "Sudoku entry",
                            false,
                        )),
                    ));
                } else {
                    for digit in self.candidates(row, col) {
                        let candidate_row = (digit as usize - 1) / 3;
                        let candidate_col = (digit as usize - 1) % 3;
                        scene.nodes.push(text_node(
                            &format!("sudoku-candidate-{row}-{col}-{digit}"),
                            None,
                            ScenePoint::new(
                                rect.x + 7.0 + candidate_col as f32 * 11.5,
                                rect.y + 4.0 + candidate_row as f32 * 10.5,
                            ),
                            digit.to_string(),
                            7.5,
                            color(0.55, 0.66, 0.77),
                            Scene2DTextAlign::Center,
                            None,
                        ));
                    }
                }
                if conflicts.contains(&(row, col)) {
                    scene.nodes.push(rounded_node(
                        &format!("sudoku-conflict-{row}-{col}"),
                        None,
                        inset_rect(rect, 2.0),
                        2.0,
                        color_alpha(1.0, 0.18, 0.25, 0.16),
                        Some(stroke(2.4, color(1.0, 0.31, 0.36))),
                        None,
                    ));
                }
            }
        }
        for boundary in 0..=9 {
            let weight = if boundary % 3 == 0 { 3.0 } else { 0.7 };
            let line_color = if boundary % 3 == 0 {
                color(0.62, 0.73, 0.82)
            } else {
                color(0.23, 0.32, 0.40)
            };
            let offset = pad + boundary as f32 * cell;
            scene.nodes.push(line_node(
                &format!("sudoku-grid-v-{boundary}"),
                ScenePoint::new(offset, pad),
                ScenePoint::new(offset, pad + 9.0 * cell),
                stroke(weight, line_color),
            ));
            scene.nodes.push(line_node(
                &format!("sudoku-grid-h-{boundary}"),
                ScenePoint::new(pad, offset),
                ScenePoint::new(pad + 9.0 * cell, offset),
                stroke(weight, line_color),
            ));
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum PieceKind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl PieceKind {
    const fn label(self) -> &'static str {
        match self {
            Self::I => "I",
            Self::O => "O",
            Self::T => "T",
            Self::S => "S",
            Self::Z => "Z",
            Self::J => "J",
            Self::L => "L",
        }
    }
    const fn color(self) -> Scene2DColor {
        match self {
            Self::I => color(0.38, 0.82, 0.89),
            Self::O => color(0.91, 0.78, 0.36),
            Self::T => color(0.71, 0.58, 0.91),
            Self::S => color(0.43, 0.81, 0.58),
            Self::Z => color(0.91, 0.42, 0.49),
            Self::J => color(0.39, 0.57, 0.89),
            Self::L => color(0.94, 0.59, 0.37),
        }
    }
    const fn blocks(self) -> [(i32, i32); 4] {
        match self {
            Self::I => [(0, 0), (0, 1), (0, 2), (0, 3)],
            Self::O => [(0, 0), (0, 1), (1, 0), (1, 1)],
            Self::T => [(0, 1), (1, 0), (1, 1), (1, 2)],
            Self::S => [(0, 1), (0, 2), (1, 0), (1, 1)],
            Self::Z => [(0, 0), (0, 1), (1, 1), (1, 2)],
            Self::J => [(0, 0), (1, 0), (1, 1), (1, 2)],
            Self::L => [(0, 2), (1, 0), (1, 1), (1, 2)],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum HoldAction {
    Left,
    Right,
    Down,
}

#[derive(Clone, Debug)]
struct TetrisGame {
    grid: [[Option<PieceKind>; TETRIS_COLS]; TETRIS_ROWS],
    current: PieceKind,
    next: PieceKind,
    piece_index: usize,
    rotation: u8,
    row: i32,
    col: i32,
    running: bool,
    paused: bool,
    over: bool,
    score: u32,
    lines: u32,
    elapsed: f32,
    repeat_elapsed: f32,
    effect_remaining: f32,
    lock_cells: Vec<(usize, usize)>,
    clear_rows: Vec<usize>,
    contacts: HashMap<u64, HoldAction>,
    keys: HashSet<String>,
}

impl Default for TetrisGame {
    fn default() -> Self {
        Self {
            grid: [[None; TETRIS_COLS]; TETRIS_ROWS],
            current: TETRIS_PIECE_ORDER[0],
            next: TETRIS_PIECE_ORDER[1],
            piece_index: 1,
            rotation: 0,
            row: 0,
            col: 3,
            running: false,
            paused: false,
            over: false,
            score: 0,
            lines: 0,
            elapsed: 0.0,
            repeat_elapsed: 0.0,
            effect_remaining: 0.0,
            lock_cells: Vec::new(),
            clear_rows: Vec::new(),
            contacts: HashMap::new(),
            keys: HashSet::new(),
        }
    }
}

impl TetrisGame {
    fn rotated_blocks(piece: PieceKind, rotation: u8) -> [(i32, i32); 4] {
        let mut blocks = piece.blocks();
        for _ in 0..rotation % 4 {
            for block in &mut blocks {
                *block = (block.1, -block.0);
            }
            let top = blocks.iter().map(|point| point.0).min().unwrap_or(0);
            let left = blocks.iter().map(|point| point.1).min().unwrap_or(0);
            for block in &mut blocks {
                block.0 -= top;
                block.1 -= left;
            }
        }
        blocks
    }

    fn cells(&self) -> [(i32, i32); 4] {
        Self::rotated_blocks(self.current, self.rotation)
    }

    fn fits(&self, row: i32, col: i32, rotation: u8) -> bool {
        Self::rotated_blocks(self.current, rotation)
            .iter()
            .all(|(dr, dc)| {
                let r = row + dr;
                let c = col + dc;
                r >= 0
                    && r < TETRIS_ROWS as i32
                    && c >= 0
                    && c < TETRIS_COLS as i32
                    && self.grid[r as usize][c as usize].is_none()
            })
    }

    fn reset_clock(&mut self) {
        self.elapsed = 0.0;
        self.repeat_elapsed = 0.0;
    }

    fn needs_ticks(&self) -> bool {
        self.effect_remaining > 0.0 || (self.running && !self.paused && !self.over)
    }

    fn new_game(&mut self) -> bool {
        *self = Self::default();
        self.running = true;
        true
    }

    fn toggle_pause(&mut self) -> bool {
        if self.over {
            return false;
        }
        if !self.running {
            self.running = true;
            self.paused = false;
        } else {
            self.paused = !self.paused;
        }
        self.reset_clock();
        true
    }

    fn spawn_next(&mut self) {
        self.current = self.next;
        self.piece_index = self.piece_index.saturating_add(1);
        self.next = TETRIS_PIECE_ORDER[self.piece_index % TETRIS_PIECE_ORDER.len()];
        self.rotation = 0;
        self.row = 0;
        self.col = 3;
        if !self.fits(self.row, self.col, self.rotation) {
            self.over = true;
            self.running = false;
        }
    }

    fn move_piece(&mut self, dr: i32, dc: i32) -> bool {
        if !self.running || self.paused || self.over {
            return false;
        }
        if self.fits(self.row + dr, self.col + dc, self.rotation) {
            self.row += dr;
            self.col += dc;
            return true;
        }
        false
    }

    fn rotate(&mut self) -> bool {
        if !self.running || self.paused || self.over {
            return false;
        }
        let next = (self.rotation + 1) % 4;
        for kick in [0, -1, 1, -2, 2] {
            if self.fits(self.row, self.col + kick, next) {
                self.rotation = next;
                self.col += kick;
                return true;
            }
        }
        false
    }

    fn ghost_row(&self) -> i32 {
        let mut row = self.row;
        while self.fits(row + 1, self.col, self.rotation) {
            row += 1;
        }
        row
    }

    fn hard_drop(&mut self) -> bool {
        if !self.running || self.paused || self.over {
            return false;
        }
        self.row = self.ghost_row();
        self.lock_piece();
        true
    }

    fn soft_step(&mut self) -> bool {
        if self.move_piece(1, 0) {
            self.score = self.score.saturating_add(1);
            true
        } else {
            self.lock_piece();
            true
        }
    }

    fn lock_piece(&mut self) {
        self.lock_cells.clear();
        for (dr, dc) in self.cells() {
            let row = self.row + dr;
            let col = self.col + dc;
            if row >= 0 && row < TETRIS_ROWS as i32 && col >= 0 && col < TETRIS_COLS as i32 {
                self.grid[row as usize][col as usize] = Some(self.current);
                self.lock_cells.push((row as usize, col as usize));
            }
        }
        let mut compacted = [[None; TETRIS_COLS]; TETRIS_ROWS];
        let mut destination = TETRIS_ROWS as isize - 1;
        let mut cleared = Vec::new();
        for source in (0..TETRIS_ROWS).rev() {
            if self.grid[source].iter().all(Option::is_some) {
                cleared.push(source);
            } else {
                compacted[destination as usize] = self.grid[source];
                destination -= 1;
            }
        }
        self.grid = compacted;
        self.clear_rows = cleared;
        if !self.clear_rows.is_empty() {
            let count = self.clear_rows.len() as u32;
            self.lines = self.lines.saturating_add(count);
            self.score = self
                .score
                .saturating_add([0, 100, 300, 500, 800][count.min(4) as usize] * self.level());
        }
        self.effect_remaining = 0.28;
        self.spawn_next();
    }

    fn level(&self) -> u32 {
        1 + self.lines / 10
    }

    fn status(&self) -> String {
        if self.over {
            "Game over — start a new game.".to_owned()
        } else if self.paused {
            "Paused.".to_owned()
        } else if self.running {
            "Playing — arrows move, Up/X rotates, Space drops, P pauses.".to_owned()
        } else {
            "Press Start / resume or New game.".to_owned()
        }
    }

    fn advance(&mut self, elapsed: Duration) -> bool {
        let dt = elapsed.as_secs_f32().min(0.25);
        if dt <= 0.0 {
            return false;
        }
        let mut changed = false;
        if self.effect_remaining > 0.0 {
            self.effect_remaining = (self.effect_remaining - dt).max(0.0);
            if self.effect_remaining == 0.0 {
                self.lock_cells.clear();
                self.clear_rows.clear();
                changed = true;
            }
        }
        if !self.running || self.paused || self.over {
            return changed;
        }
        let left = self.keys.contains("ArrowLeft")
            || self
                .contacts
                .values()
                .any(|action| *action == HoldAction::Left);
        let right = self.keys.contains("ArrowRight")
            || self
                .contacts
                .values()
                .any(|action| *action == HoldAction::Right);
        let down = self.keys.contains("ArrowDown")
            || self
                .contacts
                .values()
                .any(|action| *action == HoldAction::Down);
        if left || right || down {
            self.repeat_elapsed += dt;
            if self.repeat_elapsed >= 0.20 {
                let repeats = (((self.repeat_elapsed - 0.20) / 0.075).floor() as usize + 1).min(3);
                for _ in 0..repeats {
                    if left {
                        changed |= self.move_piece(0, -1);
                    }
                    if right {
                        changed |= self.move_piece(0, 1);
                    }
                    if down {
                        changed |= self.soft_step();
                    }
                }
                self.repeat_elapsed = 0.20 + (self.repeat_elapsed - 0.20).rem_euclid(0.075);
            }
        } else {
            self.repeat_elapsed = 0.0;
        }
        self.elapsed += dt;
        let interval = (0.62_f32 - (self.level().saturating_sub(1) as f32 * 0.045)).max(0.12);
        let mut steps = 0;
        while self.elapsed >= interval && steps < 4 && self.running && !self.paused && !self.over {
            self.elapsed -= interval;
            if self.move_piece(1, 0) {
                changed = true;
            } else {
                self.lock_piece();
                changed = true;
            }
            steps += 1;
        }
        if steps == 4 && self.elapsed >= interval {
            self.elapsed = interval * 0.5;
        }
        changed
    }

    fn release_all(&mut self) {
        self.contacts.clear();
        self.keys.clear();
        self.repeat_elapsed = 0.0;
    }

    fn handle(&mut self, event: Scene2DInput) -> bool {
        match event {
            Scene2DInput::Lifecycle { .. } => {
                let changed = !self.contacts.is_empty() || !self.keys.is_empty();
                self.release_all();
                changed
            }
            Scene2DInput::Pointer {
                phase,
                contact_id,
                hit_id,
                ..
            } => match phase {
                gpui_ui_kit::scene2d::Scene2DPointerPhase::Down => match hit_id.as_deref() {
                    Some("hold-left") => {
                        self.contacts.insert(contact_id, HoldAction::Left);
                        self.move_piece(0, -1) || true
                    }
                    Some("hold-right") => {
                        self.contacts.insert(contact_id, HoldAction::Right);
                        self.move_piece(0, 1) || true
                    }
                    Some("hold-down") => {
                        self.contacts.insert(contact_id, HoldAction::Down);
                        self.soft_step() || true
                    }
                    Some("hold-rotate") => self.rotate(),
                    _ => false,
                },
                gpui_ui_kit::scene2d::Scene2DPointerPhase::Up
                | gpui_ui_kit::scene2d::Scene2DPointerPhase::Cancel => {
                    self.contacts.remove(&contact_id).is_some()
                }
                gpui_ui_kit::scene2d::Scene2DPointerPhase::Move => false,
            },
            Scene2DInput::Key {
                phase: Scene2DKeyPhase::Down,
                key,
                repeat,
                ..
            } => {
                if key == "p" && !repeat {
                    return self.toggle_pause();
                }
                if key == "Space" && !repeat {
                    return self.hard_drop();
                }
                if key == "ArrowUp" || key == "x" {
                    return !repeat && self.rotate();
                }
                let hold = match key.as_str() {
                    "ArrowLeft" => Some(HoldAction::Left),
                    "ArrowRight" => Some(HoldAction::Right),
                    "ArrowDown" => Some(HoldAction::Down),
                    _ => None,
                };
                if let Some(action) = hold {
                    if !self.keys.insert(key.clone()) {
                        return false;
                    }
                    return match action {
                        HoldAction::Left => self.move_piece(0, -1) || true,
                        HoldAction::Right => self.move_piece(0, 1) || true,
                        HoldAction::Down => self.soft_step() || true,
                    };
                }
                false
            }
            Scene2DInput::Key {
                phase: Scene2DKeyPhase::Up,
                key,
                ..
            } => self.keys.remove(&key),
            Scene2DInput::Activate { id, hit_id, .. } => {
                let target = hit_id.as_deref().unwrap_or(&id);
                match target {
                    "hold-left" => self.move_piece(0, -1),
                    "hold-right" => self.move_piece(0, 1),
                    "hold-down" => self.soft_step(),
                    "hold-rotate" => self.rotate(),
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn scene(&self, revision: u64) -> Scene2DScene {
        let pad = 12.0;
        let cell = 22.0;
        let gap = 2.0;
        let width = pad * 2.0 + TETRIS_COLS as f32 * cell + (TETRIS_COLS - 1) as f32 * gap;
        let height = pad * 2.0 + TETRIS_ROWS as f32 * cell + (TETRIS_ROWS - 1) as f32 * gap;
        let mut scene = new_scene(
            width,
            height,
            "Tetris playfield",
            "Move the falling piece, fill rows, and avoid the top.",
        );
        scene.grid = Some(Scene2DGrid {
            rows: TETRIS_ROWS as u32,
            columns: TETRIS_COLS as u32,
            x: pad,
            y: pad,
            cell_width: cell,
            cell_height: cell,
            gap,
            row_labels: (1..=TETRIS_ROWS).map(|value| value.to_string()).collect(),
            column_labels: (1..=TETRIS_COLS).map(|value| value.to_string()).collect(),
        });
        let ghost_row = self.ghost_row();
        let current_cells = self.cells();
        let mut ghost = HashSet::new();
        for (dr, dc) in current_cells {
            ghost.insert((ghost_row + dr, self.col + dc));
        }
        let mut active = HashSet::new();
        for (dr, dc) in current_cells {
            active.insert((self.row + dr, self.col + dc));
        }
        scene.nodes.push(rounded_node(
            "tetris-well",
            None,
            SceneRect::new(4.0, 4.0, width - 8.0, height - 8.0),
            15.0,
            color(0.055, 0.09, 0.15),
            None,
            None,
        ));
        for row in 0..TETRIS_ROWS {
            for col in 0..TETRIS_COLS {
                let rect = SceneRect::new(
                    pad + col as f32 * (cell + gap),
                    pad + row as f32 * (cell + gap),
                    cell,
                    cell,
                );
                let locked = self.grid[row][col];
                let is_active = active.contains(&(row as i32, col as i32));
                let is_ghost =
                    ghost.contains(&(row as i32, col as i32)) && !is_active && locked.is_none();
                let fill = locked.map(PieceKind::color).or_else(|| {
                    if is_active {
                        Some(self.current.color())
                    } else if is_ghost {
                        Some(Scene2DColor::rgba(
                            self.current.color().r,
                            self.current.color().g,
                            self.current.color().b,
                            0.28,
                        ))
                    } else {
                        None
                    }
                });
                scene.nodes.push(rect_node(
                    &format!("tetris-cell-{row}-{col}"),
                    Some(format!("tetris-cell-{row}-{col}")),
                    rect,
                    fill.map(brush),
                    Some(stroke(0.7, color(0.21, 0.31, 0.42))),
                    Some(semantic(
                        Scene2DSemanticRole::GridCell,
                        format!("Row {}, column {}", row + 1, col + 1),
                        if is_active {
                            format!("falling {}", self.current.label())
                        } else if is_ghost {
                            "landing preview".to_owned()
                        } else if let Some(piece) = locked {
                            format!("locked {}", piece.label())
                        } else {
                            "empty".to_owned()
                        },
                        is_active,
                    )),
                ));
            }
        }
        if self.effect_remaining > 0.0 {
            for (row, col) in &self.lock_cells {
                let rect = SceneRect::new(
                    pad + *col as f32 * (cell + gap) + 1.0,
                    pad + *row as f32 * (cell + gap) + 1.0,
                    cell - 2.0,
                    cell - 2.0,
                );
                let mut flash = rounded_node(
                    &format!("tetris-lock-{row}-{col}"),
                    None,
                    rect,
                    4.0,
                    color_alpha(0.97, 0.80, 0.35, 0.22),
                    None,
                    None,
                );
                flash.transition = Some(Scene2DTransition {
                    duration_ms: 260,
                    easing: Scene2DEasing::EaseOutQuad,
                    completion_id: None,
                    animate_color: false,
                    reveal_path: false,
                });
                scene.nodes.push(flash);
            }
            for row in &self.clear_rows {
                let rect = SceneRect::new(
                    pad,
                    pad + *row as f32 * (cell + gap),
                    width - pad * 2.0,
                    cell,
                );
                let mut flash = rounded_node(
                    &format!("tetris-clear-{row}"),
                    None,
                    rect,
                    3.0,
                    color_alpha(1.0, 0.88, 0.55, 0.45),
                    None,
                    None,
                );
                flash.transition = Some(Scene2DTransition {
                    duration_ms: 260,
                    easing: Scene2DEasing::EaseOutCubic,
                    completion_id: None,
                    animate_color: false,
                    reveal_path: false,
                });
                scene.nodes.push(flash);
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

    fn preview_scene(&self, revision: u64) -> Scene2DScene {
        let cell = 31.0;
        let pad = 14.0;
        let gap = 3.0;
        let width = pad * 2.0 + 4.0 * cell + 3.0 * gap;
        let mut scene = new_scene(
            width,
            width,
            "Next Tetris piece",
            format!("Next: {}", self.next.label()),
        );
        scene.nodes.push(rounded_node(
            "tetris-preview-well",
            None,
            SceneRect::new(4.0, 4.0, width - 8.0, width - 8.0),
            14.0,
            color(0.075, 0.12, 0.18),
            None,
            None,
        ));
        let blocks = TetrisGame::rotated_blocks(self.next, 0);
        for (index, (row, col)) in blocks.iter().copied().enumerate() {
            let x = pad + col as f32 * (cell + gap);
            let y = pad + row as f32 * (cell + gap);
            scene.nodes.push(rounded_node(
                &format!("tetris-preview-{index}"),
                None,
                SceneRect::new(x, y, cell, cell),
                7.0,
                self.next.color(),
                Some(stroke(1.0, color(0.87, 0.92, 0.98))),
                Some(semantic(
                    Scene2DSemanticRole::Image,
                    format!("{} block", self.next.label()),
                    "Next piece",
                    false,
                )),
            ));
        }
        scene.revision = revision;
        scene.input = Scene2DInputConfig {
            pointer: false,
            continuous: false,
            capture: false,
            keyboard: false,
        };
        scene
    }

    fn controls_scene(&self, revision: u64) -> Scene2DScene {
        let mut scene = new_scene(
            356.0,
            88.0,
            "Tetris touch controls",
            "Hold left, right, or soft drop. Tap rotate. Multiple contacts are supported.",
        );
        let controls = [
            ("hold-left", Some("◀"), "Left", "Move left"),
            ("hold-right", Some("▶"), "Right", "Move right"),
            ("hold-down", Some("▼"), "Down", "Soft drop"),
            ("hold-rotate", None, "Rotate", "Rotate"),
        ];
        for (index, (hit_id, icon, display_label, label)) in controls.into_iter().enumerate() {
            let x = 6.0 + index as f32 * 86.0;
            let rect = SceneRect::new(x, 6.0, 78.0, 76.0);
            scene.nodes.push(rounded_node(
                &format!("tetris-control-{hit_id}"),
                Some(hit_id.to_owned()),
                rect,
                15.0,
                color(0.14, 0.21, 0.30),
                Some(stroke(1.2, color(0.32, 0.44, 0.57))),
                Some(semantic(
                    Scene2DSemanticRole::Button,
                    label,
                    if hit_id == "hold-rotate" {
                        "Tap to rotate"
                    } else {
                        "Hold to repeat"
                    },
                    false,
                )),
            ));
            if let Some(icon) = icon {
                scene.nodes.push(text_node(
                    &format!("tetris-control-icon-{hit_id}"),
                    None,
                    ScenePoint::new(x + 39.0, 24.0),
                    icon,
                    30.0,
                    color(0.91, 0.96, 0.99),
                    Scene2DTextAlign::Center,
                    None,
                ));
            } else {
                let icon_color = color(0.91, 0.96, 0.99);
                scene.nodes.push(circle_node(
                    "tetris-control-rotate-ring",
                    None,
                    ScenePoint::new(x + 39.0, 28.0),
                    10.0,
                    None,
                    Some(stroke(2.5, icon_color)),
                    None,
                ));
                scene.nodes.push(line_node(
                    "tetris-control-rotate-arrow-a",
                    ScenePoint::new(x + 39.0, 18.0),
                    ScenePoint::new(x + 49.0, 18.0),
                    stroke(2.5, icon_color),
                ));
                scene.nodes.push(line_node(
                    "tetris-control-rotate-arrow-b",
                    ScenePoint::new(x + 49.0, 18.0),
                    ScenePoint::new(x + 49.0, 28.0),
                    stroke(2.5, icon_color),
                ));
            }
            let mut label_node = text_node(
                &format!("tetris-control-label-{hit_id}"),
                None,
                ScenePoint::new(x, 58.0),
                display_label,
                9.5,
                color(0.77, 0.84, 0.91),
                Scene2DTextAlign::Center,
                None,
            );
            // Text alignment uses this bound's width. Keep the origin at the
            // control's left edge so labels center within their own button.
            label_node.hit_bounds = Some(rect);
            scene.nodes.push(label_node);
        }
        scene.revision = revision;
        scene.input = Scene2DInputConfig {
            pointer: true,
            continuous: false,
            capture: true,
            keyboard: false,
        };
        scene
    }
}

fn new_scene(width: f32, height: f32, label: &str, description: impl Into<String>) -> Scene2DScene {
    let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, width, height));
    scene.semantic = Some(Scene2DSemantic {
        role: Scene2DSemanticRole::Grid,
        label: label.to_owned(),
        description: Some(description.into()),
        value_text: None,
        selected: None,
        disabled: None,
    });
    scene.background = Some(Scene2DBrush::LinearGradient {
        angle_degrees: 135.0,
        from: color(0.035, 0.07, 0.12),
        to: color(0.08, 0.16, 0.21),
    });
    scene
}

fn apply_board_palette(mut scene: Scene2DScene, light: bool) -> Scene2DScene {
    if !light {
        return scene;
    }
    if let Some(background) = scene.background.as_mut() {
        remap_surface_brush(background);
    }
    for node in &mut scene.nodes {
        remap_board_node(node);
    }
    scene
}

fn remap_board_node(node: &mut Scene2DNode) {
    match &mut node.kind {
        Scene2DNodeKind::Group { children } => {
            for child in children {
                remap_board_node(child);
            }
        }
        Scene2DNodeKind::Rect { fill, .. }
        | Scene2DNodeKind::RoundedRect { fill, .. }
        | Scene2DNodeKind::Circle { fill, .. }
        | Scene2DNodeKind::Path { fill, .. } => {
            if let Some(fill) = fill.as_mut() {
                remap_surface_brush(fill);
            }
        }
        Scene2DNodeKind::Line { .. } => {}
        Scene2DNodeKind::Text {
            color: text_color, ..
        } => {
            if color_luminance(*text_color) > 0.48 {
                *text_color = color(0.10, 0.16, 0.22);
            }
        }
    }
}

fn remap_surface_brush(brush: &mut Scene2DBrush) {
    let remap = |color: Scene2DColor| {
        let luminance = color_luminance(color);
        let chroma = color.r.max(color.g).max(color.b) - color.r.min(color.g).min(color.b);
        if luminance < 0.36 && chroma < 0.19 {
            let base = (0.82 + luminance * 0.74).clamp(0.84, 0.97);
            Scene2DColor::rgba(
                (base + (color.r - luminance) * 0.30).clamp(0.82, 0.99),
                (base + (color.g - luminance) * 0.30).clamp(0.82, 0.99),
                (base + (color.b - luminance) * 0.30).clamp(0.82, 0.99),
                color.a,
            )
        } else {
            color
        }
    };
    match brush {
        Scene2DBrush::Solid { color } => *color = remap(*color),
        Scene2DBrush::LinearGradient { from, to, .. } => {
            *from = remap(*from);
            *to = remap(*to);
        }
    }
}

fn color_luminance(color: Scene2DColor) -> f32 {
    color.r * 0.2126 + color.g * 0.7152 + color.b * 0.0722
}

fn node(
    id: impl Into<String>,
    hit_id: Option<String>,
    semantic: Option<Scene2DSemantic>,
    kind: Scene2DNodeKind,
    transition: Option<Scene2DTransition>,
) -> Scene2DNode {
    Scene2DNode {
        id: id.into(),
        hit_id,
        hit_bounds: None,
        clip: None,
        shadow: None,
        semantic,
        transform: Default::default(),
        opacity: 1.0,
        transition,
        kind,
    }
}

fn rounded_node(
    id: &str,
    hit_id: Option<String>,
    rect: SceneRect,
    radius: f32,
    fill: Scene2DColor,
    edge: Option<Scene2DStroke>,
    semantic: Option<Scene2DSemantic>,
) -> Scene2DNode {
    node(
        id,
        hit_id,
        semantic,
        Scene2DNodeKind::RoundedRect {
            rect,
            radius,
            fill: Some(brush(fill)),
            stroke: edge,
        },
        None,
    )
}

fn rect_node(
    id: &str,
    hit_id: Option<String>,
    rect: SceneRect,
    fill: Option<Scene2DBrush>,
    edge: Option<Scene2DStroke>,
    semantic: Option<Scene2DSemantic>,
) -> Scene2DNode {
    node(
        id,
        hit_id,
        semantic,
        Scene2DNodeKind::Rect {
            rect,
            fill,
            stroke: edge,
        },
        None,
    )
}

fn circle_node(
    id: &str,
    hit_id: Option<String>,
    center: ScenePoint,
    radius: f32,
    fill: Option<Scene2DBrush>,
    edge: Option<Scene2DStroke>,
    semantic: Option<Scene2DSemantic>,
) -> Scene2DNode {
    node(
        id,
        hit_id,
        semantic,
        Scene2DNodeKind::Circle {
            center,
            radius,
            fill,
            stroke: edge,
        },
        None,
    )
}

fn text_node(
    id: &str,
    hit_id: Option<String>,
    origin: ScenePoint,
    content: impl Into<String>,
    size: f32,
    color: Scene2DColor,
    align: Scene2DTextAlign,
    semantic: Option<Scene2DSemantic>,
) -> Scene2DNode {
    node(
        id,
        hit_id,
        semantic,
        Scene2DNodeKind::Text {
            origin,
            content: content.into(),
            size,
            color,
            font: None,
            align,
        },
        None,
    )
}

fn line_node(id: &str, start: ScenePoint, end: ScenePoint, edge: Scene2DStroke) -> Scene2DNode {
    node(
        id,
        None,
        None,
        Scene2DNodeKind::Line {
            start,
            end,
            stroke: edge,
        },
        None,
    )
}

fn semantic(
    role: Scene2DSemanticRole,
    label: impl Into<String>,
    value: impl Into<String>,
    selected: bool,
) -> Scene2DSemantic {
    Scene2DSemantic {
        role,
        label: label.into(),
        description: None,
        value_text: Some(value.into()),
        selected: Some(selected),
        disabled: None,
    }
}

const fn color(r: f32, g: f32, b: f32) -> Scene2DColor {
    Scene2DColor::rgb(r, g, b)
}
const fn color_alpha(r: f32, g: f32, b: f32, a: f32) -> Scene2DColor {
    Scene2DColor::rgba(r, g, b, a)
}
const fn brush(color: Scene2DColor) -> Scene2DBrush {
    Scene2DBrush::Solid { color }
}
const fn stroke(width: f32, color: Scene2DColor) -> Scene2DStroke {
    Scene2DStroke { width, color }
}
const fn transition(duration_ms: u64) -> Scene2DTransition {
    Scene2DTransition {
        duration_ms,
        easing: Scene2DEasing::EaseOutCubic,
        completion_id: None,
        animate_color: false,
        reveal_path: false,
    }
}

fn cell_center(cell: (usize, usize), pad: f32, size: f32, gap: f32) -> (f32, f32) {
    (
        pad + cell.1 as f32 * (size + gap) + size * 0.5,
        pad + cell.0 as f32 * (size + gap) + size * 0.5,
    )
}

fn scene_point((x, y): (f32, f32)) -> ScenePoint {
    ScenePoint::new(x, y)
}

fn inset_rect(rect: SceneRect, inset: f32) -> SceneRect {
    SceneRect::new(
        rect.x + inset,
        rect.y + inset,
        rect.width - inset * 2.0,
        rect.height - inset * 2.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replace_retained_scene(
        surfaces: &[Scene2DState],
        revisions: &mut [u64; 6],
        index: usize,
        mut scene: Scene2DScene,
    ) {
        revisions[index] = revisions[index].saturating_add(1);
        scene.revision = revisions[index];
        surfaces[index]
            .replace_scene(scene)
            .expect("retained scene replacement should validate");
    }

    #[test]
    fn zip_hint_follows_a_valid_checkpoint_ordered_hamiltonian_path() {
        let mut game = ZipGame::default();
        for cell in ZIP_SOLUTION {
            assert!(game.step(cell));
        }
        assert!(game.won);
        assert_eq!(game.path.len(), ZIP_ROWS * ZIP_COLS);
        assert_eq!(
            game.path.iter().copied().collect::<HashSet<_>>().len(),
            ZIP_ROWS * ZIP_COLS
        );
    }

    #[test]
    fn zip_head_uses_a_stable_translated_node_and_rounded_path_joints() {
        let mut game = ZipGame::default();
        assert!(game.step(ZIP_START));
        let first = game.scene(1);
        assert!(game.step((0, 1)));
        let second = game.scene(2);
        let head = second
            .nodes
            .iter()
            .find(|node| node.id == "zip-head")
            .unwrap();
        let first_head = first
            .nodes
            .iter()
            .find(|node| node.id == "zip-head")
            .unwrap();
        assert_eq!(head.transform.translate_y, first_head.transform.translate_y);
        assert!(head.transform.translate_x > first_head.transform.translate_x);
        assert!(head.transition.is_some());
        assert!(
            second
                .nodes
                .iter()
                .any(|node| node.id == "zip-path-joint-1")
        );
    }

    #[test]
    fn queens_hint_satisfies_rows_columns_regions_and_touch_rule() {
        let mut game = QueensGame::default();
        for _ in 0..QUEENS_SIZE {
            assert!(game.hint());
        }
        assert!(game.won);
        assert!(game.is_valid());
        let queens = game.queens();
        assert_eq!(
            queens
                .iter()
                .map(|(row, _)| *row)
                .collect::<HashSet<_>>()
                .len(),
            QUEENS_SIZE
        );
        for first in 0..queens.len() {
            for second in first + 1..queens.len() {
                assert!(
                    queens[first].0.abs_diff(queens[second].0) > 1
                        || queens[first].1.abs_diff(queens[second].1) > 1,
                    "queens may not touch"
                );
            }
        }
        assert_eq!(
            queens
                .iter()
                .map(|(_, col)| *col)
                .collect::<HashSet<_>>()
                .len(),
            QUEENS_SIZE
        );
        assert_eq!(
            queens
                .iter()
                .map(|(row, col)| QueensGame::region(*row, *col))
                .collect::<HashSet<_>>()
                .len(),
            QUEENS_SIZE
        );
    }

    #[test]
    fn queens_regions_are_connected_and_solution_uses_each_region_once() {
        for region in 0..QUEENS_SIZE {
            let cells = (0..QUEENS_SIZE)
                .flat_map(|row| (0..QUEENS_SIZE).map(move |col| (row, col)))
                .filter(|(row, col)| QueensGame::region(*row, *col) == region)
                .collect::<HashSet<_>>();
            assert!(!cells.is_empty(), "region {region} must contain cells");
            assert_eq!(cells.len(), QUEENS_SIZE, "regions have equal area");
            let start = *cells.iter().next().unwrap();
            let mut visited = HashSet::from([start]);
            let mut pending = VecDeque::from([start]);
            while let Some((row, col)) = pending.pop_front() {
                for neighbor in [
                    row.checked_sub(1).map(|next| (next, col)),
                    (row + 1 < QUEENS_SIZE).then_some((row + 1, col)),
                    col.checked_sub(1).map(|next| (row, next)),
                    (col + 1 < QUEENS_SIZE).then_some((row, col + 1)),
                ]
                .into_iter()
                .flatten()
                {
                    if cells.contains(&neighbor) && visited.insert(neighbor) {
                        pending.push_back(neighbor);
                    }
                }
            }
            assert_eq!(visited, cells, "region {region} must be connected");
            assert_eq!(QueensGame::region(region, QUEENS_SOLUTION[region]), region);
        }
    }

    #[test]
    fn repeated_queens_and_sudoku_edits_keep_history_bounded() {
        let mut queens = QueensGame::default();
        for turn in 0..(HISTORY_LIMIT + 32) {
            let row = turn % QUEENS_SIZE;
            let col = (turn / QUEENS_SIZE) % QUEENS_SIZE;
            assert!(queens.place(row, col));
        }
        assert_eq!(queens.undo.len(), HISTORY_LIMIT);
        assert!(queens.reset());
        assert!(queens.undo.is_empty());

        let mut sudoku = SudokuGame::default();
        for turn in 0..(HISTORY_LIMIT + 32) {
            let cell = (0, 2);
            assert!(sudoku.select(cell.0, cell.1) || sudoku.selected == Some(cell));
            assert!(sudoku.enter((turn % 9 + 1) as u8));
        }
        assert_eq!(sudoku.history.len(), HISTORY_LIMIT);
        assert!(sudoku.reset());
        assert!(sudoku.history.is_empty());
    }

    #[test]
    fn sudoku_starts_on_an_editable_cell_with_candidates() {
        let sudoku = SudokuGame::default();
        let (row, col) = sudoku
            .selected
            .expect("the keypad preview needs a selection");
        assert!(!sudoku.givens[row][col]);
        assert_eq!(sudoku.values[row][col], 0);
        assert!(!sudoku.candidates(row, col).is_empty());
    }

    #[test]
    fn accessibility_activation_uses_cells_and_tetris_controls_are_one_shot() {
        use gpui_ui_kit::scene2d::Scene2DGridCell;

        let cell = |row: u32, column: u32| Scene2DGridCell {
            row,
            column,
            index: row * ZIP_COLS as u32 + column,
            id: format!("r{row}c{column}"),
        };
        let mut zip = ZipGame::default();
        assert!(zip.handle(Scene2DInput::Activate {
            id: "zip-cell-0-0".to_owned(),
            hit_id: Some("zip-cell-0-0".to_owned()),
            cell: Some(cell(0, 0)),
            timestamp_ns: 1,
        }));
        assert_eq!(zip.path, vec![(0, 0)]);

        let mut queens = QueensGame::default();
        assert!(queens.handle(Scene2DInput::Activate {
            id: "queens-cell-0-0".to_owned(),
            hit_id: Some("queens-cell-0-0".to_owned()),
            cell: Some(cell(0, 0)),
            timestamp_ns: 2,
        }));
        assert_eq!(queens.marks[0][0], 1);

        let mut sudoku = SudokuGame::default();
        assert!(sudoku.handle(Scene2DInput::Activate {
            id: "sudoku-cell-0-3".to_owned(),
            hit_id: Some("sudoku-cell-0-3".to_owned()),
            cell: Some(Scene2DGridCell {
                row: 0,
                column: 3,
                index: 3,
                id: "r0c3".to_owned(),
            }),
            timestamp_ns: 3,
        }));
        assert_eq!(sudoku.selected, Some((0, 3)));

        let mut tetris = TetrisGame::default();
        assert!(tetris.new_game());
        let original_col = tetris.col;
        assert!(tetris.handle(Scene2DInput::Activate {
            id: "hold-left".to_owned(),
            hit_id: Some("hold-left".to_owned()),
            cell: None,
            timestamp_ns: 4,
        }));
        assert_eq!(tetris.col, original_col - 1);
        assert!(tetris.contacts.is_empty());
        assert!(tetris.keys.is_empty());
    }

    #[test]
    fn sudoku_clues_candidates_undo_and_solution_are_consistent() {
        let mut game = SudokuGame::default();
        assert!(game.givens[0][0]);
        assert!(!game.candidates(0, 2).is_empty());
        assert!(game.select(0, 3));
        assert!(game.enter(SUDOKU_SOLUTION[0][3]));
        assert_eq!(game.values[0][3], 6);
        assert!(game.undo());
        assert_eq!(game.values[0][3], 0);
        for row in 0..9 {
            for col in 0..9 {
                if !game.givens[row][col] {
                    game.selected = Some((row, col));
                    assert!(game.enter(SUDOKU_SOLUTION[row][col]));
                }
            }
        }
        assert!(game.won);
        assert!(game.conflicts().is_empty());
    }

    #[test]
    fn tetris_is_idle_until_started_and_clears_completed_rows() {
        let mut game = TetrisGame::default();
        assert!(!game.needs_ticks());
        assert!(game.new_game());
        assert!(game.needs_ticks());
        game.grid[TETRIS_ROWS - 1] = [Some(PieceKind::O); TETRIS_COLS];
        for col in 3..7 {
            game.grid[TETRIS_ROWS - 1][col] = None;
        }
        game.current = PieceKind::I;
        game.rotation = 0;
        game.row = 17;
        game.col = 3;
        assert!(game.hard_drop());
        assert_eq!(game.lines, 1);
        assert_eq!(
            game.grid[TETRIS_ROWS - 1]
                .iter()
                .filter(|cell| cell.is_some())
                .count(),
            0
        );
    }

    #[test]
    fn tetris_touch_holds_are_independent_and_lifecycle_clears_them() {
        use gpui_ui_kit::scene2d::{
            Scene2DInput, Scene2DLifecycleReason, Scene2DPointerDevice, Scene2DPointerPhase,
        };
        let mut game = TetrisGame::default();
        game.new_game();
        let pointer =
            |phase: Scene2DPointerPhase, contact_id: u64, hit_id: &str| Scene2DInput::Pointer {
                phase,
                device: Scene2DPointerDevice::Touch,
                contact_id,
                timestamp_ns: 0,
                position: ScenePoint::new(0.0, 0.0),
                buttons: Vec::new(),
                modifiers: Vec::new(),
                hit_id: Some(hit_id.to_owned()),
                cell: None,
            };
        assert!(game.handle(pointer(Scene2DPointerPhase::Down, 10, "hold-left")));
        assert!(game.handle(pointer(Scene2DPointerPhase::Down, 11, "hold-down")));
        assert_eq!(game.contacts.len(), 2);
        assert!(game.handle(pointer(Scene2DPointerPhase::Up, 10, "hold-left")));
        assert_eq!(game.contacts.get(&11), Some(&HoldAction::Down));
        assert!(game.handle(Scene2DInput::Lifecycle {
            reason: Scene2DLifecycleReason::FocusLost,
            timestamp_ns: 1,
        }));
        assert!(game.contacts.is_empty());
    }

    #[test]
    fn all_game_scenes_validate_with_accessible_cells_and_focus_input() {
        let zip = ZipGame::default().scene(1);
        let queens = QueensGame::default().scene(1);
        let sudoku = SudokuGame::default().scene(1);
        let tetris = TetrisGame::default();
        for scene in [zip, queens, sudoku, tetris.scene(1)] {
            scene.validate().expect("game scene should be valid");
            assert!(scene.input.keyboard);
            assert!(scene.nodes.iter().any(|node| {
                node.semantic
                    .as_ref()
                    .is_some_and(|semantic| semantic.role == Scene2DSemanticRole::GridCell)
            }));
        }
        assert_eq!(
            tetris
                .controls_scene(1)
                .nodes
                .iter()
                .filter(|node| node.hit_id.is_some())
                .count(),
            4
        );
        assert!(!tetris.preview_scene(1).input.keyboard);
        assert!(!tetris.preview_scene(1).input.pointer);
    }

    #[test]
    fn light_palette_preserves_scene_validity_and_turns_surface_fills_light() {
        let dark = ZipGame::default().scene(1);
        let light = apply_board_palette(dark.clone(), true);
        dark.validate().expect("dark board is valid");
        light.validate().expect("light board is valid");
        assert_eq!(dark.nodes.len(), light.nodes.len());
        assert_ne!(dark.background, light.background);
    }

    /// Opt-in headless controller soak. Set `GPUI_GAMES_SOAK_SECONDS=600` to
    /// run it for ten minutes; it checks retained scene state, not a framebuffer.
    #[test]
    fn native_controller_soak_when_requested() {
        let Ok(raw_seconds) = std::env::var("GPUI_GAMES_SOAK_SECONDS") else {
            return;
        };
        let seconds = raw_seconds
            .parse::<u64>()
            .expect("GPUI_GAMES_SOAK_SECONDS must be an integer");
        if seconds == 0 {
            return;
        }
        assert!(seconds <= 3_600, "soak duration must not exceed one hour");

        let mut zip = ZipGame::default();
        let mut queens = QueensGame::default();
        let mut sudoku = SudokuGame::default();
        let mut tetris = TetrisGame::default();
        let surfaces = vec![
            Scene2DState::new(zip.scene(1)).expect("initial Zip scene"),
            Scene2DState::new(queens.scene(1)).expect("initial Queens scene"),
            Scene2DState::new(sudoku.scene(1)).expect("initial Sudoku scene"),
            Scene2DState::new(tetris.scene(1)).expect("initial Tetris scene"),
            Scene2DState::new(tetris.preview_scene(1)).expect("initial preview scene"),
            Scene2DState::new(tetris.controls_scene(1)).expect("initial controls scene"),
        ];
        // Caps reflect scene composition: Queens uses at most 64 cells, 64
        // marks, and 64 conflict rings; Sudoku can show 51 × 9 candidates.
        let surface_limits = [128, 256, 1_024, 256, 32, 16];
        let mut revisions = [1; 6];
        let deadline = Instant::now() + Duration::from_secs(seconds);
        let mut cycle = 0_u64;
        let mut active_game = GameKind::Zip;
        let mut max_retained_nodes = 0_usize;

        while Instant::now() < deadline {
            let game_turn = cycle / GameKind::ALL.len() as u64;
            match active_game {
                GameKind::Zip => {
                    if game_turn % 96 == 0 {
                        zip.reset();
                    }
                    if zip.path.is_empty() {
                        zip.step(ZIP_START);
                    } else if game_turn % 7 == 0 {
                        zip.undo();
                    } else {
                        zip.hint();
                    }
                    replace_retained_scene(&surfaces, &mut revisions, 0, zip.scene(0));
                }
                GameKind::Queens => {
                    if game_turn % 512 == 0 {
                        queens.reset();
                    }
                    let row = game_turn as usize % QUEENS_SIZE;
                    let col = game_turn as usize / QUEENS_SIZE % QUEENS_SIZE;
                    queens.place(row, col);
                    if game_turn % 9 == 0 {
                        queens.undo();
                    }
                    replace_retained_scene(&surfaces, &mut revisions, 1, queens.scene(0));
                }
                GameKind::Sudoku => {
                    if game_turn % 512 == 0 {
                        sudoku.reset();
                    }
                    sudoku.select(0, 2);
                    sudoku.enter((game_turn % 9 + 1) as u8);
                    if game_turn % 11 == 0 {
                        sudoku.undo();
                    }
                    replace_retained_scene(&surfaces, &mut revisions, 2, sudoku.scene(0));
                }
                GameKind::Tetris => {
                    if game_turn % 128 == 0 || !tetris.running || tetris.over {
                        tetris.new_game();
                    }
                    if game_turn % 8 == 0 {
                        tetris.rotate();
                    } else if game_turn % 3 == 0 {
                        tetris.move_piece(0, -1);
                    } else {
                        tetris.move_piece(0, 1);
                    }
                    let contact_id = cycle;
                    tetris.handle(Scene2DInput::Pointer {
                        phase: gpui_ui_kit::scene2d::Scene2DPointerPhase::Down,
                        device: gpui_ui_kit::scene2d::Scene2DPointerDevice::Touch,
                        contact_id,
                        timestamp_ns: cycle,
                        position: ScenePoint::new(0.0, 0.0),
                        buttons: Vec::new(),
                        modifiers: Vec::new(),
                        hit_id: Some("hold-down".to_owned()),
                        cell: None,
                    });
                    assert!(tetris.contacts.len() <= 2);
                    tetris.advance(TICK_INTERVAL);
                    replace_retained_scene(&surfaces, &mut revisions, 3, tetris.scene(0));
                    replace_retained_scene(&surfaces, &mut revisions, 4, tetris.preview_scene(0));
                    replace_retained_scene(&surfaces, &mut revisions, 5, tetris.controls_scene(0));
                    tetris.handle(Scene2DInput::Lifecycle {
                        reason: gpui_ui_kit::scene2d::Scene2DLifecycleReason::SectionChanged,
                        timestamp_ns: cycle,
                    });
                    assert!(tetris.contacts.is_empty());
                    assert!(tetris.keys.is_empty());
                }
            }

            assert!(zip.undo.len() <= HISTORY_LIMIT);
            assert!(queens.undo.len() <= HISTORY_LIMIT);
            assert!(sudoku.history.len() <= HISTORY_LIMIT);
            assert_eq!(surfaces.len(), 6, "surface cache must stay bounded");
            for (index, surface) in surfaces.iter().enumerate() {
                let scene = surface.scene();
                assert_eq!(scene.revision, revisions[index]);
                assert!(
                    scene.nodes.len() <= surface_limits[index],
                    "surface {index} retains {} nodes; limit is {}",
                    scene.nodes.len(),
                    surface_limits[index]
                );
                let ids = scene
                    .nodes
                    .iter()
                    .map(|node| &node.id)
                    .collect::<HashSet<_>>();
                assert_eq!(ids.len(), scene.nodes.len(), "node IDs remain unique");
                max_retained_nodes = max_retained_nodes.max(scene.nodes.len());
            }

            let previous_game = active_game;
            active_game = match active_game {
                GameKind::Zip => GameKind::Queens,
                GameKind::Queens => GameKind::Sudoku,
                GameKind::Sudoku => GameKind::Tetris,
                GameKind::Tetris => GameKind::Zip,
            };
            if previous_game == GameKind::Tetris && active_game != GameKind::Tetris {
                tetris.release_all();
                assert!(tetris.contacts.is_empty());
            }
            cycle = cycle.saturating_add(1);
            std::thread::sleep(Duration::from_millis(50));
        }

        eprintln!(
            "headless native controller soak: {seconds}s, {cycle} game-switch cycles, 6 retained surfaces, max {max_retained_nodes} nodes"
        );
    }
}
