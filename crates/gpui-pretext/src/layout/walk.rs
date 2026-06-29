use super::to::to_layout_line_range;
use super::to::to_line_break_data;
use super::types::LayoutLineRange;
use super::types::PreparedTextWithSegments;
use crate::line_break::{
    InternalLayoutLine, KnuthPlassParams, walk_prepared_lines, walk_prepared_lines_optimal,
};
use crate::measurement::EngineProfile;

/// Iterate line ranges with a callback (no text materialization).
pub fn walk_line_ranges(
    prepared: &PreparedTextWithSegments,
    max_width: f64,
    profile: &EngineProfile,
    mut on_line: impl FnMut(&LayoutLineRange),
) -> usize {
    let data = to_line_break_data(&prepared.core);

    if prepared.core.widths.is_empty() {
        return 0;
    }

    walk_prepared_lines(
        &data,
        max_width,
        profile,
        Some(&mut |line: &InternalLayoutLine| {
            on_line(&to_layout_line_range(line));
        }),
    )
}

/// Iterate line ranges using Knuth-Plass optimal line breaking.
pub fn walk_line_ranges_optimal(
    prepared: &PreparedTextWithSegments,
    max_width: f64,
    profile: &EngineProfile,
    params: &KnuthPlassParams,
    mut on_line: impl FnMut(&LayoutLineRange),
) -> usize {
    let data = to_line_break_data(&prepared.core);

    if prepared.core.widths.is_empty() {
        return 0;
    }

    walk_prepared_lines_optimal(
        &data,
        max_width,
        profile,
        params,
        Some(&mut |line: &InternalLayoutLine| {
            on_line(&to_layout_line_range(line));
        }),
    )
}
