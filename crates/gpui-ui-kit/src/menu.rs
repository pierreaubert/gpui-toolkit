//! Menu components - MenuItem, Menu, MenuBar, and ContextMenu
//!
//! Provides a complete menu system for application navigation and context menus.

use crate::ComponentTheme;
use crate::theme::{ThemeExt, glow_shadow};
use gpui::prelude::*;
use gpui::*;

/// Theme colors for menu styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct MenuTheme {
    /// Menu background color
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub background: Rgba,
    /// Menu border color
    #[theme(default = 0x444444ff, from = border)]
    pub border: Rgba,
    /// Separator color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub separator: Rgba,
    /// Normal item text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text: Rgba,
    /// Hovered item text color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_hover: Rgba,
    /// Disabled item text color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub text_disabled: Rgba,
    /// Shortcut text color
    #[theme(default = 0x777777ff, from = text_muted)]
    pub text_shortcut: Rgba,
    /// Item hover background color
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub hover_bg: Rgba,
    /// Danger item hover background (for destructive actions like Quit)
    #[theme(default = 0xdc2626ff, from = error)]
    pub danger_hover_bg: Rgba,
}

/// A single menu item
#[derive(Clone)]
pub struct MenuItem {
    id: SharedString,
    label: SharedString,
    shortcut: Option<SharedString>,
    icon: Option<SharedString>,
    disabled: bool,
    is_separator: bool,
    is_checkbox: bool,
    checked: bool,
    is_danger: bool,
    children: Vec<MenuItem>,
}

impl MenuItem {
    /// Create a new menu item
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            icon: None,
            disabled: false,
            is_separator: false,
            is_checkbox: false,
            checked: false,
            is_danger: false,
            children: Vec::new(),
        }
    }

    /// Create a separator item
    pub fn separator() -> Self {
        Self {
            id: "separator".into(),
            label: "".into(),
            shortcut: None,
            icon: None,
            disabled: true,
            is_separator: true,
            is_checkbox: false,
            checked: false,
            is_danger: false,
            children: Vec::new(),
        }
    }

    /// Create a checkbox menu item
    pub fn checkbox(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        checked: bool,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            icon: None,
            disabled: false,
            is_separator: false,
            is_checkbox: true,
            checked,
            is_danger: false,
            children: Vec::new(),
        }
    }

    /// Add a keyboard shortcut display
    pub fn with_shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Add an icon
    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Disable the menu item
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add submenu items
    pub fn with_children(mut self, children: Vec<MenuItem>) -> Self {
        self.children = children;
        self
    }

    /// Get the item ID
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    /// Check if this is a separator
    pub fn is_separator(&self) -> bool {
        self.is_separator
    }

    /// Mark as a danger/destructive action (e.g., Quit, Delete)
    pub fn danger(mut self) -> Self {
        self.is_danger = true;
        self
    }

    /// Check if this is a danger item
    pub fn is_danger(&self) -> bool {
        self.is_danger
    }
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
    items: Vec<MenuItem>,
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
}

impl Menu {
    /// Create a new menu with items
    pub fn new(id: impl Into<ElementId>, items: Vec<MenuItem>) -> Self {
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
    fn next_selectable_index(&self, current: Option<usize>) -> Option<usize> {
        let selectable = self.selectable_indices();
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
    fn prev_selectable_index(&self, current: Option<usize>) -> Option<usize> {
        let selectable = self.selectable_indices();
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
    fn first_selectable_index(&self) -> Option<usize> {
        self.selectable_indices().first().copied()
    }

    /// Get the last selectable index
    fn last_selectable_index(&self) -> Option<usize> {
        self.selectable_indices().last().copied()
    }

    /// Build into element with theme
    pub fn build_with_theme(self, menu_theme: &MenuTheme) -> Stateful<Div> {
        let min_width = self.min_width;
        let theme = self.theme.as_ref().unwrap_or(menu_theme);
        let focused_index = self.focused_index;

        // Pre-compute navigation indices BEFORE taking ownership
        let selectable_indices = self.selectable_indices();
        let next_index = self.next_selectable_index(focused_index);
        let prev_index = self.prev_selectable_index(focused_index);
        let first_index = self.first_selectable_index();
        let last_index = self.last_selectable_index();

        // Clone items for keyboard handler BEFORE taking ownership
        let items_for_keyboard: Vec<_> = self
            .items
            .iter()
            .map(|item| (item.id.clone(), item.is_separator, item.disabled))
            .collect();

        // Use Rc pattern for handlers (takes ownership)
        let on_select_rc = self.on_select.map(|f| std::rc::Rc::new(f));
        let on_close_rc = self.on_close.map(|f| std::rc::Rc::new(f));
        let on_focus_change_rc = self.on_focus_change.map(|f| std::rc::Rc::new(f));

        let mut menu = div()
            .id(self.id)
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
            menu = menu.track_focus(handle);
        }

        // Keyboard event handler
        if self.focus_handle.is_some() {
            let on_select_for_keyboard = on_select_rc.clone();
            let on_close_for_keyboard = on_close_rc.clone();
            let on_focus_change_for_keyboard = on_focus_change_rc.clone();
            let _selectable = selectable_indices; // For potential future use

            menu = menu.on_key_down(move |event: &KeyDownEvent, window, cx| {
                let key = event.keystroke.key.as_str();
                match key {
                    "escape" => {
                        if let Some(ref handler) = on_close_for_keyboard {
                            handler(window, cx);
                        }
                    }
                    "enter" | " " => {
                        // Select the focused item
                        if let Some(idx) = focused_index
                            && let Some((id, is_sep, disabled)) = items_for_keyboard.get(idx)
                            && !*is_sep
                            && !*disabled
                            && let Some(ref handler) = on_select_for_keyboard
                        {
                            handler(id, window, cx);
                        }
                    }
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

        for (index, item) in self.items.into_iter().enumerate() {
            if item.is_separator {
                menu = menu.child(div().my_1().h(px(1.0)).bg(theme.separator).mx_2());
            } else {
                let item_id = item.id.clone();
                let label = item.label.clone();
                let shortcut = item.shortcut.clone();
                let icon = item.icon.clone();
                let disabled = item.disabled;
                let is_checkbox = item.is_checkbox;
                let checked = item.checked;
                let is_danger = item.is_danger;
                let is_focused = focused_index == Some(index);

                let mut row = div()
                    .id(SharedString::from(format!("menu-item-{}", item_id)))
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
                    let text_hover = theme.text_hover;
                    let hover_bg = if is_danger {
                        theme.danger_hover_bg
                    } else {
                        theme.hover_bg
                    };

                    // Apply focus styling if this item is keyboard-focused
                    if is_focused {
                        row = row
                            .bg(hover_bg)
                            .text_color(text_hover)
                            .shadow(glow_shadow(hover_bg));
                    } else {
                        row = row.text_color(text_color).hover(move |style| {
                            style
                                .bg(hover_bg)
                                .text_color(text_hover)
                                .shadow(glow_shadow(hover_bg))
                        });
                    }

                    row = row.cursor_pointer();

                    if let Some(ref handler) = on_select_rc {
                        let handler = handler.clone();
                        let id = item_id.clone();
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
                if let Some(icon) = icon {
                    row = row.child(div().w(px(16.0)).child(icon));
                }

                // Label (flex-1 to push shortcut to right)
                row = row.child(div().flex_1().child(label));

                // Shortcut
                if let Some(shortcut) = shortcut {
                    let shortcut_color = theme.text_shortcut;
                    row = row.child(div().text_xs().text_color(shortcut_color).child(shortcut));
                }

                menu = menu.child(row);
            }
        }

        menu
    }
}

impl RenderOnce for Menu {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let menu_theme = MenuTheme::from(&global_theme);
        self.build_with_theme(&menu_theme)
    }
}

impl IntoElement for Menu {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// A menu bar item (top-level menu)
pub struct MenuBarItem {
    id: SharedString,
    label: SharedString,
    items: Vec<MenuItem>,
}

impl MenuBarItem {
    /// Create a new menu bar item
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            items: Vec::new(),
        }
    }

    /// Set the dropdown items
    pub fn with_items(mut self, items: Vec<MenuItem>) -> Self {
        self.items = items;
        self
    }

    /// Get the menu ID
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    /// Get the menu label
    pub fn label(&self) -> &SharedString {
        &self.label
    }

    /// Get the menu items
    pub fn items(&self) -> &[MenuItem] {
        &self.items
    }
}

/// A horizontal menu bar
pub struct MenuBar {
    items: Vec<MenuBarItem>,
    active_menu: Option<SharedString>,
    on_select: Option<Box<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
    on_menu_toggle: Option<Box<dyn Fn(Option<&SharedString>, &mut Window, &mut App) + 'static>>,
}

impl MenuBar {
    /// Create a new menu bar
    pub fn new(items: Vec<MenuBarItem>) -> Self {
        Self {
            items,
            active_menu: None,
            on_select: None,
            on_menu_toggle: None,
        }
    }

    /// Set the currently active (open) menu
    pub fn active_menu(mut self, id: Option<SharedString>) -> Self {
        self.active_menu = id;
        self
    }

    /// Set the item selection handler
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }

    /// Set the menu toggle handler
    pub fn on_menu_toggle(
        mut self,
        handler: impl Fn(Option<&SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_menu_toggle = Some(Box::new(handler));
        self
    }

    /// Get menu bar items (for external rendering with custom handlers)
    pub fn items(&self) -> &[MenuBarItem] {
        &self.items
    }

    /// Get active menu ID
    pub fn get_active_menu(&self) -> Option<&SharedString> {
        self.active_menu.as_ref()
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &MenuTheme) -> Div {
        // Use Rc pattern instead of unsafe pointer for on_menu_toggle handler
        let on_toggle_rc = self.on_menu_toggle.map(|f| std::rc::Rc::new(f));

        let mut bar = div().flex().items_center().gap_1();

        for item in &self.items {
            let is_open = self.active_menu.as_ref() == Some(&item.id);
            let menu_id = item.id.clone();
            let label = item.label.clone();

            let mut button = div()
                .id(SharedString::from(format!("menubar-{}", menu_id)))
                .px_3()
                .py_1()
                .rounded(px(3.0))
                .text_sm()
                .cursor_pointer();

            if is_open {
                button = button
                    .bg(theme.hover_bg)
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_hover);
            } else {
                let hover_bg = theme.hover_bg;
                button = button
                    .text_color(theme.text)
                    .hover(move |style| style.bg(hover_bg).shadow(glow_shadow(hover_bg)));
            }

            if let Some(ref handler) = on_toggle_rc {
                let handler = handler.clone();
                let id = menu_id.clone();
                let currently_open = is_open;
                button = button.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                    if currently_open {
                        handler(None, window, cx);
                    } else {
                        handler(Some(&id), window, cx);
                    }
                });
            }

            button = button.child(label);
            bar = bar.child(button);
        }

        bar
    }
}

impl RenderOnce for MenuBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let menu_theme = MenuTheme::from(&global_theme);
        self.build_with_theme(&menu_theme)
    }
}

impl IntoElement for MenuBar {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// Helper to build a single menu bar button without handlers
/// Use this when you need to add cx.listener() handlers
pub fn menu_bar_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    is_open: bool,
    theme: &MenuTheme,
) -> Stateful<Div> {
    let id = id.into();
    let label = label.into();

    let mut button = div()
        .id(SharedString::from(format!("menubar-{}", id)))
        .px_3()
        .py_1()
        .rounded(px(3.0))
        .text_sm()
        .cursor_pointer();

    if is_open {
        button = button
            .bg(theme.hover_bg)
            .font_weight(FontWeight::BOLD)
            .text_color(theme.text_hover);
    } else {
        let hover_bg = theme.hover_bg;
        button = button
            .text_color(theme.text)
            .hover(move |style| style.bg(hover_bg).shadow(glow_shadow(hover_bg)));
    }

    button.child(label)
}
