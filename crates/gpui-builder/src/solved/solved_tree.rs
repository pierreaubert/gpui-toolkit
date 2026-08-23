use super::layout_debug_report::LayoutDebugReport;
use super::layout_debug_warning::LayoutDebugWarning;
use super::misc::{axis_name, source_child};
use super::solved_node::SolvedNode;
use super::types::LayoutDebugWarningKind;
use crate::types::{Axis, LayoutNode};
use crate::util::format_number;
use std::collections::HashMap;

/// Index into a [`SolvedTree`] node vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeIndex(pub usize);

/// Allocation-free map-like view over a solved tree's retained id index.
#[derive(Clone, Copy)]
pub struct SolvedTreeMap<'tree, 'a> {
    tree: &'tree SolvedTree<'a>,
}

pub struct SolvedTreeMapIter<'tree, 'a> {
    index: std::collections::hash_map::Iter<'tree, &'a str, NodeIndex>,
    nodes: &'tree [SolvedNodeData<'a>],
}

impl<'tree, 'a> Iterator for SolvedTreeMapIter<'tree, 'a> {
    type Item = (&'a str, &'tree SolvedNodeData<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        self.index
            .next()
            .map(|(&id, index)| (id, &self.nodes[index.0]))
    }
}

impl<'tree, 'a> IntoIterator for &SolvedTreeMap<'tree, 'a> {
    type Item = (&'a str, &'tree SolvedNodeData<'a>);
    type IntoIter = SolvedTreeMapIter<'tree, 'a>;

    fn into_iter(self) -> Self::IntoIter {
        SolvedTreeMapIter {
            index: self.tree.index.iter(),
            nodes: &self.tree.nodes,
        }
    }
}

impl<'tree, 'a> SolvedTreeMap<'tree, 'a> {
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&'tree SolvedNodeData<'a>> {
        self.tree
            .index
            .get(id)
            .map(|index| &self.tree.nodes[index.0])
    }

    #[must_use]
    pub fn contains_key(&self, id: &str) -> bool {
        self.tree.index.contains_key(id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tree.index.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tree.index.is_empty()
    }
}

/// Stable identity and display label for a collapsed layout slot.
///
/// Both strings borrow from the source [`LayoutNode`], so iterating collapsed
/// slots does not allocate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CollapsedSlot<'a> {
    /// Stable, non-localized slot identifier.
    pub id: &'a str,
    /// Human-readable label for an overflow trigger or surface.
    pub label: &'a str,
}

/// A single node stored in the flat solved tree.
#[derive(Debug, Clone)]
pub struct SolvedNodeData<'a> {
    /// Matches the `id` from the source `LayoutNode`.
    pub id: &'a str,
    /// Resolved width in pixels.
    pub width: f32,
    /// Resolved height in pixels.
    pub height: f32,
    /// Whether this node is visible (false = collapsed or hidden).
    pub visible: bool,
    /// Which display tier is active (for slots with `display_tiers`).
    pub active_tier: Option<&'a str>,
    /// Tab label if this slot was collapsed.
    pub collapse_label: Option<&'a str>,
    /// The resolved axis for this container (`None` for slots).
    pub resolved_axis: Option<Axis>,
    /// Indices of resolved children (empty for slots, populated for containers).
    pub children: Vec<NodeIndex>,
}

impl<'a> SolvedNodeData<'a> {
    /// Returns the size along the given axis.
    pub fn size_along(&self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        }
    }
}

/// A reference to a node inside a [`SolvedTree`].
#[derive(Debug, Clone, Copy)]
pub struct SolvedNodeRef<'tree, 'a> {
    tree: &'tree SolvedTree<'a>,
    idx: NodeIndex,
}

impl<'tree, 'a> SolvedNodeRef<'tree, 'a> {
    /// Node identifier borrowed from the source tree.
    pub fn id(&self) -> &'a str {
        self.data().id
    }

    /// Resolved width in pixels.
    pub fn width(&self) -> f32 {
        self.data().width
    }

    /// Resolved height in pixels.
    pub fn height(&self) -> f32 {
        self.data().height
    }

    /// Whether this node is visible.
    pub fn visible(&self) -> bool {
        self.data().visible
    }

    /// Active display tier, if any.
    pub fn active_tier(&self) -> Option<&'a str> {
        self.data().active_tier
    }

    /// Collapse label, if any.
    pub fn collapse_label(&self) -> Option<&'a str> {
        self.data().collapse_label
    }

    /// Resolved container axis, if any.
    pub fn resolved_axis(&self) -> Option<Axis> {
        self.data().resolved_axis
    }

    /// Returns the size along the given axis.
    pub fn size_along(&self, axis: Axis) -> f32 {
        self.data().size_along(axis)
    }

    /// Iterate over this node's children.
    pub fn children(&self) -> impl Iterator<Item = SolvedNodeRef<'tree, 'a>> {
        self.data().children.iter().map(move |&idx| SolvedNodeRef {
            tree: self.tree,
            idx,
        })
    }

    fn data(&self) -> &'tree SolvedNodeData<'a> {
        &self.tree.nodes[self.idx.0]
    }
}

/// Arena-based (flat) representation of a solved layout tree.
///
/// `SolvedTree` stores all nodes in a single `Vec` and indexes them by id for
/// O(1) lookup. Iteration yields nodes in pre-order DFS order, matching the
/// structure of the original recursive [`SolvedNode`] tree.
#[derive(Debug, Clone)]
pub struct SolvedTree<'a> {
    nodes: Vec<SolvedNodeData<'a>>,
    index: HashMap<&'a str, NodeIndex>,
    /// Recycled child-index buffers, one per previously solved container.
    child_index_pool: Vec<Vec<NodeIndex>>,
}

impl<'a> SolvedTree<'a> {
    /// Create a tree from already-constructed nodes and an id index.
    ///
    /// Nodes are assumed to be stored in pre-order DFS and the first node is the
    /// root. This is used by the solver; callers building trees by hand should
    /// prefer [`SolvedNode::into_tree`].
    pub(crate) fn from_parts(
        nodes: Vec<SolvedNodeData<'a>>,
        index: HashMap<&'a str, NodeIndex>,
    ) -> Self {
        Self {
            nodes,
            index,
            child_index_pool: Vec::new(),
        }
    }

    /// Create an empty reusable solved-tree target.
    ///
    /// Populate it with [`crate::solve_tree_into`] before calling [`Self::root`].
    pub fn with_capacity(node_capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(node_capacity),
            index: HashMap::with_capacity(node_capacity),
            child_index_pool: Vec::new(),
        }
    }

    pub(crate) fn prepare_for_reuse(&mut self, node_capacity: usize) {
        self.child_index_pool.reserve(
            self.nodes
                .iter()
                .filter(|node| !node.children.is_empty())
                .count(),
        );
        // Push in reverse DFS order so `pop()` returns buffers in the same
        // parent-before-child order used by the next solve.
        for node in self.nodes.drain(..).rev() {
            if !node.children.is_empty() {
                let mut children = node.children;
                children.clear();
                self.child_index_pool.push(children);
            }
        }
        if self.nodes.capacity() < node_capacity {
            self.nodes.reserve(node_capacity);
        }
        self.index.clear();
        if self.index.capacity() < node_capacity {
            self.index.reserve(node_capacity);
        }
    }

    pub(crate) fn reusable_parts(
        &mut self,
    ) -> (
        &mut Vec<SolvedNodeData<'a>>,
        &mut HashMap<&'a str, NodeIndex>,
        &mut Vec<Vec<NodeIndex>>,
    ) {
        (&mut self.nodes, &mut self.index, &mut self.child_index_pool)
    }

    /// Find a node by id in O(1).
    pub fn find(&self, id: &str) -> Option<SolvedNodeRef<'_, 'a>> {
        self.index
            .get(id)
            .copied()
            .map(|idx| SolvedNodeRef { tree: self, idx })
    }

    /// Reference to the root node.
    pub fn root(&self) -> SolvedNodeRef<'_, 'a> {
        SolvedNodeRef {
            tree: self,
            idx: NodeIndex(0),
        }
    }

    /// Iterate over all nodes in pre-order DFS order.
    pub fn iter(&self) -> impl Iterator<Item = SolvedNodeRef<'_, 'a>> {
        self.nodes
            .iter()
            .enumerate()
            .map(move |(i, _)| SolvedNodeRef {
                tree: self,
                idx: NodeIndex(i),
            })
    }

    /// Iterate id → node pairs without allocating a temporary map.
    ///
    /// Prefer this for a complete pass over the solved tree. For targeted
    /// repeated lookups, use [`Self::find`], whose retained index is O(1).
    pub fn iter_by_id(&self) -> impl Iterator<Item = (&'a str, SolvedNodeRef<'_, 'a>)> {
        self.nodes.iter().enumerate().map(move |(i, node)| {
            (
                node.id,
                SolvedNodeRef {
                    tree: self,
                    idx: NodeIndex(i),
                },
            )
        })
    }

    /// Borrow the retained id index through an allocation-free map-like view.
    pub fn as_map(&self) -> SolvedTreeMap<'_, 'a> {
        SolvedTreeMap { tree: self }
    }

    /// Number of entries in the retained O(1) lookup index.
    #[cfg(test)]
    pub(super) fn cached_index_len(&self) -> usize {
        self.index.len()
    }

    /// Iterate over collapsed slots in stable pre-order declaration order.
    ///
    /// This iterator borrows the solved tree and performs no allocation.
    pub fn collapsed_slots(&self) -> impl Iterator<Item = CollapsedSlot<'a>> + '_ {
        self.nodes
            .iter()
            .filter(|node| !node.visible)
            .filter_map(|node| {
                node.collapse_label
                    .map(|label| CollapsedSlot { id: node.id, label })
            })
    }

    /// Collect all collapsed nodes with their labels.
    ///
    /// Compatibility alias for callers using the original tab-shaped API.
    /// New code should prefer the allocation-free [`Self::collapsed_slots`].
    pub fn collapsed_tabs(&self) -> Vec<(&str, &str)> {
        self.collapsed_slots()
            .map(|slot| (slot.id, slot.label))
            .collect()
    }

    /// Build a stable textual report for this solved tree.
    pub fn debug_report(&'a self) -> LayoutDebugReport<'a> {
        build_debug_report_for_tree(self, None)
    }

    /// Build a stable textual report enriched with source layout metadata.
    pub fn debug_report_with_source(&'a self, source: &'a LayoutNode<'a>) -> LayoutDebugReport<'a> {
        build_debug_report_for_tree(self, Some(source))
    }

    /// Total number of nodes in the tree.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the tree contains no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl<'a> From<SolvedNode<'a>> for SolvedTree<'a> {
    fn from(node: SolvedNode<'a>) -> Self {
        node.into_tree()
    }
}

const WARNING_EPSILON: f32 = 0.5;

fn build_debug_report_for_tree<'a>(
    tree: &'a SolvedTree<'a>,
    source: Option<&'a LayoutNode<'a>>,
) -> LayoutDebugReport<'a> {
    let mut lines = Vec::new();
    let mut warnings = Vec::new();
    append_debug_node_for_tree(
        tree,
        NodeIndex(0),
        source.filter(|s| s.id() == tree.root().id()),
        0,
        &mut lines,
        &mut warnings,
    );
    LayoutDebugReport {
        tree: lines.join("\n"),
        warnings,
    }
}

fn append_debug_node_for_tree<'a>(
    tree: &'a SolvedTree<'a>,
    idx: NodeIndex,
    source: Option<&'a LayoutNode<'a>>,
    depth: usize,
    lines: &mut Vec<String>,
    warnings: &mut Vec<LayoutDebugWarning<'a>>,
) {
    let node = &tree.nodes[idx.0];
    let indent = "  ".repeat(depth);
    let mut line = format!(
        "{indent}{} size={}x{} {}",
        node.id,
        format_number(node.width),
        format_number(node.height),
        visibility_label(node),
    );

    if let Some(axis) = node.resolved_axis {
        line.push_str(" axis=");
        line.push_str(axis_name(axis));
    }

    if let Some(tier) = node.active_tier {
        line.push_str(" tier=");
        line.push_str(tier);
    }

    if let Some(label) = node.collapse_label
        && !node.visible
    {
        line.push_str(" label=");
        line.push_str(&format!("{label:?}"));
    }

    if let Some(source) = source {
        use super::format::format_sizing;
        line.push_str(" sizing=");
        line.push_str(&format_sizing(source.sizing()));

        if source.collapsible() {
            line.push_str(" collapsible priority=");
            line.push_str(&format_number(source.priority()));
        }
    }

    lines.push(line);
    collect_node_warnings(tree, node, warnings);

    for &child_idx in &node.children {
        let source_child = source_child(source, tree.nodes[child_idx.0].id);
        append_debug_node_for_tree(tree, child_idx, source_child, depth + 1, lines, warnings);
    }
}

fn visibility_label(node: &SolvedNodeData<'_>) -> &'static str {
    if node.visible { "visible" } else { "collapsed" }
}

fn collect_node_warnings<'a>(
    tree: &'a SolvedTree<'a>,
    node: &SolvedNodeData<'a>,
    warnings: &mut Vec<LayoutDebugWarning<'a>>,
) {
    if !node.width.is_finite()
        || !node.height.is_finite()
        || node.width < -WARNING_EPSILON
        || node.height < -WARNING_EPSILON
    {
        warnings.push(LayoutDebugWarning {
            node_id: node.id,
            kind: LayoutDebugWarningKind::InvalidSize {
                width: node.width,
                height: node.height,
            },
        });
    }

    if !node.visible && node.collapse_label.is_none() {
        warnings.push(LayoutDebugWarning {
            node_id: node.id,
            kind: LayoutDebugWarningKind::InvisibleWithoutCollapseLabel,
        });
    }

    let Some(axis) = node.resolved_axis else {
        return;
    };
    if !node.visible {
        return;
    }

    let available = node.size_along(axis);
    let used: f32 = node
        .children
        .iter()
        .filter(|&&child_idx| tree.nodes[child_idx.0].visible)
        .map(|&child_idx| tree.nodes[child_idx.0].size_along(axis))
        .sum();

    if available.is_finite() && used.is_finite() && used > available + WARNING_EPSILON {
        warnings.push(LayoutDebugWarning {
            node_id: node.id,
            kind: LayoutDebugWarningKind::MainAxisOverflow {
                axis,
                used,
                available,
            },
        });
    }

    let cross = axis.cross();
    let available_cross = node.size_along(cross);
    if !available_cross.is_finite() {
        return;
    }

    for &child_idx in node.children.iter().filter(|&&c| tree.nodes[c.0].visible) {
        let child = &tree.nodes[child_idx.0];
        let child_cross = child.size_along(cross);
        if child_cross.is_finite() && child_cross > available_cross + WARNING_EPSILON {
            warnings.push(LayoutDebugWarning {
                node_id: node.id,
                kind: LayoutDebugWarningKind::CrossAxisOverflow {
                    axis: cross,
                    child_id: child.id,
                    used: child_cross,
                    available: available_cross,
                },
            });
        }
    }
}
