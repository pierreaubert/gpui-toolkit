use super::layout_issue_severity::LayoutIssueSeverity;
use std::collections::HashMap;

/// Machine-readable category for a validation issue.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutIssueKind {
    EmptyId,
    DuplicateId { first_path: String },
    InvalidSizing,
    InvalidAutoAxis,
    InvalidDividerSize,
    InvalidPriority,
    PriorityOutOfRange,
    MissingCollapseLabel,
    EmptyCollapseLabel,
    InvalidDisplayTier,
    DuplicateDisplayTierName { name: String },
    DuplicateDisplayTierThreshold { min_size: f32 },
    DisplayTiersNotDescending,
    EmptyContainer,
}

/// One validation issue found in a layout tree.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutIssue {
    /// Error or warning severity.
    pub severity: LayoutIssueSeverity,
    /// Node id where the issue was found.
    pub node_id: String,
    /// Slash-separated tree path where the issue was found.
    pub path: String,
    /// Machine-readable issue kind.
    pub kind: LayoutIssueKind,
    /// Stable human-readable message.
    pub message: String,
}

#[derive(Default)]
pub(super) struct ValidationState {
    pub(super) seen_ids: HashMap<String, String>,
    pub(super) issues: Vec<LayoutIssue>,
}
