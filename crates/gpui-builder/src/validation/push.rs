use super::layout_issue_severity::LayoutIssueSeverity;
use super::types::LayoutIssue;
use super::types::LayoutIssueKind;
use super::types::ValidationState;

pub(super) fn push_invalid_sizing(
    state: &mut ValidationState,
    node_id: &str,
    path: &str,
    message: String,
) {
    push_issue(
        state,
        LayoutIssueSeverity::Error,
        node_id,
        path,
        LayoutIssueKind::InvalidSizing,
        message,
    );
}

pub(super) fn push_issue(
    state: &mut ValidationState,
    severity: LayoutIssueSeverity,
    node_id: &str,
    path: &str,
    kind: LayoutIssueKind,
    message: impl Into<String>,
) {
    state.issues.push(LayoutIssue {
        severity,
        node_id: node_id.to_string(),
        path: path.to_string(),
        kind,
        message: message.into(),
    });
}
