use super::types::ChildInfo;
use crate::types::{Axis, LayoutNode, LayoutPreferences, Sizing, sanitize_ratio};

fn fractional_bounds(min: f32, max: f32) -> (f32, f32) {
    let min = min.is_finite().then_some(min.max(0.0)).unwrap_or(0.0);
    let max = if max.is_nan() || max.is_sign_negative() {
        0.0
    } else if max.is_infinite() {
        f32::MAX
    } else {
        max
    };

    (min, max.max(min))
}

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
        let mut current_space = space_after_fixed;
        let mut visible_count = initial_visible;
        while current_minimums > current_space {
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
                .min_by(|(a_index, a), (b_index, b)| {
                    nodes[a.node_index]
                        .priority()
                        .partial_cmp(&nodes[b.node_index].priority())
                        .unwrap_or(std::cmp::Ordering::Equal)
                        // Equal priorities preserve declaration order by
                        // overflowing later siblings first.
                        .then_with(|| b_index.cmp(a_index))
                })
                .map(|(idx, _)| idx)
            else {
                break;
            };
            children[idx].solver_collapsed = true;
            current_minimums -= nodes[children[idx].node_index].sizing().min_size();
            if visible_count > 1 {
                current_space += divider_size;
            }
            visible_count = visible_count.saturating_sub(1);
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
                let ratio = sanitize_ratio(prefs.ratio_for(node.id(), axis).unwrap_or(initial));
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
            let ratio = sanitize_ratio(prefs.ratio_for(node.id(), axis).unwrap_or(initial));
            let (min, max) = fractional_bounds(min, max);
            let target = (ratio * ratio_scale * distributable).max(min).min(max);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SlotNode;

    fn fixed(id: &str, size: f32) -> LayoutNode<'_> {
        LayoutNode::slot(id, Sizing::Fixed(size))
    }

    fn flex_slot(id: &str, min: f32, weight: f32) -> LayoutNode<'_> {
        LayoutNode::slot(id, Sizing::Flex { min, weight })
    }

    fn fractional_slot(id: &str, initial: f32, min: f32, max: f32) -> LayoutNode<'_> {
        LayoutNode::slot(id, Sizing::Fractional { initial, min, max })
    }

    fn collapsible_fixed(id: &str, size: f32, priority: f32) -> LayoutNode<'_> {
        LayoutNode::Slot(SlotNode {
            id,
            sizing: Sizing::Fixed(size),
            priority,
            collapsible: true,
            display_tiers: &[],
            collapse_label: None,
        })
    }

    fn infos(count: usize) -> Vec<ChildInfo> {
        (0..count)
            .map(|node_index| ChildInfo {
                node_index,
                user_collapsed: false,
                solver_collapsed: false,
                allocated_size: 0.0,
                computed_text_size: None,
            })
            .collect()
    }

    #[test]
    fn fixed_children_keep_exact_size_and_flex_takes_rest() {
        let nodes = [fixed("a", 20.0), flex_slot("b", 0.0, 1.0)];
        let prefs = LayoutPreferences::default();
        let mut children = infos(2);
        allocate_main_axis(&mut children, &nodes, 100.0, 0.0, Axis::Horizontal, &prefs);
        assert!(!children[0].solver_collapsed);
        assert!(!children[1].solver_collapsed);
        assert_eq!(children[0].allocated_size, 20.0);
        assert_eq!(children[1].allocated_size, 80.0);
    }

    #[test]
    fn lowest_priority_collapses_first() {
        let nodes = [
            collapsible_fixed("low", 60.0, 0.1),
            collapsible_fixed("high", 60.0, 0.9),
        ];
        let prefs = LayoutPreferences::default();
        let mut children = infos(2);
        allocate_main_axis(&mut children, &nodes, 100.0, 0.0, Axis::Horizontal, &prefs);
        assert!(children[0].solver_collapsed);
        assert!(!children[1].solver_collapsed);
        assert_eq!(children[1].allocated_size, 60.0);
    }

    #[test]
    fn equal_priority_collapses_later_siblings_first() {
        let nodes = [
            collapsible_fixed("a", 60.0, 0.5),
            collapsible_fixed("b", 60.0, 0.5),
        ];
        let prefs = LayoutPreferences::default();
        let mut children = infos(2);
        allocate_main_axis(&mut children, &nodes, 100.0, 0.0, Axis::Horizontal, &prefs);
        assert!(!children[0].solver_collapsed);
        assert!(children[1].solver_collapsed);
        assert_eq!(children[0].allocated_size, 60.0);
    }

    #[test]
    fn divider_space_is_reclaimed_after_collapse() {
        let nodes = [collapsible_fixed("a", 60.0, 0.1), fixed("b", 30.0)];
        let prefs = LayoutPreferences::default();
        let mut children = infos(2);
        // 80 - 30 fixed - 10 divider leaves 40 < 60 minimum, so "a" collapses
        // and the divider disappears with it.
        allocate_main_axis(&mut children, &nodes, 80.0, 10.0, Axis::Horizontal, &prefs);
        assert!(children[0].solver_collapsed);
        assert_eq!(children[1].allocated_size, 30.0);
    }

    #[test]
    fn user_collapsed_children_are_skipped() {
        let nodes = [fixed("a", 20.0), flex_slot("b", 0.0, 1.0)];
        let prefs = LayoutPreferences::default();
        let mut children = infos(2);
        children[0].user_collapsed = true;
        allocate_main_axis(&mut children, &nodes, 100.0, 0.0, Axis::Horizontal, &prefs);
        assert_eq!(children[1].allocated_size, 100.0);
    }

    #[test]
    fn distribute_remaining_scales_fractional_demand_over_full() {
        let nodes = [
            fractional_slot("a", 0.8, 0.0, f32::MAX),
            fractional_slot("b", 0.8, 0.0, f32::MAX),
        ];
        let prefs = LayoutPreferences::default();
        let mut children = infos(2);
        distribute_remaining(&mut children, &nodes, 100.0, Axis::Horizontal, &prefs);
        assert_eq!(children[0].allocated_size, 50.0);
        assert_eq!(children[1].allocated_size, 50.0);
    }

    #[test]
    fn distribute_remaining_splits_flex_by_weight() {
        let nodes = [flex_slot("a", 0.0, 1.0), flex_slot("b", 0.0, 3.0)];
        let prefs = LayoutPreferences::default();
        let mut children = infos(2);
        distribute_remaining(&mut children, &nodes, 100.0, Axis::Horizontal, &prefs);
        assert_eq!(children[0].allocated_size, 25.0);
        assert_eq!(children[1].allocated_size, 75.0);
    }

    #[test]
    fn distribute_remaining_applies_ratio_override_from_prefs() {
        let nodes = [fractional_slot("a", 0.5, 0.0, f32::MAX)];
        let mut prefs = LayoutPreferences::default();
        prefs.set_ratio("a", Axis::Horizontal, 0.25);
        let mut children = infos(1);
        distribute_remaining(&mut children, &nodes, 100.0, Axis::Horizontal, &prefs);
        assert_eq!(children[0].allocated_size, 25.0);
    }

    #[test]
    fn distribute_remaining_respects_fractional_min_clamp() {
        let nodes = [fractional_slot("a", 0.1, 40.0, f32::MAX)];
        let prefs = LayoutPreferences::default();
        let mut children = infos(1);
        distribute_remaining(&mut children, &nodes, 100.0, Axis::Horizontal, &prefs);
        assert_eq!(children[0].allocated_size, 40.0);
    }
}
