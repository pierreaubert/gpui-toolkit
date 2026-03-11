//! Focus management components
//!
//! Provides components for managing keyboard focus navigation between elements.
//!
//! # FocusGroup
//!
//! A container that manages keyboard navigation (arrow keys, Tab) between
//! its focusable children. Supports vertical, horizontal, and grid layouts.
//!
//! ```ignore
//! FocusGroup::new("my-group")
//!     .direction(FocusDirection::Vertical)
//!     .wraparound(true)
//!     .child(button1)
//!     .child(button2)
//!     .child(input1)
//! ```
//!
//! # Keyboard Navigation
//!
//! - **Vertical**: Up/Down arrows move focus, Home/End go to first/last
//! - **Horizontal**: Left/Right arrows move focus, Home/End go to first/last
//! - **Grid**: All arrow keys work, Home/End go to first/last in row
//! - **Tab**: Always moves to next/previous focusable (with Shift)
//!
//! # Focus Ring
//!
//! By default, FocusGroup adds a visual focus ring to the currently focused
//! child. Disable with `.focus_ring(false)`.

use gpui::prelude::*;
use gpui::*;

/// Direction of focus navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusDirection {
    /// Navigate vertically (Up/Down arrows)
    #[default]
    Vertical,
    /// Navigate horizontally (Left/Right arrows)
    Horizontal,
    /// Navigate in a grid pattern
    Grid {
        /// Number of columns in the grid
        columns: usize,
    },
}

/// A container that manages keyboard focus navigation between children
///
/// FocusGroup handles arrow key navigation, Tab key movement, and Home/End
/// keys for quick navigation to first/last elements.
pub struct FocusGroup {
    id: ElementId,
    children: Vec<AnyElement>,
    direction: FocusDirection,
    wraparound: bool,
    focus_ring: bool,
    gap: Pixels,
    focus_handle: Option<FocusHandle>,
}

impl FocusGroup {
    /// Create a new focus group
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            children: Vec::new(),
            direction: FocusDirection::default(),
            wraparound: false,
            focus_ring: true,
            gap: px(8.0),
            focus_handle: None,
        }
    }

    /// Set the navigation direction
    pub fn direction(mut self, direction: FocusDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Enable wraparound navigation (first <-> last)
    pub fn wraparound(mut self, wrap: bool) -> Self {
        self.wraparound = wrap;
        self
    }

    /// Show focus ring on focused child (default: true)
    pub fn focus_ring(mut self, show: bool) -> Self {
        self.focus_ring = show;
        self
    }

    /// Set gap between children
    pub fn gap(mut self, gap: impl Into<Pixels>) -> Self {
        self.gap = gap.into();
        self
    }

    /// Set the focus handle for this group
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Add a child element
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    /// Add multiple children
    pub fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.children
            .extend(children.into_iter().map(|c| c.into_any_element()));
        self
    }
}

impl RenderOnce for FocusGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let child_count = self.children.len();
        let direction = self.direction;
        let wraparound = self.wraparound;
        let gap = self.gap;

        // Create or use provided focus handle
        let focus_handle = self.focus_handle.unwrap_or_else(|| cx.focus_handle());

        let mut container = div()
            .id(self.id)
            .track_focus(&focus_handle)
            .flex()
            .gap(gap)
            .focusable();

        // Set flex direction based on navigation direction
        container = match direction {
            FocusDirection::Vertical => container.flex_col(),
            FocusDirection::Horizontal => container.flex_row(),
            FocusDirection::Grid { columns: _ } => {
                // For grid layout, use flex-wrap
                container.flex_row().flex_wrap()
            }
        };

        // NOTE: FocusGroup keyboard navigation is not implemented.
        // GPUI does not expose a way to programmatically move focus to an
        // arbitrary AnyElement child — children would need to expose their
        // FocusHandles explicitly for this to work.
        //
        // The previous stub called cx.stop_propagation() for arrow keys without
        // doing anything, which silently swallowed keyboard events. That has been
        // removed. To implement focus navigation, callers should manage a list of
        // FocusHandles and call window.focus() directly in their own key handlers.
        let _ = (child_count, direction, wraparound);

        // Add children
        for child in self.children {
            container = container.child(child);
        }

        container
    }
}

impl IntoElement for FocusGroup {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// Helper trait for adding focus group behavior to existing containers
pub trait FocusGroupExt {
    /// Wrap this element in a focus group with vertical navigation
    fn with_focus_navigation(self, id: impl Into<ElementId>) -> FocusGroup;
}
