//! `ScrollArea` component
//!
//! A bounded scrollable region (Radix `ScrollArea` parity): content larger than
//! the configured bounds scrolls on the enabled axes with a visible,
//! draggable scrollbar thumb.
//!
//! GPUI divs render no scrollbars, so the thumb is drawn here from the
//! viewport's [`ScrollHandle`]: sized by the viewport/content ratio,
//! positioned by the scroll offset, draggable with the mouse, and the
//! container is focusable so PageUp/PageDown/Home/End work without a
//! wheel. On macOS the thumb auto-hides (overlay style) except while
//! scrolling or dragging; elsewhere it is always shown when content
//! overflows.
//!
//! # Usage
//!
//! ```ignore
//! ScrollArea::new(list_element)
//!     .max_height(px(320.0))
//!     .axis(ScrollAxis::Vertical)
//! ```

use crate::theme::{Theme, ThemeExt};
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, Div, ElementId, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, Pixels, ScrollHandle, Stateful, Window, div, point, px,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;
use web_time::Instant;

/// Minimum scrollbar thumb length in pixels, so very long content
/// still has a grabbable thumb.
const MIN_THUMB_PX: f32 = 24.0;
/// Scrollbar thumb/track width in pixels.
const THUMB_WIDTH_PX: f32 = 10.0;
/// How long the thumb stays visible after scroll activity on
/// auto-hide platforms (macOS overlay style).
const AUTOHIDE_AFTER: Duration = Duration::from_millis(1200);
/// Cap on retained per-element scroll states (see registry below).
const MAX_SCROLL_AREA_STATES: usize = 1024;

/// Thumb geometry within a track of the viewport's height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollbarGeometry {
    /// Thumb top edge in pixels from the track top.
    pub thumb_top: f32,
    /// Thumb length in pixels.
    pub thumb_len: f32,
}

/// Compute scrollbar thumb geometry, or `None` when nothing overflows.
///
/// `viewport` is the visible height in pixels, `max_offset` the
/// scrollable range (`ScrollHandle::max_offset().y`), `offset` the
/// current scrolled distance in pixels (non-negative), and
/// `min_thumb` the minimum thumb length. The track spans the full
/// viewport height.
pub fn scrollbar_geometry(
    viewport: f32,
    max_offset: f32,
    offset: f32,
    min_thumb: f32,
) -> Option<ScrollbarGeometry> {
    if viewport <= 0.0 || max_offset <= 0.5 {
        return None;
    }
    let content = viewport + max_offset;
    let thumb_len = (viewport * viewport / content).clamp(min_thumb.min(viewport), viewport);
    let travel = (viewport - thumb_len).max(0.0);
    let ratio = (offset / max_offset).clamp(0.0, 1.0);
    Some(ScrollbarGeometry {
        thumb_top: travel * ratio,
        thumb_len,
    })
}

/// Map a thumb drag of `delta_px` pixels to the new scroll offset,
/// starting from `start_offset`. Inverse of [`scrollbar_geometry`]'s
/// thumb position, clamped to the scrollable range.
pub fn offset_for_drag(
    start_offset: f32,
    delta_px: f32,
    viewport: f32,
    max_offset: f32,
    min_thumb: f32,
) -> f32 {
    if max_offset <= 0.0 {
        return 0.0;
    }
    let Some(geometry) = scrollbar_geometry(viewport, max_offset, 0.0, min_thumb) else {
        return 0.0;
    };
    let travel = (viewport - geometry.thumb_len).max(0.0);
    if travel <= 0.0 {
        return start_offset.clamp(0.0, max_offset);
    }
    (start_offset + delta_px / travel * max_offset).clamp(0.0, max_offset)
}

/// Whether the scrollbar thumb is visible: always on platforms with
/// persistent scrollbars; on auto-hide platforms only while
/// interacting (dragging, or scrolled within the idle window).
pub fn scrollbar_visible(auto_hide: bool, interacting: bool, recently_scrolled: bool) -> bool {
    !auto_hide || interacting || recently_scrolled
}

/// Which axes a [`ScrollArea`] scrolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollAxis {
    /// Vertical scrolling only (default).
    #[default]
    Vertical,
    /// Horizontal scrolling only.
    Horizontal,
    /// Scroll on both axes.
    Both,
}

#[derive(Clone)]
struct ScrollAreaState {
    handle: ScrollHandle,
    focus: Option<FocusHandle>,
    drag: Option<(f32, f32)>, // (start mouse y, start offset px)
    last_scroll_activity: Option<Instant>,
}

impl Default for ScrollAreaState {
    fn default() -> Self {
        Self {
            handle: ScrollHandle::new(),
            focus: None,
            drag: None,
            last_scroll_activity: None,
        }
    }
}

thread_local! {
    /// Per-element scroll state, keyed by the ScrollArea id (same
    /// retention pattern as `Select`). RenderOnce components are
    /// rebuilt every frame, so the handle cannot live in the struct.
    static SCROLL_AREA_STATES: RefCell<HashMap<ElementId, ScrollAreaState>> =
        RefCell::new(HashMap::new());
}

fn scroll_area_state(id: &ElementId, focus: Option<FocusHandle>) -> ScrollAreaState {
    SCROLL_AREA_STATES.with(|states| {
        let mut states = states.borrow_mut();
        if !states.contains_key(id) && states.len() >= MAX_SCROLL_AREA_STATES {
            states.clear();
        }
        let state = states.entry(id.clone()).or_default();
        if state.focus.is_none() {
            state.focus = focus;
        }
        state.clone()
    })
}

fn update_scroll_area_state(id: &ElementId, update: impl FnOnce(&mut ScrollAreaState)) {
    SCROLL_AREA_STATES.with(|states| {
        if let Some(state) = states.borrow_mut().get_mut(id) {
            update(state);
        }
    });
}

/// A bounded scrollable region.
#[derive(IntoElement)]
pub struct ScrollArea {
    id: ElementId,
    child: AnyElement,
    axis: ScrollAxis,
    max_height: Option<Pixels>,
    max_width: Option<Pixels>,
    external_handle: Option<ScrollHandle>,
    flex_1: bool,
}

impl ScrollArea {
    /// Create a scroll area wrapping `element`.
    pub fn new(id: impl Into<ElementId>, element: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            child: element.into_any_element(),
            axis: ScrollAxis::default(),
            max_height: None,
            max_width: None,
            external_handle: None,
            flex_1: false,
        }
    }

    /// Drive scrolling through an external handle (e.g. shared with a
    /// list that scrolls items into view). The scrollbar follows it.
    pub fn track_scroll(mut self, handle: &ScrollHandle) -> Self {
        self.external_handle = Some(handle.clone());
        self
    }

    /// Set the scrollable axes.
    pub fn axis(mut self, axis: ScrollAxis) -> Self {
        self.axis = axis;
        self
    }

    /// Grow to fill the parent flex container (with `min_h(0)` so the
    /// viewport can shrink and scroll instead of overflowing).
    pub fn flex_1(mut self) -> Self {
        self.flex_1 = true;
        self
    }

    /// Cap the viewport height; taller content scrolls vertically.
    pub fn max_height(mut self, height: Pixels) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Cap the viewport width; wider content scrolls horizontally.
    pub fn max_width(mut self, width: Pixels) -> Self {
        self.max_width = Some(width);
        self
    }

    fn build_inner(self, theme: &Theme, state: Option<ScrollAreaState>) -> Stateful<Div> {
        let handle = self
            .external_handle
            .clone()
            .or_else(|| state.as_ref().map(|state| state.handle.clone()));
        let dragging = state.as_ref().is_some_and(|state| state.drag.is_some());
        let recently_scrolled = state
            .as_ref()
            .and_then(|state| state.last_scroll_activity)
            .is_some_and(|at| at.elapsed() < AUTOHIDE_AFTER);
        let auto_hide = cfg!(target_os = "macos");
        let show_thumb = scrollbar_visible(auto_hide, dragging, recently_scrolled);

        let geometry = handle.as_ref().and_then(|handle| {
            scrollbar_geometry(
                f32::from(handle.bounds().size.height),
                f32::from(handle.max_offset().y),
                f32::from(-handle.offset().y),
                MIN_THUMB_PX,
            )
        });
        let shows_vertical_thumb = show_thumb
            && geometry.is_some()
            && matches!(self.axis, ScrollAxis::Vertical | ScrollAxis::Both);

        let mut container = div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .relative()
            .overflow_hidden()
            .w_full();

        if self.flex_1 {
            container = container.flex_1().min_h(px(0.0));
        }
        if let Some(max_height) = self.max_height {
            container = container.max_h(max_height);
        }
        if let Some(max_width) = self.max_width {
            container = container.max_w(max_width);
        }

        // Keyboard scrolling: focusable viewport with PageUp/Down,
        // Home/End and arrow keys, for users without a wheel.
        if let Some(focus) = state.as_ref().and_then(|state| state.focus.clone()) {
            let key_handle = handle.clone();
            container = container.track_focus(&focus).on_key_down(
                move |event: &KeyDownEvent, _window, cx| {
                    let Some(handle) = key_handle.as_ref() else {
                        return;
                    };
                    let viewport: f32 = handle.bounds().size.height.into();
                    let max_offset: f32 = handle.max_offset().y.into();
                    let current: f32 = (-handle.offset().y).into();
                    let line: f32 = 40.0;
                    let next = match event.keystroke.key.as_str() {
                        "down" => Some(current + line),
                        "up" => Some(current - line),
                        "pagedown" => Some(current + viewport * 0.9),
                        "pageup" => Some(current - viewport * 0.9),
                        "home" => Some(0.0),
                        "end" => Some(max_offset),
                        _ => None,
                    };
                    if let Some(next) = next {
                        cx.stop_propagation();
                        handle.set_offset(point(
                            handle.offset().x,
                            px(-next.clamp(0.0, max_offset.max(0.0))),
                        ));
                    }
                },
            );
        }

        // Scrollbar drag + track paging. Handlers live on the
        // container; hit-testing uses the handle's viewport bounds so
        // only the right-edge strip starts scrollbar gestures.
        if let Some(handle) = handle.clone() {
            let area_id = self.id.clone();
            let down_handle = handle.clone();
            let down_id = area_id.clone();
            container = container.on_mouse_down(
                MouseButton::Left,
                move |event: &MouseDownEvent, _window, cx| {
                    let bounds = down_handle.bounds();
                    let viewport: f32 = bounds.size.height.into();
                    let max_offset: f32 = down_handle.max_offset().y.into();
                    let Some(geometry) = scrollbar_geometry(
                        viewport,
                        max_offset,
                        f32::from(-down_handle.offset().y),
                        MIN_THUMB_PX,
                    ) else {
                        return;
                    };
                    let pos = event.position;
                    let in_strip = pos.x >= bounds.right() - px(THUMB_WIDTH_PX + 4.0)
                        && pos.x <= bounds.right();
                    if !in_strip {
                        return;
                    }
                    cx.stop_propagation();
                    let local_y: f32 = (pos.y - bounds.top()).into();
                    if local_y >= geometry.thumb_top
                        && local_y <= geometry.thumb_top + geometry.thumb_len
                    {
                        let start_offset: f32 = (-down_handle.offset().y).into();
                        update_scroll_area_state(&down_id, |state| {
                            state.drag = Some((pos.y.into(), start_offset));
                            state.last_scroll_activity = Some(Instant::now());
                        });
                    } else {
                        // Track click: page toward the click.
                        let direction = if local_y < geometry.thumb_top {
                            -1.0
                        } else {
                            1.0
                        };
                        let current: f32 = (-down_handle.offset().y).into();
                        let next = (current + direction * viewport * 0.9).clamp(0.0, max_offset);
                        down_handle.set_offset(point(down_handle.offset().x, px(-next)));
                        update_scroll_area_state(&down_id, |state| {
                            state.last_scroll_activity = Some(Instant::now());
                        });
                    }
                },
            );
            let move_handle = handle.clone();
            let move_id = area_id.clone();
            container = container.on_mouse_move(move |event: &MouseMoveEvent, _window, cx| {
                if event.pressed_button != Some(MouseButton::Left) {
                    return;
                }
                let drag = SCROLL_AREA_STATES
                    .with(|states| states.borrow().get(&move_id).and_then(|state| state.drag));
                let Some((start_y, start_offset)) = drag else {
                    return;
                };
                cx.stop_propagation();
                let viewport: f32 = move_handle.bounds().size.height.into();
                let max_offset: f32 = move_handle.max_offset().y.into();
                let next = offset_for_drag(
                    start_offset,
                    f32::from(event.position.y) - start_y,
                    viewport,
                    max_offset,
                    MIN_THUMB_PX,
                );
                move_handle.set_offset(point(move_handle.offset().x, px(-next)));
                update_scroll_area_state(&move_id, |state| {
                    state.last_scroll_activity = Some(Instant::now());
                });
            });
            let up_id = area_id;
            container = container.on_mouse_up(MouseButton::Left, move |_, _window, _cx| {
                update_scroll_area_state(&up_id, |state| state.drag = None);
            });
        }

        let mut viewport = div().id((self.id, "viewport")).w_full();
        if self.flex_1 {
            viewport = viewport.flex_1().min_h(px(0.0));
        }
        if let Some(handle) = &handle {
            viewport = viewport.track_scroll(handle);
        }
        match self.axis {
            ScrollAxis::Vertical => {
                viewport = viewport.overflow_y_scroll();
            }
            ScrollAxis::Horizontal => {
                viewport = viewport.overflow_x_scroll();
            }
            ScrollAxis::Both => {
                viewport = viewport.overflow_y_scroll().overflow_x_scroll();
            }
        }
        container = container.child(viewport.child(self.child));

        if shows_vertical_thumb && let Some(geometry) = geometry {
            container = container.child(
                div()
                    .id("scrollbar-thumb")
                    .absolute()
                    .top(px(geometry.thumb_top))
                    .right(px(2.0))
                    .w(px(THUMB_WIDTH_PX - 4.0))
                    .h(px(geometry.thumb_len))
                    .rounded(px((THUMB_WIDTH_PX - 4.0) / 2.0))
                    .bg(theme.text_secondary),
            );
        }

        container
    }

    /// Build into an element with the given theme. The scrollbar is
    /// rendered when a scroll handle is available through `render()`
    /// or `track_scroll()`; direct callers without a window get the
    /// plain scrollable viewport.
    pub fn build_with_theme(self, theme: &Theme) -> Stateful<Div> {
        self.build_inner(theme, None)
    }
}

impl RenderOnce for ScrollArea {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        // The thread-local registry is the single source of truth
        // (mouse handlers mutate it between renders); rendering reads
        // a snapshot. External handles win over the stored one.
        let mut state = scroll_area_state(&self.id, Some(cx.focus_handle()));
        if let Some(handle) = &self.external_handle {
            state.handle = handle.clone();
        }
        self.build_inner(&global_theme, Some(state))
    }
}

#[cfg(test)]
mod tests {
    use super::{ScrollArea, ScrollAxis, offset_for_drag, scrollbar_geometry, scrollbar_visible};
    use crate::theme::Theme;
    use gpui::prelude::ParentElement;
    use gpui::px;

    #[test]
    fn scroll_area_builders_record_axis_and_bounds() {
        let area = ScrollArea::new("scroll", gpui::div().child("content"))
            .axis(ScrollAxis::Both)
            .max_height(px(320.0))
            .max_width(px(480.0));

        assert_eq!(area.axis, ScrollAxis::Both);
        assert_eq!(area.max_height, Some(px(320.0)));
        assert_eq!(area.max_width, Some(px(480.0)));
    }

    #[test]
    fn scroll_area_builds_with_default_theme() {
        let _el = ScrollArea::new("scroll", gpui::div().child("content"))
            .max_height(px(200.0))
            .build_with_theme(&Theme::default());
    }

    #[test]
    fn scrollbar_geometry_hidden_without_overflow() {
        assert!(scrollbar_geometry(200.0, 0.0, 0.0, 24.0).is_none());
        assert!(scrollbar_geometry(200.0, -10.0, 0.0, 24.0).is_none());
        assert!(scrollbar_geometry(0.0, 100.0, 0.0, 24.0).is_none());
    }

    #[test]
    fn scrollbar_geometry_proportional_thumb() {
        let at_top = scrollbar_geometry(200.0, 800.0, 0.0, 24.0).unwrap();
        assert!((at_top.thumb_len - 40.0).abs() < 0.01, "{at_top:?}");
        assert!((at_top.thumb_top - 0.0).abs() < 0.01, "{at_top:?}");
        let mid = scrollbar_geometry(200.0, 800.0, 400.0, 24.0).unwrap();
        assert!((mid.thumb_top - 80.0).abs() < 0.01, "{mid:?}");
        let at_bottom = scrollbar_geometry(200.0, 800.0, 800.0, 24.0).unwrap();
        assert!((at_bottom.thumb_top - 160.0).abs() < 0.01, "{at_bottom:?}");
        let over = scrollbar_geometry(200.0, 800.0, 5000.0, 24.0).unwrap();
        assert!((over.thumb_top - 160.0).abs() < 0.01, "{over:?}");
    }

    #[test]
    fn scrollbar_geometry_enforces_minimum_thumb() {
        let geo = scrollbar_geometry(200.0, 19_800.0, 19_800.0, 24.0).unwrap();
        assert!((geo.thumb_len - 24.0).abs() < 0.01, "{geo:?}");
        assert!((geo.thumb_top - 176.0).abs() < 0.01, "{geo:?}");
    }

    #[test]
    fn scrollbar_drag_maps_pixels_to_offset() {
        assert_eq!(offset_for_drag(0.0, 80.0, 200.0, 800.0, 24.0), 400.0);
        assert_eq!(offset_for_drag(400.0, 10_000.0, 200.0, 800.0, 24.0), 800.0);
        assert_eq!(offset_for_drag(400.0, -10_000.0, 200.0, 800.0, 24.0), 0.0);
        assert_eq!(offset_for_drag(0.0, 50.0, 200.0, 0.0, 24.0), 0.0);
    }

    #[test]
    fn scrollbar_visibility_follows_platform_style() {
        assert!(scrollbar_visible(false, false, false));
        assert!(!scrollbar_visible(true, false, false));
        assert!(scrollbar_visible(true, true, false));
        assert!(scrollbar_visible(true, false, true));
    }
}
