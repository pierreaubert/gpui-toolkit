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
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Div, ElementId, FontWeight, MouseButton, Rgba, SharedString, Stateful, Window, div, px,
};

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
    shortcut: Option<SharedString>,
    category: Option<SharedString>,
    icon: Option<SharedString>,
    disabled: bool,
}

impl CommandItem {
    /// Create a new command item
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            category: None,
            icon: None,
            disabled: false,
        }
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
}

/// A command palette component
pub struct CommandPalette {
    id: ElementId,
    items: Vec<CommandItem>,
    placeholder: SharedString,
    query: SharedString,
    selected_index: usize,
    max_visible: usize,
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
            max_visible: 10,
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

    /// Set max visible items
    pub fn max_visible(mut self, max: usize) -> Self {
        self.max_visible = max;
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

    /// Build with theme
    pub fn build_with_theme(self, theme: &CommandPaletteTheme) -> Stateful<Div> {
        let dismiss_id = (self.id.clone(), "backdrop");

        // Backdrop
        let mut overlay = div()
            .id(self.id)
            .absolute()
            .inset_0()
            .flex()
            .flex_col()
            .items_center()
            .pt(px(80.0))
            .bg(theme.backdrop)
            .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {});

        // Dismiss on backdrop click
        if let Some(handler) = self.on_dismiss {
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
                    .child(self.placeholder)
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
        let query_lower = self.query.to_lowercase();

        let mut visible_count = 0;
        let mut current_category: Option<SharedString> = None;

        for (i, item) in self.items.iter().enumerate() {
            // Simple filter: if query is non-empty, check if label contains it
            if !query_lower.is_empty() && !item.label.to_lowercase().contains(&query_lower) {
                continue;
            }

            if visible_count >= self.max_visible {
                break;
            }

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
            let hover_bg = theme.hover_bg;

            let mut row = div()
                .id(ElementId::from(item.id.clone()))
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
                row = row
                    .text_color(theme.item_text)
                    .hover(move |s| s.bg(hover_bg));
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
            visible_count += 1;
        }

        palette = palette.child(results);

        overlay = overlay.child(palette);

        overlay
    }
}

impl RenderOnce for CommandPalette {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = CommandPaletteTheme::from(&global_theme);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for CommandPalette {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
