//! `MetadataList` component
//!
//! A label/value detail list for "author / Ada" style file or record
//! metadata. Entries are parent-owned data; the list only aligns the
//! labels and values with dividers between rows.
//!
//! # Usage
//!
//! ```ignore
//! MetadataList::new("file-meta")
//!     .entry(MetadataEntry::new("Author", "Ada"))
//!     .entry(MetadataEntry::new("License", "MIT"))
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, ElementId, Rgba, SharedString, Stateful, Window, div, px};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// One label/value row in the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataEntry {
    /// Row label shown on the leading side.
    pub label: SharedString,
    /// Row value shown on the trailing side.
    pub value: SharedString,
}

impl MetadataEntry {
    /// Create an entry from `label` with `value`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let entry = MetadataEntry::new("Author", "Ada");
    /// ```
    pub fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// Theme colors for the metadata list.
#[derive(Debug, Clone, ComponentTheme)]
pub struct MetadataListTheme {
    /// Row label color.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub label: Rgba,
    /// Row value color.
    #[theme(default = 0xffffffff, from = text_primary)]
    pub value: Rgba,
    /// Divider color between rows.
    #[theme(default = 0x3e3e42ff, from = border)]
    pub divider: Rgba,
}

/// Metadata list size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetadataListSize {
    /// Compact rows.
    Sm,
    /// Standard rows (default).
    #[default]
    Md,
    /// Large rows.
    Lg,
}

impl From<crate::ComponentSize> for MetadataListSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Label/value detail list with dividers.
#[derive(IntoElement)]
pub struct MetadataList {
    id: ElementId,
    entries: Vec<MetadataEntry>,
    size: MetadataListSize,
    theme: Option<MetadataListTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl MetadataList {
    /// Create a new empty list.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let list = MetadataList::new("file-meta");
    /// ```
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            size: MetadataListSize::default(),
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the full entry list.
    pub fn entries(mut self, entries: Vec<MetadataEntry>) -> Self {
        self.entries = entries;
        self
    }

    /// Append one entry to the list.
    pub fn entry(mut self, entry: MetadataEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Set the list size.
    pub fn size(mut self, size: MetadataListSize) -> Self {
        self.size = size;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: MetadataListTheme) -> Self {
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

    /// Build the list with theme.
    pub fn build_with_theme(self, theme: &MetadataListTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the list with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &MetadataListTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let mut list = div().id(self.id).flex().flex_col();
        list = match self.size {
            MetadataListSize::Sm => list.text_xs(),
            MetadataListSize::Md => list.text_sm(),
            MetadataListSize::Lg => list.text_lg(),
        };

        let last = self.entries.len().saturating_sub(1);
        for (row_index, entry) in self.entries.into_iter().enumerate() {
            let mut row = div()
                .flex()
                .justify_between()
                .gap(px(design.spacing.control_gap))
                .py(px(design.spacing.control_padding_y * 0.5));
            if row_index < last {
                row = row.border_b_1().border_color(theme.divider);
            }
            row = row
                .child(div().text_color(theme.label).child(entry.label))
                .child(
                    div()
                        .text_right()
                        .text_color(theme.value)
                        .child(entry.value),
                );
            list = list.child(row);
        }
        list
    }
}

impl RenderOnce for MetadataList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or_else(|| "Metadata".into()),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Group)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| MetadataListTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{MetadataEntry, MetadataList, MetadataListSize, MetadataListTheme};
    use gpui::rgba;

    #[test]
    fn metadata_list_defaults_to_empty_standard_rows() {
        let list = MetadataList::new("file-meta");
        assert!(list.entries.is_empty());
        assert_eq!(list.size, MetadataListSize::Md);
    }

    #[test]
    fn metadata_list_builders_store_entries_and_size() {
        let list = MetadataList::new("file-meta")
            .entry(MetadataEntry::new("Author", "Ada"))
            .entries(vec![MetadataEntry::new("License", "MIT")])
            .size(MetadataListSize::Lg);
        assert_eq!(list.entries.len(), 1);
        assert_eq!(list.entries[0].label.as_ref(), "License");
        assert_eq!(list.entries[0].value.as_ref(), "MIT");
        assert_eq!(list.size, MetadataListSize::Lg);
    }

    #[test]
    fn metadata_list_theme_defaults_match_compiled_tokens() {
        let theme = MetadataListTheme::default();
        assert_eq!(theme.label, rgba(0x777777ff));
        assert_eq!(theme.value, rgba(0xffffffff));
        assert_eq!(theme.divider, rgba(0x3e3e42ff));
    }
}
