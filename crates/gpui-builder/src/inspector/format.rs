use crate::util::format_number;

pub(super) fn format_max(value: f32) -> String {
    if value == f32::MAX {
        "unbounded".to_string()
    } else {
        format_number(value)
    }
}
