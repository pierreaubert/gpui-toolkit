//! Integration tests for the native retained `Scene2D` painter.

// Rust guideline compliant 2026-02-21

use gpui::{
    Context, InteractiveElement, IntoElement, KeyBinding, KeyUpEvent, Keystroke, ParentElement,
    PointerDevice, PointerEvent, PointerPhase, Render, StatefulInteractiveElement, Styled,
    TestAppContext, Window, div, px,
};
use gpui_ui_kit::scene2d::{
    GameSurface, Scene2DBrush, Scene2DColor, Scene2DInput, Scene2DKeyPhase, Scene2DNode,
    Scene2DNodeKind, Scene2DPathCommand, Scene2DScene, Scene2DSemantic, Scene2DSemanticRole,
    Scene2DState, Scene2DStroke, Scene2DTextAlign, ScenePoint, SceneRect,
};
use std::cell::RefCell;
use std::rc::Rc;

gpui::actions!(scene2d_test, [NextTab, PreviousTab, GlobalShortcut]);

fn drawing_scene() -> Scene2DScene {
    let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 100.0, 80.0));
    scene.nodes.push(Scene2DNode {
        id: "native-rect".to_owned(),
        hit_id: Some("board-cell".to_owned()),
        hit_bounds: None,
        clip: None,
        shadow: None,
        semantic: Some(Scene2DSemantic {
            role: Scene2DSemanticRole::GridCell,
            label: "First cell".to_owned(),
            description: None,
            value_text: None,
            selected: Some(false),
            disabled: None,
        }),
        transform: Default::default(),
        opacity: 1.0,
        transition: None,
        kind: Scene2DNodeKind::RoundedRect {
            rect: SceneRect::new(5.0, 5.0, 40.0, 40.0),
            radius: 6.0,
            fill: Some(Scene2DBrush::solid(Scene2DColor::rgb(0.1, 0.4, 0.6))),
            stroke: Some(Scene2DStroke {
                width: 1.0,
                color: Scene2DColor::rgb(0.8, 0.9, 1.0),
            }),
        },
    });
    scene.nodes.push(Scene2DNode {
        id: "native-path".to_owned(),
        hit_id: None,
        hit_bounds: None,
        clip: None,
        shadow: None,
        semantic: None,
        transform: Default::default(),
        opacity: 0.8,
        transition: None,
        kind: Scene2DNodeKind::Path {
            commands: vec![
                Scene2DPathCommand::MoveTo {
                    point: ScenePoint::new(50.0, 20.0),
                },
                Scene2DPathCommand::QuadraticTo {
                    control: ScenePoint::new(70.0, 0.0),
                    point: ScenePoint::new(90.0, 20.0),
                },
            ],
            fill: None,
            stroke: Some(Scene2DStroke {
                width: 2.0,
                color: Scene2DColor::rgb(0.9, 0.8, 0.2),
            }),
        },
    });
    scene.nodes.push(Scene2DNode {
        id: "native-text".to_owned(),
        hit_id: None,
        hit_bounds: None,
        clip: None,
        shadow: None,
        semantic: None,
        transform: Default::default(),
        opacity: 1.0,
        transition: None,
        kind: Scene2DNodeKind::Text {
            origin: ScenePoint::new(8.0, 60.0),
            content: "Shaped text".to_owned(),
            size: 12.0,
            color: Scene2DColor::rgb(1.0, 1.0, 1.0),
            font: None,
            align: Scene2DTextAlign::Left,
        },
    });
    scene
}

struct Scene2DTestView;

impl Render for Scene2DTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(GameSurface::new("native-scene2d", drawing_scene()).unwrap())
    }
}

#[gpui::test]
async fn scene2d_renders_native_shapes_paths_and_shaped_text(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| Scene2DTestView);
}

struct Scene2DInputTestView {
    state: Scene2DState,
    events: Rc<RefCell<Vec<Scene2DInput>>>,
    shortcut_count: Rc<std::cell::Cell<usize>>,
    tab_action_count: Rc<std::cell::Cell<usize>>,
    neighbor_focus: gpui::FocusHandle,
}

impl Render for Scene2DInputTestView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.clone();
        let events = self.events.clone();
        let shortcut_count = self.shortcut_count.clone();
        let tab_action_count = self.tab_action_count.clone();
        let next_tab_count = tab_action_count.clone();
        let previous_tab_count = tab_action_count.clone();
        div()
            .size_full()
            .key_context("scene2d_test")
            .on_action(cx.listener(move |_, _: &NextTab, window, cx| {
                next_tab_count.set(next_tab_count.get() + 1);
                window.focus_next(cx);
            }))
            .on_action(cx.listener(move |_, _: &PreviousTab, window, cx| {
                previous_tab_count.set(previous_tab_count.get() + 1);
                window.focus_prev(cx);
            }))
            .on_action(cx.listener(move |_, _: &GlobalShortcut, _, _| {
                shortcut_count.set(shortcut_count.get() + 1);
            }))
            .child(
                div().w_full().h(px(320.0)).child(
                    GameSurface::from_state("scene2d-input-test", state)
                        .on_input(move |event, _window, _cx| events.borrow_mut().push(event)),
                ),
            )
            .child(
                div()
                    .id("focus-neighbor")
                    .h(px(32.0))
                    .track_focus(&self.neighbor_focus)
                    .focusable(),
            )
    }
}

#[gpui::test]
async fn focused_keys_survive_scene_updates_and_tab_moves_to_the_next_control(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        cx.bind_keys(vec![
            KeyBinding::new("tab", NextTab, Some("scene2d_test")),
            KeyBinding::new("shift-tab", PreviousTab, Some("scene2d_test")),
            KeyBinding::new("cmd-a", GlobalShortcut, Some("scene2d_test")),
        ]);
    });
    let events = Rc::new(RefCell::new(Vec::new()));
    let events_for_view = events.clone();
    let shortcut_count = Rc::new(std::cell::Cell::new(0));
    let shortcut_count_for_view = shortcut_count.clone();
    let tab_action_count = Rc::new(std::cell::Cell::new(0));
    let tab_action_count_for_view = tab_action_count.clone();
    let (view, cx) = cx.add_window_view(|_window, cx| {
        let neighbor_focus = cx.focus_handle().tab_stop(true);
        Scene2DInputTestView {
            state: Scene2DState::new(drawing_scene()).unwrap(),
            events: events_for_view,
            shortcut_count: shortcut_count_for_view,
            tab_action_count: tab_action_count_for_view,
            neighbor_focus,
        }
    });
    cx.run_until_parked();
    let surface_position = cx.update(|window, _| {
        let bounds = window.bounds();
        gpui::point(
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + px(160.0),
        )
    });
    let letterbox_position = cx.update(|window, _| {
        let bounds = window.bounds();
        assert!(
            bounds.size.width > px(400.0),
            "test window should leave horizontal letterboxing"
        );
        gpui::point(bounds.origin.x + px(2.0), bounds.origin.y + px(160.0))
    });

    cx.simulate_event(PointerEvent {
        phase: PointerPhase::Down,
        device: PointerDevice::Touch,
        pointer_id: 16,
        timestamp_ns: 90,
        position: letterbox_position,
        pressure: Some(0.5),
        buttons: Vec::new(),
        modifiers: Default::default(),
    });
    cx.simulate_event(PointerEvent {
        phase: PointerPhase::Up,
        device: PointerDevice::Touch,
        pointer_id: 16,
        timestamp_ns: 91,
        position: letterbox_position,
        pressure: None,
        buttons: Vec::new(),
        modifiers: Default::default(),
    });
    cx.simulate_keystrokes("up");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("up").unwrap(),
    });
    assert!(
        events.borrow().is_empty(),
        "letterbox input must not claim or focus the scene"
    );

    cx.simulate_event(PointerEvent {
        phase: PointerPhase::Down,
        device: PointerDevice::Touch,
        pointer_id: 17,
        timestamp_ns: 100,
        position: surface_position,
        pressure: Some(0.7),
        buttons: Vec::new(),
        modifiers: Default::default(),
    });
    cx.simulate_event(PointerEvent {
        phase: PointerPhase::Up,
        device: PointerDevice::Touch,
        pointer_id: 17,
        timestamp_ns: 101,
        position: surface_position,
        pressure: None,
        buttons: Vec::new(),
        modifiers: Default::default(),
    });
    view.update(cx, |view, cx| {
        let mut updated = view.state.scene();
        updated.revision += 1;
        view.state.replace_scene(updated).unwrap();
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("left");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("left").unwrap(),
    });

    let routed = events.borrow().clone();
    assert!(routed.iter().any(|event| matches!(
        event,
        Scene2DInput::Pointer {
            phase: gpui_ui_kit::scene2d::Scene2DPointerPhase::Down,
            contact_id: 17,
            ..
        }
    )));
    assert!(routed.iter().any(|event| matches!(
        event,
        Scene2DInput::Key { phase: Scene2DKeyPhase::Down, key, .. } if key == "ArrowLeft"
    )));
    assert!(routed.iter().any(|event| matches!(
        event,
        Scene2DInput::Key { phase: Scene2DKeyPhase::Up, key, .. } if key == "ArrowLeft"
    )));

    cx.simulate_keystrokes("tab");
    let neighbor = view.read_with(cx, |view, _| view.neighbor_focus.clone());
    assert_eq!(
        tab_action_count.get(),
        1,
        "Tab should reach the app key binding"
    );
    assert!(cx.update(|window, _| neighbor.is_focused(window)));

    cx.simulate_keystrokes("shift-tab");
    cx.simulate_keystrokes("right");
    assert!(!cx.update(|window, _| neighbor.is_focused(window)));
    cx.simulate_keystrokes("cmd-a");

    let routed = events.borrow().clone();
    assert!(routed.iter().any(|event| matches!(
        event,
        Scene2DInput::Key { phase: Scene2DKeyPhase::Down, key, .. } if key == "ArrowRight"
    )));
    assert!(!routed.iter().any(|event| matches!(
        event,
        Scene2DInput::Key { key, .. } if key == "a"
    )));
    assert!(
        shortcut_count.get() > 0,
        "Command+A should reach app-level handlers"
    );
}
