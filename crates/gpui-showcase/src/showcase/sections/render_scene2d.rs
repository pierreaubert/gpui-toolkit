// Rust guideline compliant 2026-02-21

use super::prelude::*;
use gpui_ui_kit::scene2d::{
    GameSurface, Scene2DBrush, Scene2DColor, Scene2DEasing, Scene2DGrid, Scene2DInput, Scene2DNode,
    Scene2DNodeKind, Scene2DPathCommand, Scene2DPointerPhase, Scene2DScene, Scene2DSemantic,
    Scene2DSemanticRole, Scene2DStroke, Scene2DTextAlign, Scene2DTransform, Scene2DTransition,
    ScenePoint, SceneRect,
};

const BOARD_SIZE: u32 = 5;
const BOARD_STEP: f32 = 45.0;
const CELL_SIZE: f32 = 40.0;

impl Showcase {
    pub(crate) fn render_scene2d_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let title = cx.t(TranslationKey::SectionScene2d);
        let handle = self.weak_entity_handle();
        let state = self.scene2d_state.clone();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(title))
            .child(
                Text::new(
                    "A retained native drawing surface with shaped text, paths, grid picking, direct pointer routing, and keyboard focus.",
                )
                .size(TextSize::Sm),
            )
            .child(
                Text::new("Click a cell or use the arrow keys. The marker eases to its new cell.")
                    .size(TextSize::Xs)
                    .muted(true),
            )
            .child(
                div()
                    .w_full()
                    .h(px(430.0))
                    .rounded_xl()
                    .overflow_hidden()
                    .child(
                        GameSurface::from_state("scene2d-demo", state.clone())
                            .aria_label("Scene2D interactive grid")
                            .on_input(move |event, _window, cx| {
                                let Some(selection) = next_selection(event, &state.scene(), None)
                                else {
                                    return;
                                };
                                handle.update(cx, |showcase, cx| {
                                    showcase.set_scene2d_selection(selection, cx);
                                });
                            }),
                    ),
            )
    }

    fn set_scene2d_selection(&mut self, selected: usize, cx: &mut Context<Self>) {
        if selected >= (BOARD_SIZE * BOARD_SIZE) as usize || selected == self.scene2d_selected {
            return;
        }
        let mut scene = demo_scene(selected);
        scene.revision = self.scene2d_state.scene().revision.saturating_add(1);
        if self.scene2d_state.replace_scene(scene).is_ok() {
            self.scene2d_selected = selected;
            self.notify_content(cx);
        }
    }
}

pub(crate) fn demo_scene(selected: usize) -> Scene2DScene {
    let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 255.0, 255.0));
    scene.semantic = Some(Scene2DSemantic {
        role: Scene2DSemanticRole::Grid,
        label: "Interactive five by five grid".to_owned(),
        description: Some("Use the arrow keys or select a cell.".to_owned()),
        value_text: None,
        selected: None,
        disabled: None,
    });
    scene.background = Some(Scene2DBrush::LinearGradient {
        angle_degrees: 135.0,
        from: Scene2DColor::rgb(0.035, 0.07, 0.12),
        to: Scene2DColor::rgb(0.08, 0.16, 0.21),
    });
    scene.grid = Some(Scene2DGrid {
        rows: BOARD_SIZE,
        columns: BOARD_SIZE,
        x: 15.0,
        y: 15.0,
        cell_width: CELL_SIZE,
        cell_height: CELL_SIZE,
        gap: BOARD_STEP - CELL_SIZE,
        row_labels: (1..=BOARD_SIZE).map(|row| row.to_string()).collect(),
        column_labels: (1..=BOARD_SIZE).map(|column| column.to_string()).collect(),
    });
    for row in 0..BOARD_SIZE {
        for column in 0..BOARD_SIZE {
            let index = (row * BOARD_SIZE + column) as usize;
            let is_selected = index == selected;
            let x = 15.0 + column as f32 * BOARD_STEP;
            let y = 15.0 + row as f32 * BOARD_STEP;
            scene.nodes.push(Scene2DNode {
                id: format!("cell-{row}-{column}"),
                hit_id: Some(format!("cell-{row}-{column}")),
                hit_bounds: None,
                clip: None,
                shadow: None,
                semantic: Some(Scene2DSemantic {
                    role: Scene2DSemanticRole::GridCell,
                    label: format!("Row {}, column {}", row + 1, column + 1),
                    description: Some("Selectable board cell".to_owned()),
                    value_text: Some(if is_selected {
                        "selected".to_owned()
                    } else {
                        "not selected".to_owned()
                    }),
                    selected: Some(is_selected),
                    disabled: None,
                }),
                transform: Scene2DTransform::identity(),
                opacity: 1.0,
                transition: None,
                kind: Scene2DNodeKind::RoundedRect {
                    rect: SceneRect::new(x, y, CELL_SIZE, CELL_SIZE),
                    radius: 8.0,
                    fill: Some(Scene2DBrush::solid(if is_selected {
                        Scene2DColor::rgb(0.16, 0.48, 0.61)
                    } else {
                        Scene2DColor::rgb(0.11, 0.2, 0.26)
                    })),
                    stroke: Some(Scene2DStroke {
                        width: 1.0,
                        color: Scene2DColor::rgba(0.49, 0.74, 0.78, 0.62),
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
                transform: Scene2DTransform::identity(),
                opacity: 1.0,
                transition: None,
                kind: Scene2DNodeKind::Text {
                    origin: ScenePoint::new(x + CELL_SIZE * 0.42, y + CELL_SIZE * 0.34),
                    content: ((index + 1) % 10).to_string(),
                    size: 15.0,
                    color: Scene2DColor::rgb(0.85, 0.94, 0.94),
                    font: None,
                    align: Scene2DTextAlign::Center,
                },
            });
        }
    }

    scene.nodes.push(Scene2DNode {
        id: "selection-marker".to_owned(),
        hit_id: None,
        hit_bounds: None,
        clip: None,
        shadow: None,
        semantic: None,
        transform: Scene2DTransform {
            translate_x: 15.0
                + (selected as u32 % BOARD_SIZE) as f32 * BOARD_STEP
                + CELL_SIZE * 0.5,
            translate_y: 15.0
                + (selected as u32 / BOARD_SIZE) as f32 * BOARD_STEP
                + CELL_SIZE * 0.5,
            ..Scene2DTransform::identity()
        },
        opacity: 1.0,
        transition: Some(Scene2DTransition {
            duration_ms: 180,
            easing: Scene2DEasing::EaseOutCubic,
            completion_id: None,
            animate_color: false,
            reveal_path: false,
        }),
        kind: Scene2DNodeKind::Circle {
            center: ScenePoint::new(0.0, 0.0),
            radius: 4.0,
            fill: Some(Scene2DBrush::solid(Scene2DColor::rgb(0.97, 0.77, 0.31))),
            stroke: Some(Scene2DStroke {
                width: 1.0,
                color: Scene2DColor::rgb(1.0, 0.9, 0.56),
            }),
        },
    });
    scene.nodes.push(Scene2DNode {
        id: "route-example".to_owned(),
        hit_id: None,
        hit_bounds: None,
        clip: None,
        shadow: None,
        semantic: None,
        transform: Scene2DTransform::identity(),
        opacity: 0.42,
        transition: None,
        kind: Scene2DNodeKind::Path {
            commands: vec![
                Scene2DPathCommand::MoveTo {
                    point: ScenePoint::new(30.0, 205.0),
                },
                Scene2DPathCommand::CubicTo {
                    control_a: ScenePoint::new(75.0, 155.0),
                    control_b: ScenePoint::new(130.0, 245.0),
                    point: ScenePoint::new(205.0, 185.0),
                },
            ],
            fill: None,
            stroke: Some(Scene2DStroke {
                width: 2.0,
                color: Scene2DColor::rgb(0.28, 0.81, 0.75),
            }),
        },
    });
    scene.input.pointer = true;
    scene.input.continuous = true;
    scene.input.capture = true;
    scene.input.keyboard = true;
    scene
}

fn next_selection(
    event: Scene2DInput,
    scene: &Scene2DScene,
    current: Option<usize>,
) -> Option<usize> {
    match event {
        Scene2DInput::Pointer {
            phase: Scene2DPointerPhase::Up,
            cell: Some(cell),
            ..
        } => Some(cell.index as usize),
        Scene2DInput::Key {
            phase: gpui_ui_kit::scene2d::Scene2DKeyPhase::Down,
            key,
            ..
        } => {
            let current = current.unwrap_or_else(|| {
                scene
                    .nodes
                    .iter()
                    .find(|node| node.id == "selection-marker")
                    .map_or(0, |node| {
                        let column = ((node.transform.translate_x - 35.0) / BOARD_STEP).round();
                        let row = ((node.transform.translate_y - 35.0) / BOARD_STEP).round();
                        row.max(0.0) as usize * BOARD_SIZE as usize + column.max(0.0) as usize
                    })
            });
            let mut row = current / BOARD_SIZE as usize;
            let mut column = current % BOARD_SIZE as usize;
            match key.as_str() {
                "ArrowLeft" => column = column.saturating_sub(1),
                "ArrowRight" => column = (column + 1).min(BOARD_SIZE as usize - 1),
                "ArrowUp" => row = row.saturating_sub(1),
                "ArrowDown" => row = (row + 1).min(BOARD_SIZE as usize - 1),
                _ => return None,
            }
            Some(row * BOARD_SIZE as usize + column)
        }
        _ => None,
    }
}
