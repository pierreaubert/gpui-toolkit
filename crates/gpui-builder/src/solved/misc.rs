use crate::types::{Axis, LayoutNode};

pub(super) const WARNING_EPSILON: f32 = 0.5;

pub(super) fn source_child<'a>(
    source: Option<&'a LayoutNode<'a>>,
    child_id: &str,
) -> Option<&'a LayoutNode<'a>> {
    match source? {
        LayoutNode::Container(container) => container
            .children
            .iter()
            .find(|child| child.id() == child_id),
        LayoutNode::Slot(_) => None,
    }
}

pub(super) fn axis_name(axis: Axis) -> &'static str {
    match axis {
        Axis::Horizontal => "horizontal",
        Axis::Vertical => "vertical",
    }
}
