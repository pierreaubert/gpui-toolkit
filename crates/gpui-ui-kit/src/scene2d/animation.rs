//! Retained Scene2D state and native transform/opacity animation.

// Rust guideline compliant 2026-02-21

use super::input::{Scene2DInput, Scene2DInputRouter, Scene2DLifecycleReason, Scene2DPointerEvent};
use super::types::{
    Scene2DNode, Scene2DNodeKind, Scene2DPathCommand, Scene2DScene, Scene2DStroke,
    Scene2DTransform, Scene2DTransition, Scene2DValidationError,
};
use crate::animation::{Easing, ease};
use gpui::{App, FocusHandle, Path, Pixels, ShapedLine, Subscription, Window};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::time::Duration;
use web_time::Instant;

const MAX_SCENE2D_CACHE_ENTRIES: usize = 512;

/// Shared mutable scene state for one surface.
#[derive(Clone)]
pub struct Scene2DState {
    inner: Rc<RefCell<Scene2DStateInner>>,
    focus_handle: Rc<RefCell<Option<FocusHandle>>>,
    focus_subscription: Rc<RefCell<Option<Subscription>>>,
    input_handler: Rc<RefCell<Option<Scene2DInputHandler>>>,
}

pub(crate) type Scene2DInputHandler = Rc<dyn Fn(Scene2DInput, &Window, &mut App) + 'static>;

struct Scene2DStateInner {
    scene: Scene2DScene,
    animations: HashMap<String, NodeMotion>,
    removals: Vec<Scene2DRemoval>,
    completed_transitions: VecDeque<Scene2DInput>,
    path_cache: HashMap<String, CacheEntry<Path<Pixels>>>,
    text_cache: HashMap<String, CacheEntry<ShapedLine>>,
    input: Scene2DInputRouter,
    created_at: Instant,
    reduced_motion: bool,
    suspended_at: Option<Duration>,
    platform_timestamp_anchor: Option<(u64, u64)>,
    last_timestamp_ns: u64,
}

#[derive(Clone)]
struct CacheEntry<T> {
    key: u64,
    value: T,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MotionValue {
    transform: Scene2DTransform,
    opacity: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct NodeMotion {
    from: MotionValue,
    to: MotionValue,
    from_kind: Scene2DNodeKind,
    to_kind: Scene2DNodeKind,
    transition: Scene2DTransition,
    started_at: Duration,
    duration: Duration,
    easing: Easing,
}

#[derive(Debug, Clone)]
struct Scene2DRemoval {
    node: Scene2DNode,
    motion: NodeMotion,
}

impl Scene2DState {
    /// Creates retained state after validating a complete scene snapshot.
    ///
    /// # Errors
    ///
    /// Returns a validation error when the scene contains malformed geometry or identities.
    pub fn new(scene: Scene2DScene) -> Result<Self, Scene2DValidationError> {
        scene.validate()?;
        Ok(Self {
            inner: Rc::new(RefCell::new(Scene2DStateInner {
                scene,
                animations: HashMap::new(),
                removals: Vec::new(),
                completed_transitions: VecDeque::new(),
                path_cache: HashMap::new(),
                text_cache: HashMap::new(),
                input: Scene2DInputRouter::new(),
                created_at: Instant::now(),
                reduced_motion: false,
                suspended_at: None,
                platform_timestamp_anchor: None,
                last_timestamp_ns: 0,
            })),
            focus_handle: Rc::new(RefCell::new(None)),
            focus_subscription: Rc::new(RefCell::new(None)),
            input_handler: Rc::new(RefCell::new(None)),
        })
    }

    pub(crate) fn focus_handle(&self, cx: &mut App) -> FocusHandle {
        let mut focus_handle = self.focus_handle.borrow_mut();
        focus_handle
            .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone()
    }

    pub(crate) fn set_input_handler(&self, handler: Option<Scene2DInputHandler>) {
        *self.input_handler.borrow_mut() = handler;
    }

    pub(crate) fn has_focus_subscription(&self) -> bool {
        self.focus_subscription.borrow().is_some()
    }

    pub(crate) fn weak_input_handler(&self) -> std::rc::Weak<RefCell<Option<Scene2DInputHandler>>> {
        Rc::downgrade(&self.input_handler)
    }

    /// Returns the current target scene, including its validated revision.
    pub fn scene(&self) -> Scene2DScene {
        self.inner.borrow().scene.clone()
    }

    /// Replaces the target scene and retargets only changed nodes.
    ///
    /// When a node already animates, the new transition starts at its current
    /// presented transform and opacity. Removed nodes lose their active motion.
    ///
    /// # Errors
    ///
    /// Returns a validation error without changing the previous valid scene.
    pub fn replace_scene(&self, scene: Scene2DScene) -> Result<(), Scene2DValidationError> {
        scene.validate()?;
        let mut inner = self.inner.borrow_mut();
        if scene.revision < inner.scene.revision {
            return Err(Scene2DValidationError::new(
                "revision",
                "scene revision cannot move backwards",
            ));
        }
        let now = effective_elapsed(&inner);
        let existing_removals = inner
            .removals
            .iter()
            .map(|removal| (removal.node.id.clone(), removal.clone()))
            .collect::<HashMap<_, _>>();
        let mut old_order = presented_nodes(&inner.scene, &inner.animations, now);
        for removal in &inner.removals {
            let mut node = removal.node.clone();
            apply_motion(&mut node, &removal.motion, now);
            old_order.push(node);
        }
        let old_by_id = old_order
            .iter()
            .map(|node| (node.id.clone(), node.clone()))
            .collect::<HashMap<_, _>>();
        let target_ids = scene
            .nodes
            .iter()
            .flat_map(all_node_ids)
            .collect::<std::collections::HashSet<_>>();
        let mut next_animations = HashMap::new();
        let mut next_removals = Vec::new();
        for target in &scene.nodes {
            let Some(transition) = target.transition.clone() else {
                continue;
            };
            let to = motion_value(target);
            let previous = old_by_id.get(&target.id);
            let from = previous.map(motion_value).unwrap_or(MotionValue {
                transform: target.transform,
                opacity: 0.0,
            });
            let from_kind = previous
                .map(|node| node.kind.clone())
                .unwrap_or_else(|| target.kind.clone());
            let changed = from != to
                || (transition.animate_color && from_kind != target.kind)
                || (transition.reveal_path && from_kind != target.kind);
            if !changed {
                continue;
            }
            let duration = Duration::from_millis(transition.duration_ms);
            let easing = scene_easing(&transition);
            if let Some(existing) = inner.animations.get(&target.id).cloned()
                && existing.to == to
                && existing.to_kind == target.kind
                && existing.transition == transition
                && existing.duration == duration
                && existing.easing == easing
            {
                next_animations.insert(target.id.clone(), existing);
                continue;
            }
            if inner.reduced_motion || duration.is_zero() {
                queue_transition_completion(&mut inner, target.id.clone(), &transition);
                continue;
            }
            next_animations.insert(
                target.id.clone(),
                NodeMotion {
                    from,
                    to,
                    from_kind,
                    to_kind: target.kind.clone(),
                    transition,
                    started_at: now,
                    duration,
                    easing,
                },
            );
        }
        if !inner.reduced_motion {
            for node in &old_order {
                if target_ids.contains(node.id.as_str()) {
                    continue;
                }
                if let Some(existing) = existing_removals.get(&node.id) {
                    let mut existing = existing.clone();
                    existing.node = node.clone();
                    next_removals.push(existing);
                    continue;
                }
                let Some(transition) = node.transition.clone() else {
                    continue;
                };
                if transition.duration_ms == 0 || node.opacity <= 0.0 {
                    continue;
                }
                let from = motion_value(node);
                next_removals.push(Scene2DRemoval {
                    node: node.clone(),
                    motion: NodeMotion {
                        from,
                        to: MotionValue {
                            transform: node.transform,
                            opacity: 0.0,
                        },
                        from_kind: node.kind.clone(),
                        to_kind: node.kind.clone(),
                        transition: transition.clone(),
                        started_at: now,
                        duration: Duration::from_millis(transition.duration_ms),
                        easing: scene_easing(&transition),
                    },
                });
            }
        }
        inner.scene = scene;
        inner.animations = next_animations;
        inner.removals = next_removals;
        prune_scene_caches(&mut inner);
        Ok(())
    }

    /// Sets reduced-motion behavior and snaps active transitions to their targets.
    pub fn set_reduced_motion(&self, reduced_motion: bool) {
        let mut inner = self.inner.borrow_mut();
        if reduced_motion {
            inner.animations.clear();
            inner.removals.clear();
        }
        inner.reduced_motion = reduced_motion;
    }

    /// Pauses or resumes native transitions while the containing scene is inactive.
    ///
    /// Returns `true` when resuming an active animation so the caller can request
    /// a fresh frame after the surface becomes visible again.
    pub fn set_suspended(&self, suspended: bool) -> bool {
        let mut inner = self.inner.borrow_mut();
        if suspended {
            if inner.suspended_at.is_none() {
                inner.suspended_at = Some(inner.created_at.elapsed());
            }
            return false;
        }

        let Some(suspended_at) = inner.suspended_at.take() else {
            return false;
        };
        let now = inner.created_at.elapsed();
        let paused_for = now.saturating_sub(suspended_at);
        for motion in inner.animations.values_mut() {
            motion.started_at += paused_for;
        }
        for removal in &mut inner.removals {
            removal.motion.started_at += paused_for;
        }
        !inner.animations.is_empty() || !inner.removals.is_empty()
    }

    /// Returns a scene snapshot with current native interpolated transforms.
    pub fn presented_scene(&self) -> (Scene2DScene, bool) {
        let mut inner = self.inner.borrow_mut();
        let now = effective_elapsed(&inner);
        let target = inner.scene.clone();
        let (mut scene, completed) = presented_scene(&target, &mut inner.animations, now);
        for (id, completion_id) in completed {
            queue_transition_complete(&mut inner, id, completion_id);
        }
        let mut active_removals = Vec::with_capacity(inner.removals.len());
        for mut removal in std::mem::take(&mut inner.removals) {
            apply_motion(&mut removal.node, &removal.motion, now);
            if now.saturating_sub(removal.motion.started_at) < removal.motion.duration {
                scene.nodes.push(removal.node.clone());
                active_removals.push(removal);
            } else if let Some(completion_id) = removal.motion.transition.completion_id.clone() {
                queue_transition_complete(&mut inner, removal.node.id.clone(), completion_id);
            }
        }
        inner.removals = active_removals;
        let active = !inner.animations.is_empty() || !inner.removals.is_empty();
        (scene, active && inner.suspended_at.is_none())
    }

    /// Routes an external mouse or direct-contact event through this surface.
    pub fn route_pointer(&self, event: Scene2DPointerEvent) -> Vec<Scene2DInput> {
        let mut inner = self.inner.borrow_mut();
        let mut scene = inner.scene.clone();
        let now = effective_elapsed(&inner);
        scene.nodes = presented_nodes(&scene, &inner.animations, now);
        let mut event = event;
        event.timestamp_ns = normalize_platform_timestamp(&mut inner, event.timestamp_ns);
        inner.input.route_pointer(&scene, event)
    }

    /// Routes a focused key event through this surface.
    pub fn route_key(
        &self,
        phase: super::input::Scene2DKeyPhase,
        key: &str,
        repeat: bool,
        modifiers: Vec<super::input::Scene2DModifier>,
    ) -> Option<Scene2DInput> {
        let mut inner = self.inner.borrow_mut();
        let config = inner.scene.input.clone();
        let timestamp_ns = next_local_timestamp(&mut inner);
        inner
            .input
            .route_key(&config, phase, key, repeat, modifiers, timestamp_ns)
    }

    /// Clears captured pointers and held keys for a focus or lifecycle boundary.
    pub fn cancel_inputs(&self, reason: Scene2DLifecycleReason) -> Vec<Scene2DInput> {
        let mut inner = self.inner.borrow_mut();
        let timestamp_ns = next_local_timestamp(&mut inner);
        inner.input.cancel_all(reason, timestamp_ns)
    }

    /// Cancels active input and removes all scene objects and animations.
    pub fn remove(&self) -> Vec<Scene2DInput> {
        let mut inner = self.inner.borrow_mut();
        let timestamp_ns = next_local_timestamp(&mut inner);
        let cancelled = inner
            .input
            .cancel_all(Scene2DLifecycleReason::Removed, timestamp_ns);
        inner.animations.clear();
        inner.removals.clear();
        inner.scene.nodes.clear();
        inner.scene.grid = None;
        inner.scene.semantic = None;
        inner.scene.background = None;
        inner.scene.input.pointer = false;
        inner.scene.input.continuous = false;
        inner.scene.input.capture = false;
        inner.scene.input.keyboard = false;
        inner.path_cache.clear();
        inner.text_cache.clear();
        inner.completed_transitions.clear();
        drop(inner);
        *self.focus_subscription.borrow_mut() = None;
        *self.focus_handle.borrow_mut() = None;
        *self.input_handler.borrow_mut() = None;
        cancelled
    }

    /// Returns whether the current scene still has native animation work.
    pub fn is_animating(&self) -> bool {
        self.presented_scene().1
    }

    /// Returns the monotonic nanosecond clock used for emitted input events.
    pub fn timestamp_ns(&self) -> u64 {
        let mut inner = self.inner.borrow_mut();
        next_local_timestamp(&mut inner)
    }

    /// Drains completion events queued while preparing the latest presented scene.
    pub(crate) fn take_transition_completions(&self) -> Vec<Scene2DInput> {
        self.inner
            .borrow_mut()
            .completed_transitions
            .drain(..)
            .collect()
    }

    /// Returns the number of retained path and text shaping cache entries.
    pub fn cache_entry_counts(&self) -> (usize, usize) {
        let inner = self.inner.borrow();
        (inner.path_cache.len(), inner.text_cache.len())
    }

    pub(crate) fn cached_path(
        &self,
        cache_id: &str,
        key: u64,
        build: impl FnOnce() -> Option<Path<Pixels>>,
    ) -> Option<Path<Pixels>> {
        if let Some(entry) = self.inner.borrow().path_cache.get(cache_id)
            && entry.key == key
        {
            return Some(entry.value.clone());
        }
        let path = build()?;
        let mut inner = self.inner.borrow_mut();
        trim_cache(&mut inner.path_cache);
        inner.path_cache.insert(
            cache_id.to_owned(),
            CacheEntry {
                key,
                value: path.clone(),
            },
        );
        Some(path)
    }

    pub(crate) fn cached_text_line(
        &self,
        node_id: &str,
        key: u64,
        build: impl FnOnce() -> ShapedLine,
    ) -> ShapedLine {
        if let Some(entry) = self.inner.borrow().text_cache.get(node_id)
            && entry.key == key
        {
            return entry.value.clone();
        }
        let line = build();
        let mut inner = self.inner.borrow_mut();
        trim_cache(&mut inner.text_cache);
        inner.text_cache.insert(
            node_id.to_owned(),
            CacheEntry {
                key,
                value: line.clone(),
            },
        );
        line
    }

    /// Returns whether this surface is configured to capture active pointers.
    pub fn wants_pointer_capture(&self) -> bool {
        let inner = self.inner.borrow();
        inner.scene.input.pointer && inner.scene.input.capture
    }

    pub(crate) fn set_focus_subscription(&self, subscription: Subscription) {
        *self.focus_subscription.borrow_mut() = Some(subscription);
    }

    pub(crate) fn lifecycle_callback(
        &self,
    ) -> impl Fn(Scene2DLifecycleReason) -> Vec<Scene2DInput> + 'static {
        let inner = Rc::downgrade(&self.inner);
        move |reason| {
            let Some(inner) = inner.upgrade() else {
                return Vec::new();
            };
            let mut inner = inner.borrow_mut();
            let timestamp_ns = next_local_timestamp(&mut inner);
            inner.input.cancel_all(reason, timestamp_ns)
        }
    }

    pub(crate) fn route_pointer_for_scene(
        &self,
        scene: &Scene2DScene,
        event: Scene2DPointerEvent,
    ) -> Vec<Scene2DInput> {
        let mut inner = self.inner.borrow_mut();
        let mut event = event;
        event.timestamp_ns = normalize_platform_timestamp(&mut inner, event.timestamp_ns);
        inner.input.route_pointer(scene, event)
    }

    pub(crate) fn route_key_for_scene(
        &self,
        scene: &Scene2DScene,
        phase: super::input::Scene2DKeyPhase,
        key: &str,
        repeat: bool,
        modifiers: Vec<super::input::Scene2DModifier>,
    ) -> Option<Scene2DInput> {
        let mut inner = self.inner.borrow_mut();
        let timestamp_ns = next_local_timestamp(&mut inner);
        inner
            .input
            .route_key(&scene.input, phase, key, repeat, modifiers, timestamp_ns)
    }
}

impl std::fmt::Debug for Scene2DState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.borrow();
        f.debug_struct("Scene2DState")
            .field("revision", &inner.scene.revision)
            .field("nodes", &inner.scene.nodes.len())
            .field("active_animations", &inner.animations.len())
            .field("reduced_motion", &inner.reduced_motion)
            .finish()
    }
}

fn effective_elapsed(inner: &Scene2DStateInner) -> Duration {
    inner
        .suspended_at
        .unwrap_or_else(|| inner.created_at.elapsed())
}

fn next_local_timestamp(inner: &mut Scene2DStateInner) -> u64 {
    let candidate = effective_elapsed(inner)
        .as_nanos()
        .min(u128::from(u64::MAX)) as u64;
    let timestamp_ns = candidate.max(inner.last_timestamp_ns);
    inner.last_timestamp_ns = timestamp_ns;
    timestamp_ns
}

fn normalize_platform_timestamp(inner: &mut Scene2DStateInner, platform_timestamp_ns: u64) -> u64 {
    let local_timestamp_ns = next_local_timestamp(inner);
    let (platform_origin, local_origin) = *inner
        .platform_timestamp_anchor
        .get_or_insert((platform_timestamp_ns, local_timestamp_ns));
    let candidate =
        local_origin.saturating_add(platform_timestamp_ns.saturating_sub(platform_origin));
    let timestamp_ns = candidate.max(inner.last_timestamp_ns);
    inner.last_timestamp_ns = timestamp_ns;
    timestamp_ns
}

fn queue_transition_completion(
    inner: &mut Scene2DStateInner,
    id: String,
    transition: &Scene2DTransition,
) {
    if let Some(completion_id) = transition.completion_id.clone() {
        queue_transition_complete(inner, id, completion_id);
    }
}

fn queue_transition_complete(inner: &mut Scene2DStateInner, id: String, completion_id: String) {
    let timestamp_ns = next_local_timestamp(inner);
    inner
        .completed_transitions
        .push_back(Scene2DInput::TransitionComplete {
            id,
            completion_id,
            timestamp_ns,
        });
}

fn all_node_ids(node: &Scene2DNode) -> Vec<&str> {
    let mut ids = vec![node.id.as_str()];
    if let Scene2DNodeKind::Group { children } = &node.kind {
        for child in children {
            ids.extend(all_node_ids(child));
        }
    }
    ids
}

fn prune_scene_caches(inner: &mut Scene2DStateInner) {
    let node_ids = inner
        .scene
        .nodes
        .iter()
        .flat_map(all_node_ids)
        .collect::<std::collections::HashSet<_>>();
    inner
        .text_cache
        .retain(|node_id, _| node_ids.contains(node_id.as_str()));
    inner.path_cache.retain(|cache_id, _| {
        cache_id
            .split('\0')
            .next()
            .is_some_and(|node_id| node_ids.contains(node_id))
    });
}

fn trim_cache<T>(cache: &mut HashMap<String, CacheEntry<T>>) {
    if cache.len() >= MAX_SCENE2D_CACHE_ENTRIES
        && let Some(key) = cache.keys().next().cloned()
    {
        cache.remove(&key);
    }
}

fn presented_nodes(
    scene: &Scene2DScene,
    animations: &HashMap<String, NodeMotion>,
    now: Duration,
) -> Vec<Scene2DNode> {
    scene
        .nodes
        .iter()
        .cloned()
        .map(|mut node| {
            if let Some(motion) = animations.get(&node.id) {
                apply_motion(&mut node, motion, now);
            }
            node
        })
        .collect()
}

fn presented_scene(
    target: &Scene2DScene,
    animations: &mut HashMap<String, NodeMotion>,
    now: Duration,
) -> (Scene2DScene, Vec<(String, String)>) {
    let mut scene = target.clone();
    let mut completed = Vec::new();
    scene.nodes = target
        .nodes
        .iter()
        .cloned()
        .map(|mut node| {
            if let Some(motion) = animations.get(&node.id).cloned() {
                apply_motion(&mut node, &motion, now);
                if now.saturating_sub(motion.started_at) >= motion.duration {
                    animations.remove(&node.id);
                    if let Some(completion_id) = motion.transition.completion_id {
                        completed.push((node.id.clone(), completion_id));
                    }
                }
            }
            node
        })
        .collect();
    (scene, completed)
}

fn apply_motion(node: &mut Scene2DNode, motion: &NodeMotion, now: Duration) {
    let progress = if motion.duration.is_zero() {
        1.0
    } else {
        (now.saturating_sub(motion.started_at).as_secs_f64() / motion.duration.as_secs_f64())
            .clamp(0.0, 1.0) as f32
    };
    let t = ease(motion.easing, progress);
    node.transform = interpolate_transform(motion.from.transform, motion.to.transform, t);
    node.opacity = lerp(motion.from.opacity, motion.to.opacity, t).clamp(0.0, 1.0);
    node.kind = if motion.transition.animate_color {
        interpolate_kind(&motion.from_kind, &motion.to_kind, t)
    } else {
        motion.to_kind.clone()
    };
    if motion.transition.reveal_path
        && let Scene2DNodeKind::Path { commands, .. } = &mut node.kind
    {
        *commands = reveal_path_commands(commands, t);
    }
}

fn interpolate_kind(from: &Scene2DNodeKind, to: &Scene2DNodeKind, t: f32) -> Scene2DNodeKind {
    use Scene2DNodeKind as Kind;
    match (from, to) {
        (
            Kind::Rect {
                rect: _,
                fill: from_fill,
                stroke: from_stroke,
            },
            Kind::Rect { rect, fill, stroke },
        ) => Kind::Rect {
            rect: *rect,
            fill: interpolate_brush(*from_fill, *fill, t),
            stroke: interpolate_stroke(*from_stroke, *stroke, t),
        },
        (
            Kind::RoundedRect {
                fill: from_fill,
                stroke: from_stroke,
                ..
            },
            Kind::RoundedRect {
                rect,
                radius,
                fill,
                stroke,
            },
        ) => Kind::RoundedRect {
            rect: *rect,
            radius: *radius,
            fill: interpolate_brush(*from_fill, *fill, t),
            stroke: interpolate_stroke(*from_stroke, *stroke, t),
        },
        (
            Kind::Circle {
                fill: from_fill,
                stroke: from_stroke,
                ..
            },
            Kind::Circle {
                center,
                radius,
                fill,
                stroke,
            },
        ) => Kind::Circle {
            center: *center,
            radius: *radius,
            fill: interpolate_brush(*from_fill, *fill, t),
            stroke: interpolate_stroke(*from_stroke, *stroke, t),
        },
        (
            Kind::Line {
                stroke: from_stroke,
                ..
            },
            Kind::Line { start, end, stroke },
        ) => Kind::Line {
            start: *start,
            end: *end,
            stroke: interpolate_stroke(Some(*from_stroke), Some(*stroke), t).unwrap_or(*stroke),
        },
        (
            Kind::Path {
                fill: from_fill,
                stroke: from_stroke,
                ..
            },
            Kind::Path {
                commands,
                fill,
                stroke,
            },
        ) => Kind::Path {
            commands: commands.clone(),
            fill: interpolate_brush(*from_fill, *fill, t),
            stroke: interpolate_stroke(*from_stroke, *stroke, t),
        },
        (
            Kind::Text { .. },
            Kind::Text {
                origin,
                content,
                size,
                color,
                font,
                align,
            },
        ) => {
            let color = match from {
                Kind::Text {
                    color: from_color, ..
                } => interpolate_color(*from_color, *color, t),
                _ => *color,
            };
            Kind::Text {
                origin: *origin,
                content: content.clone(),
                size: *size,
                color,
                font: font.clone(),
                align: *align,
            }
        }
        (
            Kind::Group {
                children: from_children,
            },
            Kind::Group { children },
        ) => {
            let children = children
                .iter()
                .map(|to_child| {
                    let mut child = to_child.clone();
                    if let Some(from_child) = from_children
                        .iter()
                        .find(|from_child| from_child.id == to_child.id)
                    {
                        child.kind = interpolate_kind(&from_child.kind, &to_child.kind, t);
                    }
                    child
                })
                .collect();
            Kind::Group { children }
        }
        (_, kind) => kind.clone(),
    }
}

fn interpolate_color(
    from: super::types::Scene2DColor,
    to: super::types::Scene2DColor,
    t: f32,
) -> super::types::Scene2DColor {
    super::types::Scene2DColor {
        r: lerp(from.r, to.r, t),
        g: lerp(from.g, to.g, t),
        b: lerp(from.b, to.b, t),
        a: lerp(from.a, to.a, t),
    }
}

fn interpolate_brush(
    from: Option<super::types::Scene2DBrush>,
    to: Option<super::types::Scene2DBrush>,
    t: f32,
) -> Option<super::types::Scene2DBrush> {
    use super::types::Scene2DBrush as Brush;
    match (from, to) {
        (Some(Brush::Solid { color: from }), Some(Brush::Solid { color: to })) => {
            Some(Brush::Solid {
                color: interpolate_color(from, to, t),
            })
        }
        (
            Some(Brush::LinearGradient {
                angle_degrees: from_angle,
                from: from_color,
                to: from_to,
            }),
            Some(Brush::LinearGradient {
                angle_degrees: to_angle,
                from: to_color,
                to: to_to,
            }),
        ) => Some(Brush::LinearGradient {
            angle_degrees: lerp(from_angle, to_angle, t),
            from: interpolate_color(from_color, to_color, t),
            to: interpolate_color(from_to, to_to, t),
        }),
        (None, Some(brush)) => Some(with_brush_alpha(brush, t)),
        (Some(brush), None) => Some(with_brush_alpha(brush, 1.0 - t)),
        (_, brush) => brush,
    }
}

fn with_brush_alpha(brush: super::types::Scene2DBrush, opacity: f32) -> super::types::Scene2DBrush {
    use super::types::Scene2DBrush as Brush;
    match brush {
        Brush::Solid { mut color } => {
            color.a *= opacity;
            Brush::Solid { color }
        }
        Brush::LinearGradient {
            angle_degrees,
            mut from,
            mut to,
        } => {
            from.a *= opacity;
            to.a *= opacity;
            Brush::LinearGradient {
                angle_degrees,
                from,
                to,
            }
        }
    }
}

fn interpolate_stroke(
    from: Option<Scene2DStroke>,
    to: Option<Scene2DStroke>,
    t: f32,
) -> Option<Scene2DStroke> {
    match (from, to) {
        (Some(from), Some(to)) => Some(Scene2DStroke {
            width: to.width,
            color: interpolate_color(from.color, to.color, t),
        }),
        (None, Some(mut stroke)) => {
            stroke.color.a *= t;
            Some(stroke)
        }
        (Some(mut stroke), None) => {
            stroke.color.a *= 1.0 - t;
            Some(stroke)
        }
        (_, stroke) => stroke,
    }
}

fn reveal_path_commands(commands: &[Scene2DPathCommand], progress: f32) -> Vec<Scene2DPathCommand> {
    use Scene2DPathCommand as Command;
    if progress >= 1.0 {
        return commands.to_vec();
    }
    let segment_count = commands
        .iter()
        .filter(|command| {
            matches!(
                command,
                Command::LineTo { .. }
                    | Command::QuadraticTo { .. }
                    | Command::CubicTo { .. }
                    | Command::Close
            )
        })
        .count();
    if segment_count == 0 {
        return commands.iter().take(1).copied().collect();
    }
    let visible = progress.clamp(0.0, 1.0) * segment_count as f32;
    let full_segments = visible.floor() as usize;
    let partial = visible.fract();
    let mut segments = 0_usize;
    let mut current = None;
    let mut output = Vec::new();
    for command in commands {
        match *command {
            Command::MoveTo { point } => {
                current = Some(point);
                output.push(*command);
            }
            Command::LineTo { point } => {
                if segments < full_segments {
                    output.push(*command);
                    current = Some(point);
                } else if segments == full_segments && partial > 0.0 {
                    if let Some(start) = current {
                        output.push(Command::LineTo {
                            point: point_lerp(start, point, partial),
                        });
                    }
                    break;
                } else {
                    break;
                }
                segments += 1;
            }
            Command::QuadraticTo { control, point } => {
                if segments < full_segments {
                    output.push(*command);
                    current = Some(point);
                } else if segments == full_segments && partial > 0.0 {
                    if let Some(start) = current {
                        let control_a = point_lerp(start, control, partial);
                        let endpoint = point_lerp(control, point, partial);
                        let endpoint = point_lerp(control_a, endpoint, partial);
                        output.push(Command::QuadraticTo {
                            control: control_a,
                            point: endpoint,
                        });
                    }
                    break;
                } else {
                    break;
                }
                segments += 1;
            }
            Command::CubicTo {
                control_a,
                control_b,
                point,
            } => {
                if segments < full_segments {
                    output.push(*command);
                    current = Some(point);
                } else if segments == full_segments && partial > 0.0 {
                    if let Some(start) = current {
                        let a = point_lerp(start, control_a, partial);
                        let b = point_lerp(control_a, control_b, partial);
                        let c = point_lerp(control_b, point, partial);
                        let d = point_lerp(a, b, partial);
                        let e = point_lerp(b, c, partial);
                        let endpoint = point_lerp(d, e, partial);
                        output.push(Command::CubicTo {
                            control_a: a,
                            control_b: d,
                            point: endpoint,
                        });
                    }
                    break;
                } else {
                    break;
                }
                segments += 1;
            }
            Command::Close => {
                if segments < full_segments {
                    output.push(*command);
                } else {
                    break;
                }
                segments += 1;
            }
        }
    }
    output
}

fn point_lerp(
    from: super::types::ScenePoint,
    to: super::types::ScenePoint,
    t: f32,
) -> super::types::ScenePoint {
    super::types::ScenePoint::new(lerp(from.x, to.x, t), lerp(from.y, to.y, t))
}

fn interpolate_transform(from: Scene2DTransform, to: Scene2DTransform, t: f32) -> Scene2DTransform {
    Scene2DTransform {
        translate_x: lerp(from.translate_x, to.translate_x, t),
        translate_y: lerp(from.translate_y, to.translate_y, t),
        scale_x: lerp(from.scale_x, to.scale_x, t),
        scale_y: lerp(from.scale_y, to.scale_y, t),
        shear_x: lerp(from.shear_x, to.shear_x, t),
        rotation_degrees: lerp(from.rotation_degrees, to.rotation_degrees, t),
    }
}

fn motion_value(node: &Scene2DNode) -> MotionValue {
    MotionValue {
        transform: node.transform,
        opacity: node.opacity,
    }
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

fn scene_easing(transition: &Scene2DTransition) -> Easing {
    use super::types::Scene2DEasing;
    match transition.easing {
        Scene2DEasing::Linear => Easing::Linear,
        Scene2DEasing::EaseOutQuad => Easing::EaseOutQuad,
        Scene2DEasing::EaseOutCubic => Easing::EaseOutCubic,
        Scene2DEasing::EaseInOutCubic => Easing::EaseInOutCubic,
        Scene2DEasing::EaseOutBack => Easing::EaseOutBack,
        Scene2DEasing::EaseOutBounce => Easing::EaseOutBounce,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene2d::{Scene2DBrush, Scene2DColor, Scene2DNodeKind, Scene2DStroke, SceneRect};

    fn scene(revision: u64, x: f32, transition: Option<Scene2DTransition>) -> Scene2DScene {
        let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 100.0, 100.0));
        scene.revision = revision;
        scene.nodes.push(Scene2DNode {
            id: "piece".to_owned(),
            hit_id: None,
            hit_bounds: None,
            clip: None,
            shadow: None,
            semantic: None,
            transform: Scene2DTransform::translation(x, 0.0),
            opacity: 1.0,
            transition,
            kind: Scene2DNodeKind::Rect {
                rect: SceneRect::new(0.0, 0.0, 10.0, 10.0),
                fill: Some(Scene2DBrush::solid(Scene2DColor::rgb(0.3, 0.6, 0.9))),
                stroke: None,
            },
        });
        scene
    }

    #[test]
    fn transition_retargets_from_current_presented_value() {
        let state = Scene2DState::new(scene(1, 0.0, None)).unwrap();
        let target = scene(
            2,
            20.0,
            Some(Scene2DTransition {
                duration_ms: 200,
                easing: super::super::types::Scene2DEasing::Linear,
                ..Default::default()
            }),
        );
        state.replace_scene(target).unwrap();
        let (_, active) = state.presented_scene();
        assert!(active);
        let retarget = scene(
            3,
            40.0,
            Some(Scene2DTransition {
                duration_ms: 200,
                easing: super::super::types::Scene2DEasing::Linear,
                ..Default::default()
            }),
        );
        state.replace_scene(retarget).unwrap();
        let (presented, active) = state.presented_scene();
        assert!(active);
        assert!(presented.nodes[0].transform.translate_x < 40.0);
    }

    #[test]
    fn unrelated_scene_revisions_do_not_restart_an_active_transition() {
        let state = Scene2DState::new(scene(1, 0.0, None)).unwrap();
        let transition = Some(Scene2DTransition {
            duration_ms: 100,
            easing: super::super::types::Scene2DEasing::Linear,
            ..Default::default()
        });
        state
            .replace_scene(scene(2, 20.0, transition.clone()))
            .unwrap();
        state.inner.borrow_mut().created_at = Instant::now() - Duration::from_millis(70);
        state.replace_scene(scene(3, 20.0, transition)).unwrap();
        state.inner.borrow_mut().created_at = Instant::now() - Duration::from_millis(120);
        let (presented, active) = state.presented_scene();
        assert!(!active);
        assert_eq!(presented.nodes[0].transform.translate_x, 20.0);
    }

    #[test]
    fn color_transition_interpolates_and_reports_completion_once() {
        let state = Scene2DState::new(scene(1, 0.0, None)).unwrap();
        let mut target = scene(
            2,
            0.0,
            Some(Scene2DTransition {
                duration_ms: 100,
                easing: super::super::types::Scene2DEasing::Linear,
                completion_id: Some("color-done".to_owned()),
                animate_color: true,
                ..Default::default()
            }),
        );
        if let Scene2DNodeKind::Rect { fill, .. } = &mut target.nodes[0].kind {
            *fill = Some(Scene2DBrush::solid(Scene2DColor::rgb(1.0, 0.0, 0.0)));
        }
        state.replace_scene(target).unwrap();
        state.inner.borrow_mut().created_at = Instant::now() - Duration::from_millis(50);
        let (presented, active) = state.presented_scene();
        assert!(active);
        let Scene2DNodeKind::Rect {
            fill: Some(Scene2DBrush::Solid { color }),
            ..
        } = presented.nodes[0].kind
        else {
            panic!("expected the scene fill to remain a solid brush");
        };
        assert!(color.r > 0.3 && color.r < 1.0);
        assert!(color.g < 0.6 && color.g > 0.0);

        state.inner.borrow_mut().created_at = Instant::now() - Duration::from_millis(110);
        let (_, active) = state.presented_scene();
        assert!(!active);
        let events = state.take_transition_completions();
        assert!(matches!(
            events.as_slice(),
            [Scene2DInput::TransitionComplete { id, completion_id, .. }]
                if id == "piece" && completion_id == "color-done"
        ));
        assert!(state.take_transition_completions().is_empty());
    }

    #[test]
    fn path_reveal_progressively_truncates_the_stroke_commands() {
        let mut initial = scene(1, 0.0, None);
        initial.nodes[0].kind = Scene2DNodeKind::Path {
            commands: vec![
                Scene2DPathCommand::MoveTo {
                    point: super::super::types::ScenePoint::new(0.0, 0.0),
                },
                Scene2DPathCommand::LineTo {
                    point: super::super::types::ScenePoint::new(5.0, 0.0),
                },
            ],
            fill: None,
            stroke: Some(Scene2DStroke {
                width: 1.0,
                color: Scene2DColor::rgb(1.0, 1.0, 1.0),
            }),
        };
        let state = Scene2DState::new(initial).unwrap();
        let mut target = scene(
            2,
            0.0,
            Some(Scene2DTransition {
                duration_ms: 100,
                easing: super::super::types::Scene2DEasing::Linear,
                reveal_path: true,
                ..Default::default()
            }),
        );
        target.nodes[0].kind = Scene2DNodeKind::Path {
            commands: vec![
                Scene2DPathCommand::MoveTo {
                    point: super::super::types::ScenePoint::new(0.0, 0.0),
                },
                Scene2DPathCommand::LineTo {
                    point: super::super::types::ScenePoint::new(10.0, 0.0),
                },
                Scene2DPathCommand::LineTo {
                    point: super::super::types::ScenePoint::new(20.0, 0.0),
                },
                Scene2DPathCommand::LineTo {
                    point: super::super::types::ScenePoint::new(30.0, 0.0),
                },
            ],
            fill: None,
            stroke: Some(Scene2DStroke {
                width: 1.0,
                color: Scene2DColor::rgb(1.0, 1.0, 1.0),
            }),
        };
        state.replace_scene(target).unwrap();
        state.inner.borrow_mut().created_at = Instant::now() - Duration::from_millis(50);
        let (presented, active) = state.presented_scene();
        assert!(active);
        let Scene2DNodeKind::Path { commands, .. } = &presented.nodes[0].kind else {
            panic!("expected a path");
        };
        assert_eq!(commands.len(), 3);
        assert!(
            matches!(commands[2], Scene2DPathCommand::LineTo { point } if point.x > 10.0 && point.x < 20.0)
        );
    }

    #[test]
    fn suspending_a_surface_freezes_animation_elapsed_time() {
        let state = Scene2DState::new(scene(1, 0.0, None)).unwrap();
        state
            .replace_scene(scene(
                2,
                20.0,
                Some(Scene2DTransition {
                    duration_ms: 100,
                    easing: super::super::types::Scene2DEasing::Linear,
                    ..Default::default()
                }),
            ))
            .unwrap();
        state.inner.borrow_mut().created_at = Instant::now() - Duration::from_millis(40);
        state.set_suspended(true);
        state.inner.borrow_mut().created_at = Instant::now() - Duration::from_secs(2);
        assert!(state.set_suspended(false));
        let (presented, active) = state.presented_scene();
        assert!(active);
        assert!(presented.nodes[0].transform.translate_x < 20.0);
    }

    #[test]
    fn reduced_motion_and_removal_clear_animation_state() {
        let state = Scene2DState::new(scene(1, 0.0, None)).unwrap();
        state
            .replace_scene(scene(
                2,
                20.0,
                Some(Scene2DTransition {
                    duration_ms: 500,
                    easing: super::super::types::Scene2DEasing::EaseOutCubic,
                    ..Default::default()
                }),
            ))
            .unwrap();
        assert!(state.is_animating());
        state.set_reduced_motion(true);
        assert!(!state.is_animating());
        assert_eq!(
            state.presented_scene().0.nodes[0].transform.translate_x,
            20.0
        );
        state.remove();
        let removed_scene = state.scene();
        assert!(removed_scene.nodes.is_empty());
        assert!(removed_scene.background.is_none());
        assert!(!removed_scene.input.pointer);
        assert!(!removed_scene.input.keyboard);
    }

    #[test]
    fn removed_nodes_fade_out_then_leave_the_presented_scene() {
        let transition = Scene2DTransition {
            duration_ms: 500,
            easing: super::super::types::Scene2DEasing::Linear,
            ..Default::default()
        };
        let state = Scene2DState::new(scene(1, 0.0, Some(transition))).unwrap();
        let mut replacement = Scene2DScene::new(SceneRect::new(0.0, 0.0, 100.0, 100.0));
        replacement.revision = 2;
        state.replace_scene(replacement).unwrap();

        let (presented, active) = state.presented_scene();
        assert!(active);
        assert_eq!(presented.nodes.len(), 1);
        assert_eq!(presented.nodes[0].id, "piece");

        state.inner.borrow_mut().created_at = Instant::now() - Duration::from_millis(250);
        let mut another_replacement = Scene2DScene::new(SceneRect::new(0.0, 0.0, 100.0, 100.0));
        another_replacement.revision = 3;
        state.replace_scene(another_replacement).unwrap();

        state.inner.borrow_mut().created_at = Instant::now() - Duration::from_secs(1);
        let (presented, active) = state.presented_scene();
        assert!(!active);
        assert!(presented.nodes.is_empty());
    }

    #[test]
    fn invalid_and_stale_replacements_preserve_last_valid_scene() {
        let state = Scene2DState::new(scene(2, 1.0, None)).unwrap();
        assert!(state.replace_scene(scene(1, 3.0, None)).is_err());
        let invalid = Scene2DScene::new(SceneRect::new(0.0, 0.0, 0.0, 1.0));
        assert!(state.replace_scene(invalid).is_err());
        assert_eq!(state.scene().revision, 2);
        assert_eq!(state.scene().nodes[0].transform.translate_x, 1.0);
    }
}
