//! `Citation` component
//!
//! A standalone source reference: quoted text with an optional source
//! attribution. `Blockquote` owns the quotation styling; `Citation` is the
//! reusable reference itself, either inline within a sentence or stacked
//! as a block.
//!
//! # Usage
//!
//! ```ignore
//! Citation::new("ref-1", "Move slowly and fix things.")
//!     .source("Maintenance handbook")
//!     .variant(CitationVariant::Block)
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, ElementId, Rgba, SharedString, Stateful, Window, div, px};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Theme colors for the citation.
#[derive(Debug, Clone, ComponentTheme)]
pub struct CitationTheme {
    /// Quoted text color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text: Rgba,
    /// Source attribution color.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub source: Rgba,
}

/// Citation layout variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CitationVariant {
    /// Text and source on one line (default).
    #[default]
    Inline,
    /// Text stacked above the source.
    Block,
}

/// Citation size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CitationSize {
    /// Compact citation.
    Sm,
    /// Standard citation (default).
    #[default]
    Md,
    /// Large citation.
    Lg,
}

impl From<crate::ComponentSize> for CitationSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Standalone source reference with attribution.
#[derive(IntoElement)]
pub struct Citation {
    id: ElementId,
    text: SharedString,
    source: Option<SharedString>,
    variant: CitationVariant,
    size: CitationSize,
    theme: Option<CitationTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Citation {
    /// Create a citation for `text`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let quote = Citation::new("ref-1", "Move slowly and fix things.");
    /// ```
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            source: None,
            variant: CitationVariant::default(),
            size: CitationSize::default(),
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the quoted text.
    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.text = text.into();
        self
    }

    /// Set the source attribution.
    pub fn source(mut self, source: impl Into<SharedString>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set the citation layout.
    pub fn variant(mut self, variant: CitationVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the citation size.
    pub fn size(mut self, size: CitationSize) -> Self {
        self.size = size;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: CitationTheme) -> Self {
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

    /// Build the citation with theme.
    pub fn build_with_theme(self, theme: &CitationTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the citation with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &CitationTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let mut root = div().id(self.id).flex();
        root = match self.variant {
            CitationVariant::Inline => root
                .flex_row()
                .items_baseline()
                .gap(px(design.spacing.control_gap * 0.5)),
            CitationVariant::Block => root.flex_col().gap(px(design.spacing.control_gap * 0.5)),
        };
        root = match self.size {
            CitationSize::Sm => root.text_xs(),
            CitationSize::Md => root.text_sm(),
            CitationSize::Lg => root.text_lg(),
        };

        root = root.child(div().text_color(theme.text).child(self.text));
        if let Some(source) = self.source {
            let attribution = match self.variant {
                CitationVariant::Inline => SharedString::from(format!("— {source}")),
                CitationVariant::Block => source,
            };
            let mut source_row = div().text_color(theme.source);
            if self.variant == CitationVariant::Block {
                source_row = source_row.text_xs();
            }
            root = root.child(source_row.child(attribution));
        }
        root
    }
}

impl RenderOnce for Citation {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_else(|| self.text.clone()),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| CitationTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{Citation, CitationSize, CitationTheme, CitationVariant};
    use gpui::rgba;

    #[test]
    fn citation_defaults_to_inline_unsourced_quote() {
        let quote = Citation::new("ref-1", "Move slowly.");
        assert_eq!(quote.text.as_ref(), "Move slowly.");
        assert!(quote.source.is_none());
        assert_eq!(quote.variant, CitationVariant::Inline);
        assert_eq!(quote.size, CitationSize::Md);
    }

    #[test]
    fn citation_builders_store_text_source_and_layout() {
        let quote = Citation::new("ref-1", "Move slowly.")
            .text("Fix things.")
            .source("Handbook")
            .variant(CitationVariant::Block)
            .size(CitationSize::Lg);
        assert_eq!(quote.text.as_ref(), "Fix things.");
        assert_eq!(quote.source.as_deref(), Some("Handbook"));
        assert_eq!(quote.variant, CitationVariant::Block);
        assert_eq!(quote.size, CitationSize::Lg);
    }

    #[test]
    fn citation_theme_defaults_match_compiled_tokens() {
        let theme = CitationTheme::default();
        assert_eq!(theme.text, rgba(0xccccccff));
        assert_eq!(theme.source, rgba(0x777777ff));
    }
}
