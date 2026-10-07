//! Native GPUI painting for retained `Scene2D` scenes.

// Rust guideline compliant 2026-02-21

use super::animation::Scene2DState;
use super::geometry::Scene2DViewTransform;
use super::input::{
    Scene2DButton, Scene2DInput, Scene2DKeyPhase, Scene2DModifier, Scene2DPointerDevice,
    Scene2DPointerEvent, Scene2DPointerPhase,
};
use super::types::{
    Scene2DBrush, Scene2DColor, Scene2DNode, Scene2DNodeKind, Scene2DSemantic, Scene2DSemanticRole,
    Scene2DStroke, Scene2DTextAlign, Scene2DTransform, Scene2DValidationError, ScenePoint,
    SceneRect,
};
use crate::ThemeExt;
use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState, apply_native_accessibility,
};
use gpui::{
    App, Bounds, BoxShadow, ContentMask, Corners, ElementId, InteractiveElement, IntoElement,
    KeyDownEvent, KeyUpEvent, MouseButton, MouseMoveEvent, MouseUpEvent, ParentElement,
    PathBuilder, PathStyle, Pixels, Point, PointerDevice, PointerEvent, RenderOnce, Rgba,
    SharedString, StatefulInteractiveElement, StrokeOptions, Styled, TextAlign, Window, canvas,
    div, fill, linear_color_stop, linear_gradient, point, px, quad, size,
};
use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

/// A retained native drawing surface with focused keyboard and pointer input.
pub struct GameSurface {
    id: ElementId,
    state: Scene2DState,
    on_input: Option<Rc<dyn Fn(Scene2DInput, &Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl std::fmt::Debug for GameSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameSurface")
            .field("id", &self.id)
            .field("state", &self.state)
            .field("aria_label", &self.aria_label)
            .field("aria_role", &self.aria_role)
            .finish_non_exhaustive()
    }
}

impl GameSurface {
    /// Creates a surface from an already validated scene snapshot.
    ///
    /// # Errors
    ///
    /// Returns a validation error when the scene snapshot is malformed.
    pub fn new(
        id: impl Into<SharedString>,
        scene: super::types::Scene2DScene,
    ) -> Result<Self, Scene2DValidationError> {
        Ok(Self::from_state(id, Scene2DState::new(scene)?))
    }

    /// Creates a surface backed by retained state for updates and animation.
    pub fn from_state(id: impl Into<SharedString>, state: Scene2DState) -> Self {
        Self {
            id: ElementId::Name(id.into()),
            state,
            on_input: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Returns a cloneable handle for replacing scenes and routing direct input.
    pub fn state(&self) -> Scene2DState {
        self.state.clone()
    }

    /// Sets the callback for normalized pointer, key, and lifecycle events.
    pub fn on_input(mut self, handler: impl Fn(Scene2DInput, &Window, &mut App) + 'static) -> Self {
        self.on_input = Some(Rc::new(handler));
        self
    }

    /// Sets an optional accessible name for the surface element.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Sets an optional platform accessibility role for the surface element.
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }
}

impl RenderOnce for GameSurface {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.clone();
        let scene = state.scene();
        let focus_handle = state.focus_handle(cx);
        state.set_input_handler(self.on_input.clone());
        let layout = Rc::new(RefCell::new(None::<Scene2DLayout>));
        let layout_prepaint = layout.clone();
        let prepaint_state = state.clone();
        let paint_state = state.clone();
        let canvas = canvas(
            move |bounds, window, _cx| {
                let scene = prepaint_state.scene();
                let transform = Scene2DViewTransform::contain(
                    scene.view_box,
                    bounds.size.width.into(),
                    bounds.size.height.into(),
                );
                if let Some(transform) = transform {
                    *layout_prepaint.borrow_mut() =
                        Some((bounds, transform, window.scale_factor()));
                    Some(transform)
                } else {
                    *layout_prepaint.borrow_mut() = None;
                    None
                }
            },
            move |bounds, transform, window, cx| {
                let Some(transform) = transform else {
                    return;
                };
                let (scene, animating) = paint_state.presented_scene();
                let transition_handler = paint_state
                    .weak_input_handler()
                    .upgrade()
                    .and_then(|handler| handler.borrow().clone());
                dispatch_all(
                    transition_handler.as_ref(),
                    paint_state.take_transition_completions(),
                    window,
                    cx,
                );
                if let Some(background) = scene.background {
                    window.paint_quad(fill(bounds, background_for(background, 1.0)));
                }
                for node in &scene.nodes {
                    paint_node(node, transform, bounds, window, cx, &paint_state);
                }
                if animating {
                    window.request_animation_frame();
                }
            },
        )
        .size_full();

        let mut root = div()
            .id(self.id.clone())
            .size_full()
            .overflow_hidden()
            .child(canvas);
        let receives_input = scene.input.pointer || scene.input.keyboard;
        if receives_input {
            root = root.track_focus(&focus_handle).focusable();
        }

        register_scene_accessibility(&scene, &self.id, cx);
        let has_semantic_children = scene.nodes.iter().any(|node| node.semantic.is_some());
        if let Some(semantic) = scene.semantic.as_ref() {
            let mut props = semantic_props(semantic);
            if let Some(role) = self.aria_role {
                props.role = role;
            }
            root = apply_native_accessibility(
                root,
                self.aria_label
                    .clone()
                    .unwrap_or_else(|| semantic.label.clone().into()),
                &props,
            );
        } else if let Some(label) = self.aria_label.clone() {
            root = apply_native_accessibility(
                root,
                label,
                &AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group)),
            );
        } else if has_semantic_children {
            root = apply_native_accessibility(
                root,
                format!("{} scene", self.id),
                &AriaProps::with_role(AriaRole::Group),
            );
        }
        if has_semantic_children {
            let semantic_scene = scene.clone();
            let semantic_layout = layout.clone();
            let semantic_state = state.clone();
            let has_input_callback = self.on_input.is_some();
            root = root.a11y_synthetic_children(move |builder| {
                add_native_semantic_children(
                    &semantic_scene,
                    &semantic_layout,
                    &semantic_state,
                    has_input_callback,
                    builder,
                );
            });
        }
        if receives_input && !state.has_focus_subscription() {
            let lifecycle = state.lifecycle_callback();
            let lifecycle_handler = state.weak_input_handler();
            let subscription = window.on_focus_out(&focus_handle, cx, move |_, window, cx| {
                let callback = lifecycle_handler
                    .upgrade()
                    .and_then(|handler| handler.borrow().clone());
                dispatch_all(
                    callback.as_ref(),
                    lifecycle(super::input::Scene2DLifecycleReason::FocusLost),
                    window,
                    cx,
                );
            });
            state.set_focus_subscription(subscription);
        }

        if scene.input.pointer {
            let pointer_down_state = state.clone();
            let pointer_down_layout = layout.clone();
            let pointer_down_input = self.on_input.clone();
            let pointer_down_focus = focus_handle.clone();
            root = root.on_pointer_down(move |event, window, cx| {
                dispatch_direct_pointer(
                    &pointer_down_state,
                    &pointer_down_layout,
                    pointer_down_input.as_ref(),
                    pointer_down_focus.clone(),
                    event,
                    Scene2DPointerPhase::Down,
                    window,
                    cx,
                );
            });

            let pointer_move_state = state.clone();
            let pointer_move_layout = layout.clone();
            let pointer_move_input = self.on_input.clone();
            let pointer_move_focus = focus_handle.clone();
            root = root.on_pointer_move(move |event, window, cx| {
                dispatch_direct_pointer(
                    &pointer_move_state,
                    &pointer_move_layout,
                    pointer_move_input.as_ref(),
                    pointer_move_focus.clone(),
                    event,
                    Scene2DPointerPhase::Move,
                    window,
                    cx,
                );
            });

            let pointer_up_state = state.clone();
            let pointer_up_layout = layout.clone();
            let pointer_up_input = self.on_input.clone();
            let pointer_up_focus = focus_handle.clone();
            root = root.on_pointer_up(move |event, window, cx| {
                dispatch_direct_pointer(
                    &pointer_up_state,
                    &pointer_up_layout,
                    pointer_up_input.as_ref(),
                    pointer_up_focus.clone(),
                    event,
                    Scene2DPointerPhase::Up,
                    window,
                    cx,
                );
            });

            let pointer_cancel_state = state.clone();
            let pointer_cancel_layout = layout.clone();
            let pointer_cancel_input = self.on_input.clone();
            let pointer_cancel_focus = focus_handle.clone();
            root = root.on_pointer_cancel(move |event, window, cx| {
                dispatch_direct_pointer(
                    &pointer_cancel_state,
                    &pointer_cancel_layout,
                    pointer_cancel_input.as_ref(),
                    pointer_cancel_focus.clone(),
                    event,
                    Scene2DPointerPhase::Cancel,
                    window,
                    cx,
                );
            });

            let down_state = state.clone();
            let down_layout = layout.clone();
            let down_input = self.on_input.clone();
            let down_focus = focus_handle.clone();
            root = root.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                if let Some(position) = logical_position(&down_layout, event.position) {
                    down_focus.focus(window, cx);
                    let (scene, _) = down_state.presented_scene();
                    let routed = down_state.route_pointer_for_scene(
                        &scene,
                        Scene2DPointerEvent {
                            phase: Scene2DPointerPhase::Down,
                            device: Scene2DPointerDevice::Mouse,
                            contact_id: 0,
                            timestamp_ns: down_state.timestamp_ns(),
                            position,
                            buttons: vec![Scene2DButton::Left],
                            modifiers: scene_modifiers(event.modifiers),
                        },
                    );
                    if !routed.is_empty() {
                        cx.stop_propagation();
                    }
                    dispatch_all(down_input.as_ref(), routed, window, cx);
                }
            });

            let move_state = state.clone();
            let move_layout = layout.clone();
            let move_input = self.on_input.clone();
            root = root.on_mouse_move(move |event: &MouseMoveEvent, window, cx| {
                if let Some(position) = logical_position(&move_layout, event.position) {
                    let (scene, _) = move_state.presented_scene();
                    let routed = move_state.route_pointer_for_scene(
                        &scene,
                        Scene2DPointerEvent {
                            phase: Scene2DPointerPhase::Move,
                            device: Scene2DPointerDevice::Mouse,
                            contact_id: 0,
                            timestamp_ns: move_state.timestamp_ns(),
                            position,
                            buttons: event
                                .pressed_button
                                .and_then(scene_button)
                                .into_iter()
                                .collect(),
                            modifiers: scene_modifiers(event.modifiers),
                        },
                    );
                    if !routed.is_empty() && event.pressed_button.is_some() {
                        cx.stop_propagation();
                    }
                    dispatch_all(move_input.as_ref(), routed, window, cx);
                }
            });

            let up_state = state.clone();
            let up_layout = layout.clone();
            let up_input = self.on_input.clone();
            let outside_state = up_state.clone();
            let outside_layout = up_layout.clone();
            let outside_input = up_input.clone();
            root = root
                .on_mouse_up(MouseButton::Left, move |event, window, cx| {
                    dispatch_mouse_up(&up_state, &up_layout, up_input.as_ref(), event, window, cx);
                })
                .on_mouse_up_out(MouseButton::Left, move |event, window, cx| {
                    dispatch_mouse_up(
                        &outside_state,
                        &outside_layout,
                        outside_input.as_ref(),
                        event,
                        window,
                        cx,
                    );
                });
        }

        if scene.input.keyboard {
            let key_down_state = state.clone();
            let key_down_input = self.on_input.clone();
            let key_down_scene = scene.clone();
            let key_down_focus = focus_handle.clone();
            root = root.on_key_down(move |event: &KeyDownEvent, window, cx| {
                if !key_down_focus.is_focused(window) {
                    return;
                }
                let routed = key_down_state
                    .route_key_for_scene(
                        &key_down_scene,
                        Scene2DKeyPhase::Down,
                        event.keystroke.key.as_str(),
                        event.is_held,
                        scene_modifiers(event.keystroke.modifiers),
                    )
                    .into_iter()
                    .collect::<Vec<_>>();
                if !routed.is_empty() {
                    cx.stop_propagation();
                }
                dispatch_all(key_down_input.as_ref(), routed, window, cx);
            });

            let key_up_state = state.clone();
            let key_up_input = self.on_input.clone();
            let key_up_scene = scene.clone();
            let key_up_focus = focus_handle.clone();
            root = root.on_key_up(move |event: &KeyUpEvent, window, cx| {
                if !key_up_focus.is_focused(window) {
                    return;
                }
                let routed = key_up_state
                    .route_key_for_scene(
                        &key_up_scene,
                        Scene2DKeyPhase::Up,
                        event.keystroke.key.as_str(),
                        false,
                        scene_modifiers(event.keystroke.modifiers),
                    )
                    .into_iter()
                    .collect::<Vec<_>>();
                if !routed.is_empty() {
                    cx.stop_propagation();
                }
                dispatch_all(key_up_input.as_ref(), routed, window, cx);
            });
        }
        root
    }
}

impl IntoElement for GameSurface {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

type Scene2DLayout = (Bounds<Pixels>, Scene2DViewTransform, f32);

fn dispatch_mouse_up(
    state: &Scene2DState,
    layout: &Rc<RefCell<Option<Scene2DLayout>>>,
    callback: Option<&Rc<dyn Fn(Scene2DInput, &Window, &mut App) + 'static>>,
    event: &MouseUpEvent,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(position) = logical_position(layout, event.position) else {
        return;
    };
    let (scene, _) = state.presented_scene();
    let routed = state.route_pointer_for_scene(
        &scene,
        Scene2DPointerEvent {
            phase: Scene2DPointerPhase::Up,
            device: Scene2DPointerDevice::Mouse,
            contact_id: 0,
            timestamp_ns: state.timestamp_ns(),
            position,
            buttons: Vec::new(),
            modifiers: scene_modifiers(event.modifiers),
        },
    );
    if !routed.is_empty() {
        cx.stop_propagation();
    }
    dispatch_all(callback, routed, window, cx);
}

fn dispatch_direct_pointer(
    state: &Scene2DState,
    layout: &Rc<RefCell<Option<Scene2DLayout>>>,
    callback: Option<&Rc<dyn Fn(Scene2DInput, &Window, &mut App) + 'static>>,
    focus_handle: gpui::FocusHandle,
    event: &PointerEvent,
    phase: Scene2DPointerPhase,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(position) = direct_logical_position(layout, event.position, phase) else {
        // A Down in the contain letterbox is outside the scene's gesture
        // region. Keep the platform's compatibility mouse/scroll path live,
        // but suppress Div's generic tracked-focus listener for this pointer.
        if phase == Scene2DPointerPhase::Down {
            window.prevent_default();
        }
        return;
    };
    if phase == Scene2DPointerPhase::Down {
        focus_handle.focus(window, cx);
    }
    let (scene, _) = state.presented_scene();
    let routed = state.route_pointer_for_scene(
        &scene,
        Scene2DPointerEvent {
            phase,
            device: match event.device {
                PointerDevice::Touch => Scene2DPointerDevice::Touch,
                PointerDevice::Pen => Scene2DPointerDevice::Pen,
            },
            contact_id: event.pointer_id,
            timestamp_ns: event.timestamp_ns,
            position,
            buttons: event
                .buttons
                .iter()
                .filter_map(|button| scene_button(*button))
                .collect(),
            modifiers: scene_modifiers(event.modifiers),
        },
    );
    if phase == Scene2DPointerPhase::Down
        && state.wants_pointer_capture()
        && window.capture_direct_pointer(event.pointer_id)
    {
        cx.stop_propagation();
    }
    if !routed.is_empty() {
        cx.stop_propagation();
    }
    dispatch_all(callback, routed, window, cx);
}

fn direct_logical_position(
    layout: &Rc<RefCell<Option<Scene2DLayout>>>,
    position: gpui::Point<Pixels>,
    phase: Scene2DPointerPhase,
) -> Option<ScenePoint> {
    let layout = layout.borrow();
    let (bounds, transform, _) = layout.as_ref()?;
    let x: f32 = position.x.into();
    let y: f32 = position.y.into();
    let bounds_x: f32 = bounds.origin.x.into();
    let bounds_y: f32 = bounds.origin.y.into();
    let viewport_position = ScenePoint::new(x - bounds_x, y - bounds_y);
    match phase {
        Scene2DPointerPhase::Down => transform.viewport_to_scene(viewport_position),
        Scene2DPointerPhase::Move | Scene2DPointerPhase::Up | Scene2DPointerPhase::Cancel => {
            Some(transform.viewport_to_scene_unclamped(viewport_position))
        }
    }
}

fn logical_position(
    layout: &Rc<RefCell<Option<Scene2DLayout>>>,
    position: gpui::Point<Pixels>,
) -> Option<ScenePoint> {
    let layout = layout.borrow();
    let (bounds, transform, _) = layout.as_ref()?;
    let x: f32 = position.x.into();
    let y: f32 = position.y.into();
    let bounds_x: f32 = bounds.origin.x.into();
    let bounds_y: f32 = bounds.origin.y.into();
    Some(transform.viewport_to_scene_unclamped(ScenePoint::new(x - bounds_x, y - bounds_y)))
}

fn scene_modifiers(modifiers: gpui::Modifiers) -> Vec<Scene2DModifier> {
    let mut result = Vec::with_capacity(5);
    if modifiers.shift {
        result.push(Scene2DModifier::Shift);
    }
    if modifiers.control {
        result.push(Scene2DModifier::Control);
    }
    if modifiers.alt {
        result.push(Scene2DModifier::Alt);
    }
    if modifiers.platform {
        result.push(Scene2DModifier::Meta);
    }
    if modifiers.function {
        result.push(Scene2DModifier::Fn);
    }
    result
}

fn scene_button(button: MouseButton) -> Option<Scene2DButton> {
    match button {
        MouseButton::Left => Some(Scene2DButton::Left),
        MouseButton::Right => Some(Scene2DButton::Right),
        MouseButton::Middle => Some(Scene2DButton::Middle),
        MouseButton::Navigate(_) => None,
    }
}

fn dispatch_all(
    callback: Option<&Rc<dyn Fn(Scene2DInput, &Window, &mut App) + 'static>>,
    events: Vec<Scene2DInput>,
    window: &Window,
    cx: &mut App,
) {
    if let Some(callback) = callback {
        for event in events {
            callback(event, window, cx);
        }
    }
}

fn register_scene_accessibility(scene: &super::types::Scene2DScene, id: &ElementId, cx: &mut App) {
    if let Some(semantic) = &scene.semantic {
        cx.register_accessible(AccessibilityNode {
            element_id: id.clone(),
            label: semantic.label.clone().into(),
            props: semantic_props(semantic),
        });
    }
    for node in &scene.nodes {
        let Some(semantic) = &node.semantic else {
            continue;
        };
        let element_id = ElementId::Name(format!("{}:{}", id, node.id).into());
        cx.register_accessible(AccessibilityNode {
            element_id,
            label: semantic.label.clone().into(),
            props: semantic_props(semantic),
        });
    }
}

fn add_native_semantic_children(
    scene: &super::types::Scene2DScene,
    layout: &Rc<RefCell<Option<Scene2DLayout>>>,
    state: &Scene2DState,
    has_input_callback: bool,
    builder: &mut gpui::A11ySubtreeBuilder,
) {
    let layout = layout.borrow();
    let fitted = layout.as_ref();
    for node in &scene.nodes {
        add_native_semantic_node(
            node,
            Scene2DTransform::identity(),
            None,
            scene,
            fitted,
            state,
            has_input_callback,
            builder,
        );
    }
}

fn add_native_semantic_node(
    node: &Scene2DNode,
    parent_transform: Scene2DTransform,
    parent_clip: Option<SceneRect>,
    scene: &super::types::Scene2DScene,
    fitted: Option<&Scene2DLayout>,
    state: &Scene2DState,
    has_input_callback: bool,
    builder: &mut gpui::A11ySubtreeBuilder,
) {
    let node_transform = Scene2DTransform::compose(parent_transform, node.transform);
    let clip = node
        .clip
        .map(|clip| transformed_rect_aabb(clip, node_transform));
    let clip = match (parent_clip, clip) {
        (Some(parent), Some(clip)) => Some(intersect_rect(parent, clip)),
        (Some(parent), None) => Some(parent),
        (None, clip) => clip,
    };

    if let Some(semantic) = &node.semantic {
        let role = semantic_role(semantic.role)
            .native_role()
            .unwrap_or(gpui::accesskit::Role::Group);
        let mut accessible = gpui::accesskit::Node::new(role);
        accessible.set_label(semantic.label.clone());
        if let Some(description) = &semantic.description {
            accessible.set_description(description.clone());
        }
        if let Some(value) = &semantic.value_text {
            accessible.set_value(value.clone());
        }
        if let Some(selected) = semantic.selected {
            accessible.set_selected(selected);
        }
        if semantic.disabled == Some(true) {
            accessible.set_disabled();
        }
        if let (Some((bounds, transform, scale_factor)), Some(scene_bounds)) =
            (fitted, semantic_node_bounds(node))
        {
            let mut scene_bounds = transformed_rect_aabb(scene_bounds, node_transform);
            if let Some(clip) = clip {
                scene_bounds = intersect_rect(scene_bounds, clip);
            }
            let native_bounds = scene_rect_to_native_bounds(
                scene_bounds,
                Scene2DTransform::identity(),
                *transform,
                *bounds,
                *scale_factor,
            );
            accessible.set_bounds(native_bounds);
        }
        let actionable = semantic_is_actionable(semantic, scene, has_input_callback);
        if actionable {
            accessible.add_action(gpui::accesskit::Action::Click);
        }
        let id = builder.synthetic_node_id(node.id.as_str());
        if builder.push_child(id, accessible) && actionable {
            let state = state.clone();
            let object_id = node.id.clone();
            let hit_id = node.hit_id.clone();
            let cell = if semantic.role == Scene2DSemanticRole::GridCell {
                semantic_node_bounds(node).and_then(|rect| {
                    let center =
                        ScenePoint::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
                    scene
                        .grid
                        .as_ref()
                        .and_then(|grid| grid.hit_cell(transform_point(center, node_transform)))
                })
            } else {
                None
            };
            builder.on_action(id, gpui::accesskit::Action::Click, move |_, window, cx| {
                dispatch_all(
                    state
                        .weak_input_handler()
                        .upgrade()
                        .and_then(|handler| handler.borrow().clone())
                        .as_ref(),
                    vec![Scene2DInput::Activate {
                        id: object_id.clone(),
                        hit_id: hit_id.clone(),
                        cell: cell.clone(),
                        timestamp_ns: state.timestamp_ns(),
                    }],
                    window,
                    cx,
                );
            });
        }
    }

    if let Scene2DNodeKind::Group { children } = &node.kind {
        for child in children {
            add_native_semantic_node(
                child,
                node_transform,
                clip,
                scene,
                fitted,
                state,
                has_input_callback,
                builder,
            );
        }
    }
}

fn semantic_is_actionable(
    semantic: &Scene2DSemantic,
    scene: &super::types::Scene2DScene,
    has_input_callback: bool,
) -> bool {
    matches!(
        semantic.role,
        Scene2DSemanticRole::GridCell | Scene2DSemanticRole::Button
    ) && semantic.disabled != Some(true)
        && (scene.input.pointer || scene.input.keyboard)
        && has_input_callback
}

fn intersect_rect(a: SceneRect, b: SceneRect) -> SceneRect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width).max(x);
    let bottom = (a.y + a.height).min(b.y + b.height).max(y);
    SceneRect::new(x, y, right - x, bottom - y)
}

fn semantic_node_bounds(node: &Scene2DNode) -> Option<SceneRect> {
    if let Some(bounds) = node.hit_bounds {
        return Some(bounds);
    }
    match &node.kind {
        Scene2DNodeKind::Rect { rect, stroke, .. }
        | Scene2DNodeKind::RoundedRect { rect, stroke, .. } => Some(expand_rect(
            *rect,
            stroke.map_or(0.0, |stroke| stroke.width * 0.5),
        )),
        Scene2DNodeKind::Circle {
            center,
            radius,
            stroke,
            ..
        } => {
            let radius = *radius + stroke.map_or(0.0, |stroke| stroke.width * 0.5);
            Some(SceneRect::new(
                center.x - radius,
                center.y - radius,
                radius * 2.0,
                radius * 2.0,
            ))
        }
        Scene2DNodeKind::Line { start, end, stroke } => Some(SceneRect::new(
            start.x.min(end.x) - stroke.width * 0.5,
            start.y.min(end.y) - stroke.width * 0.5,
            (start.x - end.x).abs() + stroke.width,
            (start.y - end.y).abs() + stroke.width,
        )),
        Scene2DNodeKind::Path {
            commands, stroke, ..
        } => {
            let points = commands.iter().flat_map(|command| match command {
                super::types::Scene2DPathCommand::MoveTo { point }
                | super::types::Scene2DPathCommand::LineTo { point } => vec![*point],
                super::types::Scene2DPathCommand::QuadraticTo { control, point } => {
                    vec![*control, *point]
                }
                super::types::Scene2DPathCommand::CubicTo {
                    control_a,
                    control_b,
                    point,
                } => vec![*control_a, *control_b, *point],
                super::types::Scene2DPathCommand::Close => Vec::new(),
            });
            let mut points = points.peekable();
            let first = points.peek().copied()?;
            let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.x, first.y, first.x, first.y);
            for point in points {
                min_x = min_x.min(point.x);
                min_y = min_y.min(point.y);
                max_x = max_x.max(point.x);
                max_y = max_y.max(point.y);
            }
            let padding = stroke.map_or(0.0, |stroke| stroke.width * 0.5);
            Some(SceneRect::new(
                min_x - padding,
                min_y - padding,
                max_x - min_x + padding * 2.0,
                max_y - min_y + padding * 2.0,
            ))
        }
        Scene2DNodeKind::Text {
            origin,
            content,
            size,
            ..
        } => Some(SceneRect::new(
            origin.x,
            origin.y,
            content.chars().count() as f32 * *size * 0.62,
            *size * 1.4,
        )),
        Scene2DNodeKind::Group { children } => {
            let mut bounds: Option<SceneRect> = None;
            for child in children {
                let Some(child_bounds) = semantic_node_bounds(child) else {
                    continue;
                };
                let transformed = transformed_rect_aabb(child_bounds, child.transform);
                bounds = Some(match bounds {
                    Some(bounds) => union_rect(bounds, transformed),
                    None => transformed,
                });
            }
            bounds
        }
    }
}

fn transformed_rect_aabb(rect: SceneRect, transform: Scene2DTransform) -> SceneRect {
    let corners = [
        ScenePoint::new(rect.x, rect.y),
        ScenePoint::new(rect.x + rect.width, rect.y),
        ScenePoint::new(rect.x, rect.y + rect.height),
        ScenePoint::new(rect.x + rect.width, rect.y + rect.height),
    ]
    .map(|point| transform_point(point, transform));
    let min_x = corners
        .iter()
        .map(|point| point.x)
        .fold(f32::INFINITY, f32::min);
    let min_y = corners
        .iter()
        .map(|point| point.y)
        .fold(f32::INFINITY, f32::min);
    let max_x = corners
        .iter()
        .map(|point| point.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let max_y = corners
        .iter()
        .map(|point| point.y)
        .fold(f32::NEG_INFINITY, f32::max);
    SceneRect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

fn union_rect(a: SceneRect, b: SceneRect) -> SceneRect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    let right = (a.x + a.width).max(b.x + b.width);
    let bottom = (a.y + a.height).max(b.y + b.height);
    SceneRect::new(x, y, right - x, bottom - y)
}

fn expand_rect(rect: SceneRect, padding: f32) -> SceneRect {
    SceneRect::new(
        rect.x - padding,
        rect.y - padding,
        rect.width + padding * 2.0,
        rect.height + padding * 2.0,
    )
}

fn scene_rect_to_native_bounds(
    rect: SceneRect,
    node_transform: Scene2DTransform,
    fit: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
    scale_factor: f32,
) -> gpui::accesskit::Rect {
    let corners = [
        ScenePoint::new(rect.x, rect.y),
        ScenePoint::new(rect.x + rect.width, rect.y),
        ScenePoint::new(rect.x, rect.y + rect.height),
        ScenePoint::new(rect.x + rect.width, rect.y + rect.height),
    ]
    .map(|point| fit.scene_to_viewport(transform_point(point, node_transform)));
    let min_x = corners
        .iter()
        .map(|point| point.x)
        .fold(f32::INFINITY, f32::min);
    let min_y = corners
        .iter()
        .map(|point| point.y)
        .fold(f32::INFINITY, f32::min);
    let max_x = corners
        .iter()
        .map(|point| point.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let max_y = corners
        .iter()
        .map(|point| point.y)
        .fold(f32::NEG_INFINITY, f32::max);
    let origin_x: f32 = bounds.origin.x.into();
    let origin_y: f32 = bounds.origin.y.into();
    gpui::accesskit::Rect::new(
        f64::from((origin_x + min_x) * scale_factor),
        f64::from((origin_y + min_y) * scale_factor),
        f64::from((origin_x + max_x) * scale_factor),
        f64::from((origin_y + max_y) * scale_factor),
    )
}

fn semantic_props(semantic: &Scene2DSemantic) -> AriaProps {
    let mut props = AriaProps::with_role(semantic_role(semantic.role));
    props.description = semantic.description.clone().map(Into::into);
    props.value_text = semantic.value_text.clone().map(Into::into);
    if let Some(selected) = semantic.selected {
        props.states.push(AriaState::Selected(selected));
    }
    if semantic.disabled == Some(true) {
        props.states.push(AriaState::Disabled);
    }
    props
}

fn semantic_role(role: Scene2DSemanticRole) -> AriaRole {
    match role {
        Scene2DSemanticRole::Group => AriaRole::Group,
        Scene2DSemanticRole::Grid => AriaRole::Table,
        Scene2DSemanticRole::Row => AriaRole::Row,
        Scene2DSemanticRole::GridCell => AriaRole::Cell,
        Scene2DSemanticRole::Button => AriaRole::Button,
        Scene2DSemanticRole::Image => AriaRole::Img,
        Scene2DSemanticRole::Status => AriaRole::Status,
    }
}

fn paint_node(
    node: &Scene2DNode,
    transform: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
    state: &Scene2DState,
) {
    paint_node_with_parent(
        node,
        transform,
        bounds,
        Scene2DTransform::identity(),
        1.0,
        window,
        cx,
        state,
    );
}

fn paint_node_with_parent(
    node: &Scene2DNode,
    transform: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
    parent_transform: Scene2DTransform,
    parent_opacity: f32,
    window: &mut Window,
    cx: &mut App,
    state: &Scene2DState,
) {
    let mut node = node.clone();
    node.transform = Scene2DTransform::compose(parent_transform, node.transform);
    node.opacity = (node.opacity * parent_opacity).clamp(0.0, 1.0);
    let clip = node.clip.map(|clip| ContentMask {
        bounds: mapped_rect_bounds(clip, node.transform, transform, bounds),
    });

    if let Scene2DNodeKind::Group { children } = &node.kind {
        let children = children.clone();
        window.with_content_mask(clip, |window| {
            for child in &children {
                paint_node_with_parent(
                    child,
                    transform,
                    bounds,
                    node.transform,
                    node.opacity,
                    window,
                    cx,
                    state,
                );
            }
        });
        return;
    }

    window.with_content_mask(clip, |window| {
        paint_node_shadow(&node, transform, bounds, window);
        paint_leaf_node(&node, transform, bounds, window, cx, state);
    });
}

fn paint_node_shadow(
    node: &Scene2DNode,
    transform: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    let Some(shadow) = node.shadow else {
        return;
    };
    let (rect, radius) = match &node.kind {
        Scene2DNodeKind::Rect { rect, .. } => (*rect, 0.0),
        Scene2DNodeKind::RoundedRect { rect, radius, .. } => (*rect, *radius),
        _ => return,
    };
    let shadow_bounds = mapped_rect_bounds(rect, node.transform, transform, bounds);
    let scale = transform.scale() * transform_scale(node.transform);
    let shadow_style = BoxShadow::new(
        px(shadow.offset_x * transform.scale()),
        px(shadow.offset_y * transform.scale()),
        rgba_color(shadow.color, node.opacity).into(),
    )
    .blur_radius(px(shadow.blur_radius * transform.scale()));
    window.paint_drop_shadows(
        shadow_bounds,
        Corners::all(px(radius * scale)),
        &[shadow_style],
    );
}

fn mapped_rect_bounds(
    rect: SceneRect,
    node_transform: Scene2DTransform,
    fit: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
) -> Bounds<Pixels> {
    let corners = [
        (rect.x, rect.y),
        (rect.x + rect.width, rect.y),
        (rect.x, rect.y + rect.height),
        (rect.x + rect.width, rect.y + rect.height),
    ]
    .map(|(x, y)| window_point(x, y, node_transform, fit, bounds));
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for corner in corners {
        let x: f32 = corner.x.into();
        let y: f32 = corner.y.into();
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    Bounds {
        origin: point(px(min_x), px(min_y)),
        size: size(px(max_x - min_x), px(max_y - min_y)),
    }
}

fn paint_leaf_node(
    node: &Scene2DNode,
    transform: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
    state: &Scene2DState,
) {
    match &node.kind {
        Scene2DNodeKind::Rect { rect, fill, stroke } => {
            paint_rect(node, *rect, 0.0, *fill, *stroke, transform, bounds, window);
        }
        Scene2DNodeKind::RoundedRect {
            rect,
            radius,
            fill,
            stroke,
        } => paint_rect(
            node, *rect, *radius, *fill, *stroke, transform, bounds, window,
        ),
        Scene2DNodeKind::Circle {
            center,
            radius,
            fill,
            stroke,
        } => paint_circle(
            node, *center, *radius, *fill, *stroke, transform, bounds, window,
        ),
        Scene2DNodeKind::Line { start, end, stroke } => {
            paint_line(node, *start, *end, *stroke, transform, bounds, window);
        }
        Scene2DNodeKind::Path {
            commands,
            fill,
            stroke,
        } => paint_path(
            node, commands, *fill, *stroke, transform, bounds, window, cx, state,
        ),
        Scene2DNodeKind::Text {
            origin,
            content,
            size,
            color,
            font: family,
            align,
        } => paint_text(
            node,
            TextDraw {
                origin: *origin,
                content,
                size: *size,
                color: *color,
                family: family.as_deref(),
                align: *align,
            },
            transform,
            bounds,
            window,
            cx,
            state,
        ),
        Scene2DNodeKind::Group { .. } => {}
    }
}

fn paint_rect(
    node: &Scene2DNode,
    rect: SceneRect,
    radius: f32,
    fill_brush: Option<Scene2DBrush>,
    stroke: Option<Scene2DStroke>,
    transform: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    if node.transform.rotation_degrees.abs() < f32::EPSILON
        && node.transform.shear_x.abs() < f32::EPSILON
    {
        let top_left = window_point(rect.x, rect.y, node.transform, transform, bounds);
        let bottom_right = window_point(
            rect.x + rect.width,
            rect.y + rect.height,
            node.transform,
            transform,
            bounds,
        );
        let top_left_x: f32 = top_left.x.into();
        let top_left_y: f32 = top_left.y.into();
        let bottom_right_x: f32 = bottom_right.x.into();
        let bottom_right_y: f32 = bottom_right.y.into();
        let x = top_left_x.min(bottom_right_x);
        let y = top_left_y.min(bottom_right_y);
        let width = (bottom_right_x - top_left_x).abs();
        let height = (bottom_right_y - top_left_y).abs();
        let mapped_radius = radius * transform_scale(node.transform) * transform.scale();
        let background = fill_brush.map_or_else(
            || gpui::transparent_black().into(),
            |brush| background_for(brush, node.opacity),
        );
        let mut painted = quad(
            Bounds {
                origin: point(px(x), px(y)),
                size: size(px(width), px(height)),
            },
            Corners::all(px(mapped_radius)),
            background,
            stroke.map_or(px(0.0), |edge| {
                px(edge.width * transform.scale() * transform_scale(node.transform))
            }),
            stroke.map_or_else(gpui::transparent_black, |edge| {
                rgba_color(edge.color, node.opacity).into()
            }),
            gpui::BorderStyle::default(),
        );
        painted.background = background;
        window.paint_quad(painted);
    } else {
        if let Some(fill_brush) = fill_brush {
            let path = rounded_rect_path(rect, radius, node.transform, transform, bounds);
            if let Ok(path) = path.build() {
                window.paint_path(path, background_for(fill_brush, node.opacity));
            }
        }
        if let Some(stroke) = stroke
            && let Ok(path) = rounded_rect_path(rect, radius, node.transform, transform, bounds)
                .with_style(PathStyle::Stroke(
                    StrokeOptions::default().with_line_width(stroke_width(node, stroke, transform)),
                ))
                .build()
        {
            window.paint_path(path, rgba_color(stroke.color, node.opacity));
        }
    }
}

fn paint_circle(
    node: &Scene2DNode,
    center: ScenePoint,
    radius: f32,
    fill_brush: Option<Scene2DBrush>,
    stroke: Option<Scene2DStroke>,
    transform: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    if is_uniform_orthogonal(node.transform) {
        let mapped_center = window_point(center.x, center.y, node.transform, transform, bounds);
        let mapped_radius = radius * transform_scale(node.transform) * transform.scale();
        let background = fill_brush.map_or_else(
            || gpui::transparent_black().into(),
            |brush| background_for(brush, node.opacity),
        );
        let painted = quad(
            Bounds {
                origin: point(
                    px({
                        let value: f32 = mapped_center.x.into();
                        value - mapped_radius
                    }),
                    px({
                        let value: f32 = mapped_center.y.into();
                        value - mapped_radius
                    }),
                ),
                size: size(px(mapped_radius * 2.0), px(mapped_radius * 2.0)),
            },
            Corners::all(px(mapped_radius)),
            background,
            stroke.map_or(px(0.0), |edge| px(stroke_width(node, edge, transform))),
            stroke.map_or_else(gpui::transparent_black, |edge| {
                rgba_color(edge.color, node.opacity).into()
            }),
            gpui::BorderStyle::default(),
        );
        window.paint_quad(painted);
    } else {
        let kappa = CIRCLE_KAPPA;
        let left = ScenePoint::new(center.x - radius, center.y);
        let top = ScenePoint::new(center.x, center.y - radius);
        let right = ScenePoint::new(center.x + radius, center.y);
        let bottom = ScenePoint::new(center.x, center.y + radius);
        if let Some(fill_brush) = fill_brush {
            let mut path = PathBuilder::fill();
            path.move_to(window_point(
                left.x,
                left.y,
                node.transform,
                transform,
                bounds,
            ));
            path.cubic_bezier_to(
                window_point(top.x, top.y, node.transform, transform, bounds),
                window_point(
                    left.x,
                    left.y - kappa * radius,
                    node.transform,
                    transform,
                    bounds,
                ),
                window_point(
                    top.x - kappa * radius,
                    top.y,
                    node.transform,
                    transform,
                    bounds,
                ),
            );
            path.cubic_bezier_to(
                window_point(right.x, right.y, node.transform, transform, bounds),
                window_point(
                    top.x + kappa * radius,
                    top.y,
                    node.transform,
                    transform,
                    bounds,
                ),
                window_point(
                    right.x,
                    right.y - kappa * radius,
                    node.transform,
                    transform,
                    bounds,
                ),
            );
            path.cubic_bezier_to(
                window_point(bottom.x, bottom.y, node.transform, transform, bounds),
                window_point(
                    right.x,
                    right.y + kappa * radius,
                    node.transform,
                    transform,
                    bounds,
                ),
                window_point(
                    bottom.x + kappa * radius,
                    bottom.y,
                    node.transform,
                    transform,
                    bounds,
                ),
            );
            path.cubic_bezier_to(
                window_point(left.x, left.y, node.transform, transform, bounds),
                window_point(
                    bottom.x - kappa * radius,
                    bottom.y,
                    node.transform,
                    transform,
                    bounds,
                ),
                window_point(
                    left.x,
                    left.y + kappa * radius,
                    node.transform,
                    transform,
                    bounds,
                ),
            );
            path.close();
            if let Ok(path) = path.build() {
                window.paint_path(path, background_for(fill_brush, node.opacity));
            }
        }
        if let Some(stroke) = stroke {
            let mut path = PathBuilder::stroke(px(stroke_width(node, stroke, transform)));
            path.move_to(window_point(
                left.x,
                left.y,
                node.transform,
                transform,
                bounds,
            ));
            path.cubic_bezier_to(
                window_point(top.x, top.y, node.transform, transform, bounds),
                window_point(
                    left.x,
                    left.y - kappa * radius,
                    node.transform,
                    transform,
                    bounds,
                ),
                window_point(
                    top.x - kappa * radius,
                    top.y,
                    node.transform,
                    transform,
                    bounds,
                ),
            );
            path.cubic_bezier_to(
                window_point(right.x, right.y, node.transform, transform, bounds),
                window_point(
                    top.x + kappa * radius,
                    top.y,
                    node.transform,
                    transform,
                    bounds,
                ),
                window_point(
                    right.x,
                    right.y - kappa * radius,
                    node.transform,
                    transform,
                    bounds,
                ),
            );
            path.cubic_bezier_to(
                window_point(bottom.x, bottom.y, node.transform, transform, bounds),
                window_point(
                    right.x,
                    right.y + kappa * radius,
                    node.transform,
                    transform,
                    bounds,
                ),
                window_point(
                    bottom.x + kappa * radius,
                    bottom.y,
                    node.transform,
                    transform,
                    bounds,
                ),
            );
            path.cubic_bezier_to(
                window_point(left.x, left.y, node.transform, transform, bounds),
                window_point(
                    bottom.x - kappa * radius,
                    bottom.y,
                    node.transform,
                    transform,
                    bounds,
                ),
                window_point(
                    left.x,
                    left.y + kappa * radius,
                    node.transform,
                    transform,
                    bounds,
                ),
            );
            path.close();
            if let Ok(path) = path.build() {
                window.paint_path(path, rgba_color(stroke.color, node.opacity));
            }
        }
    }
}

fn paint_line(
    node: &Scene2DNode,
    start: ScenePoint,
    end: ScenePoint,
    stroke: Scene2DStroke,
    transform: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    let mut path = PathBuilder::stroke(px(stroke_width(node, stroke, transform)));
    path.move_to(window_point(
        start.x,
        start.y,
        node.transform,
        transform,
        bounds,
    ));
    path.line_to(window_point(
        end.x,
        end.y,
        node.transform,
        transform,
        bounds,
    ));
    if let Ok(path) = path.build() {
        window.paint_path(path, rgba_color(stroke.color, node.opacity));
    }
}

fn paint_path(
    node: &Scene2DNode,
    commands: &[super::types::Scene2DPathCommand],
    fill_brush: Option<Scene2DBrush>,
    stroke: Option<Scene2DStroke>,
    transform: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
    state: &Scene2DState,
) {
    let theme_key = format!("{:?}", cx.theme());
    let cache_key = hash_debug(&format!(
        "{:?}",
        (
            commands,
            &node.kind,
            node.transform,
            bounds,
            transform,
            window.scale_factor(),
            theme_key
        )
    ));
    if let Some(fill_brush) = fill_brush
        && let Some(path) = state.cached_path(
            &format!("{}\0fill", node.id),
            cache_key ^ 0x9e37_79b9_7f4a_7c15,
            || build_scene_path(commands, node.transform, transform, bounds, None),
        )
    {
        window.paint_path(path, background_for(fill_brush, node.opacity));
    }
    if let Some(stroke) = stroke {
        let width = stroke_width(node, stroke, transform);
        if let Some(path) = state.cached_path(
            &format!("{}\0stroke", node.id),
            hash_debug(&(cache_key, width.to_bits())),
            || build_scene_path(commands, node.transform, transform, bounds, Some(width)),
        ) {
            window.paint_path(path, rgba_color(stroke.color, node.opacity));
        }
    }
}

fn build_scene_path(
    commands: &[super::types::Scene2DPathCommand],
    transform: Scene2DTransform,
    fit: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
    stroke_width: Option<f32>,
) -> Option<gpui::Path<Pixels>> {
    let mut path =
        stroke_width.map_or_else(PathBuilder::fill, |width| PathBuilder::stroke(px(width)));
    use super::types::Scene2DPathCommand as Command;
    for command in commands {
        match command {
            Command::MoveTo { point: to } => {
                path.move_to(window_point(to.x, to.y, transform, fit, bounds));
            }
            Command::LineTo { point: to } => {
                path.line_to(window_point(to.x, to.y, transform, fit, bounds));
            }
            Command::QuadraticTo { control, point: to } => {
                path.curve_to(
                    window_point(to.x, to.y, transform, fit, bounds),
                    window_point(control.x, control.y, transform, fit, bounds),
                );
            }
            Command::CubicTo {
                control_a,
                control_b,
                point: to,
            } => {
                path.cubic_bezier_to(
                    window_point(to.x, to.y, transform, fit, bounds),
                    window_point(control_a.x, control_a.y, transform, fit, bounds),
                    window_point(control_b.x, control_b.y, transform, fit, bounds),
                );
            }
            Command::Close => path.close(),
        }
    }
    path.build().ok()
}

/// Text content and styling for [`paint_text`], grouped so the painter
/// takes a single text parameter instead of six positional scalars.
struct TextDraw<'a> {
    origin: ScenePoint,
    content: &'a str,
    size: f32,
    color: Scene2DColor,
    family: Option<&'a str>,
    align: Scene2DTextAlign,
}

fn paint_text(
    node: &Scene2DNode,
    text: TextDraw<'_>,
    transform: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
    state: &Scene2DState,
) {
    if text.content.is_empty() {
        return;
    }
    let mut style = window.text_style();
    let theme = cx.theme();
    if let Some(family) = text.family {
        style.font_family = family.into();
    } else {
        style.font_family = theme.font_family.clone();
    }
    style.color = rgba_color(text.color, node.opacity).into();
    let run = style.to_run(text.content.len());
    let text_size = px(text.size * transform.scale() * transform_scale(node.transform));
    let align_width = node.hit_bounds.map(|hit_bounds| {
        px(hit_bounds.width * transform.scale() * transform_scale(node.transform))
    });
    let text_size_value: f32 = text_size.into();
    let align_width_value = align_width.map(|width| {
        let width: f32 = width.into();
        width.to_bits()
    });
    let style_key = format!(
        "{:?}",
        (
            (
                node.id.as_str(),
                text.content,
                style.font_family,
                style.font_features,
                style.font_fallbacks,
                style.font_weight,
                style.font_style,
            ),
            (
                style.color,
                text_size_value.to_bits(),
                align_width_value,
                bounds.size,
                window.scale_factor(),
                theme,
            )
        )
    );
    let line = state.cached_text_line(node.id.as_str(), hash_debug(&style_key), || {
        window
            .text_system()
            .shape_line(text.content.into(), text_size, &[run], None)
    });
    let mut origin = window_point(
        text.origin.x,
        text.origin.y,
        node.transform,
        transform,
        bounds,
    );
    // `ShapedLine::paint` aligns within `align_width` and falls back to the
    // line's own width when it is `None`, which paints Center/Right text as if
    // left-aligned at the origin. Shift the origin so the origin acts as the
    // anchor point for markers that carry no width bound.
    origin = aligned_text_origin(origin, text.align, line.width(), align_width);
    let align = match text.align {
        Scene2DTextAlign::Left => TextAlign::Left,
        Scene2DTextAlign::Center => TextAlign::Center,
        Scene2DTextAlign::Right => TextAlign::Right,
    };
    let line_height = px(text.size * transform.scale() * transform_scale(node.transform) * 1.25);
    let _ = line.paint(origin, line_height, align, align_width, window, cx);
}

/// Anchors Center/Right text origins when no alignment width is set.
fn aligned_text_origin(
    origin: Point<Pixels>,
    align: Scene2DTextAlign,
    line_width: Pixels,
    align_width: Option<Pixels>,
) -> Point<Pixels> {
    if align_width.is_some() {
        return origin;
    }
    match align {
        Scene2DTextAlign::Left => origin,
        Scene2DTextAlign::Center => point(origin.x - line_width * 0.5, origin.y),
        Scene2DTextAlign::Right => point(origin.x - line_width, origin.y),
    }
}

fn rounded_rect_path(
    rect: SceneRect,
    radius: f32,
    node_transform: Scene2DTransform,
    fit: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
) -> PathBuilder {
    let radius = radius.min(rect.width * 0.5).min(rect.height * 0.5);
    let mut path = PathBuilder::fill();
    path.move_to(window_point(
        rect.x + radius,
        rect.y,
        node_transform,
        fit,
        bounds,
    ));
    path.line_to(window_point(
        rect.x + rect.width - radius,
        rect.y,
        node_transform,
        fit,
        bounds,
    ));
    path.curve_to(
        window_point(
            rect.x + rect.width,
            rect.y + radius,
            node_transform,
            fit,
            bounds,
        ),
        window_point(rect.x + rect.width, rect.y, node_transform, fit, bounds),
    );
    path.line_to(window_point(
        rect.x + rect.width,
        rect.y + rect.height - radius,
        node_transform,
        fit,
        bounds,
    ));
    path.curve_to(
        window_point(
            rect.x + rect.width - radius,
            rect.y + rect.height,
            node_transform,
            fit,
            bounds,
        ),
        window_point(
            rect.x + rect.width,
            rect.y + rect.height,
            node_transform,
            fit,
            bounds,
        ),
    );
    path.line_to(window_point(
        rect.x + radius,
        rect.y + rect.height,
        node_transform,
        fit,
        bounds,
    ));
    path.curve_to(
        window_point(
            rect.x,
            rect.y + rect.height - radius,
            node_transform,
            fit,
            bounds,
        ),
        window_point(rect.x, rect.y + rect.height, node_transform, fit, bounds),
    );
    path.line_to(window_point(
        rect.x,
        rect.y + radius,
        node_transform,
        fit,
        bounds,
    ));
    path.curve_to(
        window_point(rect.x + radius, rect.y, node_transform, fit, bounds),
        window_point(rect.x, rect.y, node_transform, fit, bounds),
    );
    path.close();
    path
}

fn window_point(
    x: f32,
    y: f32,
    node_transform: Scene2DTransform,
    fit: Scene2DViewTransform,
    bounds: Bounds<Pixels>,
) -> gpui::Point<Pixels> {
    let transformed = transform_point(ScenePoint::new(x, y), node_transform);
    let viewport = fit.scene_to_viewport(transformed);
    let origin_x: f32 = bounds.origin.x.into();
    let origin_y: f32 = bounds.origin.y.into();
    point(px(origin_x + viewport.x), px(origin_y + viewport.y))
}

fn transform_point(point: ScenePoint, transform: Scene2DTransform) -> ScenePoint {
    let [a, b, c, d] = transform.linear();
    ScenePoint::new(
        a * point.x + b * point.y + transform.translate_x,
        c * point.x + d * point.y + transform.translate_y,
    )
}

fn stroke_width(node: &Scene2DNode, stroke: Scene2DStroke, transform: Scene2DViewTransform) -> f32 {
    stroke.width * transform.scale() * transform_scale(node.transform)
}

fn transform_scale(transform: Scene2DTransform) -> f32 {
    let [a, b, c, d] = transform.linear();
    let sum = a * a + b * b + c * c + d * d;
    let determinant = a * d - b * c;
    let discriminant = (sum * sum - 4.0 * determinant * determinant).max(0.0);
    f32::midpoint(sum, discriminant.sqrt()).sqrt()
}

fn is_uniform_orthogonal(transform: Scene2DTransform) -> bool {
    let [a, b, c, d] = transform.linear();
    let first_length = (a * a + c * c).sqrt();
    let second_length = (b * b + d * d).sqrt();
    let dot = a * b + c * d;
    (first_length - second_length).abs() < 1e-4 && dot.abs() < 1e-4 && first_length > f32::EPSILON
}

fn rgba_color(color: Scene2DColor, opacity: f32) -> Rgba {
    Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a * opacity,
    }
}

fn hash_debug(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn background_for(brush: Scene2DBrush, opacity: f32) -> gpui::Background {
    match brush {
        Scene2DBrush::Solid { color } => rgba_color(color, opacity).into(),
        Scene2DBrush::LinearGradient {
            angle_degrees,
            from,
            to,
        } => linear_gradient(
            angle_degrees.rem_euclid(360.0),
            linear_color_stop(rgba_color(from, opacity), 0.0),
            linear_color_stop(rgba_color(to, opacity), 1.0),
        ),
    }
}

// Standard cubic approximation constant for a quarter-circle Bézier curve.
const CIRCLE_KAPPA: f32 = 0.552_284_8;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene2d::{Scene2DScene, SceneRect};

    #[test]
    fn focusable_surface_accepts_empty_valid_scene() {
        let scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 10.0, 10.0));
        let state = Scene2DState::new(scene).unwrap();
        let surface = GameSurface::from_state("surface", state)
            .aria_label("Test board")
            .aria_role(AriaRole::Group)
            .on_input(|_, _, _| {});
        assert!(format!("{surface:?}").contains("GameSurface"));
    }

    #[test]
    fn synthetic_semantic_actions_are_limited_to_enabled_interactive_targets() {
        let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 10.0, 10.0));
        let mut semantic = Scene2DSemantic {
            role: Scene2DSemanticRole::GridCell,
            label: "cell".to_owned(),
            ..Default::default()
        };

        assert!(semantic_is_actionable(&semantic, &scene, true));
        semantic.role = Scene2DSemanticRole::Button;
        assert!(semantic_is_actionable(&semantic, &scene, true));
        semantic.role = Scene2DSemanticRole::Status;
        assert!(!semantic_is_actionable(&semantic, &scene, true));
        semantic.role = Scene2DSemanticRole::GridCell;
        semantic.disabled = Some(true);
        assert!(!semantic_is_actionable(&semantic, &scene, true));
        semantic.disabled = None;
        scene.input.pointer = false;
        scene.input.keyboard = false;
        assert!(!semantic_is_actionable(&semantic, &scene, true));
        scene.input.pointer = true;
        assert!(!semantic_is_actionable(&semantic, &scene, false));
    }

    #[test]
    fn unanchored_center_and_right_text_shift_by_the_line_width() {
        let origin = point(px(100.0), px(50.0));
        let centered = aligned_text_origin(origin, Scene2DTextAlign::Center, px(20.0), None);
        assert_eq!(centered, point(px(90.0), px(50.0)));
        let right = aligned_text_origin(origin, Scene2DTextAlign::Right, px(20.0), None);
        assert_eq!(right, point(px(80.0), px(50.0)));
        let left = aligned_text_origin(origin, Scene2DTextAlign::Left, px(20.0), None);
        assert_eq!(left, origin);
        let bounded =
            aligned_text_origin(origin, Scene2DTextAlign::Center, px(20.0), Some(px(60.0)));
        assert_eq!(bounded, origin);
    }

    #[test]
    fn synthetic_semantic_bounds_use_scaled_window_coordinates() {
        let bounds = Bounds::new(point(px(10.0), px(20.0)), size(px(100.0), px(100.0)));
        let fit =
            Scene2DViewTransform::contain(SceneRect::new(0.0, 0.0, 100.0, 100.0), 100.0, 100.0)
                .unwrap();
        let native = scene_rect_to_native_bounds(
            SceneRect::new(2.0, 3.0, 10.0, 12.0),
            Scene2DTransform::identity(),
            fit,
            bounds,
            2.5,
        );

        assert_eq!(native.x0, 30.0);
        assert_eq!(native.y0, 57.5);
        assert_eq!(native.x1, 55.0);
        assert_eq!(native.y1, 87.5);
    }
}
