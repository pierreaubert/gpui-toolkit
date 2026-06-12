//! Layout interaction state for preference-driven layouts.
//!
//! This module provides an owned state object for user-driven layout updates
//! (ratio drags and collapse toggles) and turns that state into
//! [`LayoutPreferences`] when calling the solver.

use crate::types::{Axis, LayoutPreferences};

/// A stored fractional ratio override for a specific slot/axis pair.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutRatioOverride {
    /// Slot identifier.
    pub slot_id: String,
    /// Parent axis used for this override.
    pub axis: Axis,
    /// Ratio override in the same unit as `Sizing::Fractional::initial`.
    pub ratio: f32,
}

/// A stored collapsed state for a specific slot.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutCollapsedState {
    /// Slot identifier.
    pub slot_id: String,
    /// Whether the slot is explicitly collapsed.
    pub collapsed: bool,
}

/// Action-oriented updates that can be applied to [`LayoutState`].
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutAction {
    /// Persist or update a ratio override.
    SetRatio {
        /// Slot identifier.
        slot_id: String,
        /// Axis in which the ratio applies.
        axis: Axis,
        /// New ratio value.
        ratio: f32,
    },
    /// Remove a ratio override.
    ClearRatio {
        /// Slot identifier.
        slot_id: String,
        /// Axis for the override.
        axis: Axis,
    },
    /// Set explicit collapsed state.
    SetCollapsed {
        /// Slot identifier.
        slot_id: String,
        /// Whether the slot is collapsed.
        collapsed: bool,
    },
    /// Toggle explicit collapsed state for a slot.
    ToggleCollapsed {
        /// Slot identifier.
        slot_id: String,
    },
    /// Remove collapsed state for a slot.
    ClearCollapsed {
        /// Slot identifier.
        slot_id: String,
    },
    /// Clear all stored interaction state.
    Reset,
}

/// Mutable UI state that feeds layout preferences.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LayoutState {
    ratio_overrides: Vec<LayoutRatioOverride>,
    collapsed: Vec<LayoutCollapsedState>,
}

impl LayoutState {
    /// Construct empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct with a pre-sized allocation.
    pub fn with_capacity(ratios: usize, collapsed: usize) -> Self {
        Self {
            ratio_overrides: Vec::with_capacity(ratios),
            collapsed: Vec::with_capacity(collapsed),
        }
    }

    /// Current ratio overrides.
    pub fn ratios(&self) -> &[LayoutRatioOverride] {
        &self.ratio_overrides
    }

    /// Current explicit collapsed slots.
    pub fn collapsed(&self) -> &[LayoutCollapsedState] {
        &self.collapsed
    }

    /// Look up ratio preference for a slot/axis pair.
    pub fn ratio_for(&self, slot_id: &str, axis: Axis) -> Option<f32> {
        self.ratio_overrides
            .iter()
            .rev()
            .find(|entry| entry.slot_id == slot_id && entry.axis == axis)
            .map(|entry| entry.ratio)
    }

    /// Whether a slot is explicitly collapsed.
    pub fn is_collapsed(&self, slot_id: &str) -> bool {
        self.collapsed.iter().any(|entry| entry.slot_id == slot_id)
    }

    /// Set (or replace) a ratio override.
    pub fn set_ratio(&mut self, slot_id: &str, axis: Axis, ratio: f32) {
        if let Some(entry) = self
            .ratio_overrides
            .iter_mut()
            .find(|entry| entry.slot_id == slot_id && entry.axis == axis)
        {
            entry.ratio = ratio;
            return;
        }

        self.ratio_overrides.push(LayoutRatioOverride {
            slot_id: slot_id.to_string(),
            axis,
            ratio,
        });
    }

    /// Remove a ratio override if present.
    pub fn clear_ratio(&mut self, slot_id: &str, axis: Axis) {
        self.ratio_overrides
            .retain(|entry| !(entry.slot_id == slot_id && entry.axis == axis));
    }

    /// Set explicit collapsed state.
    pub fn set_collapsed(&mut self, slot_id: &str, collapsed: bool) {
        if !collapsed {
            self.clear_collapsed(slot_id);
            return;
        }

        if self.collapsed.iter().any(|entry| entry.slot_id == slot_id) {
            return;
        }

        self.collapsed.push(LayoutCollapsedState {
            slot_id: slot_id.to_string(),
            collapsed,
        });
    }

    /// Toggle explicit collapsed state.
    pub fn toggle_collapsed(&mut self, slot_id: &str) {
        if self.is_collapsed(slot_id) {
            self.clear_collapsed(slot_id);
        } else {
            self.collapsed.push(LayoutCollapsedState {
                slot_id: slot_id.to_string(),
                collapsed: true,
            });
        }
    }

    /// Remove any explicit collapsed state for the slot.
    pub fn clear_collapsed(&mut self, slot_id: &str) {
        self.collapsed.retain(|entry| entry.slot_id != slot_id);
    }

    /// Clear ratio/collapse state.
    pub fn reset(&mut self) {
        self.ratio_overrides.clear();
        self.collapsed.clear();
    }

    /// Apply a state action in reducer style.
    pub fn apply(&mut self, action: LayoutAction) {
        match action {
            LayoutAction::SetRatio {
                slot_id,
                axis,
                ratio,
            } => self.set_ratio(&slot_id, axis, ratio),
            LayoutAction::ClearRatio { slot_id, axis } => self.clear_ratio(&slot_id, axis),
            LayoutAction::SetCollapsed { slot_id, collapsed } => {
                self.set_collapsed(&slot_id, collapsed)
            }
            LayoutAction::ToggleCollapsed { slot_id } => self.toggle_collapsed(&slot_id),
            LayoutAction::ClearCollapsed { slot_id } => self.clear_collapsed(&slot_id),
            LayoutAction::Reset => self.reset(),
        }
    }

    /// Build a solver-ready preference snapshot that borrows from this state.
    pub fn preferences(&self) -> LayoutPreferenceSnapshot<'_> {
        let ratios: Vec<(&str, Axis, f32)> = self
            .ratio_overrides
            .iter()
            .map(|entry| (entry.slot_id.as_str(), entry.axis, entry.ratio))
            .collect();

        let collapsed: Vec<(&str, bool)> = self
            .collapsed
            .iter()
            .map(|entry| (entry.slot_id.as_str(), entry.collapsed))
            .collect();

        LayoutPreferenceSnapshot { ratios, collapsed }
    }
}

/// Borrowed preferences derived from [`LayoutState`].
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutPreferenceSnapshot<'a> {
    ratios: Vec<(&'a str, Axis, f32)>,
    collapsed: Vec<(&'a str, bool)>,
}

impl<'a> LayoutPreferenceSnapshot<'a> {
    /// Borrow these preference slices as a solver input.
    pub fn as_preferences(&'a self) -> LayoutPreferences<'a> {
        LayoutPreferences {
            ratios: &self.ratios,
            collapsed: &self.collapsed,
        }
    }

    /// Access the ratio overrides represented in this snapshot.
    pub fn ratios(&self) -> &[(&str, Axis, f32)] {
        &self.ratios
    }

    /// Access the explicit collapsed states represented in this snapshot.
    pub fn collapsed(&self) -> &[(&str, bool)] {
        &self.collapsed
    }
}

#[cfg(test)]
mod tests {
    use super::{LayoutAction, LayoutState};
    use crate::Axis;
    use crate::solver::solve;
    use crate::types::{ContainerNode, DisplayTier, LayoutNode, Sizing, SlotNode};

    fn simple_slot<'a>(
        id: &'a str,
        sizing: Sizing<'a>,
        priority: f32,
        collapsible: bool,
    ) -> LayoutNode<'a> {
        LayoutNode::Slot(SlotNode {
            id,
            sizing,
            priority,
            collapsible,
            display_tiers: &[],
            collapse_label: None,
        })
    }

    #[test]
    fn ratio_overrides_can_be_set_and_cleared() {
        let mut state = LayoutState::new();

        state.set_ratio("panel", Axis::Horizontal, 0.3);
        state.set_ratio("panel", Axis::Horizontal, 0.45);
        assert_eq!(state.ratio_for("panel", Axis::Horizontal), Some(0.45));

        state.set_ratio("panel", Axis::Vertical, 0.5);
        assert_eq!(state.ratio_for("panel", Axis::Vertical), Some(0.5));

        state.clear_ratio("panel", Axis::Horizontal);
        assert_eq!(state.ratio_for("panel", Axis::Horizontal), None);
        assert_eq!(state.ratio_for("panel", Axis::Vertical), Some(0.5));
    }

    #[test]
    fn collapsed_state_is_toggleable() {
        let mut state = LayoutState::new();

        assert!(!state.is_collapsed("panel"));
        state.set_collapsed("panel", true);
        assert!(state.is_collapsed("panel"));

        state.toggle_collapsed("panel");
        assert!(!state.is_collapsed("panel"));

        state.toggle_collapsed("panel");
        assert!(state.is_collapsed("panel"));

        state.clear_collapsed("panel");
        assert!(!state.is_collapsed("panel"));
    }

    #[test]
    fn reducer_actions_update_state() {
        let mut state = LayoutState::new();

        state.apply(LayoutAction::SetRatio {
            slot_id: "left".to_string(),
            axis: Axis::Horizontal,
            ratio: 0.2,
        });
        assert_eq!(state.ratio_for("left", Axis::Horizontal), Some(0.2));

        state.apply(LayoutAction::SetCollapsed {
            slot_id: "left".to_string(),
            collapsed: true,
        });
        assert!(state.is_collapsed("left"));

        state.apply(LayoutAction::ToggleCollapsed {
            slot_id: "left".to_string(),
        });
        assert!(!state.is_collapsed("left"));

        state.apply(LayoutAction::ClearRatio {
            slot_id: "left".to_string(),
            axis: Axis::Horizontal,
        });
        assert_eq!(state.ratio_for("left", Axis::Horizontal), None);

        state.apply(LayoutAction::Reset);
        assert!(state.ratios().is_empty());
        assert!(state.collapsed().is_empty());
    }

    #[test]
    fn snapshot_drives_solver_preferences() {
        static TIERS: &[DisplayTier<'_>] = &[DisplayTier {
            name: "Full",
            min_size: 200.0,
        }];

        let children = [
            simple_slot("left", Sizing::fractional(0.3, 80.0), 0.5, true),
            LayoutNode::Slot(SlotNode {
                id: "main",
                sizing: Sizing::flex(200.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: TIERS,
                collapse_label: None,
            }),
        ];
        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: None,
            sizing: Sizing::flex(0.0),
            children: &children,
            divider_size: 0.0,
        });

        let mut state = LayoutState::new();
        state.set_ratio("left", Axis::Horizontal, 0.5);

        let solved = solve(&root, 1000.0, 600.0, &state.preferences().as_preferences());
        assert_eq!(solved.find("left").unwrap().width, 500.0);

        state.set_collapsed("left", true);
        let solved = solve(&root, 1000.0, 600.0, &state.preferences().as_preferences());
        let left = solved.find("left").unwrap();
        assert!(!left.visible);
        assert_eq!(left.width, 0.0);

        let main = solved.find("main").unwrap();
        assert_eq!(main.width, 1000.0);
    }
}
