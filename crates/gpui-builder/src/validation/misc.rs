pub(super) fn is_non_negative(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

pub(super) fn node_path(parent_path: Option<&str>, id: &str) -> String {
    let segment = if id.is_empty() { "<empty>" } else { id };
    match parent_path {
        Some(parent_path) => format!("{parent_path}/{segment}"),
        None => segment.to_string(),
    }
}
