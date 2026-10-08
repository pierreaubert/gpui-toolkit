//! `List` component
//!
//! A static selectable row list with parent-owned state: the caller
//! holds the item set and the selected id, while the list renders rows
//! with hover, selected, and disabled states. Clicks and Enter report
//! through `on_select`; Up/Down/Home/End move the active row, skipping
//! disabled items. For reorderable rows use [`crate::DragList`], for
//! tabular data use [`crate::Table`]; this is the plain list in between.
//!
//! # Usage
//!
//! ```ignore
//! List::new("servers", vec![ListItem::new("a", "Alpha")])
//!     .selected("a")
//!     .on_select(|id, _window, _cx| {
//!         // Show details for `id`.
//!     })
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    App, ClickEvent, Div, ElementId, KeyDownEvent, Rgba, SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::rc::Rc;
use std::sync::Arc;

/// Theme colors for the list.
#[derive(Debug, Clone, ComponentTheme)]
pub struct ListTheme {
    /// Row label color.
    #[theme(default = 0xffffffff, from = text_primary)]
    pub item: Rgba,
    /// Row description color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub description: Rgba,
    /// Selected row background.
    #[theme(default = 0x007accff, from = accent)]
    pub selected: Rgba,
    /// Selected row label color.
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub selected_text: Rgba,
    /// Disabled row label color.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub disabled: Rgba,
}

/// List size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListSize {
    /// Compact rows.
    Sm,
    /// Standard rows (default).
    #[default]
    Md,
    /// Large rows.
    Lg,
}

impl From<crate::ComponentSize> for ListSize {
    /// Maps the shared size scale onto list rows.
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// One selectable row: stable id, label, and optional description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    id: SharedString,
    label: SharedString,
    description: Option<SharedString>,
    disabled: bool,
}

impl ListItem {
    /// Create an enabled row with a label and no description.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            disabled: false,
        }
    }

    /// Set the secondary description line.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark the row as disabled (visible but not selectable).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Row id used for selection and element identity.
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    /// Whether the row can be selected.
    pub fn is_selectable(&self) -> bool {
        !self.disabled
    }
}

/// Static selectable row list with parent-owned state.
#[derive(IntoElement)]
pub struct List {
    id: ElementId,
    items: Vec<ListItem>,
    selected: Option<SharedString>,
    size: ListSize,
    theme: Option<ListTheme>,
    design: Option<Arc<DesignSystem>>,
    on_select: Option<Rc<dyn Fn(SharedString, &mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl List {
    /// Create a list with rows and no selection.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let list = List::new("servers", vec![ListItem::new("a", "Alpha")]);
    /// ```
    pub fn new(id: impl Into<ElementId>, items: Vec<ListItem>) -> Self {
        Self {
            id: id.into(),
            items,
            selected: None,
            size: ListSize::default(),
            theme: None,
            design: None,
            on_select: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the full row list.
    pub fn items(mut self, items: Vec<ListItem>) -> Self {
        self.items = items;
        self
    }

    /// Append one row.
    pub fn item(mut self, item: ListItem) -> Self {
        self.items.push(item);
        self
    }

    /// Set the selected row id.
    pub fn selected(mut self, id: impl Into<SharedString>) -> Self {
        self.selected = Some(id.into());
        self
    }

    /// Set the list size.
    pub fn size(mut self, size: ListSize) -> Self {
        self.size = size;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: ListTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set the handler called with the newly selected row id.
    pub fn on_select(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Set an explicit ARIA label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Listbox).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// First selectable row id, if any.
    pub fn first_selectable(items: &[ListItem]) -> Option<SharedString> {
        items
            .iter()
            .find(|item| item.is_selectable())
            .map(|item| item.id.clone())
    }

    /// Last selectable row id, if any.
    pub fn last_selectable(items: &[ListItem]) -> Option<SharedString> {
        items
            .iter()
            .rev()
            .find(|item| item.is_selectable())
            .map(|item| item.id.clone())
    }

    /// Neighbor of `current` in `direction` (+1 down, -1 up), skipping
    /// disabled rows and stopping at the ends.
    pub fn neighbor(items: &[ListItem], current: &str, direction: i32) -> Option<SharedString> {
        let position = items.iter().position(|item| item.id.as_ref() == current)?;
        let mut index = position as i32 + direction;
        while let Some(item) = items.get(usize::try_from(index).ok()?) {
            if item.is_selectable() {
                return Some(item.id.clone());
            }
            index += direction;
        }
        None
    }

    /// Build the list with theme.
    pub fn build_with_theme(self, theme: &ListTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the list with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &ListTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let root_id = self.id.clone();
        let mut root = div().id(self.id).flex().flex_col();
        root = match self.size {
            ListSize::Sm => root.text_xs(),
            ListSize::Md => root.text_sm(),
            ListSize::Lg => root.text_lg(),
        };
        for item in &self.items {
            root = root.child(Self::row(
                &root_id,
                item,
                self.selected.as_ref() == Some(&item.id),
                theme,
                design,
                self.on_select.clone(),
            ));
        }

        if let Some(on_select) = self.on_select.clone() {
            let items = self.items.clone();
            let selected = self.selected.clone();
            root.on_key_down(move |event: &KeyDownEvent, window, cx| {
                let target = match event.keystroke.key.as_str() {
                    "down" => selected.as_ref().and_then(|current| {
                        Self::neighbor(&items, current, 1)
                    }),
                    "up" => selected
                        .as_ref()
                        .and_then(|current| Self::neighbor(&items, current, -1)),
                    "home" => Self::first_selectable(&items),
                    "end" => Self::last_selectable(&items),
                    "enter" | "space" => selected.clone().filter(|current| {
                        items
                            .iter()
                            .any(|item| &item.id == current && item.is_selectable())
                    }),
                    _ => None,
                };
                if let Some(next) = target {
                    on_select(next, window, cx);
                    cx.stop_propagation();
                }
            })
        } else {
            root
        }
    }

    /// One row with selected/disabled states and click handling.
    fn row(
        root_id: &ElementId,
        item: &ListItem,
        selected: bool,
        theme: &ListTheme,
        design: &DesignSystem,
        on_select: Option<Rc<dyn Fn(SharedString, &mut Window, &mut App) + 'static>>,
    ) -> Stateful<Div> {
        let mut row = div()
            .id((root_id.clone(), item.id.clone()))
            .flex()
            .flex_col()
            .px(px(design.spacing.control_padding_x))
            .py(px(design.spacing.control_padding_y * 0.5))
            .rounded(px(design.corners.sm));
        if selected {
            row = row
                .bg(theme.selected)
                .text_color(theme.selected_text)
                .child(item.label.clone());
        } else if item.disabled {
            row = row
                .text_color(theme.disabled)
                .opacity(0.6)
                .child(item.label.clone());
        } else {
            row = row.text_color(theme.item).child(item.label.clone());
            if let Some(on_select) = on_select {
                let id = item.id.clone();
                let key_id = item.id.clone();
                let key_select = on_select.clone();
                row = row
                    .cursor_pointer()
                    .on_click(move |_event: &ClickEvent, window, cx| {
                        on_select(id.clone(), window, cx);
                    })
                    .on_key_down(move |event: &KeyDownEvent, window, cx| {
                        let key = event.keystroke.key.as_str();
                        if key == "enter" || key == "space" {
                            key_select(key_id.clone(), window, cx);
                            cx.stop_propagation();
                        }
                    });
            }
        }
        if let Some(description) = &item.description {
            row = row.child(
                div()
                    .text_color(if selected {
                        theme.selected_text
                    } else {
                        theme.description
                    })
                    .child(description.clone()),
            );
        }
        row
    }
}

impl RenderOnce for List {
    /// Registers accessibility, resolves theme, and builds the rows.
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_else(|| "List".into()),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Listbox)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| ListTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{List, ListItem, ListSize, ListTheme};
    use gpui::rgba;

    /// Three rows with the middle one disabled.
    fn rows() -> Vec<ListItem> {
        vec![
            ListItem::new("a", "Alpha"),
            ListItem::new("b", "Beta").disabled(true),
            ListItem::new("c", "Gamma").description("Third row"),
        ]
    }

    #[test]
    fn list_defaults_to_unselected_standard_rows() {
        let list = List::new("servers", rows());
        assert_eq!(list.items.len(), 3);
        assert_eq!(list.selected, None);
        assert_eq!(list.size, ListSize::Md);
        assert!(list.on_select.is_none());
    }

    #[test]
    fn list_builders_store_rows_selection_and_size() {
        let list = List::new("servers", Vec::new())
            .items(rows())
            .item(ListItem::new("d", "Delta"))
            .selected("c")
            .size(ListSize::Lg);
        assert_eq!(list.items.len(), 4);
        assert_eq!(list.selected.as_deref(), Some("c"));
        assert_eq!(list.size, ListSize::Lg);
    }

    #[test]
    fn selection_helpers_skip_disabled_rows() {
        let items = rows();
        assert_eq!(List::first_selectable(&items).as_deref(), Some("a"));
        assert_eq!(List::last_selectable(&items).as_deref(), Some("c"));
        assert_eq!(List::neighbor(&items, "a", 1).as_deref(), Some("c"));
        assert_eq!(List::neighbor(&items, "c", -1).as_deref(), Some("a"));
        assert_eq!(List::neighbor(&items, "c", 1), None);
        assert_eq!(List::neighbor(&items, "missing", 1), None);
    }

    #[test]
    fn list_theme_defaults_match_compiled_tokens() {
        let theme = ListTheme::default();
        assert_eq!(theme.item, rgba(0xffffffff));
        assert_eq!(theme.description, rgba(0xccccccff));
        assert_eq!(theme.selected, rgba(0x007accff));
        assert_eq!(theme.selected_text, rgba(0xffffffff));
        assert_eq!(theme.disabled, rgba(0x777777ff));
    }
}
