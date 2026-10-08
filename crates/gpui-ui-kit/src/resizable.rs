//! User-resizable single-child panel.
//!
//! [`Resizable`] wraps one child with a drag handle on an edge or corner.
//! Unlike [`SplitPane`](crate::SplitPane), which divides two panes, this
//! resizes one panel against minimum/maximum constraints and reports the
//! committed size through [`Resizable::on_resize`].
//!
//! # Usage
//!
//! ```ignore
//! Resizable::new("panel")
//!     .handle(ResizableHandle::Corner)
//!     .width(px(320.0))
//!     .height(px(240.0))
//!     .min_width(px(160.0))
//!     .min_height(px(120.0))
//!     .on_resize(|width, height, _window, _cx| {
//!         println!("new size: {width:?} x {height:?}");
//!     })
//!     .child(panel_body)
//! ```
//!
//! Drags anchor to the committed size, falling back to the `width` and
//! `height` props. Axes without an anchor keep their natural size and
//! stay fixed during drags.

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, CursorStyle, Div, ElementId, MouseButton, Pixels, Rgba, SharedString,
    Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Which edge or corner carries the resize handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResizableHandle {
    /// Right edge, resizes width.
    Right,
    /// Bottom edge, resizes height.
    Bottom,
    /// Bottom-right corner, resizes both (default).
    #[default]
    Corner,
}

impl ResizableHandle {
    const fn resizes_width(self) -> bool {
        matches!(self, Self::Right | Self::Corner)
    }

    const fn resizes_height(self) -> bool {
        matches!(self, Self::Bottom | Self::Corner)
    }
}

/// Theme colors for resizable panel styling.
#[derive(Debug, Clone, ComponentTheme)]
pub struct ResizableTheme {
    /// Panel background tint (transparent by default).
    #[theme(default = 0x00000000, from = transparent)]
    pub background: Rgba,
    /// Resize handle color.
    #[theme(default = 0x3a3a3aff, from = border)]
    pub handle: Rgba,
    /// Resize handle color when hovered.
    #[theme(default = 0x007accff, from = accent)]
    pub handle_hover: Rgba,
    /// Resize handle color while dragging.
    #[theme(default = 0x0098ffff, from = accent_hover)]
    pub handle_active: Rgba,
}

/// Drag state stored in thread-local storage so it survives re-renders.
#[derive(Clone, Copy, Debug)]
struct ResizableDragState {
    start_x: f32,
    start_y: f32,
    start_width: f32,
    start_height: f32,
    horizontal: bool,
    vertical: bool,
}

thread_local! {
    static RESIZABLE_DRAG_STATES: RefCell<HashMap<ElementId, ResizableDragState>> =
        RefCell::new(HashMap::new());
    static RESIZABLE_SIZES: RefCell<HashMap<ElementId, (Option<f32>, Option<f32>)>> =
        RefCell::new(HashMap::new());
}

/// Cap for retained sizes; guards against unbounded per-id growth.
const MAX_RESIZABLE_SIZES: usize = 1024;

/// Clamp a candidate size to the optional constraints (pixels).
///
/// The result never drops below one pixel so panels cannot collapse.
#[allow(clippy::too_many_arguments)]
fn clamp_size(
    width: f32,
    height: f32,
    min_width: Option<Pixels>,
    min_height: Option<Pixels>,
    max_width: Option<Pixels>,
    max_height: Option<Pixels>,
) -> (f32, f32) {
    let min_w = min_width.map_or(1.0, f32::from);
    let min_h = min_height.map_or(1.0, f32::from);
    let w = width.max(min_w);
    let h = height.max(min_h);
    let w = max_width.map_or(w, |max| w.min(f32::from(max).max(min_w)));
    let h = max_height.map_or(h, |max| h.min(f32::from(max).max(min_h)));
    (w, h)
}

fn remember_size(id: &ElementId, size: (Option<f32>, Option<f32>)) {
    RESIZABLE_SIZES.with(|sizes| {
        let mut sizes = sizes.borrow_mut();
        while sizes.len() >= MAX_RESIZABLE_SIZES {
            if let Some(key) = sizes.keys().next().cloned() {
                sizes.remove(&key);
            } else {
                break;
            }
        }
        sizes.insert(id.clone(), size);
    });
}

fn recalled_size(id: &ElementId) -> (Option<f32>, Option<f32>) {
    RESIZABLE_SIZES.with(|sizes| sizes.borrow().get(id).copied().unwrap_or((None, None)))
}

/// A user-resizable single-child panel.
///
/// Dragging the handle commits a new size (retained across re-renders)
/// and notifies [`Self::on_resize`]. Drags anchor to the committed size
/// or the `width`/`height` props; unanchored axes keep natural sizing.
/// An empty slot renders nothing. Unlabeled panels register with an
/// empty accessible label.
///
/// # Examples
///
/// ```ignore
/// Resizable::new("panel")
///     .handle(ResizableHandle::Right)
///     .min_width(px(160.0))
///     .child(panel_body)
/// ```
#[derive(IntoElement)]
pub struct Resizable {
    id: ElementId,
    child: Option<AnyElement>,
    handle: ResizableHandle,
    width: Option<Pixels>,
    height: Option<Pixels>,
    min_width: Option<Pixels>,
    min_height: Option<Pixels>,
    max_width: Option<Pixels>,
    max_height: Option<Pixels>,
    handle_size: Pixels,
    on_resize: Option<Rc<dyn Fn(Option<Pixels>, Option<Pixels>, &mut Window, &mut App) + 'static>>,
    theme: Option<ResizableTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Resizable {
    /// Create a new resizable panel with a corner handle.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            child: None,
            handle: ResizableHandle::default(),
            width: None,
            height: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            handle_size: px(12.0),
            on_resize: None,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the resizable child element.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }

    /// Set which edge or corner carries the handle.
    pub fn handle(mut self, handle: ResizableHandle) -> Self {
        self.handle = handle;
        self
    }

    /// Set the initial width (`None`, the default, sizes naturally).
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the initial height (`None`, the default, sizes naturally).
    pub fn height(mut self, height: Pixels) -> Self {
        self.height = Some(height);
        self
    }

    /// Set the minimum width.
    pub fn min_width(mut self, min: Pixels) -> Self {
        self.min_width = Some(min);
        self
    }

    /// Set the minimum height.
    pub fn min_height(mut self, min: Pixels) -> Self {
        self.min_height = Some(min);
        self
    }

    /// Set the maximum width.
    pub fn max_width(mut self, max: Pixels) -> Self {
        self.max_width = Some(max);
        self
    }

    /// Set the maximum height.
    pub fn max_height(mut self, max: Pixels) -> Self {
        self.max_height = Some(max);
        self
    }

    /// Set the handle strip thickness.
    pub fn handle_size(mut self, size: Pixels) -> Self {
        self.handle_size = size;
        self
    }

    /// Called with the committed size after each drag update.
    ///
    /// Each axis reports `None` while it keeps its natural size (no width
    /// or height prop and no committed drag yet on that axis).
    pub fn on_resize(
        mut self,
        handler: impl Fn(Option<Pixels>, Option<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resize = Some(Rc::new(handler));
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: ResizableTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit accessible label for the panel group.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default accessible role (`Group`).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Build the panel into a `Stateful<Div>` for further composition.
    ///
    /// Note: This bypasses accessibility registration. Prefer using the
    /// component directly via `RenderOnce` for automatic accessibility
    /// tree integration.
    pub fn build(self) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        let mut this = self;
        let theme = this.theme.take().unwrap_or_default();
        this.build_with_theme_and_design(&theme, &design)
    }

    /// Build with explicit theme and design defaults.
    pub fn build_with_theme_and_design(
        self,
        theme: &ResizableTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let (recalled_w, recalled_h) = recalled_size(&self.id);
        let start_w = recalled_w.or(self.width.map(f32::from));
        let start_h = recalled_h.or(self.height.map(f32::from));
        let handle = self.handle_size.max(px(design.spacing.grid_unit));
        let nub_long = px(design.interaction.min_touch_target.max(24.0));
        let nub_short = px(design.interaction.border_width.max(2.0));

        let mut content = div().overflow_hidden().bg(theme.background);
        if let Some(w) = start_w {
            content = content.w(px(w));
        }
        if let Some(h) = start_h {
            content = content.h(px(h));
        }
        content = content.children(self.child);

        let handle_color = theme.handle;
        let handle_hover = theme.handle_hover;
        let handle_active = theme.handle_active;
        let id = self.id.clone();
        // Drags anchor to the committed size, falling back to the width and
        // height props. Axes without an anchor stay fixed: GPUI event
        // handlers cannot measure the natural size, so starting from zero
        // would make the panel jump on the first drag.
        let anchor_w = start_w;
        let anchor_h = start_h;
        let wants_horizontal = self.handle.resizes_width();
        let wants_vertical = self.handle.resizes_height();
        let min_width = self.min_width;
        let min_height = self.min_height;
        let max_width = self.max_width;
        let max_height = self.max_height;

        let mut grip = div()
            .id((self.id.clone(), "handle"))
            .bg(handle_color)
            .flex()
            .items_center()
            .justify_center()
            .flex_shrink_0()
            .hover(move |s| s.bg(handle_hover))
            .active(move |s| s.bg(handle_active));
        grip = match self.handle {
            ResizableHandle::Right => grip.w(handle).h_full().cursor_col_resize().child(
                div()
                    .w(nub_short)
                    .h(nub_long)
                    .rounded(px(1.0))
                    .bg(handle_color),
            ),
            ResizableHandle::Bottom => grip.h(handle).w_full().cursor_row_resize().child(
                div()
                    .w(nub_long)
                    .h(nub_short)
                    .rounded(px(1.0))
                    .bg(handle_color),
            ),
            ResizableHandle::Corner => grip
                .absolute()
                .right(px(0.0))
                .bottom(px(0.0))
                .w(handle)
                .h(handle)
                .cursor(CursorStyle::ResizeUpLeftDownRight),
        };
        let id_down = id.clone();
        grip = grip.on_mouse_down(MouseButton::Left, move |event, _window, _cx| {
            let horizontal = wants_horizontal && anchor_w.is_some();
            let vertical = wants_vertical && anchor_h.is_some();
            if !horizontal && !vertical {
                return;
            }
            RESIZABLE_DRAG_STATES.with(|states| {
                states.borrow_mut().insert(
                    id_down.clone(),
                    ResizableDragState {
                        start_x: event.position.x.into(),
                        start_y: event.position.y.into(),
                        start_width: anchor_w.unwrap_or(0.0),
                        start_height: anchor_h.unwrap_or(0.0),
                        horizontal,
                        vertical,
                    },
                );
            });
        });

        let mut container = div()
            .id(self.id.clone())
            .flex()
            .overflow_hidden()
            .flex_shrink_0();
        container = match self.handle {
            ResizableHandle::Right => container.flex_row().child(content).child(grip),
            ResizableHandle::Bottom => container.flex_col().child(content).child(grip),
            ResizableHandle::Corner => container.relative().child(content).child(grip),
        };

        let on_resize = self.on_resize;
        let id_move = id.clone();
        let move_anchor_w = anchor_w;
        let move_anchor_h = anchor_h;
        container = container.on_mouse_move(move |event, window, cx| {
            RESIZABLE_DRAG_STATES.with(|states| {
                if let Some(state) = states.borrow().get(&id_move).copied() {
                    let pos_x: f32 = event.position.x.into();
                    let pos_y: f32 = event.position.y.into();
                    let new_w = if state.horizontal {
                        Some(state.start_width + (pos_x - state.start_x))
                    } else {
                        move_anchor_w
                    };
                    let new_h = if state.vertical {
                        Some(state.start_height + (pos_y - state.start_y))
                    } else {
                        move_anchor_h
                    };
                    let (w, h) = clamp_size(
                        new_w.unwrap_or(0.0),
                        new_h.unwrap_or(0.0),
                        min_width,
                        min_height,
                        max_width,
                        max_height,
                    );
                    let committed = (new_w.map(|_| w), new_h.map(|_| h));
                    remember_size(&id_move, committed);
                    if let Some(resize_cb) = &on_resize {
                        resize_cb(committed.0.map(px), committed.1.map(px), window, cx);
                    }
                }
            });
        });

        let id_up = id.clone();
        container.on_mouse_up(MouseButton::Left, move |_event, _window, _cx| {
            RESIZABLE_DRAG_STATES.with(|states| {
                states.borrow_mut().remove(&id_up);
            });
        })
    }
}

impl RenderOnce for Resizable {
    fn render(mut self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_default(),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group)),
        });

        let design = crate::design::resolve_design(self.design.clone(), cx);
        let global_theme = cx.theme();
        let theme = self
            .theme
            .take()
            .unwrap_or_else(|| ResizableTheme::from(global_theme.as_ref()));
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_use_corner_handle_without_constraints() {
        let panel = Resizable::new("panel");
        assert_eq!(panel.handle, ResizableHandle::Corner);
        assert!(panel.child.is_none());
        assert!(panel.width.is_none());
        assert!(panel.min_width.is_none());
        assert!(panel.max_height.is_none());
    }

    #[test]
    fn builders_set_fields() {
        let panel = Resizable::new("panel")
            .child(div())
            .handle(ResizableHandle::Right)
            .width(px(200.0))
            .height(px(120.0))
            .min_width(px(100.0))
            .min_height(px(80.0))
            .max_width(px(400.0))
            .max_height(px(300.0))
            .handle_size(px(10.0))
            .aria_label("Inspector")
            .aria_role(AriaRole::Region);
        assert_eq!(panel.handle, ResizableHandle::Right);
        assert_eq!(panel.width, Some(px(200.0)));
        assert_eq!(panel.height, Some(px(120.0)));
        assert_eq!(panel.min_width, Some(px(100.0)));
        assert_eq!(panel.max_height, Some(px(300.0)));
        assert_eq!(panel.handle_size, px(10.0));
        assert_eq!(panel.aria_label, Some(SharedString::from("Inspector")));
        assert_eq!(panel.aria_role, Some(AriaRole::Region));
    }

    #[test]
    fn clamp_size_enforces_minimums_and_maximums() {
        let (w, h) = clamp_size(
            50.0,
            500.0,
            Some(px(100.0)),
            Some(px(80.0)),
            Some(px(400.0)),
            Some(px(300.0)),
        );
        assert_eq!((w, h), (100.0, 300.0));

        let (w, h) = clamp_size(200.0, 150.0, None, None, None, None);
        assert_eq!((w, h), (200.0, 150.0));

        let (w, h) = clamp_size(0.0, -5.0, None, None, None, None);
        assert_eq!((w, h), (1.0, 1.0));
    }

    #[test]
    fn handles_resize_expected_axes() {
        assert!(ResizableHandle::Right.resizes_width());
        assert!(!ResizableHandle::Right.resizes_height());
        assert!(!ResizableHandle::Bottom.resizes_width());
        assert!(ResizableHandle::Bottom.resizes_height());
        assert!(ResizableHandle::Corner.resizes_width());
        assert!(ResizableHandle::Corner.resizes_height());
    }

    #[test]
    fn build_does_not_panic_for_each_handle() {
        for handle in [
            ResizableHandle::Right,
            ResizableHandle::Bottom,
            ResizableHandle::Corner,
        ] {
            let _el = Resizable::new("panel")
                .handle(handle)
                .width(px(200.0))
                .height(px(120.0))
                .on_resize(|_w, _h, _window, _cx| {})
                .child(div())
                .build();
        }
    }
}
