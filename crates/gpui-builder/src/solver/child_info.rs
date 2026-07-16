use super::types::ChildInfo;
use crate::types::{Axis, LayoutNode, LayoutPreferences, Sizing};

pub(super) fn allocate_main_axis(
    children: &mut [ChildInfo],
    nodes: &[LayoutNode<'_>],
    available: f32,
    divider_size: f32,
    axis: Axis,
    prefs: &LayoutPreferences<'_>,
) {
    // Pass A: Allocate non-collapsible Fixed and Text children unconditionally.
    // Collapsible Fixed/Text children participate in collapse logic below.
    let mut unconditional_fixed = 0.0_f32;
    for child in children.iter_mut() {
        if child.user_collapsed {
            continue;
        }
        let node = &nodes[child.node_index];
        if node.collapsible() {
            continue;
        }
        match node.sizing() {
            Sizing::Fixed(size) => {
                child.allocated_size = size;
                unconditional_fixed += size;
            }
            Sizing::Text { min, .. } => {
                let size = child.computed_text_size.unwrap_or(min);
                child.allocated_size = size;
                unconditional_fixed += size;
            }
            _ => {}
        }
    }

    // Count initially visible children for divider space
    let initial_visible = children.iter().filter(|c| !c.user_collapsed).count();
    let initial_divider_space = if initial_visible > 1 {
        divider_size * (initial_visible - 1) as f32
    } else {
        0.0
    };

    let space_after_fixed = (available - unconditional_fixed - initial_divider_space).max(0.0);

    // Pass B: Sum minimums of all non-unconditional-fixed visible children
    // (collapsible Fixed/Text + Fractional + Flex)
    let total_minimums: f32 = children
        .iter()
        .filter(|c| {
            !c.user_collapsed
                && (nodes[c.node_index].collapsible()
                    || !matches!(
                        nodes[c.node_index].sizing(),
                        Sizing::Fixed(_) | Sizing::Text { .. }
                    ))
        })
        .map(|c| nodes[c.node_index].sizing().min_size())
        .sum();

    // Pass C: Priority-based collapse if minimums exceed remaining
    if total_minimums > space_after_fixed {
        let mut current_minimums = total_minimums;
        while current_minimums > space_after_fixed {
            // Select the next lowest-priority candidate in place. Typical UI
            // sibling counts are small, and this avoids allocating/sorting an
            // index vector in the resize hot path.
            let Some(idx) = children
                .iter()
                .enumerate()
                .filter(|(_, child)| {
                    !child.user_collapsed
                        && !child.solver_collapsed
                        && nodes[child.node_index].collapsible()
                })
                .min_by(|(_, a), (_, b)| {
                    nodes[a.node_index]
                        .priority()
                        .partial_cmp(&nodes[b.node_index].priority())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(idx, _)| idx)
            else {
                break;
            };
            children[idx].solver_collapsed = true;
            current_minimums -= nodes[children[idx].node_index].sizing().min_size();
        }
    }

    // Recompute available after collapse (divider count may have changed)
    let visible_after = children
        .iter()
        .filter(|c| !c.user_collapsed && !c.solver_collapsed)
        .count();
    let divider_space_after = if visible_after > 1 {
        divider_size * (visible_after - 1) as f32
    } else {
        0.0
    };
    let remaining = (available - unconditional_fixed - divider_space_after).max(0.0);

    // Pass D: Distribute remaining among visible collapsible-Fixed + Fractional + Flex
    distribute_remaining(children, nodes, remaining, axis, prefs);
}

pub(super) fn distribute_remaining(
    children: &mut [ChildInfo],
    nodes: &[LayoutNode<'_>],
    remaining: f32,
    axis: Axis,
    prefs: &LayoutPreferences<'_>,
) {
    // Collapsible Fixed/Text nodes that survived collapse get their fixed/measured size
    let mut used_by_fixed = 0.0_f32;
    for child in children.iter_mut() {
        if child.user_collapsed || child.solver_collapsed {
            continue;
        }
        let node = &nodes[child.node_index];
        if !node.collapsible() {
            continue;
        }
        match node.sizing() {
            Sizing::Fixed(size) => {
                child.allocated_size = size;
                used_by_fixed += size;
            }
            Sizing::Text { min, .. } => {
                let size = child.computed_text_size.unwrap_or(min);
                child.allocated_size = size;
                used_by_fixed += size;
            }
            _ => {}
        }
    }

    let distributable = (remaining - used_by_fixed).max(0.0);

    // Collect fractional and flex demands
    let mut fractional_demand = 0.0_f32;
    let mut flex_total_weight = 0.0_f32;

    for child in children.iter() {
        if child.user_collapsed || child.solver_collapsed {
            continue;
        }
        let node = &nodes[child.node_index];
        match node.sizing() {
            Sizing::Fractional { initial, .. } => {
                let ratio = prefs.ratio_for(node.id(), axis).unwrap_or(initial);
                fractional_demand += ratio;
            }
            Sizing::Flex { weight, .. } => {
                flex_total_weight += weight;
            }
            Sizing::Fixed(_) | Sizing::Text { .. } => {}
        }
    }

    // If total fractional ratios > 1.0, scale them down proportionally
    let ratio_scale = if fractional_demand > 1.0 {
        1.0 / fractional_demand
    } else {
        1.0
    };

    // Allocate fractional children their share
    let mut used_by_fractional = 0.0_f32;
    for child in children.iter_mut() {
        if child.user_collapsed || child.solver_collapsed {
            continue;
        }
        let node = &nodes[child.node_index];
        if let Sizing::Fractional { initial, min, max } = node.sizing() {
            let ratio = prefs.ratio_for(node.id(), axis).unwrap_or(initial);
            let target = (ratio * ratio_scale * distributable).clamp(min, max);
            child.allocated_size = target;
            used_by_fractional += target;
        }
    }

    // Flex children split leftover (clamped to available, not unbounded)
    let flex_remaining = (distributable - used_by_fractional).max(0.0);
    if flex_total_weight > 0.0 {
        // First pass: compute proportional shares with min floor.
        let mut total_flex = 0.0_f32;
        for child in children.iter() {
            if child.user_collapsed || child.solver_collapsed {
                continue;
            }
            if let Sizing::Flex { min, weight } = nodes[child.node_index].sizing() {
                let proportional = flex_remaining * (weight / flex_total_weight);
                let share = proportional.max(min).min(flex_remaining);
                total_flex += share;
            }
        }

        // Second pass: if total exceeds available space, scale proportionally.
        let scale = if total_flex > flex_remaining && total_flex > 0.0 {
            flex_remaining / total_flex
        } else {
            1.0
        };

        for child in children.iter_mut() {
            if child.user_collapsed || child.solver_collapsed {
                continue;
            }
            if let Sizing::Flex { min, weight } = nodes[child.node_index].sizing() {
                let proportional = flex_remaining * (weight / flex_total_weight);
                let share = proportional.max(min).min(flex_remaining);
                child.allocated_size = share * scale;
            }
        }
    }
}
