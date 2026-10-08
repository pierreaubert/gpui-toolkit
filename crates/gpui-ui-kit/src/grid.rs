//! Fixed-column grid layout primitive.
//!
//! [`Grid`] arranges children in equal-width columns using GPUI's native
//! CSS grid support. It is a primitive: no opinions about cell content,
//! only columns, an optional row count, and design-system gaps.
//!
//! # Usage
//!
//! ```ignore
//! Grid::new("cards")
//!     .columns(3)
//!     .gap(StackSpacing::Md)
//!     .children([card_a, card_b, card_c])
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::StackSpacing;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    AnyElement, App, Div, ElementId, Pixels, Rgba, SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Theme colors for grid styling.
#[derive(Debug, Clone, ComponentTheme)]
pub struct GridTheme {
    /// Container background tint (transparent by default).
    #[theme(default = 0x00000000, from = transparent)]
    pub background: Rgba,
}

/// Resolve a [`StackSpacing`] step to pixels.
///
/// Mirrors the `stack` module mapping against the same [`DesignSystem`]
/// fields. It lives here so `Grid` does not widen the stack module API.
fn gap_pixels(gap: StackSpacing, design: &DesignSystem) -> Pixels {
    let grid = design.spacing.grid_unit;
    match gap {
        StackSpacing::None => px(0.0),
        StackSpacing::Xs => px(grid * 0.5),
        StackSpacing::Sm => px(grid),
        StackSpacing::Md => px(design.spacing.control_gap),
        StackSpacing::Lg => px(design.spacing.section_gap),
        StackSpacing::Xl => px(design.spacing.section_gap + design.spacing.control_gap),
        StackSpacing::Xxl => px(design.spacing.section_gap * 2.0),
        StackSpacing::Custom(pixels) => pixels,
    }
}

/// A fixed-column grid layout.
///
/// Lays children out in equal-width columns with design-system gaps.
/// Unlabeled grids register with an empty accessible label; set
/// [`Self::aria_label`] when assistive tech should announce the group.
///
/// # Examples
///
/// ```ignore
/// Grid::new("cards")
///     .columns(3)
///     .gap(StackSpacing::Md)
///     .child(card_a)
///     .child(card_b)
/// ```
#[derive(IntoElement)]
pub struct Grid {
    id: ElementId,
    children: Vec<AnyElement>,
    columns: u16,
    rows: Option<u16>,
    gap: StackSpacing,
    theme: Option<GridTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Grid {
    /// Create a new grid with two columns and medium gaps.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            children: Vec::new(),
            columns: 2,
            rows: None,
            gap: StackSpacing::default(),
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Add a child element to the grid.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    /// Add multiple children to the grid.
    pub fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.children.extend(
            children
                .into_iter()
                .map(gpui::IntoElement::into_any_element),
        );
        self
    }

    /// Set the column count (clamped to at least one).
    pub fn columns(mut self, columns: u16) -> Self {
        self.columns = columns.max(1);
        self
    }

    /// Set an explicit row count (`None`, the default, sizes rows automatically).
    pub fn rows(mut self, rows: u16) -> Self {
        self.rows = Some(rows.max(1));
        self
    }

    /// Set the gap between cells.
    pub fn gap(mut self, gap: StackSpacing) -> Self {
        self.gap = gap;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: GridTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit accessible label for the grid group.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default accessible role (`Group`).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Build the grid into a `Stateful<Div>` for further composition.
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
        theme: &GridTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let mut el = div()
            .id(self.id)
            .grid()
            .grid_cols(self.columns.max(1))
            .gap(gap_pixels(self.gap, design))
            .bg(theme.background);
        if let Some(rows) = self.rows {
            el = el.grid_rows(rows.max(1));
        }
        el.children(self.children)
    }
}

impl RenderOnce for Grid {
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
            .unwrap_or_else(|| GridTheme::from(global_theme.as_ref()));
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_use_two_columns_and_medium_gap() {
        let grid = Grid::new("grid");
        assert_eq!(grid.columns, 2);
        assert!(grid.rows.is_none());
        assert!(matches!(grid.gap, StackSpacing::Md));
        assert!(grid.children.is_empty());
    }

    #[test]
    fn builders_set_fields() {
        let grid = Grid::new("grid")
            .columns(4)
            .rows(2)
            .gap(StackSpacing::Lg)
            .aria_label("Cards")
            .aria_role(AriaRole::Region);
        assert_eq!(grid.columns, 4);
        assert_eq!(grid.rows, Some(2));
        assert!(matches!(grid.gap, StackSpacing::Lg));
        assert_eq!(grid.aria_label, Some(SharedString::from("Cards")));
        assert_eq!(grid.aria_role, Some(AriaRole::Region));
    }

    #[test]
    fn column_and_row_counts_clamp_to_one() {
        let grid = Grid::new("grid").columns(0).rows(0);
        assert_eq!(grid.columns, 1);
        assert_eq!(grid.rows, Some(1));
    }

    #[test]
    fn gap_steps_derive_from_design_system() {
        let design = crate::design::neutral_design();
        assert_eq!(
            gap_pixels(StackSpacing::Sm, &design),
            px(design.spacing.grid_unit)
        );
        assert_eq!(
            gap_pixels(StackSpacing::Md, &design),
            px(design.spacing.control_gap)
        );
        assert_eq!(gap_pixels(StackSpacing::None, &design), px(0.0));
        assert_eq!(gap_pixels(StackSpacing::Custom(px(7.0)), &design), px(7.0));
    }

    #[test]
    fn build_does_not_panic() {
        let _el = Grid::new("grid")
            .columns(3)
            .child(div())
            .children([div(), div()])
            .build();
    }
}
