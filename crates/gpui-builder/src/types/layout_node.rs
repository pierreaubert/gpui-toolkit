use super::axis::Axis;
use super::container_node::ContainerNode;
use super::sizing::Sizing;
use super::slot_node::SlotNode;

/// A node in the declarative layout tree.
#[derive(Debug, Clone, Copy)]
pub enum LayoutNode<'a> {
    /// A leaf: a named region where the consumer renders content.
    Slot(SlotNode<'a>),
    /// A branch: arranges children along an axis.
    Container(ContainerNode<'a>),
}

impl<'a> LayoutNode<'a> {
    /// Construct a leaf node with default slot options.
    ///
    /// Use [`SlotNode::new`] when you need to chain slot-specific options such
    /// as collapse labels or display tiers before converting into a
    /// `LayoutNode`.
    pub const fn slot(id: &'a str, sizing: Sizing<'a>) -> Self {
        LayoutNode::Slot(SlotNode::new(id, sizing))
    }

    /// Construct a container node with default container options.
    ///
    /// Use [`ContainerNode::new`] when you need to chain options such as
    /// `auto_axis` or `divider_size` before converting into a `LayoutNode`.
    pub const fn container(
        id: &'a str,
        axis: Axis,
        sizing: Sizing<'a>,
        children: &'a [LayoutNode<'a>],
    ) -> Self {
        LayoutNode::Container(ContainerNode::new(id, axis, sizing, children))
    }

    /// Returns the node's unique identifier.
    pub fn id(&self) -> &'a str {
        match self {
            LayoutNode::Slot(s) => s.id,
            LayoutNode::Container(c) => c.id,
        }
    }

    /// Returns the node's sizing constraint.
    pub fn sizing(&self) -> Sizing<'a> {
        match self {
            LayoutNode::Slot(s) => s.sizing,
            LayoutNode::Container(c) => c.sizing,
        }
    }

    /// Returns true if this node can be collapsed.
    pub fn collapsible(&self) -> bool {
        match self {
            LayoutNode::Slot(s) => s.collapsible,
            LayoutNode::Container(_) => false,
        }
    }

    /// Returns the collapse priority (0.0 = first to collapse, 1.0 = never).
    pub fn priority(&self) -> f32 {
        match self {
            LayoutNode::Slot(s) => s.priority,
            LayoutNode::Container(_) => 1.0,
        }
    }

    /// Total number of nodes in this subtree, including this node.
    pub fn node_count(&self) -> usize {
        match self {
            LayoutNode::Slot(_) => 1,
            LayoutNode::Container(c) => {
                1 + c
                    .children
                    .iter()
                    .map(|child| child.node_count())
                    .sum::<usize>()
            }
        }
    }
}

impl<'a> From<SlotNode<'a>> for LayoutNode<'a> {
    fn from(slot: SlotNode<'a>) -> Self {
        LayoutNode::Slot(slot)
    }
}

impl<'a> From<ContainerNode<'a>> for LayoutNode<'a> {
    fn from(container: ContainerNode<'a>) -> Self {
        LayoutNode::Container(container)
    }
}
