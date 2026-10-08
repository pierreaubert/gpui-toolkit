//! `Pagination` component
//!
//! A page-button control for long result sets: previous/next steppers
//! around a window of numbered pages with ellipsis gaps. State is
//! parent-owned — the caller holds the current page and total — and
//! every move reports through `on_change`. Arrow keys step one page at
//! a time. Table scrolling uses
//! [`crate::table::PaginationState`](crate::table) for its data window;
//! this is the standalone control for everything else.
//!
//! # Usage
//!
//! ```ignore
//! Pagination::new("results")
//!     .page(3)
//!     .total_pages(12)
//!     .on_change(|page, _window, _cx| {
//!         // Fetch and show `page`.
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

/// Theme colors for pagination.
#[derive(Debug, Clone, ComponentTheme)]
pub struct PaginationTheme {
    /// Page button text color.
    #[theme(default = 0xffffffff, from = text_primary)]
    pub page: Rgba,
    /// Current page button background.
    #[theme(default = 0x007accff, from = accent)]
    pub page_current: Rgba,
    /// Current page button text color.
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub page_current_text: Rgba,
    /// Ellipsis and stepper color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub control: Rgba,
    /// Disabled stepper color.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub control_disabled: Rgba,
}

/// Pagination size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaginationSize {
    /// Compact page buttons.
    Sm,
    /// Standard page buttons (default).
    #[default]
    Md,
    /// Large page buttons.
    Lg,
}

impl From<crate::ComponentSize> for PaginationSize {
    /// Maps the shared size scale onto page buttons.
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// One slot in the page window: a button or an ellipsis gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageItem {
    /// Numbered page button (1-based).
    Page(usize),
    /// Collapsed gap marker.
    Ellipsis,
}

/// Page-button control with parent-owned state.
#[derive(IntoElement)]
pub struct Pagination {
    id: ElementId,
    page: usize,
    total_pages: usize,
    siblings: usize,
    size: PaginationSize,
    theme: Option<PaginationTheme>,
    design: Option<Arc<DesignSystem>>,
    on_change: Option<Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Pagination {
    /// Create pagination on page 1 of a single page.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let pager = Pagination::new("results").page(2).total_pages(9);
    /// ```
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            page: 1,
            total_pages: 1,
            siblings: 1,
            size: PaginationSize::default(),
            theme: None,
            design: None,
            on_change: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the current page (1-based, clamped to the total at render).
    pub fn page(mut self, page: usize) -> Self {
        self.page = page;
        self
    }

    /// Set the total page count (0 renders as 1).
    pub fn total_pages(mut self, total: usize) -> Self {
        self.total_pages = total;
        self
    }

    /// Set how many pages flank the current one on each side.
    pub fn siblings(mut self, count: usize) -> Self {
        self.siblings = count;
        self
    }

    /// Set the pagination size.
    pub fn size(mut self, size: PaginationSize) -> Self {
        self.size = size;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: PaginationTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set the handler called with the newly requested page.
    pub fn on_change(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
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

    /// Effective page and total after clamping.
    fn clamped(&self) -> (usize, usize) {
        let total = self.total_pages.max(1);
        (self.page.clamp(1, total), total)
    }

    /// Page window: first page, last page, and siblings around the
    /// current page, with [`PageItem::Ellipsis`] marking each gap.
    pub fn page_items(page: usize, total_pages: usize, siblings: usize) -> Vec<PageItem> {
        let total = total_pages.max(1);
        let current = page.clamp(1, total);
        let mut pages = vec![1, total];
        let low = current.saturating_sub(siblings).max(1);
        let high = current.saturating_add(siblings).min(total);
        for page in low..=high {
            pages.push(page);
        }
        pages.sort_unstable();
        pages.dedup();
        let mut items = Vec::with_capacity(pages.len() + 1);
        let mut previous = 0;
        for page in pages {
            if previous > 0 && page > previous + 1 {
                items.push(PageItem::Ellipsis);
            }
            items.push(PageItem::Page(page));
            previous = page;
        }
        items
    }

    /// Build the control with theme.
    pub fn build_with_theme(self, theme: &PaginationTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the control with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &PaginationTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let (page, total) = self.clamped();
        let root_id = self.id.clone();
        let mut root = div()
            .id(self.id)
            .flex()
            .flex_row()
            .items_center()
            .gap(px(design.spacing.control_gap * 0.5));
        root = match self.size {
            PaginationSize::Sm => root.text_xs(),
            PaginationSize::Md => root.text_sm(),
            PaginationSize::Lg => root.text_lg(),
        };

        root = root.child(Self::stepper(
            (root_id.clone(), "pagination-prev"),
            "‹",
            (page > 1).then_some(page - 1),
            theme,
            self.on_change.clone(),
        ));
        for item in Self::page_items(page, total, self.siblings) {
            root = root.child(match item {
                PageItem::Ellipsis => div()
                    .text_color(theme.control)
                    .child("…")
                    .into_any_element(),
                PageItem::Page(number) => {
                    let current = number == page;
                    let mut button = div()
                        .id((
                            root_id.clone(),
                            SharedString::from(format!("pagination-page-{number}")),
                        ))
                        .px(px(design.spacing.control_padding_x))
                        .py(px(design.spacing.control_padding_y * 0.5))
                        .rounded(px(design.corners.sm))
                        .text_color(if current {
                            theme.page_current_text
                        } else {
                            theme.page
                        })
                        .child(number.to_string());
                    if current {
                        button = button.bg(theme.page_current);
                    } else if let Some(on_change) = &self.on_change {
                        let handler = on_change.clone();
                        let key_handler = on_change.clone();
                        button = button.cursor_pointer().on_click(
                            move |_event: &ClickEvent, window, cx| {
                                handler(number, window, cx);
                            },
                        );
                        button = button.on_key_down(move |event: &KeyDownEvent, window, cx| {
                            let key = event.keystroke.key.as_str();
                            if key == "enter" || key == "space" {
                                key_handler(number, window, cx);
                                cx.stop_propagation();
                            }
                        });
                    }
                    button.into_any_element()
                }
            });
        }
        root = root.child(Self::stepper(
            (root_id.clone(), "pagination-next"),
            "›",
            (page < total).then_some(page + 1),
            theme,
            self.on_change.clone(),
        ));

        if let Some(on_change) = self.on_change.clone() {
            root.on_key_down(move |event: &KeyDownEvent, window, cx| {
                let target = match event.keystroke.key.as_str() {
                    "left" => (page > 1).then_some(page - 1),
                    "right" => (page < total).then_some(page + 1),
                    "home" => (page > 1).then_some(1),
                    "end" => (page < total).then_some(total),
                    _ => None,
                };
                if let Some(next) = target {
                    on_change(next, window, cx);
                    cx.stop_propagation();
                }
            })
        } else {
            root
        }
    }

    /// Previous/next stepper reporting `target` when enabled.
    fn stepper(
        id: impl Into<ElementId>,
        label: &'static str,
        target: Option<usize>,
        theme: &PaginationTheme,
        on_change: Option<Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    ) -> Stateful<Div> {
        let mut control = div().id(id).child(label);
        match (target, on_change) {
            (Some(next), Some(handler)) => {
                let key_handler = handler.clone();
                control = control
                    .text_color(theme.control)
                    .cursor_pointer()
                    .on_click(move |_event: &ClickEvent, window, cx| {
                        handler(next, window, cx);
                    })
                    .on_key_down(move |event: &KeyDownEvent, window, cx| {
                        let key = event.keystroke.key.as_str();
                        if key == "enter" || key == "space" {
                            key_handler(next, window, cx);
                            cx.stop_propagation();
                        }
                    });
            }
            _ => {
                control = control.text_color(theme.control_disabled).opacity(0.4);
            }
        }
        control
    }
}

impl RenderOnce for Pagination {
    /// Registers accessibility, resolves theme, and builds the control.
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (page, total) = self.clamped();
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| format!("Pagination page {page} of {total}").into()),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Navigation)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| PaginationTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{PageItem, Pagination, PaginationSize, PaginationTheme};
    use gpui::rgba;

    #[test]
    fn pagination_defaults_to_first_of_one() {
        let pagination = Pagination::new("results");
        assert_eq!(pagination.page, 1);
        assert_eq!(pagination.total_pages, 1);
        assert_eq!(pagination.siblings, 1);
        assert_eq!(pagination.size, PaginationSize::Md);
        assert!(pagination.on_change.is_none());
    }

    #[test]
    fn pagination_builders_store_page_total_and_siblings() {
        let pagination = Pagination::new("results")
            .page(3)
            .total_pages(12)
            .siblings(2)
            .size(PaginationSize::Lg);
        assert_eq!((pagination.page, pagination.total_pages), (3, 12));
        assert_eq!(pagination.siblings, 2);
        assert_eq!(pagination.size, PaginationSize::Lg);
        assert_eq!(pagination.clamped(), (3, 12));
    }

    #[test]
    fn page_items_window_with_ellipsis_gaps() {
        use PageItem::{Ellipsis, Page};
        assert_eq!(
            Pagination::page_items(5, 10, 1),
            vec![
                Page(1),
                Ellipsis,
                Page(4),
                Page(5),
                Page(6),
                Ellipsis,
                Page(10)
            ]
        );
        assert_eq!(
            Pagination::page_items(1, 10, 1),
            vec![Page(1), Page(2), Ellipsis, Page(10)]
        );
        assert_eq!(
            Pagination::page_items(2, 3, 1),
            vec![Page(1), Page(2), Page(3)]
        );
        assert_eq!(
            Pagination::page_items(9, 4, 1),
            vec![Page(1), Ellipsis, Page(3), Page(4)]
        );
    }

    #[test]
    fn pagination_theme_defaults_match_compiled_tokens() {
        let theme = PaginationTheme::default();
        assert_eq!(theme.page, rgba(0xffffffff));
        assert_eq!(theme.page_current, rgba(0x007accff));
        assert_eq!(theme.page_current_text, rgba(0xffffffff));
        assert_eq!(theme.control, rgba(0xccccccff));
        assert_eq!(theme.control_disabled, rgba(0x777777ff));
    }
}
