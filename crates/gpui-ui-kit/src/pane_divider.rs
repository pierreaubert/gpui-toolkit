//! Pane Divider Component
//!
//! An interactive divider between panels that supports:
//! - Arrows indicating collapse direction
//! - Collapsed state with vertical label
//! - Double-click to toggle collapse
//! - Drag to resize (via parent tracking mouse state)

use crate::ComponentTheme;
use gpui::prelude::*;
use gpui::*;

/// Direction the divider collapses toward
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollapseDirection {
    /// Collapse the panel to the left (for vertical dividers)
    Left,
    /// Collapse the panel to the right (for vertical dividers)
    Right,
    /// Collapse the panel upward (for horizontal dividers)
    Up,
    /// Collapse the panel downward (for horizontal dividers)
    Down,
}

impl CollapseDirection {
    /// Get the opposite direction (for showing expand arrows when collapsed)
    pub fn opposite(&self) -> Self {
        match self {
            CollapseDirection::Left => CollapseDirection::Right,
            CollapseDirection::Right => CollapseDirection::Left,
            CollapseDirection::Up => CollapseDirection::Down,
            CollapseDirection::Down => CollapseDirection::Up,
        }
    }

    /// Get the arrow character(s) for this direction
    pub fn arrows(&self) -> &'static str {
        match self {
            CollapseDirection::Left => "\u{25C0}\u{25C0}",  // ◀◀
            CollapseDirection::Right => "\u{25B6}\u{25B6}", // ▶▶
            CollapseDirection::Up => "\u{25B2}\u{25B2}",    // ▲▲
            CollapseDirection::Down => "\u{25BC}\u{25BC}",  // ▼▼
        }
    }

    /// Get single arrow for hover state
    pub fn single_arrow(&self) -> &'static str {
        match self {
            CollapseDirection::Left => "\u{25C0}",  // ◀
            CollapseDirection::Right => "\u{25B6}", // ▶
            CollapseDirection::Up => "\u{25B2}",    // ▲
            CollapseDirection::Down => "\u{25BC}",  // ▼
        }
    }

    /// Whether this is a horizontal collapse (for vertical dividers)
    pub fn is_horizontal(&self) -> bool {
        matches!(self, CollapseDirection::Left | CollapseDirection::Right)
    }
}

/// Theme for the pane divider
#[derive(Debug, Clone, ComponentTheme)]
pub struct PaneDividerTheme {
    /// Background color of the divider
    #[theme(default = 0x2d2d2d, from = surface)]
    pub background: Rgba,
    /// Background color when hovered
    #[theme(default = 0x3a3a3a, from = surface_hover)]
    pub background_hover: Rgba,
    /// Background color when collapsed
    #[theme(default = 0x252525, from = muted)]
    pub background_collapsed: Rgba,
    /// Arrow/text color
    #[theme(default = 0x808080, from = text_muted)]
    pub foreground: Rgba,
    /// Arrow/text color when hovered
    #[theme(default = 0xcccccc, from = text_secondary)]
    pub foreground_hover: Rgba,
    /// Border color
    #[theme(default = 0x3a3a3a, from = border)]
    pub border: Rgba,
}

/// Interactive pane divider with collapse support
///
/// # Drag Handling
///
/// For resize drag support, the parent component must:
/// 1. Listen to `on_drag_start` to know when dragging begins
/// 2. Track mouse position globally (e.g., via window-level mouse_move)
/// 3. Call `on_drag_end` when the mouse is released
///
/// Example:
/// ```ignore
/// struct MyApp {
///     left_width: f32,
///     dragging_divider: bool,
///     drag_start_x: f32,
///     drag_start_width: f32,
/// }
///
/// // In render:
/// PaneDivider::vertical("divider", CollapseDirection::Left)
///     .on_drag_start(cx.listener(|app, position, window, cx| {
///         app.dragging_divider = true;
///         app.drag_start_x = position;
///         app.drag_start_width = app.left_width;
///     }))
///
/// // Parent div should have on_mouse_move and on_mouse_up handlers
/// ```
pub struct PaneDivider {
    id: SharedString,
    /// The label shown when collapsed (e.g., "Sidebar", "Left Panel")
    label: SharedString,
    /// Direction this divider collapses toward
    collapse_direction: CollapseDirection,
    /// Whether the panel is currently collapsed
    collapsed: bool,
    /// Callback when collapse state changes (receives new collapsed state)
    on_toggle: Option<Box<dyn Fn(bool, &mut Window, &mut App) + 'static>>,
    /// Callback when drag starts (receives position in the drag axis: x for vertical, y for horizontal)
    on_drag_start: Option<Box<dyn Fn(f32, &mut Window, &mut App) + 'static>>,
    /// Theme for styling
    theme: PaneDividerTheme,
    /// Thickness of the divider when not collapsed
    thickness: Pixels,
    /// Width of the collapsed bar (perpendicular to divider orientation)
    collapsed_size: Pixels,
}

impl PaneDivider {
    /// Create a new vertical pane divider (sits between left and right panels)
    pub fn vertical(id: impl Into<SharedString>, collapse_direction: CollapseDirection) -> Self {
        assert!(
            collapse_direction.is_horizontal(),
            "Vertical dividers must use Left or Right collapse direction"
        );
        Self {
            id: id.into(),
            label: SharedString::from(""),
            collapse_direction,
            collapsed: false,
            on_toggle: None,
            on_drag_start: None,
            theme: PaneDividerTheme::default(),
            thickness: px(6.0),
            collapsed_size: px(24.0),
        }
    }

    /// Create a new horizontal pane divider (sits between top and bottom panels)
    pub fn horizontal(id: impl Into<SharedString>, collapse_direction: CollapseDirection) -> Self {
        assert!(
            !collapse_direction.is_horizontal(),
            "Horizontal dividers must use Up or Down collapse direction"
        );
        Self {
            id: id.into(),
            label: SharedString::from(""),
            collapse_direction,
            collapsed: false,
            on_toggle: None,
            on_drag_start: None,
            theme: PaneDividerTheme::default(),
            thickness: px(6.0),
            collapsed_size: px(24.0),
        }
    }

    /// Set the label shown when collapsed
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    /// Set whether the divider is collapsed
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Set callback for when collapse state toggles (double-click or click when collapsed)
    pub fn on_toggle(mut self, callback: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Box::new(callback));
        self
    }

    /// Set callback for when drag starts
    ///
    /// The callback receives the position (x for vertical dividers, y for horizontal).
    /// The parent component should then track mouse movement globally.
    pub fn on_drag_start(
        mut self,
        callback: impl Fn(f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_drag_start = Some(Box::new(callback));
        self
    }

    /// Set the theme
    pub fn theme(mut self, theme: PaneDividerTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the thickness of the divider (when not collapsed)
    pub fn thickness(mut self, thickness: Pixels) -> Self {
        self.thickness = thickness;
        self
    }

    /// Set the size of the collapsed bar
    pub fn collapsed_size(mut self, size: Pixels) -> Self {
        self.collapsed_size = size;
        self
    }

    /// Build the element
    fn build(self) -> Stateful<Div> {
        let is_vertical = self.collapse_direction.is_horizontal();

        if self.collapsed {
            self.build_collapsed(is_vertical)
        } else {
            self.build_expanded(is_vertical)
        }
    }

    /// Build the expanded (normal) divider
    fn build_expanded(self, is_vertical: bool) -> Stateful<Div> {
        let theme = self.theme.clone();
        let id = self.id.clone();
        let collapse_dir = self.collapse_direction;
        let on_toggle = self.on_toggle;
        let on_drag_start = self.on_drag_start;

        let cursor = if is_vertical {
            CursorStyle::ResizeLeftRight
        } else {
            CursorStyle::ResizeUpDown
        };

        // Arrow indicator
        let arrow = collapse_dir.single_arrow();

        let mut base = if is_vertical {
            // Vertical divider (between left/right panels)
            div()
                .id(id)
                .w(self.thickness)
                .h_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .bg(theme.background)
                .border_x_1()
                .border_color(theme.border)
                .cursor(cursor)
                .child(
                    div()
                        .text_color(theme.foreground)
                        .text_size(px(10.0))
                        .child(arrow),
                )
        } else {
            // Horizontal divider (between top/bottom panels)
            div()
                .id(id)
                .h(self.thickness)
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.background)
                .border_y_1()
                .border_color(theme.border)
                .cursor(cursor)
                .child(
                    div()
                        .text_color(theme.foreground)
                        .text_size(px(10.0))
                        .child(arrow),
                )
        };

        // Hover styling
        let hover_bg = theme.background_hover;
        let hover_fg = theme.foreground_hover;
        base = base.hover(move |style| style.bg(hover_bg).text_color(hover_fg));

        // Mouse down handler: double-click toggles, single click starts drag
        if on_toggle.is_some() || on_drag_start.is_some() {
            base = base.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                if event.click_count == 2 {
                    // Double-click: toggle collapse
                    if let Some(ref toggle_cb) = on_toggle {
                        toggle_cb(true, window, cx);
                    }
                } else if event.click_count == 1 {
                    // Single click: start drag
                    if let Some(ref drag_cb) = on_drag_start {
                        let pos: f32 = if is_vertical {
                            event.position.x.into()
                        } else {
                            event.position.y.into()
                        };
                        drag_cb(pos, window, cx);
                    }
                }
            });
        }

        base
    }

    /// Build the collapsed divider with vertical label
    fn build_collapsed(self, is_vertical: bool) -> Stateful<Div> {
        let theme = self.theme.clone();
        let id = self.id.clone();
        let expand_dir = self.collapse_direction.opposite();
        let on_toggle = self.on_toggle;
        let label = self.label.clone();

        // When collapsed, show arrows pointing to expand
        let arrows = expand_dir.arrows();

        let mut base = if is_vertical {
            // Collapsed vertical divider - becomes a narrow vertical bar with rotated text
            let label_chars: Vec<char> = label.chars().collect();
            let label_elements: Vec<AnyElement> = label_chars
                .into_iter()
                .map(|c| {
                    div()
                        .text_color(theme.foreground)
                        .text_size(px(11.0))
                        .child(c.to_string())
                        .into_any_element()
                })
                .collect();

            div()
                .id(id)
                .w(self.collapsed_size)
                .h_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(4.0))
                .bg(theme.background_collapsed)
                .border_x_1()
                .border_color(theme.border)
                .cursor_pointer()
                // Top arrows
                .child(
                    div()
                        .text_color(theme.foreground)
                        .text_size(px(10.0))
                        .child(arrows),
                )
                // Vertical label (each char stacked)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(2.0))
                        .py_2()
                        .children(label_elements),
                )
                // Bottom arrows
                .child(
                    div()
                        .text_color(theme.foreground)
                        .text_size(px(10.0))
                        .child(arrows),
                )
        } else {
            // Collapsed horizontal divider - becomes a narrow horizontal bar
            div()
                .id(id)
                .h(self.collapsed_size)
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .bg(theme.background_collapsed)
                .border_y_1()
                .border_color(theme.border)
                .cursor_pointer()
                // Left arrows
                .child(
                    div()
                        .text_color(theme.foreground)
                        .text_size(px(10.0))
                        .child(arrows),
                )
                // Label
                .child(
                    div()
                        .text_color(theme.foreground)
                        .text_size(px(11.0))
                        .child(label.to_string()),
                )
                // Right arrows
                .child(
                    div()
                        .text_color(theme.foreground)
                        .text_size(px(10.0))
                        .child(arrows),
                )
        };

        // Hover styling
        let hover_bg = theme.background_hover;
        let hover_fg = theme.foreground_hover;
        base = base.hover(move |style| style.bg(hover_bg).text_color(hover_fg));

        // Click to expand
        if let Some(toggle_cb) = on_toggle {
            base = base.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                // Any click on collapsed divider expands it
                toggle_cb(false, window, cx);
            });
        }

        base
    }
}

impl IntoElement for PaneDivider {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        self.build()
    }
}
