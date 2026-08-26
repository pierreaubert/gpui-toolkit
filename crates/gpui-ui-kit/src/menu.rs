//! Menu components - MenuItem, Menu, MenuBar, and ContextMenu
//!
//! Provides a complete menu system for application navigation and context menus.

use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState, apply_native_accessibility,
};
use crate::mobile::is_mobile;
use crate::swipe_panel::{SwipePanel, SwipePanelState};
use crate::theme::{ThemeExt, glow_shadow};
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, Div, ElementId, FocusHandle, KeyDownEvent, MouseButton, Pixels, SharedString,
    Stateful, Window, div, px,
};

mod menu_bar;
mod menu_bar_item;
mod menu_item;
mod types;

pub use menu_bar::MenuBar;
pub use menu_bar_item::MenuBarItem;
pub use menu_item::MenuItem;
pub use types::{MenuTheme, menu_bar_button};

fn menu_item_accessibility(item: &menu_item::MenuItem) -> (SharedString, AriaProps) {
    let role = if item.is_checkbox {
        AriaRole::Checkbox
    } else {
        AriaRole::Menuitem
    };
    let props = AriaProps::with_role(role)
        .maybe_state(item.disabled, AriaState::Disabled)
        .maybe_state(item.is_checkbox, AriaState::Checked(item.checked));
    (item.label.clone(), props)
}

/// A dropdown menu containing menu items
///
/// # Keyboard Navigation
///
/// When a `focus_handle` is provided, the menu supports keyboard navigation:
/// - **Arrow Up/Down**: Move through items (skips separators and disabled items)
/// - **Home/End**: Jump to first/last selectable item
/// - **Enter/Space**: Select the focused item
/// - **Escape**: Close the menu (triggers on_close callback)
pub struct Menu {
    id: ElementId,
    items: Vec<menu_item::MenuItem>,
    min_width: Pixels,
    theme: Option<MenuTheme>,
    /// Index of the currently keyboard-focused item (0-based, skips separators)
    focused_index: Option<usize>,
    /// Focus handle for keyboard events
    focus_handle: Option<FocusHandle>,
    on_select: Option<Box<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
    /// Called when the menu should close (e.g., Escape pressed)
    on_close: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    /// Called when keyboard focus changes (arrow up/down, home/end)
    on_focus_change: Option<Box<dyn Fn(Option<usize>, &mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Menu {
    /// Create a new menu with items
    pub fn new(id: impl Into<ElementId>, items: Vec<menu_item::MenuItem>) -> Self {
        Self {
            id: id.into(),
            items,
            min_width: px(180.0),
            theme: None,
            focused_index: None,
            focus_handle: None,
            on_select: None,
            on_close: None,
            on_focus_change: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set minimum width
    pub fn min_width(mut self, width: Pixels) -> Self {
        self.min_width = width;
        self
    }

    /// Set theme
    pub fn theme(mut self, theme: MenuTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set the currently focused item index (for keyboard navigation)
    pub fn focused_index(mut self, index: usize) -> Self {
        self.focused_index = Some(index);
        self
    }

    /// Set the focus handle for keyboard events
    ///
    /// When provided, enables keyboard navigation with arrow keys, Enter, and Escape.
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Set the selection handler
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }

    /// Set an explicit ARIA label
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Menu)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Set the close handler (triggered by Escape key)
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Box::new(handler));
        self
    }

    /// Set the focus change handler (triggered by arrow keys, home/end)
    ///
    /// The handler receives the new focused index (or None if no item is focused).
    /// Use this to update your state and re-render the menu with the new focused_index.
    pub fn on_focus_change(
        mut self,
        handler: impl Fn(Option<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_focus_change = Some(Box::new(handler));
        self
    }

    /// Get indices of selectable items (not separators, not disabled)
    fn selectable_indices(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.is_separator && !item.disabled)
            .map(|(i, _)| i)
            .collect()
    }

    /// Get the next selectable index after the current one
    fn next_selectable_index(selectable: &[usize], current: Option<usize>) -> Option<usize> {
        if selectable.is_empty() {
            return None;
        }

        match current {
            None => selectable.first().copied(),
            Some(curr) => {
                // Find first selectable after current
                selectable.iter().find(|&&i| i > curr).copied().or_else(|| {
                    // Wrap around
                    selectable.first().copied()
                })
            }
        }
    }

    /// Get the previous selectable index before the current one
    fn prev_selectable_index(selectable: &[usize], current: Option<usize>) -> Option<usize> {
        if selectable.is_empty() {
            return None;
        }

        match current {
            None => selectable.last().copied(),
            Some(curr) => {
                // Find last selectable before current
                selectable
                    .iter()
                    .rev()
                    .find(|&&i| i < curr)
                    .copied()
                    .or_else(|| {
                        // Wrap around
                        selectable.last().copied()
                    })
            }
        }
    }

    /// Get the first selectable index
    fn first_selectable_index(selectable: &[usize]) -> Option<usize> {
        selectable.first().copied()
    }

    /// Get the last selectable index
    fn last_selectable_index(selectable: &[usize]) -> Option<usize> {
        selectable.last().copied()
    }

    /// Build into element with theme
    pub fn build_with_theme(self, menu_theme: &MenuTheme) -> Stateful<Div> {
        let min_width = self.min_width;
        let theme = self.theme.as_ref().unwrap_or(menu_theme);
        let focused_index = self.focused_index;

        // Pre-compute navigation indices once per render
        let selectable_indices = self.selectable_indices();
        let next_index = Self::next_selectable_index(&selectable_indices, focused_index);
        let prev_index = Self::prev_selectable_index(&selectable_indices, focused_index);
        let first_index = Self::first_selectable_index(&selectable_indices);
        let last_index = Self::last_selectable_index(&selectable_indices);

        // Use Rc pattern for handlers (takes ownership)
        let on_select_rc = self.on_select.map(|f| std::rc::Rc::new(f));
        let on_close_rc = self.on_close.map(|f| std::rc::Rc::new(f));
        let on_focus_change_rc = self.on_focus_change.map(|f| std::rc::Rc::new(f));

        let mut menu = div()
            .id(self.id.clone())
            .min_w(min_width)
            .max_h(px(600.0))
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded(px(4.0))
            .shadow_lg()
            .py_1()
            .overflow_y_scroll();

        // Add focus styling if focus handle is provided
        if let Some(ref handle) = self.focus_handle {
            menu = menu.track_focus(handle).focusable();
        }

        // Pre-build the two possible hover closures once per render; both
        // captured colors are `Copy`, so the closures can be reused for every
        // non-focused, non-disabled row.
        let normal_hover = {
            let hover_bg = theme.hover_bg;
            let text_hover = theme.text_hover;
            move |style: gpui::StyleRefinement| {
                style
                    .bg(hover_bg)
                    .text_color(text_hover)
                    .shadow(glow_shadow(hover_bg))
            }
        };
        let danger_hover = {
            let hover_bg = theme.danger_hover_bg;
            let text_hover = theme.text_hover;
            move |style: gpui::StyleRefinement| {
                style
                    .bg(hover_bg)
                    .text_color(text_hover)
                    .shadow(glow_shadow(hover_bg))
            }
        };

        // Build rows and collect item ids in a single pass.
        let mut rows: Vec<AnyElement> = Vec::with_capacity(self.items.len());
        let mut item_ids = Vec::with_capacity(self.items.len());
        for (index, item) in self.items.into_iter().enumerate() {
            item_ids.push(item.id.clone());
            if item.is_separator {
                rows.push(
                    div()
                        .my_1()
                        .h(px(1.0))
                        .bg(theme.separator)
                        .mx_2()
                        .into_any_element(),
                );
            } else {
                let disabled = item.disabled;
                let is_checkbox = item.is_checkbox;
                let checked = item.checked;
                let is_danger = item.is_danger;
                let is_focused = focused_index == Some(index);
                let (accessible_label, accessible_props) = menu_item_accessibility(&item);

                let mut row = div()
                        .id(ElementId::from((self.id.clone(), item.element_id.clone())))
                    .px_3()
                    .py(px(6.0))
                    .mx_1()
                    .rounded(px(3.0))
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_sm();

                if disabled {
                    row = row.text_color(theme.text_disabled).cursor_not_allowed();
                } else {
                    let text_color = theme.text;
                    let hover_bg = if is_danger {
                        theme.danger_hover_bg
                    } else {
                        theme.hover_bg
                    };

                    // Apply focus styling if this item is keyboard-focused
                    if is_focused {
                        row = row
                            .bg(hover_bg)
                            .text_color(theme.text_hover)
                            .shadow(glow_shadow(hover_bg));
                    } else {
                        row = if is_danger {
                            row.text_color(text_color).hover(danger_hover)
                        } else {
                            row.text_color(text_color).hover(normal_hover)
                        };
                    }

                    row = row.cursor_pointer();

                    if let Some(ref handler) = on_select_rc {
                        let handler = handler.clone();
                        let id = item_ids[index].clone();
                        row = row.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                            handler(&id, window, cx);
                        });
                    }
                }

                // Checkbox indicator
                if is_checkbox {
                    row = row.child(div().w(px(16.0)).text_xs().child(if checked {
                        "✓"
                    } else {
                        " "
                    }));
                }

                // Icon
                if let Some(icon) = item.icon {
                    row = row.child(div().w(px(16.0)).child(icon));
                }

                row = apply_native_accessibility(row, accessible_label, &accessible_props);

                // Label (flex-1 to push shortcut to right)
                row = row.child(div().flex_1().child(item.label));

                // Shortcut
                if let Some(shortcut) = item.shortcut {
                    let shortcut_color = theme.text_shortcut;
                    row = row.child(div().text_xs().text_color(shortcut_color).child(shortcut));
                }

                rows.push(row.into_any_element());
            }
        }

        // Keyboard event handler
        if self.focus_handle.is_some() {
            let on_select_for_keyboard = on_select_rc.clone();
            let on_close_for_keyboard = on_close_rc.clone();
            let on_focus_change_for_keyboard = on_focus_change_rc.clone();

            menu = menu.on_key_down(move |event: &KeyDownEvent, window, cx| {
                let key = event.keystroke.key.as_str();
                if let Some(action) = crate::interaction::overlay_key_action(key, true) {
                    match action {
                        crate::interaction::OverlayKeyAction::Dismiss => {
                            if let Some(ref handler) = on_close_for_keyboard {
                                handler(window, cx);
                            }
                        }
                        crate::interaction::OverlayKeyAction::Activate => {
                            // Select the focused item
                            if let Some(idx) = focused_index
                                && selectable_indices.contains(&idx)
                                && let Some(id) = item_ids.get(idx)
                                && let Some(ref handler) = on_select_for_keyboard
                            {
                                handler(id, window, cx);
                            }
                        }
                    }
                    return;
                }
                match key {
                    "down" | "arrowdown" => {
                        if let Some(ref handler) = on_focus_change_for_keyboard {
                            handler(next_index, window, cx);
                        }
                    }
                    "up" | "arrowup" => {
                        if let Some(ref handler) = on_focus_change_for_keyboard {
                            handler(prev_index, window, cx);
                        }
                    }
                    "home" => {
                        if let Some(ref handler) = on_focus_change_for_keyboard {
                            handler(first_index, window, cx);
                        }
                    }
                    "end" => {
                        if let Some(ref handler) = on_focus_change_for_keyboard {
                            handler(last_index, window, cx);
                        }
                    }
                    _ => {}
                }
            });
        }

        for row in rows {
            menu = menu.child(row);
        }

        menu
    }
}

impl RenderOnce for Menu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let menu_label = self.aria_label.clone().unwrap_or_else(|| "Menu".into());
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: menu_label,
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Menu)),
        });
        for item in &self.items {
            if item.is_separator {
                continue;
            }
            let (label, props) = menu_item_accessibility(item);
            cx.register_accessible(AccessibilityNode {
                element_id: ElementId::from((self.id.clone(), item.element_id.clone())),
                label,
                props,
            });
        }

        let id = self.id.clone();
        let global_theme = cx.theme();
        let menu_theme = MenuTheme::from(global_theme);
        let menu = self.build_with_theme(&menu_theme);

        if is_mobile(window, cx) {
            SwipePanel::new(id)
                .anchor(crate::swipe_panel::SwipePanelAnchor::Bottom)
                .state(SwipePanelState::Expanded)
                .show_backdrop(true)
                .content(menu)
                .into_any_element()
        } else {
            menu.into_any_element()
        }
    }
}

impl IntoElement for Menu {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
