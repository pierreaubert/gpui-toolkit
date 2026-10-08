//! `MobileNav` component
//!
//! A bottom tab bar for mobile layouts. Tabs are parent-owned: the
//! selected tab ID is a prop, and every tap reports through `on_select`.
//!
//! # Usage
//!
//! ```ignore
//! MobileNav::new("tabs")
//!     .item(MobileNavItem::new("home", "Home").icon("⌂"))
//!     .item(MobileNavItem::new("search", "Search").icon("⌕"))
//!     .selected("home")
//!     .on_select(|id, _window, _cx| { /* store id */ })
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, apply_native_accessibility,
};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Div, ElementId, FontWeight, KeyDownEvent, MouseButton, Pixels, Rgba, SharedString,
    Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::rc::Rc;
use std::sync::Arc;

/// Selection handler reporting the tapped tab ID.
type MobileNavSelect = Rc<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>;

/// Theme colors for mobile navigation.
#[derive(Debug, Clone, ComponentTheme)]
pub struct MobileNavTheme {
    /// Bar background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Top divider.
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Unselected tab color.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub item_text: Rgba,
    /// Selected tab color.
    #[theme(default = 0x007accff, from = accent)]
    pub item_active: Rgba,
}

/// Mobile navigation size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MobileNavSize {
    /// Compact tabs.
    Sm,
    /// Standard tabs (default).
    #[default]
    Md,
    /// Large tabs.
    Lg,
}

impl MobileNavSize {
    fn bar_height(self, design: &DesignSystem) -> Pixels {
        match self {
            Self::Sm => px(design.interaction.min_touch_target * 1.3),
            Self::Md => px(design.interaction.min_touch_target * 1.5),
            Self::Lg => px(design.interaction.min_touch_target * 1.8),
        }
    }
}

impl From<crate::ComponentSize> for MobileNavSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// One tab in a mobile navigation bar.
pub struct MobileNavItem {
    id: SharedString,
    label: SharedString,
    icon: Option<SharedString>,
}

impl MobileNavItem {
    /// Create a navigation tab.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
        }
    }

    /// Set the tab icon glyph.
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Get the tab ID.
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    /// Get the tab label.
    pub fn label(&self) -> &SharedString {
        &self.label
    }

    /// Get the tab icon glyph.
    pub fn icon_glyph(&self) -> Option<&SharedString> {
        self.icon.as_ref()
    }
}

/// Bottom tab bar for mobile layouts.
#[derive(IntoElement)]
pub struct MobileNav {
    id: ElementId,
    items: Vec<MobileNavItem>,
    selected: Option<SharedString>,
    size: MobileNavSize,
    on_select: Option<MobileNavSelect>,
    theme: Option<MobileNavTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl MobileNav {
    /// Create a new empty tab bar.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            selected: None,
            size: MobileNavSize::default(),
            on_select: None,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Append a tab.
    pub fn item(mut self, item: MobileNavItem) -> Self {
        self.items.push(item);
        self
    }

    /// Set the selected tab ID.
    pub fn selected(mut self, id: impl Into<SharedString>) -> Self {
        self.selected = Some(id.into());
        self
    }

    /// Set the tab size.
    pub fn size(mut self, size: MobileNavSize) -> Self {
        self.size = size;
        self
    }

    /// Set the selection handler.
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: MobileNavTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit ARIA label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Navigation).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Render one tab with selection state.
    fn render_item(
        item: &MobileNavItem,
        selected: bool,
        on_select: Option<&MobileNavSelect>,
        theme: &MobileNavTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let indicator = if selected {
            div()
                .w(px(design.interaction.min_touch_target * 0.5))
                .h(px(3.0))
                .rounded_full()
                .bg(theme.item_active)
        } else {
            div()
                .w(px(design.interaction.min_touch_target * 0.5))
                .h(px(3.0))
                .bg(theme.background)
        };
        let label_color = if selected {
            theme.item_active
        } else {
            theme.item_text
        };

        let mut tab = div()
            .id(SharedString::from(format!("mobile-nav-tab-{}", item.id)))
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(design.spacing.grid_unit * 0.5))
            .py(px(design.spacing.control_padding_y * 0.5))
            .cursor_pointer()
            .child(indicator);
        if let Some(icon) = item.icon.clone() {
            tab = tab.child(div().text_lg().text_color(label_color).child(icon));
        }
        let mut label = div()
            .text_xs()
            .text_color(label_color)
            .child(item.label.clone());
        if selected {
            label = label.font_weight(FontWeight::SEMIBOLD);
        }
        tab = tab.child(label);

        if let Some(handler) = on_select {
            let mouse_handler = handler.clone();
            let mouse_id = item.id.clone();
            tab = tab.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                mouse_handler(&mouse_id, window, cx);
            });
            let key_handler = handler.clone();
            let key_id = item.id.clone();
            tab = tab.on_key_down(move |event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
                    key_handler(&key_id, window, cx);
                    cx.stop_propagation();
                }
            });
        }

        tab
    }

    /// Build the bar with theme.
    pub fn build_with_theme(self, theme: &MobileNavTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the bar with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &MobileNavTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let native_label = self
            .aria_label
            .clone()
            .unwrap_or_else(|| SharedString::from("Mobile navigation"));
        let native_props = AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Navigation));
        // Bottom inset keeps the last row clear of home indicators.
        let bottom_inset = px(design.spacing.control_padding_y + design.spacing.grid_unit * 0.5);

        let mut bar = div()
            .id(self.id)
            .flex()
            .w_full()
            .flex_shrink_0()
            .h(self.size.bar_height(design))
            .pb(bottom_inset)
            .bg(theme.background)
            .border_t_1()
            .border_color(theme.border);

        for item in &self.items {
            let selected = self.selected.as_ref() == Some(&item.id);
            bar = bar.child(Self::render_item(
                item,
                selected,
                self.on_select.as_ref(),
                theme,
                design,
            ));
        }

        apply_native_accessibility(bar, native_label, &native_props)
    }
}

impl RenderOnce for MobileNav {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| SharedString::from("Mobile navigation")),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Navigation)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| MobileNavTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{MobileNav, MobileNavItem, MobileNavSize, MobileNavTheme};
    use gpui::rgba;

    #[test]
    fn nav_defaults_to_unselected_standard_bar() {
        let nav = MobileNav::new("tabs");
        assert_eq!(nav.size, MobileNavSize::Md);
        assert!(nav.items.is_empty());
        assert!(nav.selected.is_none());
        assert!(nav.theme.is_none());
    }

    #[test]
    fn nav_item_builders_store_state() {
        let item = MobileNavItem::new("home", "Home").icon("H");
        assert_eq!(item.id().as_ref(), "home");
        assert_eq!(item.label().as_ref(), "Home");
        assert_eq!(item.icon_glyph().map(AsRef::as_ref), Some("H"));

        let nav = MobileNav::new("tabs")
            .item(item)
            .selected("home")
            .size(MobileNavSize::Lg)
            .on_select(|_, _, _| {});
        assert_eq!(nav.items.len(), 1);
        assert_eq!(
            nav.selected.map(|id| id.to_string()),
            Some("home".to_string())
        );
        assert_eq!(nav.size, MobileNavSize::Lg);
    }

    #[test]
    fn nav_theme_defaults_match_compiled_tokens() {
        let theme = MobileNavTheme::default();
        assert_eq!(theme.background, rgba(0x1e1e1eff));
        assert_eq!(theme.border, rgba(0x3a3a3aff));
        assert_eq!(theme.item_active, rgba(0x007accff));
    }

    #[test]
    fn nav_bar_height_grows_with_size() {
        let design = crate::design::neutral_design();
        let sm = MobileNavSize::Sm.bar_height(&design);
        let md = MobileNavSize::Md.bar_height(&design);
        let lg = MobileNavSize::Lg.bar_height(&design);
        assert!(sm < md);
        assert!(md < lg);
    }
}
