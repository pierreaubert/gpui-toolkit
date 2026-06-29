use crate::Axis;

pub(super) fn node_path(parent_path: Option<&str>, id: &str) -> String {
    let segment = if id.is_empty() { "<empty>" } else { id };
    match parent_path {
        Some(parent_path) => format!("{parent_path}/{segment}"),
        None => segment.to_string(),
    }
}

pub(super) fn axis_name(axis: Axis) -> &'static str {
    match axis {
        Axis::Horizontal => "horizontal",
        Axis::Vertical => "vertical",
    }
}
