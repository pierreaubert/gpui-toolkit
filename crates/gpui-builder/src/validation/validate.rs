use super::layout_issue_severity::LayoutIssueSeverity;
use super::layout_validation_report::LayoutValidationReport;
use super::misc::is_non_negative;
use super::misc::node_path;
use super::push::push_invalid_sizing;
use super::push::push_issue;
use super::types::LayoutIssueKind;
use super::types::ValidationState;
use crate::util::format_number;
use crate::{ContainerNode, DisplayTier, LayoutNode, Sizing, SlotNode};
use std::collections::HashSet;

/// Validate a layout declaration tree without solving it.
pub fn validate_layout(root: &LayoutNode<'_>) -> LayoutValidationReport {
    let mut state = ValidationState::default();
    validate_node(root, None, &mut state);
    LayoutValidationReport {
        issues: state.issues,
    }
}

fn validate_node(node: &LayoutNode<'_>, parent_path: Option<&str>, state: &mut ValidationState) {
    let id = node.id();
    let path = node_path(parent_path, id);

    validate_common_node(node, &path, state);

    match node {
        LayoutNode::Slot(slot) => validate_slot(slot, &path, state),
        LayoutNode::Container(container) => {
            validate_container(container, &path, state);
            for child in container.children {
                validate_node(child, Some(&path), state);
            }
        }
    }
}

fn validate_common_node(node: &LayoutNode<'_>, path: &str, state: &mut ValidationState) {
    let id = node.id();

    if id.trim().is_empty() {
        push_issue(
            state,
            LayoutIssueSeverity::Error,
            id,
            path,
            LayoutIssueKind::EmptyId,
            "id must not be empty",
        );
    } else if let Some(first_path) = state.seen_ids.get(id).cloned() {
        push_issue(
            state,
            LayoutIssueSeverity::Error,
            id,
            path,
            LayoutIssueKind::DuplicateId {
                first_path: first_path.clone(),
            },
            format!("duplicate id {id:?} (first seen at {first_path})"),
        );
    } else {
        state.seen_ids.insert(id.to_string(), path.to_string());
    }

    validate_sizing(node.id(), path, node.sizing(), state);
}

fn validate_slot(slot: &SlotNode<'_>, path: &str, state: &mut ValidationState) {
    if !slot.priority.is_finite() {
        push_issue(
            state,
            LayoutIssueSeverity::Error,
            slot.id,
            path,
            LayoutIssueKind::InvalidPriority,
            format!(
                "priority must be finite, got {}",
                format_number(slot.priority)
            ),
        );
    } else if !(0.0..=1.0).contains(&slot.priority) {
        push_issue(
            state,
            LayoutIssueSeverity::Warning,
            slot.id,
            path,
            LayoutIssueKind::PriorityOutOfRange,
            format!(
                "priority should be in 0.0..=1.0, got {}",
                format_number(slot.priority)
            ),
        );
    }

    if slot.collapsible && slot.collapse_label.is_none() {
        push_issue(
            state,
            LayoutIssueSeverity::Warning,
            slot.id,
            path,
            LayoutIssueKind::MissingCollapseLabel,
            "collapsible slot should set collapse_label for restore UI/accessibility",
        );
    }

    if let Some(label) = slot.collapse_label
        && label.trim().is_empty()
    {
        push_issue(
            state,
            LayoutIssueSeverity::Warning,
            slot.id,
            path,
            LayoutIssueKind::EmptyCollapseLabel,
            "collapse_label should not be empty",
        );
    }

    validate_display_tiers(slot.id, path, slot.display_tiers, state);
}

fn validate_container(container: &ContainerNode<'_>, path: &str, state: &mut ValidationState) {
    if let Some(threshold) = container.auto_axis
        && (!threshold.is_finite() || threshold <= 0.0)
    {
        push_issue(
            state,
            LayoutIssueSeverity::Error,
            container.id,
            path,
            LayoutIssueKind::InvalidAutoAxis,
            format!(
                "auto_axis threshold must be finite and > 0, got {}",
                format_number(threshold)
            ),
        );
    }

    if !container.divider_size.is_finite() || container.divider_size < 0.0 {
        push_issue(
            state,
            LayoutIssueSeverity::Error,
            container.id,
            path,
            LayoutIssueKind::InvalidDividerSize,
            format!(
                "divider_size must be finite and >= 0, got {}",
                format_number(container.divider_size)
            ),
        );
    }

    if container.children.is_empty() {
        push_issue(
            state,
            LayoutIssueSeverity::Warning,
            container.id,
            path,
            LayoutIssueKind::EmptyContainer,
            "container has no children",
        );
    }
}

fn validate_sizing(node_id: &str, path: &str, sizing: Sizing<'_>, state: &mut ValidationState) {
    match sizing {
        Sizing::Fixed(size) => {
            if !is_non_negative(size) {
                push_invalid_sizing(
                    state,
                    node_id,
                    path,
                    format!(
                        "Fixed size must be finite and >= 0, got {}",
                        format_number(size)
                    ),
                );
            }
        }
        Sizing::Fractional { initial, min, max } => {
            if !initial.is_finite() || !(0.0..=1.0).contains(&initial) {
                push_invalid_sizing(
                    state,
                    node_id,
                    path,
                    format!(
                        "Fractional initial must be finite and in 0.0..=1.0, got {}",
                        format_number(initial)
                    ),
                );
            }
            if !is_non_negative(min) {
                push_invalid_sizing(
                    state,
                    node_id,
                    path,
                    format!(
                        "Fractional min must be finite and >= 0, got {}",
                        format_number(min)
                    ),
                );
            }
            if !is_non_negative(max) {
                push_invalid_sizing(
                    state,
                    node_id,
                    path,
                    format!(
                        "Fractional max must be finite and >= 0, got {}",
                        format_number(max)
                    ),
                );
            } else if max < min {
                push_invalid_sizing(
                    state,
                    node_id,
                    path,
                    format!(
                        "Fractional max must be >= min (min={}, max={})",
                        format_number(min),
                        format_number(max)
                    ),
                );
            }
        }
        Sizing::Flex { min, weight } => {
            if !is_non_negative(min) {
                push_invalid_sizing(
                    state,
                    node_id,
                    path,
                    format!(
                        "Flex min must be finite and >= 0, got {}",
                        format_number(min)
                    ),
                );
            }
            if !weight.is_finite() || weight <= 0.0 {
                push_invalid_sizing(
                    state,
                    node_id,
                    path,
                    format!(
                        "Flex weight must be finite and > 0, got {}",
                        format_number(weight)
                    ),
                );
            }
        }
        Sizing::Text {
            line_height, min, ..
        } => {
            if !line_height.is_finite() || line_height <= 0.0 {
                push_invalid_sizing(
                    state,
                    node_id,
                    path,
                    format!(
                        "Text line_height must be finite and > 0, got {}",
                        format_number(line_height)
                    ),
                );
            }
            if !is_non_negative(min) {
                push_invalid_sizing(
                    state,
                    node_id,
                    path,
                    format!(
                        "Text min must be finite and >= 0, got {}",
                        format_number(min)
                    ),
                );
            }
        }
    }
}

fn validate_display_tiers(
    node_id: &str,
    path: &str,
    tiers: &[DisplayTier<'_>],
    state: &mut ValidationState,
) {
    let mut names = HashSet::new();
    let mut thresholds = HashSet::new();
    let mut previous_min_size: Option<f32> = None;
    let mut descending_warning_emitted = false;

    for tier in tiers {
        if tier.name.trim().is_empty() {
            push_issue(
                state,
                LayoutIssueSeverity::Warning,
                node_id,
                path,
                LayoutIssueKind::InvalidDisplayTier,
                "display tier name should not be empty",
            );
        } else if !names.insert(tier.name) {
            push_issue(
                state,
                LayoutIssueSeverity::Warning,
                node_id,
                path,
                LayoutIssueKind::DuplicateDisplayTierName {
                    name: tier.name.to_string(),
                },
                format!("duplicate display tier name {:?}", tier.name),
            );
        }

        if !is_non_negative(tier.min_size) {
            push_issue(
                state,
                LayoutIssueSeverity::Error,
                node_id,
                path,
                LayoutIssueKind::InvalidDisplayTier,
                format!(
                    "display tier {:?} min_size must be finite and >= 0, got {}",
                    tier.name,
                    format_number(tier.min_size)
                ),
            );
        } else {
            let threshold_key = tier.min_size.to_bits();
            if !thresholds.insert(threshold_key) {
                push_issue(
                    state,
                    LayoutIssueSeverity::Warning,
                    node_id,
                    path,
                    LayoutIssueKind::DuplicateDisplayTierThreshold {
                        min_size: tier.min_size,
                    },
                    format!(
                        "duplicate display tier min_size {}",
                        format_number(tier.min_size)
                    ),
                );
            }

            if let Some(previous) = previous_min_size
                && tier.min_size > previous
                && !descending_warning_emitted
            {
                descending_warning_emitted = true;
                push_issue(
                    state,
                    LayoutIssueSeverity::Warning,
                    node_id,
                    path,
                    LayoutIssueKind::DisplayTiersNotDescending,
                    "display tiers should be ordered from largest min_size to smallest",
                );
            }
            previous_min_size = Some(tier.min_size);
        }
    }
}
