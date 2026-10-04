//! `Scene2D` surface API and normalized input tests.

// Rust guideline compliant 2026-02-21

use gpui_ui_kit::scene2d::{
    GameSurface, Scene2DInput, Scene2DInputConfig, Scene2DScene, Scene2DState,
    Scene2DViewTransform, ScenePoint, SceneRect,
};

#[test]
fn test_game_surface_accepts_retained_state_and_input_handler() {
    let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 80.0, 60.0));
    scene.input = Scene2DInputConfig {
        pointer: true,
        continuous: true,
        capture: true,
        keyboard: true,
    };
    let state = Scene2DState::new(scene).unwrap();
    assert!(state.wants_pointer_capture());

    let surface = GameSurface::from_state("scene2d-test", state)
        .aria_label("Test drawing surface")
        .on_input(|event: Scene2DInput, _window, _cx| {
            let _ = event;
        });
    assert!(format!("{surface:?}").contains("GameSurface"));
}

#[test]
fn test_game_surface_can_be_created_from_a_scene_snapshot() {
    let scene = Scene2DScene::new(SceneRect::new(2.0, 4.0, 80.0, 60.0));
    let surface = GameSurface::new("scene2d-snapshot", scene).unwrap();
    assert!(format!("{surface:?}").contains("scene2d-snapshot"));
}

#[test]
fn test_contain_transform_round_trips_with_letterboxing() {
    let transform =
        Scene2DViewTransform::contain(SceneRect::new(0.0, 0.0, 80.0, 40.0), 160.0, 160.0).unwrap();
    let scene_point = ScenePoint::new(60.0, 20.0);
    assert_eq!(
        transform.viewport_to_scene(transform.scene_to_viewport(scene_point)),
        Some(scene_point)
    );
    assert_eq!(
        transform.content_bounds(),
        SceneRect::new(0.0, 40.0, 160.0, 80.0)
    );
}
