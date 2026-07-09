//! SwipePanel — a mobile bottom/top sheet that can be dragged up or down.
//!
//! The panel has three states:
//! - `Collapsed` — fully off-screen
//! - `Peek` — partially visible (a "half-hidden" handle and snippet of content)
//! - `Expanded` — fully visible up to the configured height
//!
//! Usage:
//! ```ignore
//! let panel = cx.new(|cx| {
//!     SwipePanel::new("my-panel")
//!         .content(my_content)
//!         .state(SwipePanelState::Peek)
//! });
//! parent.child(panel)
//! ```

use crate::animation::Spring;
use crate::mobile::{MomentumScroller, VelocityTracker};
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, Render, RenderOnce, StatefulInteractiveElement,
    Styled,
};
use gpui::{
    AnyElement, App, AppContext, Context, ElementId, Entity, FocusHandle, KeyDownEvent,
    MouseButton, Pixels, Rgba, WeakEntity, Window, div, px,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

thread_local! {
    static SWIPE_PANEL_ENTITIES: RefCell<HashMap<ElementId, WeakEntity<SwipePanelEntity>>> =
        RefCell::new(HashMap::new());
}

/// Where the panel is anchored on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwipePanelAnchor {
    /// Panel slides up from the bottom (default).
    #[default]
    Bottom,
    /// Panel slides down from the top.
    Top,
}

/// Visual state of the swipe panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwipePanelState {
    /// Fully hidden off-screen.
    Collapsed,
    /// Partially visible, e.g. a handle height.
    #[default]
    Peek,
    /// Fully visible up to the configured height.
    Expanded,
}

/// Builder for a swipe panel.
pub struct SwipePanel {
    id: ElementId,
    anchor: SwipePanelAnchor,
    state: SwipePanelState,
    peek_height: Pixels,
    expanded_height: Option<Pixels>,
    show_handle: bool,
    show_backdrop: bool,
    focus_handle: Option<FocusHandle>,
    restore_focus_to: Option<FocusHandle>,
    content: Option<AnyElement>,
    on_state_change: Option<Box<dyn Fn(SwipePanelState, &mut Window, &mut App) + 'static>>,
}

impl std::fmt::Debug for SwipePanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwipePanel")
            .field("id", &self.id)
            .field("anchor", &self.anchor)
            .field("state", &self.state)
            .field("peek_height", &self.peek_height)
            .field("expanded_height", &self.expanded_height)
            .field("show_handle", &self.show_handle)
            .field("show_backdrop", &self.show_backdrop)
            .field("has_focus_handle", &self.focus_handle.is_some())
            .field("has_restore_focus_to", &self.restore_focus_to.is_some())
            .field("has_content", &self.content.is_some())
            .field("has_on_state_change", &self.on_state_change.is_some())
            .finish()
    }
}

impl SwipePanel {
    /// Create a new swipe panel with the given element ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            anchor: SwipePanelAnchor::default(),
            state: SwipePanelState::default(),
            peek_height: px(72.0),
            expanded_height: None,
            show_handle: true,
            show_backdrop: true,
            focus_handle: None,
            restore_focus_to: None,
            content: None,
            on_state_change: None,
        }
    }

    /// Set the anchor edge.
    pub fn anchor(mut self, anchor: SwipePanelAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Set the initial/controlled state.
    pub fn state(mut self, state: SwipePanelState) -> Self {
        self.state = state;
        self
    }

    /// Set the visible height in the peek state.
    pub fn peek_height(mut self, height: Pixels) -> Self {
        self.peek_height = height;
        self
    }

    /// Set the height when expanded. Defaults to 85% of the viewport.
    pub fn expanded_height(mut self, height: Pixels) -> Self {
        self.expanded_height = Some(height);
        self
    }

    /// Show or hide the drag handle.
    pub fn show_handle(mut self, show: bool) -> Self {
        self.show_handle = show;
        self
    }

    /// Show or hide the backdrop overlay.
    pub fn show_backdrop(mut self, show: bool) -> Self {
        self.show_backdrop = show;
        self
    }

    /// Set the focus handle used for keyboard operation.
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Set the focus handle to restore when Escape or backdrop collapse hides the panel.
    pub fn restore_focus_to(mut self, handle: FocusHandle) -> Self {
        self.restore_focus_to = Some(handle);
        self
    }

    /// Set the panel content.
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    /// Set a callback invoked when the panel state changes.
    pub fn on_state_change(
        mut self,
        handler: impl Fn(SwipePanelState, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_state_change = Some(Box::new(handler));
        self
    }

    fn panel_height(&self, viewport: f32) -> f32 {
        self.expanded_height
            .map(f32::from)
            .unwrap_or(viewport * 0.85)
    }

    fn target_offset_for_state(&self, state: SwipePanelState, viewport: f32) -> f32 {
        let panel_height = self.panel_height(viewport);
        match state {
            SwipePanelState::Collapsed => -panel_height,
            SwipePanelState::Peek => f32::from(self.peek_height) - panel_height,
            SwipePanelState::Expanded => 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SwipePanelKeyboardAction {
    ExpandStep,
    CollapseStep,
    ExpandFully,
    CollapseFully,
    Toggle,
}

impl SwipePanelKeyboardAction {
    fn from_key(key: &str, anchor: SwipePanelAnchor) -> Option<Self> {
        match key {
            "escape" | "end" => Some(Self::CollapseFully),
            "home" => Some(Self::ExpandFully),
            "enter" | "space" | " " => Some(Self::Toggle),
            "up" | "arrowup" => Some(match anchor {
                SwipePanelAnchor::Bottom => Self::ExpandStep,
                SwipePanelAnchor::Top => Self::CollapseStep,
            }),
            "down" | "arrowdown" => Some(match anchor {
                SwipePanelAnchor::Bottom => Self::CollapseStep,
                SwipePanelAnchor::Top => Self::ExpandStep,
            }),
            _ => None,
        }
    }
}

fn keyboard_target_state(
    anchor: SwipePanelAnchor,
    current: SwipePanelState,
    key: &str,
) -> Option<SwipePanelState> {
    match SwipePanelKeyboardAction::from_key(key, anchor)? {
        SwipePanelKeyboardAction::ExpandStep => Some(match current {
            SwipePanelState::Collapsed => SwipePanelState::Peek,
            SwipePanelState::Peek | SwipePanelState::Expanded => SwipePanelState::Expanded,
        }),
        SwipePanelKeyboardAction::CollapseStep => Some(match current {
            SwipePanelState::Expanded => SwipePanelState::Peek,
            SwipePanelState::Peek | SwipePanelState::Collapsed => SwipePanelState::Collapsed,
        }),
        SwipePanelKeyboardAction::ExpandFully => Some(SwipePanelState::Expanded),
        SwipePanelKeyboardAction::CollapseFully => Some(SwipePanelState::Collapsed),
        SwipePanelKeyboardAction::Toggle => Some(match current {
            SwipePanelState::Collapsed => SwipePanelState::Peek,
            SwipePanelState::Peek => SwipePanelState::Expanded,
            SwipePanelState::Expanded => SwipePanelState::Peek,
        }),
    }
}

/// Internal entity that owns the swipe panel's interactive state.
pub struct SwipePanelEntity {
    props: SwipePanel,
    content: Entity<SwipePanelContentEntity>,
    state: SwipePanelState,
    current_offset: f32,
    target_offset: f32,
    velocity: f32,
    tracker: VelocityTracker,
    scroller: MomentumScroller,
    dragging: bool,
    drag_start_pos: f32,
    drag_start_offset: f32,
    drag_distance: f32,
    animating: bool,
    last_anim_time: Instant,
}

struct SwipePanelContentEntity {
    content: Option<AnyElement>,
}

impl Render for SwipePanelContentEntity {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut content_area = div().flex_1().min_h_0().w_full();
        if let Some(content) = self.content.take() {
            content_area = content_area.child(content);
        }
        content_area
    }
}

impl SwipePanelEntity {
    fn new(props: SwipePanel, cx: &mut Context<Self>) -> Self {
        let content = cx.new(|_cx| SwipePanelContentEntity { content: None });
        Self {
            content,
            state: props.state,
            current_offset: 0.0,
            target_offset: 0.0,
            velocity: 0.0,
            tracker: VelocityTracker::new(),
            scroller: MomentumScroller::new(),
            dragging: false,
            drag_start_pos: 0.0,
            drag_start_offset: 0.0,
            drag_distance: 0.0,
            animating: false,
            last_anim_time: Instant::now(),
            props,
        }
    }

    fn viewport_height(&self, window: &mut Window) -> f32 {
        window.viewport_size().height.into()
    }

    fn panel_height(&self, window: &mut Window) -> f32 {
        self.props.panel_height(self.viewport_height(window))
    }

    fn update_target(&mut self, window: &mut Window) {
        self.target_offset = self
            .props
            .target_offset_for_state(self.state, self.viewport_height(window));
    }

    fn set_state(&mut self, state: SwipePanelState, window: &mut Window, cx: &mut Context<Self>) {
        if self.state != state {
            self.state = state;
            self.update_target(window);
            self.ensure_animation(cx);
            if let Some(ref handler) = self.props.on_state_change {
                handler(state, window, cx);
            }
        }
    }

    fn start_drag(&mut self, pos: f32) {
        self.dragging = true;
        self.drag_start_pos = pos;
        self.drag_start_offset = self.current_offset;
        self.drag_distance = 0.0;
        self.velocity = 0.0;
        self.scroller.cancel();
        self.tracker.reset();
        self.tracker.record(pos, pos);
    }

    fn update_drag(&mut self, pos: f32, window: &mut Window) {
        if !self.dragging {
            return;
        }
        let delta = match self.props.anchor {
            SwipePanelAnchor::Bottom => self.drag_start_pos - pos,
            SwipePanelAnchor::Top => pos - self.drag_start_pos,
        };
        self.drag_distance = self.drag_distance.max(delta.abs());
        let panel_height = self.panel_height(window);
        self.current_offset = (self.drag_start_offset + delta).clamp(-panel_height, 0.0);
        self.tracker.record(pos, pos);
    }

    fn end_drag(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dragging {
            return;
        }
        self.dragging = false;
        let (_vx, vy) = self.tracker.velocity();
        let velocity = match self.props.anchor {
            SwipePanelAnchor::Bottom => -vy,
            SwipePanelAnchor::Top => vy,
        };
        self.tracker.reset();

        // Treat a very small movement as a tap on the handle/header.
        if self.drag_distance < 8.0 {
            self.toggle_state(window, cx);
            return;
        }

        let viewport = self.viewport_height(window);
        let peek_offset = self
            .props
            .target_offset_for_state(SwipePanelState::Peek, viewport);
        let expanded_offset = self
            .props
            .target_offset_for_state(SwipePanelState::Expanded, viewport);
        let collapsed_offset = self
            .props
            .target_offset_for_state(SwipePanelState::Collapsed, viewport);

        let significant_velocity = velocity.abs() > 400.0;
        let new_state = if significant_velocity {
            if velocity > 0.0 {
                // Dragging toward expanded.
                if self.current_offset > peek_offset + (expanded_offset - peek_offset) * 0.5 {
                    SwipePanelState::Expanded
                } else {
                    SwipePanelState::Peek
                }
            } else {
                // Dragging toward collapsed.
                if self.current_offset < collapsed_offset + (peek_offset - collapsed_offset) * 0.5 {
                    SwipePanelState::Collapsed
                } else {
                    SwipePanelState::Peek
                }
            }
        } else {
            // Snap to the nearest state.
            let dist_collapsed = (self.current_offset - collapsed_offset).abs();
            let dist_peek = (self.current_offset - peek_offset).abs();
            let dist_expanded = (self.current_offset - expanded_offset).abs();
            if dist_collapsed < dist_peek && dist_collapsed < dist_expanded {
                SwipePanelState::Collapsed
            } else if dist_peek < dist_expanded {
                SwipePanelState::Peek
            } else {
                SwipePanelState::Expanded
            }
        };

        self.set_state(new_state, window, cx);
    }

    fn toggle_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let next = match self.state {
            SwipePanelState::Collapsed => SwipePanelState::Peek,
            SwipePanelState::Peek => SwipePanelState::Expanded,
            SwipePanelState::Expanded => SwipePanelState::Peek,
        };
        self.set_state(next, window, cx);
    }

    fn ensure_animation(&mut self, cx: &mut Context<Self>) {
        if !self.animating {
            self.animating = true;
            self.last_anim_time = Instant::now();
            cx.spawn(async move |this: WeakEntity<Self>, cx| {
                loop {
                    smol::Timer::after(Duration::from_millis(16)).await;
                    let alive = this.update(cx, |model, cx| {
                        model.step_animation(cx);
                    });
                    if alive.is_err() {
                        break;
                    }
                    let still_animating = this
                        .update(cx, |model, _cx| model.animating)
                        .unwrap_or(false);
                    if !still_animating {
                        break;
                    }
                }
            })
            .detach();
        }
    }

    fn step_animation(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        let dt = now
            .duration_since(self.last_anim_time)
            .as_secs_f32()
            .min(0.05);
        self.last_anim_time = now;

        let spring = Spring::default();
        let (new_offset, new_velocity) =
            spring.step(self.current_offset, self.target_offset, self.velocity, dt);
        self.current_offset = new_offset;
        self.velocity = new_velocity;

        if spring.is_settled(self.current_offset, self.target_offset, self.velocity, 0.5) {
            self.current_offset = self.target_offset;
            self.velocity = 0.0;
            self.animating = false;
        }
        cx.notify();
    }
}

impl Render for SwipePanelEntity {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let bg = theme.surface;
        let border = theme.border;
        let handle_bg = theme.text_muted;
        let text = theme.text_primary;

        let _viewport = self.viewport_height(window);
        let panel_height = self.panel_height(window);
        self.update_target(window);

        if !self.dragging && !self.animating {
            self.current_offset = self.target_offset;
        }

        let offset_px = px(self.current_offset);
        let panel_height_px = px(panel_height);

        let backdrop_opacity = if self.state == SwipePanelState::Collapsed {
            0.0
        } else {
            let visible = 1.0 + self.current_offset / panel_height.max(1.0);
            visible.clamp(0.0, 1.0) * 0.4
        };

        let mut container = div().absolute().inset_0().overflow_hidden();

        if self.props.show_backdrop && backdrop_opacity > 0.001 {
            let backdrop_color = Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: backdrop_opacity,
            };
            let entity = cx.entity().clone();
            let restore_focus_to = self.props.restore_focus_to.clone();
            container =
                container.child(div().absolute().inset_0().bg(backdrop_color).on_mouse_down(
                    MouseButton::Left,
                    move |_event, window, cx| {
                        if let Some(ref handle) = restore_focus_to {
                            window.focus(handle, cx);
                        }
                        entity.update(cx, |model, cx| {
                            if model.props.show_backdrop {
                                model.set_state(SwipePanelState::Collapsed, window, cx);
                            }
                        });
                    },
                ));
        }

        let mut panel = div()
            .id(self.props.id.clone())
            .absolute()
            .left_0()
            .right_0()
            .h(panel_height_px)
            .flex()
            .flex_col()
            .bg(bg)
            .border_color(border)
            .text_color(text)
            .overflow_hidden();

        if let Some(ref handle) = self.props.focus_handle {
            panel = panel.track_focus(handle).focusable();
        }

        match self.props.anchor {
            SwipePanelAnchor::Bottom => {
                panel = panel.bottom(offset_px).border_t_1().rounded_t_lg();
            }
            SwipePanelAnchor::Top => {
                panel = panel.top(offset_px).border_b_1().rounded_b_lg();
            }
        }

        let entity = cx.entity().clone();
        panel = panel.on_mouse_down(MouseButton::Left, move |event, _window, cx| {
            let pos: f32 = event.position.y.into();
            entity.update(cx, |model, _cx| {
                model.start_drag(pos);
            });
        });

        if let Some(handle) = self.props.focus_handle.clone() {
            let entity = cx.entity().clone();
            let anchor = self.props.anchor;
            let state = self.state;
            let restore_focus_to = self.props.restore_focus_to.clone();
            panel = panel.on_key_down(
                move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
                    if !handle.is_focused(window) {
                        return;
                    }

                    let Some(target) =
                        keyboard_target_state(anchor, state, event.keystroke.key.as_str())
                    else {
                        return;
                    };

                    cx.stop_propagation();
                    if target == SwipePanelState::Collapsed
                        && event.keystroke.key.as_str() == "escape"
                        && let Some(ref handle) = restore_focus_to
                    {
                        window.focus(handle, cx);
                    }
                    entity.update(cx, |model, cx| {
                        model.set_state(target, window, cx);
                    });
                },
            );
        }

        // Handle bar.
        if self.props.show_handle {
            let handle = div()
                .w_full()
                .h(px(28.0))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .child(div().w(px(36.0)).h(px(4.0)).rounded(px(2.0)).bg(handle_bg));
            panel = panel.child(handle);
        }

        panel = panel.child(self.content.clone());

        // Mouse-move/up handlers cover the whole window while dragging.
        let entity = cx.entity().clone();
        container = container.on_mouse_move(move |event, window, cx| {
            let pos: f32 = event.position.y.into();
            entity.update(cx, |model, _cx| {
                model.update_drag(pos, window);
            });
        });
        let entity = cx.entity().clone();
        container = container.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
            entity.update(cx, |model, cx| {
                model.end_drag(window, cx);
            });
        });

        container.child(panel)
    }
}

impl RenderOnce for SwipePanel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let state = self.state;
        let entity: Entity<SwipePanelEntity> = SWIPE_PANEL_ENTITIES.with(|map| {
            let mut map = map.borrow_mut();
            if let Some(weak) = map.get(&id)
                && let Some(entity) = weak.upgrade()
            {
                return entity;
            }
            let mut placeholder = SwipePanel::new(id.clone());
            placeholder.state = state;
            let entity = cx.new(|cx| SwipePanelEntity::new(placeholder, cx));
            map.insert(id.clone(), entity.downgrade());
            entity
        });

        entity.update(cx, |model, cx| {
            let mut props = self;
            if let Some(content) = props.content.take() {
                model.content.update(cx, |model, cx| {
                    model.content = Some(content);
                    cx.notify();
                });
            }
            model.props = props;
        });
        entity.clone().into_any_element()
    }
}

impl IntoElement for SwipePanel {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_offsets_are_ordered() {
        let panel = SwipePanel::new("test")
            .peek_height(px(50.0))
            .expanded_height(px(200.0));
        let collapsed = panel.target_offset_for_state(SwipePanelState::Collapsed, 400.0);
        let peek = panel.target_offset_for_state(SwipePanelState::Peek, 400.0);
        let expanded = panel.target_offset_for_state(SwipePanelState::Expanded, 400.0);
        assert!(collapsed < peek);
        assert!(peek < expanded);
    }

    #[test]
    fn keyboard_target_state_steps_bottom_panel() {
        assert_eq!(
            keyboard_target_state(SwipePanelAnchor::Bottom, SwipePanelState::Collapsed, "up"),
            Some(SwipePanelState::Peek)
        );
        assert_eq!(
            keyboard_target_state(SwipePanelAnchor::Bottom, SwipePanelState::Peek, "up"),
            Some(SwipePanelState::Expanded)
        );
        assert_eq!(
            keyboard_target_state(SwipePanelAnchor::Bottom, SwipePanelState::Expanded, "down"),
            Some(SwipePanelState::Peek)
        );
        assert_eq!(
            keyboard_target_state(SwipePanelAnchor::Bottom, SwipePanelState::Peek, "down"),
            Some(SwipePanelState::Collapsed)
        );
    }

    #[test]
    fn keyboard_target_state_steps_top_panel_by_physical_direction() {
        assert_eq!(
            keyboard_target_state(SwipePanelAnchor::Top, SwipePanelState::Collapsed, "down"),
            Some(SwipePanelState::Peek)
        );
        assert_eq!(
            keyboard_target_state(SwipePanelAnchor::Top, SwipePanelState::Peek, "down"),
            Some(SwipePanelState::Expanded)
        );
        assert_eq!(
            keyboard_target_state(SwipePanelAnchor::Top, SwipePanelState::Expanded, "up"),
            Some(SwipePanelState::Peek)
        );
        assert_eq!(
            keyboard_target_state(SwipePanelAnchor::Top, SwipePanelState::Peek, "up"),
            Some(SwipePanelState::Collapsed)
        );
    }

    #[test]
    fn keyboard_target_state_handles_shortcuts() {
        assert_eq!(
            keyboard_target_state(SwipePanelAnchor::Bottom, SwipePanelState::Peek, "home"),
            Some(SwipePanelState::Expanded)
        );
        assert_eq!(
            keyboard_target_state(SwipePanelAnchor::Bottom, SwipePanelState::Expanded, "end"),
            Some(SwipePanelState::Collapsed)
        );
        assert_eq!(
            keyboard_target_state(
                SwipePanelAnchor::Bottom,
                SwipePanelState::Expanded,
                "escape"
            ),
            Some(SwipePanelState::Collapsed)
        );
        assert_eq!(
            keyboard_target_state(SwipePanelAnchor::Bottom, SwipePanelState::Peek, "space"),
            Some(SwipePanelState::Expanded)
        );
        assert_eq!(
            keyboard_target_state(SwipePanelAnchor::Bottom, SwipePanelState::Expanded, "tab"),
            None
        );
    }

    #[test]
    fn swipe_panel_builder_records_keyboard_contract() {
        let panel = SwipePanel::new("mobile-panel")
            .anchor(SwipePanelAnchor::Top)
            .state(SwipePanelState::Expanded)
            .show_backdrop(false)
            .on_state_change(|_, _, _| {});

        assert_eq!(panel.anchor, SwipePanelAnchor::Top);
        assert_eq!(panel.state, SwipePanelState::Expanded);
        assert!(!panel.show_backdrop);
        assert!(panel.on_state_change.is_some());
    }
}
