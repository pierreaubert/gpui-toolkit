use super::axis::Axis;
use std::collections::HashMap;

/// Keep externally supplied ratio state in the domain understood by the
/// solver. Invalid user or persisted values should not poison layout output.
pub(crate) fn sanitize_ratio(ratio: f32) -> f32 {
    if ratio.is_finite() {
        ratio.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// External state provided to the solver. The solver reads but never writes.
///
/// Internally stores overrides in hash maps so repeated `ratio_for` / `is_collapsed`
/// lookups are O(1) instead of linear scans.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayoutPreferences<'a> {
    pub(crate) ratios: HashMap<(&'a str, Axis), f32>,
    pub(crate) collapsed: HashMap<&'a str, bool>,
}

impl<'a> LayoutPreferences<'a> {
    /// Build preferences from override slices.
    pub fn new(ratios: &'a [(&'a str, Axis, f32)], collapsed: &'a [(&'a str, bool)]) -> Self {
        Self {
            ratios: ratios
                .iter()
                .map(|(slot_id, axis, ratio)| ((*slot_id, *axis), sanitize_ratio(*ratio)))
                .collect(),
            collapsed: collapsed
                .iter()
                .map(|(slot_id, collapsed)| (*slot_id, *collapsed))
                .collect(),
        }
    }

    /// Per-slot ratio overrides, keyed by (slot_id, parent_axis).
    /// When the solver resolves a `Fractional` slot, it looks here first.
    /// If not found, uses `Sizing::Fractional::initial`.
    pub fn ratios(&self) -> &HashMap<(&'a str, Axis), f32> {
        &self.ratios
    }

    /// Per-slot collapsed state from user toggle.
    /// If a slot's id appears here with `true`, and the slot is `collapsible`,
    /// the solver treats it as collapsed (0 size) regardless of available space.
    pub fn collapsed(&self) -> &HashMap<&'a str, bool> {
        &self.collapsed
    }

    /// Update a ratio override while retaining the existing lookup map.
    pub fn set_ratio(&mut self, id: &'a str, axis: Axis, ratio: f32) {
        self.ratios.insert((id, axis), sanitize_ratio(ratio));
    }

    /// Update an explicit collapse override while retaining the existing lookup map.
    pub fn set_collapsed(&mut self, id: &'a str, collapsed: bool) {
        self.collapsed.insert(id, collapsed);
    }

    /// Look up the user's ratio override for a slot in a given axis.
    pub fn ratio_for(&self, id: &str, axis: Axis) -> Option<f32> {
        self.ratios.get(&(id, axis)).copied()
    }

    /// Check if a slot is user-collapsed.
    pub fn is_collapsed(&self, id: &str) -> bool {
        self.collapsed.get(id).copied().unwrap_or(false)
    }
}
