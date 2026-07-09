use super::layout_debug_warning::LayoutDebugWarning;
use super::layout_debug_warning::append_debug_node;
use super::solved_node::SolvedNode;
use crate::types::LayoutNode;
use std::fmt;

/// Count summary for the warnings in a [`LayoutDebugReport`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LayoutDebugSummary {
    pub total: usize,
    pub invalid_size: usize,
    pub invisible_without_collapse_label: usize,
    pub main_axis_overflow: usize,
    pub cross_axis_overflow: usize,
}

impl LayoutDebugSummary {
    /// Returns true when no diagnostic warnings were counted.
    pub fn is_clean(&self) -> bool {
        self.total == 0
    }
}

/// Textual diagnostics for a solved layout tree.
///
/// The report is intended for examples, debug panes, logs, and tests. It keeps
/// the tree output stable and exposes warnings as structured values so callers
/// can surface them however they like.
///
/// Warning identifiers are borrowed from the underlying [`SolvedNode`] tree;
/// the report must not outlive the tree it describes.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutDebugReport<'a> {
    pub(super) tree: String,
    pub(super) warnings: Vec<LayoutDebugWarning<'a>>,
}

impl<'a> LayoutDebugReport<'a> {
    /// Returns the stable, line-oriented solved tree.
    pub fn tree(&self) -> &str {
        &self.tree
    }

    /// Returns warnings found while inspecting the solved tree.
    pub fn warnings(&self) -> &[LayoutDebugWarning<'a>] {
        &self.warnings
    }

    /// Returns true when no diagnostic warnings were found.
    pub fn is_clean(&self) -> bool {
        self.warnings.is_empty()
    }

    /// Returns true when at least one diagnostic warning was found.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Returns warning counts grouped by stable diagnostic category.
    pub fn summary(&self) -> LayoutDebugSummary {
        let mut summary = LayoutDebugSummary {
            total: self.warnings.len(),
            ..LayoutDebugSummary::default()
        };

        for warning in &self.warnings {
            match warning.kind {
                super::types::LayoutDebugWarningKind::InvalidSize { .. } => {
                    summary.invalid_size += 1;
                }
                super::types::LayoutDebugWarningKind::InvisibleWithoutCollapseLabel => {
                    summary.invisible_without_collapse_label += 1;
                }
                super::types::LayoutDebugWarningKind::MainAxisOverflow { .. } => {
                    summary.main_axis_overflow += 1;
                }
                super::types::LayoutDebugWarningKind::CrossAxisOverflow { .. } => {
                    summary.cross_axis_overflow += 1;
                }
            }
        }

        summary
    }

    /// Returns a Markdown table with stable warning codes and remediation hints.
    pub fn warnings_markdown_table(&self) -> String {
        if self.warnings.is_empty() {
            return "No layout warnings.".to_string();
        }

        let mut out = String::from("| code | node | diagnostic | remediation |\n");
        out.push_str("| --- | --- | --- | --- |\n");
        for warning in &self.warnings {
            out.push_str("| `");
            out.push_str(warning.code());
            out.push_str("` | `");
            out.push_str(warning.node_id);
            out.push_str("` | ");
            out.push_str(&escape_markdown_cell(&warning.to_string()));
            out.push_str(" | ");
            out.push_str(&escape_markdown_cell(warning.remediation()));
            out.push_str(" |\n");
        }
        out
    }
}

impl fmt::Display for LayoutDebugReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.tree)?;
        if self.warnings.is_empty() {
            return Ok(());
        }

        writeln!(f, "warnings:")?;
        for warning in &self.warnings {
            writeln!(f, "- {warning}")?;
        }
        Ok(())
    }
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

pub(super) fn build_debug_report<'a>(
    root: &'a SolvedNode<'a>,
    source: Option<&'a LayoutNode<'a>>,
) -> LayoutDebugReport<'a> {
    let mut lines = Vec::new();
    let mut warnings = Vec::new();
    append_debug_node(
        root,
        source.filter(|s| s.id() == root.id),
        0,
        &mut lines,
        &mut warnings,
    );
    LayoutDebugReport {
        tree: lines.join("\n"),
        warnings,
    }
}
