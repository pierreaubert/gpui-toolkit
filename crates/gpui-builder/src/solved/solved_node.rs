use super::layout_debug_report::LayoutDebugReport;
use super::layout_debug_report::build_debug_report;
use super::solved_tree::{NodeIndex, SolvedNodeData, SolvedTree};
use crate::types::{Axis, LayoutNode};
use std::collections::HashMap;

/// A resolved node in the layout tree.
///
/// Strings are borrowed from the source [`LayoutNode`] tree, so solving a
/// declaration never allocates ids, collapse labels, or display-tier names.
#[derive(Debug, Clone)]
pub struct SolvedNode<'a> {
    /// Matches the `id` from the source `LayoutNode`.
    pub id: &'a str,
    /// Resolved width in pixels.
    pub width: f32,
    /// Resolved height in pixels.
    pub height: f32,
    /// Whether this node is visible (false = collapsed or hidden).
    pub visible: bool,
    /// Which display tier is active (for slots with `display_tiers`).
    /// `None` if no tier matches or node has no tiers.
    pub active_tier: Option<&'a str>,
    /// Tab label if this slot was collapsed.
    pub collapse_label: Option<&'a str>,
    /// The resolved axis for this container (`None` for slots).
    pub resolved_axis: Option<Axis>,
    /// Space inserted between each pair of visible children for this
    /// container. Slots always use `0.0`.
    pub divider_size: f32,
    /// Resolved children (empty for slots, populated for containers).
    pub children: Vec<SolvedNode<'a>>,
}

/// Owned counterpart to [`SolvedNode`].
///
/// This is useful when a layout declaration is intentionally short-lived, as
/// with [`crate::solve_layout!`]. Converting copies the small amount of text
/// metadata needed by the solved result and releases the declaration tree.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedSolvedNode {
    /// Matches `id` from the source layout node.
    pub id: String,
    /// Resolved width in pixels.
    pub width: f32,
    /// Resolved height in pixels.
    pub height: f32,
    /// Whether this node is visible.
    pub visible: bool,
    /// Active display tier, if any.
    pub active_tier: Option<String>,
    /// Tab label if this slot was collapsed.
    pub collapse_label: Option<String>,
    /// Resolved container axis, if this node is a container.
    pub resolved_axis: Option<Axis>,
    /// Space inserted between visible children of this container.
    pub divider_size: f32,
    /// Resolved children.
    pub children: Vec<OwnedSolvedNode>,
}

impl OwnedSolvedNode {
    /// Find a solved node by id with depth-first traversal.
    pub fn find(&self, id: &str) -> Option<&Self> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(id))
    }
}

impl From<SolvedNode<'_>> for OwnedSolvedNode {
    fn from(node: SolvedNode<'_>) -> Self {
        Self {
            id: node.id.to_owned(),
            width: node.width,
            height: node.height,
            visible: node.visible,
            active_tier: node.active_tier.map(str::to_owned),
            collapse_label: node.collapse_label.map(str::to_owned),
            resolved_axis: node.resolved_axis,
            divider_size: node.divider_size,
            children: node.children.into_iter().map(Self::from).collect(),
        }
    }
}

impl<'a> SolvedNode<'a> {
    /// Convert this borrowed solved tree into an owned one.
    pub fn into_owned(self) -> OwnedSolvedNode {
        OwnedSolvedNode::from(self)
    }

    /// Convert this recursive tree into an arena/flat [`SolvedTree`].
    pub fn into_tree(self) -> SolvedTree<'a> {
        let mut nodes = Vec::new();
        let mut index = HashMap::new();
        collect_into_tree(self, &mut nodes, &mut index);
        SolvedTree::from_parts(nodes, index)
    }

    /// Find a solved node by id (depth-first search).
    pub fn find(&self, id: &str) -> Option<&SolvedNode<'a>> {
        if self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find(id) {
                return Some(found);
            }
        }
        None
    }

    /// Build a flat id → node map for O(1) repeated lookups.
    ///
    /// This walks the tree once. Use it when you need to query many ids in a
    /// single render or inspection pass.
    pub fn as_map(&self) -> HashMap<&str, &SolvedNode<'a>> {
        let mut map = HashMap::new();
        self.collect_into_map(&mut map);
        map
    }

    fn collect_into_map<'b>(&'b self, map: &mut HashMap<&'b str, &'b SolvedNode<'a>>) {
        map.insert(self.id, self);
        for child in &self.children {
            child.collect_into_map(map);
        }
    }

    /// Returns the size along the given axis.
    pub fn size_along(&self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        }
    }

    /// Collect all collapsed nodes with their labels.
    pub fn collapsed_tabs(&self) -> Vec<(&str, &str)> {
        let mut tabs = Vec::new();
        self.collect_collapsed(&mut tabs);
        tabs
    }

    pub(super) fn collect_collapsed<'b>(&'b self, tabs: &mut Vec<(&'b str, &'b str)>) {
        if !self.visible
            && let Some(label) = self.collapse_label
        {
            tabs.push((self.id, label));
        }
        for child in &self.children {
            child.collect_collapsed(tabs);
        }
    }

    /// Build a stable textual report for this solved tree.
    ///
    /// This solved-only variant includes concrete sizes, visibility, active
    /// display tier, resolved container axis, and warnings for suspicious output.
    /// Use [`Self::debug_report_with_source`] when the source `LayoutNode` tree
    /// is available and you also want declared sizing metadata in each line.
    pub fn debug_report(&'a self) -> LayoutDebugReport<'a> {
        build_debug_report(self, None)
    }

    /// Build a stable textual report enriched with source layout metadata.
    ///
    /// When `source` mirrors the solved tree, each line includes the original
    /// sizing mode, collapsibility, and priority. If a solved node is missing
    /// from the source tree, the report still renders the solved node.
    pub fn debug_report_with_source(&'a self, source: &'a LayoutNode<'a>) -> LayoutDebugReport<'a> {
        build_debug_report(self, Some(source))
    }
}

pub(super) fn visibility_label(node: &SolvedNode<'_>) -> &'static str {
    if node.visible { "visible" } else { "collapsed" }
}

fn collect_into_tree<'a>(
    node: SolvedNode<'a>,
    nodes: &mut Vec<SolvedNodeData<'a>>,
    index: &mut HashMap<&'a str, NodeIndex>,
) -> NodeIndex {
    let idx = NodeIndex(nodes.len());
    index.insert(node.id, idx);
    nodes.push(SolvedNodeData {
        id: node.id,
        width: node.width,
        height: node.height,
        visible: node.visible,
        active_tier: node.active_tier,
        collapse_label: node.collapse_label,
        resolved_axis: node.resolved_axis,
        divider_size: node.divider_size,
        children: Vec::new(),
    });

    let mut child_indices = Vec::with_capacity(node.children.len());
    for child in node.children {
        child_indices.push(collect_into_tree(child, nodes, index));
    }
    nodes[idx.0].children = child_indices;
    idx
}
