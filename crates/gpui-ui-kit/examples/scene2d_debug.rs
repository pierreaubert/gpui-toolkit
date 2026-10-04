//! Native retained `Scene2D` drawing and input example.

// Rust guideline compliant 2026-02-21

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::ThemeExt;
use gpui_ui_kit::scene2d::{
    GameSurface, Scene2DBrush, Scene2DColor, Scene2DGrid, Scene2DInput, Scene2DNode,
    Scene2DNodeKind, Scene2DPathCommand, Scene2DScene, Scene2DSemantic, Scene2DSemanticRole,
    Scene2DState, Scene2DStroke, Scene2DTextAlign, ScenePoint, SceneRect,
};

struct Scene2DDebug {
    state: Scene2DState,
}

impl Scene2DDebug {
    fn new() -> Self {
        let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 320.0, 240.0));
        scene.semantic = Some(Scene2DSemantic {
            role: Scene2DSemanticRole::Grid,
            label: "Scene2D example board".to_owned(),
            description: Some("Click the cell or focus the board and press a key.".to_owned()),
            value_text: None,
            selected: None,
            disabled: None,
        });
        scene.background = Some(Scene2DBrush::LinearGradient {
            angle_degrees: 135.0,
            from: Scene2DColor::rgb(0.04, 0.09, 0.15),
            to: Scene2DColor::rgb(0.16, 0.25, 0.31),
        });
        scene.grid = Some(Scene2DGrid {
            rows: 3,
            columns: 3,
            x: 70.0,
            y: 45.0,
            cell_width: 52.0,
            cell_height: 52.0,
            gap: 6.0,
            row_labels: vec!["1".into(), "2".into(), "3".into()],
            column_labels: vec!["A".into(), "B".into(), "C".into()],
        });
        for row in 0..3 {
            for column in 0..3 {
                let x = 70.0 + column as f32 * 58.0;
                let y = 45.0 + row as f32 * 58.0;
                scene.nodes.push(Scene2DNode {
                    id: format!("cell-{row}-{column}"),
                    hit_id: Some(format!("cell-{row}-{column}")),
                    hit_bounds: None,
                    clip: None,
                    shadow: None,
                    semantic: Some(Scene2DSemantic {
                        role: Scene2DSemanticRole::GridCell,
                        label: format!("Row {}, column {}", row + 1, column + 1),
                        description: None,
                        value_text: Some("open".to_owned()),
                        selected: Some(row == 1 && column == 1),
                        disabled: None,
                    }),
                    transform: Default::default(),
                    opacity: 1.0,
                    transition: None,
                    kind: Scene2DNodeKind::RoundedRect {
                        rect: SceneRect::new(x, y, 52.0, 52.0),
                        radius: 7.0,
                        fill: Some(Scene2DBrush::solid(if row == 1 && column == 1 {
                            Scene2DColor::rgb(0.18, 0.48, 0.62)
                        } else {
                            Scene2DColor::rgb(0.11, 0.22, 0.3)
                        })),
                        stroke: Some(Scene2DStroke {
                            width: 1.0,
                            color: Scene2DColor::rgba(0.7, 0.86, 0.91, 0.7),
                        }),
                    },
                });
                scene.nodes.push(Scene2DNode {
                    id: format!("label-{row}-{column}"),
                    hit_id: None,
                    hit_bounds: None,
                    clip: None,
                    shadow: None,
                    semantic: None,
                    transform: Default::default(),
                    opacity: 1.0,
                    transition: None,
                    kind: Scene2DNodeKind::Text {
                        origin: ScenePoint::new(x + 21.0, y + 17.0),
                        content: char::from_digit(row * 3 + column + 1, 10)
                            .expect("board labels are digits")
                            .to_string(),
                        size: 16.0,
                        color: Scene2DColor::rgb(0.92, 0.97, 0.97),
                        font: None,
                        align: Scene2DTextAlign::Center,
                    },
                });
            }
        }
        scene.nodes.push(Scene2DNode {
            id: "curve".to_owned(),
            hit_id: None,
            hit_bounds: None,
            clip: None,
            shadow: None,
            semantic: None,
            transform: Default::default(),
            opacity: 0.7,
            transition: None,
            kind: Scene2DNodeKind::Path {
                commands: vec![
                    Scene2DPathCommand::MoveTo {
                        point: ScenePoint::new(45.0, 215.0),
                    },
                    Scene2DPathCommand::CubicTo {
                        control_a: ScenePoint::new(110.0, 180.0),
                        control_b: ScenePoint::new(190.0, 240.0),
                        point: ScenePoint::new(275.0, 205.0),
                    },
                ],
                fill: None,
                stroke: Some(Scene2DStroke {
                    width: 2.0,
                    color: Scene2DColor::rgb(0.34, 0.82, 0.72),
                }),
            },
        });
        scene.input.pointer = true;
        scene.input.continuous = true;
        scene.input.capture = true;
        scene.input.keyboard = true;
        Self {
            state: Scene2DState::new(scene).expect("debug scene is valid"),
        }
    }
}

impl Render for Scene2DDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let state = self.state.clone();
        div()
            .id("scene2d-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_4()
            .child("Click a cell, drag across the board, or press a key after focusing it.")
            .child(div().w_full().h(px(500.0)).child(
                GameSurface::from_state("scene2d-debug", state.clone()).on_input(
                    move |event, _window, _cx| {
                        if let Scene2DInput::Pointer { hit_id, cell, .. } = event {
                            println!("Scene2D pointer hit={hit_id:?}, cell={cell:?}");
                        }
                    },
                ),
            ))
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Scene2D Debug")
            .size(700.0, 620.0)
            .scrollable(false)
            .with_theme(true),
        |cx| cx.new(|_cx| Scene2DDebug::new()),
    );
}
