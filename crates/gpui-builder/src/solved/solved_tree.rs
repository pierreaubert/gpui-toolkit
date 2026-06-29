use super::layout_debug_report::LayoutDebugReport;
use super::layout_debug_warning::LayoutDebugWarning;
use super::misc::{axis_name, source_child};
use super::solved_node::SolvedNode;
use super::types::LayoutDebugWarningKind;
use crate::types::{Axis, LayoutNode};
use crate::util::format_number;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Index into a [`SolvedTree`] node vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeIndex(pub usize);

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
    /// Lazily-built id → node index map reused by [`Self::as_map`].
    cached_as_map_index: OnceLock<HashMap<&'a str, NodeIndex>>,
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
            cached_as_map_index: OnceLock::new(),
        }
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

    /// Build a flat id → node map for O(1) repeated lookups.
    ///
    /// The first call walks the node list once and caches an internal id →
    /// index map. Subsequent calls reuse that cached index, so repeated
    /// lookups are cheap even when callers discard the returned map.
    pub fn as_map(&self) -> HashMap<&str, &SolvedNodeData<'a>> {
        let index = self.cached_as_map_index.get_or_init(|| {
            let mut map = HashMap::with_capacity(self.nodes.len());
            for (i, node) in self.nodes.iter().enumerate() {
                map.insert(node.id, NodeIndex(i));
            }
            map
        });

        index
            .iter()
            .map(|(&id, &idx)| (id, &self.nodes[idx.0]))
            .collect()
    }

    /// Length of the lazily-built index used by [`Self::as_map`].
    ///
    /// Returns `0` before the first `as_map` call, and the number of indexed
    /// ids afterwards.
    #[cfg(test)]
    pub(super) fn cached_index_len(&self) -> usize {
        self.cached_as_map_index.get().map_or(0, |m| m.len())
    }

    /// Collect all collapsed nodes with their labels.
    pub fn collapsed_tabs(&self) -> Vec<(&str, &str)> {
        self.nodes
            .iter()
            .filter_map(|node| {
                if !node.visible {
                    node.collapse_label.map(|label| (node.id, label))
                } else {
                    None
                }
            })
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
