//! `Blockquote` component
//!
//! A styled quotation with an accent bar and an optional citation.
//! The quote is parent-owned text; the component only styles the
//! quotation and its attribution.
//!
//! # Usage
//!
//! ```ignore
//! Blockquote::new("review", "Ship it.")
//!     .cite("Release captain")
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, ElementId, Rgba, SharedString, Stateful, Window, div, px};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Theme colors for the quotation.
#[derive(Debug, Clone, ComponentTheme)]
pub struct BlockquoteTheme {
    /// Accent bar color.
    #[theme(default = 0x007accff, from = accent)]
    pub bar: Rgba,
    /// Quotation text color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text: Rgba,
    /// Citation color.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub cite: Rgba,
}

/// Quotation size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlockquoteSize {
    /// Compact quotation.
    Sm,
    /// Standard quotation (default).
    #[default]
    Md,
    /// Large quotation.
    Lg,
}

impl From<crate::ComponentSize> for BlockquoteSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Styled quotation with optional citation.
#[derive(IntoElement)]
pub struct Blockquote {
    id: ElementId,
    quote: SharedString,
    cite: Option<SharedString>,
    size: BlockquoteSize,
    theme: Option<BlockquoteTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Blockquote {
    /// Create a quotation with `quote`.
    pub fn new(id: impl Into<ElementId>, quote: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            quote: quote.into(),
            cite: None,
            size: BlockquoteSize::default(),
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the quotation text.
    pub fn quote(mut self, quote: impl Into<SharedString>) -> Self {
        self.quote = quote.into();
        self
    }

    /// Set the citation attribution.
    pub fn cite(mut self, cite: impl Into<SharedString>) -> Self {
        self.cite = Some(cite.into());
        self
    }

    /// Set the quotation size.
    pub fn size(mut self, size: BlockquoteSize) -> Self {
        self.size = size;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: BlockquoteTheme) -> Self {
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

    /// Override the default ARIA role (Group).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Build the quotation with theme.
    pub fn build_with_theme(self, theme: &BlockquoteTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the quotation with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &BlockquoteTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let mut row = div().id(self.id).flex().gap(px(design.spacing.control_gap));
        row = match self.size {
            BlockquoteSize::Sm => row.text_xs(),
            BlockquoteSize::Md => row.text_sm(),
            BlockquoteSize::Lg => row.text_lg(),
        };
        row = row.child(
            div()
                .w(px(design.spacing.grid_unit * 0.5))
                .rounded(px(design.corners.sm))
                .bg(theme.bar),
        );

        let mut body = div()
            .flex()
            .flex_col()
            .gap(px(design.spacing.control_gap * 0.5))
            .text_color(theme.text)
            .child(self.quote);
        if let Some(cite) = self.cite {
            body = body.child(div().text_xs().text_color(theme.cite).child(cite));
        }
        row.child(body)
    }
}

impl RenderOnce for Blockquote {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| self.quote.clone()),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| BlockquoteTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{Blockquote, BlockquoteSize, BlockquoteTheme};
    use gpui::rgba;

    #[test]
    fn blockquote_defaults_to_standard_uncited_quote() {
        let quote = Blockquote::new("review", "Ship it.");
        assert_eq!(quote.quote.as_ref(), "Ship it.");
        assert!(quote.cite.is_none());
        assert_eq!(quote.size, BlockquoteSize::Md);
    }

    #[test]
    fn blockquote_builders_store_quote_cite_and_size() {
        let quote = Blockquote::new("review", "Ship it.")
            .quote("Hold it.")
            .cite("Release captain")
            .size(BlockquoteSize::Lg);
        assert_eq!(quote.quote.as_ref(), "Hold it.");
        assert_eq!(quote.cite.as_deref(), Some("Release captain"));
        assert_eq!(quote.size, BlockquoteSize::Lg);
    }

    #[test]
    fn blockquote_theme_defaults_match_compiled_tokens() {
        let theme = BlockquoteTheme::default();
        assert_eq!(theme.bar, rgba(0x007accff));
        assert_eq!(theme.text, rgba(0xccccccff));
        assert_eq!(theme.cite, rgba(0x777777ff));
    }
}
