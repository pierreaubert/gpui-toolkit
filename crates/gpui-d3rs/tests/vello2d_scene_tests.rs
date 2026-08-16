use d3rs::vello2d::kurbo::{Circle, Rect, Shape, Stroke};
use d3rs::vello2d::peniko::{Brush, Color, Fill};
use d3rs::vello2d::to_vello_scene;
use d3rs::vello2d::{ChartCmd, ChartScene};

fn red() -> Brush {
    Brush::Solid(Color::from_rgb8(255, 0, 0))
}

#[test]
fn new_scene_is_empty() {
    let scene = ChartScene::new();
    assert!(scene.is_empty());
    assert_eq!(scene.len(), 0);
}

#[test]
fn fill_circle_appends_fill_cmd_with_circle_path() {
    let mut scene = ChartScene::new();
    scene.fill_circle(10.0, 20.0, 3.0, red());
    assert_eq!(scene.len(), 1);
    let ChartCmd::Fill { path, fill, .. } = &scene.commands()[0] else {
        panic!("expected Fill command");
    };
    assert_eq!(*fill, Fill::NonZero);
    // Circle::to_path emits move_to + 4 cubic segments + close = 6 elements.
    assert_eq!(path.elements().len(), 6);
}

#[test]
fn fill_rect_uses_rect_shape() {
    let mut scene = ChartScene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 5.0, 7.0), red());
    let ChartCmd::Fill { path, .. } = &scene.commands()[0] else {
        panic!("expected Fill command");
    };
    let els = path.elements();
    assert_eq!(els.len(), 5); // M + 3 L + close
    assert_eq!(els[0], kurbo::PathEl::MoveTo((0.0, 0.0).into()));
}

#[test]
fn stroke_polyline_builds_single_open_path() {
    let mut scene = ChartScene::new();
    scene.stroke_polyline(&[(0.0, 0.0), (1.0, 2.0), (3.0, 4.0)], Stroke::new(2.0), red());
    let ChartCmd::Stroke { path, stroke, .. } = &scene.commands()[0] else {
        panic!("expected Stroke command");
    };
    assert_eq!(stroke.width, 2.0);
    let els = path.elements();
    assert_eq!(els.len(), 3); // M + 2 L, NOT closed
    assert!(!matches!(els.last(), Some(kurbo::PathEl::ClosePath)));
}

#[test]
fn stroke_polyline_with_fewer_than_two_points_is_noop() {
    let mut scene = ChartScene::new();
    scene.stroke_polyline(&[(1.0, 1.0)], Stroke::new(1.0), red());
    assert!(scene.is_empty());
}

#[test]
fn circle_helper_matches_kurbo_circle_path() {
    let mut a = ChartScene::new();
    a.fill_circle(5.0, 5.0, 2.0, red());
    let ChartCmd::Fill { path, .. } = &a.commands()[0] else {
        panic!("expected Fill command");
    };
    let expected = Circle::new((5.0, 5.0), 2.0).to_path(0.1);
    assert_eq!(path.elements(), expected.elements());
}

#[test]
fn gpu_replay_emits_one_draw_per_command() {
    let mut scene = ChartScene::new();
    scene.fill_circle(10.0, 10.0, 2.0, red());
    scene.stroke_polyline(&[(0.0, 0.0), (5.0, 5.0)], Stroke::new(1.0), red());
    let vello_scene = to_vello_scene(&scene);
    let encoding = vello_scene.encoding();
    assert_eq!(encoding.draw_tags.len(), 2);
    assert!(!encoding.path_data.is_empty());
}

#[test]
fn gpu_replay_of_empty_scene_has_no_draws() {
    let vello_scene = to_vello_scene(&ChartScene::new());
    assert_eq!(vello_scene.encoding().draw_tags.len(), 0);
}
