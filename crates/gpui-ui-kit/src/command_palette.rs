//! CommandPalette component
//!
//! A Cmd+K / Ctrl+K style fuzzy command search overlay.
//!
//! # Usage
//!
//! ```ignore
//! CommandPalette::new("cmd-palette", vec![
//!     CommandItem::new("open-file", "Open File").shortcut("Cmd+O"),
//!     CommandItem::new("save", "Save").shortcut("Cmd+S"),
//!     CommandItem::new("settings", "Open Settings").category("Preferences"),
//! ])
//! .placeholder("Type a command...")
//! .on_select(|id, window, cx| { /* handle selection */ })
//! ```
use crate::ComponentTheme;
use crate::data_navigation::{DataNavigationAction, DataNavigationState};
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    App, Div, ElementId, FocusHandle, FontWeight, KeyDownEvent, MouseButton, Rgba, SharedString,
    Stateful, Window, div, px,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Theme colors for command palette
#[derive(Debug, Clone, ComponentTheme)]
pub struct CommandPaletteTheme {
    /// Overlay backdrop
    #[theme(default = 0x00000088, from = overlay_bg)]
    pub backdrop: Rgba,
    /// Palette background
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Palette border
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Input text color
    #[theme(default = 0xeeeeeeff, from = text_primary)]
    pub input_text: Rgba,
    /// Input placeholder color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub placeholder_text: Rgba,
    /// Item text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub item_text: Rgba,
    /// Highlighted/selected item background
    #[theme(default = 0x2a2a4aff, from = accent)]
    pub selected_bg: Rgba,
    /// Selected item text
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub selected_text: Rgba,
    /// Item hover background
    #[theme(default = 0x2a2a2aff, from = surface_hover)]
    pub hover_bg: Rgba,
    /// Category/shortcut label color
    #[theme(default = 0x888888ff, from = text_muted)]
    pub meta_text: Rgba,
    /// Separator
    #[theme(default = 0x2a2a2aff, from = border)]
    pub separator: Rgba,
}

/// A command item
pub struct CommandItem {
    id: SharedString,
    label: SharedString,
    /// Lowercased label for case-insensitive filtering without re-allocating
    /// on every keystroke.
    label_lower: SharedString,
    shortcut: Option<SharedString>,
    category: Option<SharedString>,
    icon: Option<SharedString>,
    disabled: bool,
}

impl CommandItem {
    /// Create a new command item
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        let label_lower = label.to_lowercase().into();
        Self {
            id: id.into(),
            label,
            label_lower,
            shortcut: None,
            category: None,
            icon: None,
            disabled: false,
        }
    }

    /// Set the item label (also refreshes the cached lowercase form)
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        self.label_lower = label.to_lowercase().into();
        self.label = label;
        self
    }

    /// Set keyboard shortcut label
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Set category
    pub fn category(mut self, category: impl Into<SharedString>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set icon
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set disabled
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn content_hash<H: Hasher>(&self, hasher: &mut H) {
        self.id.hash(hasher);
        self.label.hash(hasher);
        self.label_lower.hash(hasher);
        self.shortcut.hash(hasher);
        self.category.hash(hasher);
        self.icon.hash(hasher);
        self.disabled.hash(hasher);
    }
}

fn items_hash(items: &[CommandItem]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for item in items {
        item.content_hash(&mut hasher);
    }
    hasher.finish()
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct FilteredIndicesKey {
    id: ElementId,
    query: SharedString,
    items_hash: u64,
    max_visible: usize,
}

const MAX_FILTERED_INDICES_CACHE_ENTRIES: usize = 64;

thread_local! {
    static FILTERED_INDICES_CACHE: RefCell<HashMap<FilteredIndicesKey, Rc<[usize]>>> =
        RefCell::new(HashMap::new());
}

/// A command palette component
pub struct CommandPalette {
    id: ElementId,
    items: Vec<CommandItem>,
    placeholder: SharedString,
    query: SharedString,
    selected_index: usize,
    focus_handle: Option<FocusHandle>,
    max_visible: usize,
    on_highlight_change: Option<Box<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    on_select: Option<Box<dyn Fn(SharedString, &mut Window, &mut App) + 'static>>,
    on_dismiss: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl CommandPalette {
    /// Create a new command palette
    pub fn new(id: impl Into<ElementId>, items: Vec<CommandItem>) -> Self {
        Self {
            id: id.into(),
            items,
            placeholder: "Type a command...".into(),
            query: "".into(),
            selected_index: 0,
            focus_handle: None,
            max_visible: 10,
            on_highlight_change: None,
            on_select: None,
            on_dismiss: None,
        }
    }

    /// Set placeholder text
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the current query text
    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    /// Set selected index
    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    /// Set the focus handle used for keyboard navigation.
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Set max visible items
    pub fn max_visible(mut self, max: usize) -> Self {
        self.max_visible = max;
        self
    }

    /// Called when keyboard navigation highlights a different command.
    pub fn on_highlight_change(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_highlight_change = Some(Box::new(handler));
        self
    }

    /// Called when a command is selected
    pub fn on_select(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }

    /// Called when the palette is dismissed
    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Box::new(handler));
        self
    }

    /// Return the indices of items visible for the current query, using a
    /// thread-local cache keyed by the lowercased query and a hash of the items.
    fn filtered_indices(&self) -> Rc<[usize]> {
        let query_lower: SharedString = self.query.to_lowercase().into();
        let hash = items_hash(&self.items);
        let key = FilteredIndicesKey {
            id: self.id.clone(),
            query: query_lower.clone(),
            items_hash: hash,
            max_visible: self.max_visible,
        };

        FILTERED_INDICES_CACHE.with(|cache| {
            if let Some(cached) = cache.borrow().get(&key) {
                return Rc::clone(cached);
            }

            let mut indices = Vec::new();
            for (i, item) in self.items.iter().enumerate() {
                if !query_lower.is_empty() && !item.label_lower.contains(query_lower.as_ref()) {
                    continue;
                }
                indices.push(i);
                if indices.len() >= self.max_visible {
                    break;
                }
            }

            let result: Rc<[usize]> = indices.into();
            let mut cache = cache.borrow_mut();
            if cache.len() >= MAX_FILTERED_INDICES_CACHE_ENTRIES {
                cache.clear();
            }
            cache.insert(key, Rc::clone(&result));
            result
        })
    }

    /// Build with theme
    pub fn build_with_theme(self, theme: &CommandPaletteTheme, cx: &mut App) -> Stateful<Div> {
        let palette_id = self.id.clone();
        let filtered = self.filtered_indices();
        let focus_handle = self
            .focus_handle
            .clone()
            .unwrap_or_else(|| cx.focus_handle());
        let hover_bg = theme.hover_bg;
        let hover_handler = move |style: gpui::StyleRefinement| style.bg(hover_bg);
        let dismiss_id = (palette_id.clone(), "backdrop");
        let on_highlight_change = self.on_highlight_change.map(std::rc::Rc::new);
        let on_select = self.on_select.map(std::rc::Rc::new);
        let on_dismiss = self.on_dismiss.map(std::rc::Rc::new);

        // Backdrop
        let mut overlay = div()
            .id(palette_id.clone())
            .absolute()
            .inset_0()
            .flex()
            .flex_col()
            .items_center()
            .pt(px(80.0))
            .bg(theme.backdrop)
            .track_focus(&focus_handle)
            .focusable()
            .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {});

        if on_highlight_change.is_some() || on_select.is_some() || on_dismiss.is_some() {
            let focus_handle_for_key = focus_handle.clone();
            let selected_index = self.selected_index;
            let filtered_for_key = filtered.clone();
            let item_ids: Vec<SharedString> =
                self.items.iter().map(|item| item.id.clone()).collect();
            let disabled: Vec<bool> = self.items.iter().map(|item| item.disabled).collect();
            let on_highlight_change_for_key = on_highlight_change.clone();
            let on_select_for_key = on_select.clone();
            let on_dismiss_for_key = on_dismiss.clone();

            overlay = overlay.on_key_down(
                move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
                    if !focus_handle_for_key.is_focused(window) {
                        return;
                    }

                    let Some(action) = DataNavigationAction::from_key(event.keystroke.key.as_str())
                    else {
                        return;
                    };

                    match action {
                        DataNavigationAction::Previous
                        | DataNavigationAction::Next
                        | DataNavigationAction::First
                        | DataNavigationAction::Last => {
                            let selected_visible_index = filtered_for_key
                                .iter()
                                .position(|index| *index == selected_index);
                            if let Some(next_visible_index) =
                                DataNavigationState::new(filtered_for_key.len())
                                    .selected_index(selected_visible_index)
                                    .move_selection(action)
                                && let Some(&next_index) = filtered_for_key.get(next_visible_index)
                                && next_index != selected_index
                            {
                                cx.stop_propagation();
                                if let Some(ref handler) = on_highlight_change_for_key {
                                    handler(next_index, window, cx);
                                }
                            }
                        }
                        DataNavigationAction::Activate => {
                            if !disabled.get(selected_index).copied().unwrap_or(true)
                                && let Some(id) = item_ids.get(selected_index).cloned()
                                && let Some(ref handler) = on_select_for_key
                            {
                                cx.stop_propagation();
                                handler(id, window, cx);
                            }
                        }
                        DataNavigationAction::Dismiss => {
                            if let Some(ref handler) = on_dismiss_for_key {
                                cx.stop_propagation();
                                handler(window, cx);
                            }
                        }
                        _ => {}
                    }
                },
            );
        }

        // Dismiss on backdrop click
        if let Some(handler) = on_dismiss {
            overlay = overlay.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                handler(window, cx);
            });
        }

        // Palette container
        let mut palette = div()
            .id(dismiss_id)
            .w(px(500.0))
            .max_h(px(400.0))
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded(px(12.0))
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {
                // Stop propagation to prevent backdrop dismiss
            });

        // Search input area
        let input_area = div()
            .w_full()
            .flex()
            .items_center()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(theme.separator)
            .child(if self.query.is_empty() {
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(theme.placeholder_text)
                    .child(self.placeholder.clone())
            } else {
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(theme.input_text)
                    .child(self.query.clone())
            });

        palette = palette.child(input_area);

        // Results list
        let mut results = div().flex_1().flex().flex_col().overflow_y_hidden();

        let mut current_category: Option<SharedString> = None;

        for &i in filtered.iter() {
            let item = &self.items[i];

            // Category header
            if let Some(cat) = &item.category
                && current_category.as_ref() != Some(cat)
            {
                current_category = Some(cat.clone());
                results = results.child(
                    div()
                        .px_4()
                        .py_1()
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.meta_text)
                        .child(cat.clone()),
                );
            }

            let is_selected = i == self.selected_index;

            let mut row = div()
                .id(ElementId::from((palette_id.clone(), item.id.clone())))
                .w_full()
                .flex()
                .items_center()
                .gap_3()
                .px_4()
                .py_2()
                .cursor_pointer();

            if is_selected {
                row = row.bg(theme.selected_bg).text_color(theme.selected_text);
            } else if item.disabled {
                row = row.text_color(theme.meta_text).opacity(0.5);
            } else {
                row = row.text_color(theme.item_text).hover(hover_handler);
            }

            if !item.disabled {
                let item_id = item.id.clone();
                let on_select = on_select.clone();
                row = row.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                    cx.stop_propagation();
                    if let Some(handler) = &on_select {
                        handler(item_id.clone(), window, cx);
                    }
                });
            }

            // Icon
            if let Some(icon) = &item.icon {
                row = row.child(div().w(px(16.0)).child(icon.clone()));
            }

            // Label
            row = row.child(div().flex_1().text_sm().child(item.label.clone()));

            // Shortcut
            if let Some(shortcut) = &item.shortcut {
                row = row.child(
                    div()
                        .text_xs()
                        .text_color(theme.meta_text)
                        .px_2()
                        .py(px(1.0))
                        .bg(theme.separator)
                        .rounded(px(3.0))
                        .child(shortcut.clone()),
                );
            }

            results = results.child(row);
        }

        palette = palette.child(results);

        overlay = overlay.child(palette);

        overlay
    }
}

impl RenderOnce for CommandPalette {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = CommandPaletteTheme::from(global_theme);
        self.build_with_theme(&theme, cx)
    }
}

impl IntoElement for CommandPalette {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandItem, CommandPalette};

    #[test]
    fn command_item_caches_lowercase_label() {
        let item = CommandItem::new("open", "Open File");
        assert_eq!(item.label_lower.as_ref(), "open file");

        let renamed = CommandItem::new("save", "Save").label("Save As...");
        assert_eq!(renamed.label_lower.as_ref(), "save as...");
    }

    #[test]
    fn command_palette_builder_records_keyboard_navigation_handlers() {
        let palette = CommandPalette::new(
            "commands",
            vec![
                CommandItem::new("open", "Open File"),
                CommandItem::new("save", "Save").disabled(true),
            ],
        )
        .selected_index(1)
        .on_highlight_change(|_, _, _| {})
        .on_select(|_, _, _| {})
        .on_dismiss(|_, _| {});

        assert_eq!(palette.selected_index, 1);
        assert!(palette.on_highlight_change.is_some());
        assert!(palette.on_select.is_some());
        assert!(palette.on_dismiss.is_some());
        assert!(palette.items[1].disabled);
    }
}
