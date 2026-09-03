use super::get::get_breakable_advance;
use super::knuth_plass_params::KnuthPlassParams;
use super::misc::is_simple_collapsible_space;
use super::misc::skipped_at_fresh_line_start;
use super::types::PreparedLineBreakData;
use super::walk::walk_prepared_lines;
use super::walk::walk_prepared_lines_optimal;
use crate::measurement::EngineProfile;

pub fn count_prepared_lines(
    prepared: &PreparedLineBreakData,
    max_width: f64,
    profile: &EngineProfile,
) -> usize {
    if prepared.simple_line_walk_fast_path {
        count_prepared_lines_simple(prepared, max_width, profile)
    } else {
        walk_prepared_lines(prepared, max_width, profile, None)
    }
}

fn count_prepared_lines_simple(
    prepared: &PreparedLineBreakData,
    max_width: f64,
    profile: &EngineProfile,
) -> usize {
    let widths = &prepared.widths;
    let kinds = &prepared.kinds;
    let breakable_widths = &prepared.breakable_widths;
    let breakable_prefix_widths = &prepared.breakable_prefix_widths;

    if widths.is_empty() {
        return 0;
    }

    let eps = profile.line_fit_epsilon;
    let mut line_count = 0usize;
    let mut line_w = 0.0f64;
    let mut has_content = false;

    let place_on_fresh_line =
        |seg_idx: usize, line_count: &mut usize, line_w: &mut f64, has_content: &mut bool| {
            let w = widths[seg_idx];
            if w > max_width && breakable_widths[seg_idx].is_some() {
                let g_widths = breakable_widths[seg_idx].as_ref().unwrap();
                let g_prefix = breakable_prefix_widths[seg_idx].as_deref();
                *line_w = 0.0;
                for g in 0..g_widths.len() {
                    let gw = get_breakable_advance(
                        g_widths,
                        g_prefix,
                        g,
                        profile.prefer_prefix_widths_for_breakable_runs,
                    );
                    if *line_w > 0.0 && *line_w + gw > max_width + eps {
                        *line_count += 1;
                        *line_w = gw;
                    } else {
                        if *line_w == 0.0 {
                            *line_count += 1;
                        }
                        *line_w += gw;
                    }
                }
            } else {
                *line_w = w;
                *line_count += 1;
            }
            *has_content = true;
        };

    let mut i = 0;
    while i < widths.len() {
        // Mirror normalize_line_start (incremental path): a fresh line never
        // starts with a collapsible/break-opportunity segment.
        if !has_content {
            while i < widths.len() && skipped_at_fresh_line_start(kinds[i]) {
                i += 1;
            }
            if i >= widths.len() {
                break;
            }
        }
        let w = widths[i];
        let kind = kinds[i];

        if !has_content {
            place_on_fresh_line(i, &mut line_count, &mut line_w, &mut has_content);
            i += 1;
            continue;
        }

        let new_w = line_w + w;
        if new_w > max_width + eps {
            if is_simple_collapsible_space(kind) {
                line_w = 0.0;
                has_content = false;
                i += 1;
                continue;
            }
            line_w = 0.0;
            has_content = false;
            place_on_fresh_line(i, &mut line_count, &mut line_w, &mut has_content);
            i += 1;
            continue;
        }

        line_w = new_w;
        i += 1;
    }

    line_count
}

/// Count lines using Knuth-Plass optimal algorithm.
pub fn count_prepared_lines_optimal(
    prepared: &PreparedLineBreakData,
    max_width: f64,
    profile: &EngineProfile,
    params: &KnuthPlassParams,
) -> usize {
    walk_prepared_lines_optimal(prepared, max_width, profile, params, None)
}
