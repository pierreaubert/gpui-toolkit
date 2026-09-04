use super::get::get_breakable_advance;
use super::get::get_tab_advance;
use super::knuth_plass_params::KnuthPlassParams;
use super::misc::can_break_after;
use super::misc::fit_soft_hyphen_break;
use super::misc::skipped_at_fresh_line_start;
use super::types::InternalLayoutLine;
use super::types::PreparedLineBreakData;
use super::types::breakpoints_to_lines;
use super::types::build_kp_items;
use super::types::knuth_plass_chunk;
/// Line breaking algorithms, ported from chenglou/pretext.
///
/// Implements both:
/// 1. **Greedy** line breaking with pending-break tracking, soft hyphen
///    support, tab stops, and overflow-wrap grapheme-level breaking.
/// 2. **Knuth-Plass** optimal line breaking that minimizes total demerits
///    (badness + penalties) across the entire paragraph using dynamic
///    programming over feasible breakpoints.
///
/// The greedy algorithm has both a "simple" fast path (no tabs/soft-hyphens)
/// and a full complex path.
use crate::analysis::SegmentBreakKind;
use crate::measurement::EngineProfile;

fn walk_prepared_lines_simple(
    prepared: &PreparedLineBreakData,
    max_width: f64,
    profile: &EngineProfile,
    on_line: &mut Option<&mut dyn FnMut(&InternalLayoutLine)>,
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
    let mut line_start_seg = 0usize;
    let mut line_start_graph = 0usize;
    let mut line_end_seg = 0usize;
    let mut line_end_graph = 0usize;
    let mut pending_break_seg: Option<usize> = None;
    let mut pending_break_paint_w = 0.0f64;

    macro_rules! emit_line {
        ($end_seg:expr, $end_graph:expr, $w:expr) => {{
            line_count += 1;
            if let Some(cb) = on_line.as_mut() {
                cb(&InternalLayoutLine {
                    start_segment_index: line_start_seg,
                    start_grapheme_index: line_start_graph,
                    end_segment_index: $end_seg,
                    end_grapheme_index: $end_graph,
                    width: $w,
                });
            }
            line_w = 0.0;
            has_content = false;
            pending_break_seg = None;
            pending_break_paint_w = 0.0;
        }};
        () => {
            emit_line!(line_end_seg, line_end_graph, line_w)
        };
    }

    macro_rules! start_at_segment {
        ($seg:expr, $w:expr) => {{
            has_content = true;
            line_start_seg = $seg;
            line_start_graph = 0;
            line_end_seg = $seg + 1;
            line_end_graph = 0;
            line_w = $w;
        }};
    }

    macro_rules! append_segment {
        ($seg:expr, $w:expr) => {{
            if !has_content {
                start_at_segment!($seg, $w);
            } else {
                line_w += $w;
                line_end_seg = $seg + 1;
                line_end_graph = 0;
            }
        }};
    }

    let append_breakable_from =
        |seg_idx: usize,
         start_g: usize,
         line_count: &mut usize,
         line_w: &mut f64,
         has_content: &mut bool,
         line_start_seg: &mut usize,
         line_start_graph: &mut usize,
         line_end_seg: &mut usize,
         line_end_graph: &mut usize,
         pending_break_seg: &mut Option<usize>,
         pending_break_paint_w: &mut f64,
         on_line: &mut Option<&mut dyn FnMut(&InternalLayoutLine)>| {
            let g_widths = breakable_widths[seg_idx].as_ref().unwrap();
            let g_prefix = breakable_prefix_widths[seg_idx].as_deref();
            for g in start_g..g_widths.len() {
                let gw = get_breakable_advance(
                    g_widths,
                    g_prefix,
                    g,
                    profile.prefer_prefix_widths_for_breakable_runs,
                );
                if !*has_content {
                    *has_content = true;
                    *line_start_seg = seg_idx;
                    *line_start_graph = g;
                    *line_end_seg = seg_idx;
                    *line_end_graph = g + 1;
                    *line_w = gw;
                    continue;
                }
                if *line_w + gw > max_width + eps {
                    // emit
                    *line_count += 1;
                    if let Some(cb) = on_line.as_mut() {
                        cb(&InternalLayoutLine {
                            start_segment_index: *line_start_seg,
                            start_grapheme_index: *line_start_graph,
                            end_segment_index: *line_end_seg,
                            end_grapheme_index: *line_end_graph,
                            width: *line_w,
                        });
                    }
                    *line_w = gw;
                    *has_content = true;
                    *line_start_seg = seg_idx;
                    *line_start_graph = g;
                    *line_end_seg = seg_idx;
                    *line_end_graph = g + 1;
                    *pending_break_seg = None;
                    *pending_break_paint_w = 0.0;
                } else {
                    *line_w += gw;
                    *line_end_seg = seg_idx;
                    *line_end_graph = g + 1;
                }
            }
            if *has_content && *line_end_seg == seg_idx && *line_end_graph == g_widths.len() {
                *line_end_seg = seg_idx + 1;
                *line_end_graph = 0;
            }
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
            if w > max_width && breakable_widths[i].is_some() {
                append_breakable_from(
                    i,
                    0,
                    &mut line_count,
                    &mut line_w,
                    &mut has_content,
                    &mut line_start_seg,
                    &mut line_start_graph,
                    &mut line_end_seg,
                    &mut line_end_graph,
                    &mut pending_break_seg,
                    &mut pending_break_paint_w,
                    on_line,
                );
            } else {
                start_at_segment!(i, w);
            }
            if can_break_after(kind) {
                pending_break_seg = Some(i + 1);
                pending_break_paint_w = line_w - w;
            }
            i += 1;
            continue;
        }

        let new_w = line_w + w;
        if new_w > max_width + eps {
            if can_break_after(kind) {
                append_segment!(i, w);
                emit_line!(i + 1, 0, line_w - w);
                i += 1;
                continue;
            }

            if let Some(pb_seg) = pending_break_seg {
                emit_line!(pb_seg, 0, pending_break_paint_w);
                continue;
            }

            if w > max_width && breakable_widths[i].is_some() {
                emit_line!();
                append_breakable_from(
                    i,
                    0,
                    &mut line_count,
                    &mut line_w,
                    &mut has_content,
                    &mut line_start_seg,
                    &mut line_start_graph,
                    &mut line_end_seg,
                    &mut line_end_graph,
                    &mut pending_break_seg,
                    &mut pending_break_paint_w,
                    on_line,
                );
                i += 1;
                continue;
            }

            emit_line!();
            continue;
        }

        append_segment!(i, w);
        if can_break_after(kind) {
            pending_break_seg = Some(i + 1);
            pending_break_paint_w = line_w - w;
        }
        i += 1;
    }

    if has_content {
        emit_line!();
    }
    line_count
}

pub fn walk_prepared_lines(
    prepared: &PreparedLineBreakData,
    max_width: f64,
    profile: &EngineProfile,
    mut on_line: Option<&mut dyn FnMut(&InternalLayoutLine)>,
) -> usize {
    if prepared.simple_line_walk_fast_path {
        return walk_prepared_lines_simple(prepared, max_width, profile, &mut on_line);
    }

    let widths = &prepared.widths;
    let line_end_fit_advances = &prepared.line_end_fit_advances;
    let line_end_paint_advances = &prepared.line_end_paint_advances;
    let kinds = &prepared.kinds;
    let breakable_widths = &prepared.breakable_widths;
    let breakable_prefix_widths = &prepared.breakable_prefix_widths;
    let discretionary_hyphen_width = prepared.discretionary_hyphen_width;
    let tab_stop_advance = prepared.tab_stop_advance;
    let chunks = &prepared.chunks;

    if widths.is_empty() || chunks.is_empty() {
        return 0;
    }

    let eps = profile.line_fit_epsilon;
    let mut line_count = 0usize;
    let mut line_w = 0.0f64;
    let mut has_content = false;
    let mut line_start_seg = 0usize;
    let mut line_start_graph = 0usize;
    let mut line_end_seg = 0usize;
    let mut line_end_graph = 0usize;
    let mut pending_break_seg: Option<usize> = None;
    let mut pending_break_fit_w = 0.0f64;
    let mut pending_break_paint_w = 0.0f64;
    let mut pending_break_kind: Option<SegmentBreakKind> = None;

    macro_rules! clear_pending {
        () => {{
            pending_break_seg = None;
            pending_break_fit_w = 0.0;
            pending_break_paint_w = 0.0;
            pending_break_kind = None;
        }};
    }

    macro_rules! emit_line {
        ($end_seg:expr, $end_graph:expr, $w:expr) => {{
            line_count += 1;
            if let Some(cb) = on_line.as_mut() {
                cb(&InternalLayoutLine {
                    start_segment_index: line_start_seg,
                    start_grapheme_index: line_start_graph,
                    end_segment_index: $end_seg,
                    end_grapheme_index: $end_graph,
                    width: $w,
                });
            }
            line_w = 0.0;
            has_content = false;
            clear_pending!();
        }};
        () => {
            emit_line!(line_end_seg, line_end_graph, line_w)
        };
    }

    for chunk in chunks.as_ref() {
        if chunk.start_segment_index == chunk.end_segment_index {
            // Empty chunk (hard break only)
            line_count += 1;
            if let Some(cb) = on_line.as_mut() {
                cb(&InternalLayoutLine {
                    start_segment_index: chunk.start_segment_index,
                    start_grapheme_index: 0,
                    end_segment_index: chunk.consumed_end_segment_index,
                    end_grapheme_index: 0,
                    width: 0.0,
                });
            }
            clear_pending!();
            continue;
        }

        has_content = false;
        line_w = 0.0;
        line_start_seg = chunk.start_segment_index;
        line_start_graph = 0;
        line_end_seg = chunk.start_segment_index;
        line_end_graph = 0;
        clear_pending!();

        let mut i = chunk.start_segment_index;
        while i < chunk.end_segment_index {
            // Mirror normalize_line_start (incremental path): a fresh line
            // never starts with a collapsible/break-opportunity segment.
            // Reaching the chunk end this way contributes no line, exactly as
            // the incremental path continues past the chunk.
            if !has_content {
                while i < chunk.end_segment_index && skipped_at_fresh_line_start(kinds[i]) {
                    i += 1;
                }
                if i >= chunk.end_segment_index {
                    break;
                }
            }
            let kind = kinds[i];
            let w = if kind == SegmentBreakKind::Tab {
                get_tab_advance(line_w, tab_stop_advance)
            } else {
                widths[i]
            };

            if kind == SegmentBreakKind::SoftHyphen {
                if has_content {
                    line_end_seg = i + 1;
                    line_end_graph = 0;
                    pending_break_seg = Some(i + 1);
                    pending_break_fit_w = line_w + discretionary_hyphen_width;
                    pending_break_paint_w = line_w + discretionary_hyphen_width;
                    pending_break_kind = Some(kind);
                }
                i += 1;
                continue;
            }

            if !has_content {
                if w > max_width && breakable_widths[i].is_some() {
                    // appendBreakableSegment inline
                    let g_widths = breakable_widths[i].as_ref().unwrap();
                    let g_prefix = breakable_prefix_widths[i].as_deref();
                    for g in 0..g_widths.len() {
                        let gw = get_breakable_advance(
                            g_widths,
                            g_prefix,
                            g,
                            profile.prefer_prefix_widths_for_breakable_runs,
                        );
                        if !has_content {
                            has_content = true;
                            line_start_seg = i;
                            line_start_graph = g;
                            line_end_seg = i;
                            line_end_graph = g + 1;
                            line_w = gw;
                        } else if line_w + gw > max_width + eps {
                            emit_line!();
                            has_content = true;
                            line_start_seg = i;
                            line_start_graph = g;
                            line_end_seg = i;
                            line_end_graph = g + 1;
                            line_w = gw;
                        } else {
                            line_w += gw;
                            line_end_seg = i;
                            line_end_graph = g + 1;
                        }
                    }
                    if has_content && line_end_seg == i && line_end_graph == g_widths.len() {
                        line_end_seg = i + 1;
                        line_end_graph = 0;
                    }
                } else {
                    has_content = true;
                    line_start_seg = i;
                    line_start_graph = 0;
                    line_end_seg = i + 1;
                    line_end_graph = 0;
                    line_w = w;
                }
                // update pending break
                if can_break_after(kind) {
                    let fit_adv = if kind == SegmentBreakKind::Tab {
                        0.0
                    } else {
                        line_end_fit_advances[i]
                    };
                    let paint_adv = if kind == SegmentBreakKind::Tab {
                        w
                    } else {
                        line_end_paint_advances[i]
                    };
                    pending_break_seg = Some(i + 1);
                    pending_break_fit_w = line_w - w + fit_adv;
                    pending_break_paint_w = line_w - w + paint_adv;
                    pending_break_kind = Some(kind);
                }
                i += 1;
                continue;
            }

            let new_w = line_w + w;
            if new_w > max_width + eps {
                let current_fit_w = line_w
                    + if kind == SegmentBreakKind::Tab {
                        0.0
                    } else {
                        line_end_fit_advances[i]
                    };
                let current_paint_w = line_w
                    + if kind == SegmentBreakKind::Tab {
                        w
                    } else {
                        line_end_paint_advances[i]
                    };

                // Early soft-hyphen break
                if pending_break_kind == Some(SegmentBreakKind::SoftHyphen)
                    && profile.prefer_early_soft_hyphen_break
                    && pending_break_fit_w <= max_width + eps
                    && let Some(pb_seg) = pending_break_seg
                {
                    emit_line!(pb_seg, 0, pending_break_paint_w);
                    continue;
                }

                // Soft-hyphen breakable continuation
                if pending_break_kind == Some(SegmentBreakKind::SoftHyphen)
                    && let Some(ref gw) = breakable_widths[i]
                {
                    let fit_widths = if profile.prefer_prefix_widths_for_breakable_runs {
                        breakable_prefix_widths[i].as_deref().unwrap_or(gw)
                    } else {
                        gw
                    };
                    let uses_prefix = fit_widths.as_ptr() != gw.as_ptr();
                    let (fit_count, fitted_width) = fit_soft_hyphen_break(
                        fit_widths,
                        line_w,
                        max_width,
                        eps,
                        discretionary_hyphen_width,
                        uses_prefix,
                    );
                    if fit_count > 0 {
                        line_w = fitted_width;
                        line_end_seg = i;
                        line_end_graph = fit_count;
                        clear_pending!();
                        if fit_count == gw.len() {
                            line_end_seg = i + 1;
                            line_end_graph = 0;
                            i += 1;
                            continue;
                        }
                        emit_line!(i, fit_count, fitted_width + discretionary_hyphen_width);
                        // continue from fit_count
                        let remaining_gw = breakable_widths[i].as_ref().unwrap();
                        let remaining_prefix = breakable_prefix_widths[i].as_deref();
                        for g in fit_count..remaining_gw.len() {
                            let gw_val = get_breakable_advance(
                                remaining_gw,
                                remaining_prefix,
                                g,
                                profile.prefer_prefix_widths_for_breakable_runs,
                            );
                            if !has_content {
                                has_content = true;
                                line_start_seg = i;
                                line_start_graph = g;
                                line_end_seg = i;
                                line_end_graph = g + 1;
                                line_w = gw_val;
                            } else if line_w + gw_val > max_width + eps {
                                emit_line!();
                                has_content = true;
                                line_start_seg = i;
                                line_start_graph = g;
                                line_end_seg = i;
                                line_end_graph = g + 1;
                                line_w = gw_val;
                            } else {
                                line_w += gw_val;
                                line_end_seg = i;
                                line_end_graph = g + 1;
                            }
                        }
                        if has_content && line_end_seg == i && line_end_graph == remaining_gw.len()
                        {
                            line_end_seg = i + 1;
                            line_end_graph = 0;
                        }
                        i += 1;
                        continue;
                    }
                }

                if can_break_after(kind) && current_fit_w <= max_width + eps {
                    line_w += w;
                    line_end_seg = i + 1;
                    line_end_graph = 0;
                    emit_line!(i + 1, 0, current_paint_w);
                    i += 1;
                    continue;
                }

                if let Some(pb_seg) = pending_break_seg
                    && pending_break_fit_w <= max_width + eps
                {
                    emit_line!(pb_seg, 0, pending_break_paint_w);
                    continue;
                }

                if w > max_width && breakable_widths[i].is_some() {
                    emit_line!();
                    let g_widths = breakable_widths[i].as_ref().unwrap();
                    let g_prefix = breakable_prefix_widths[i].as_deref();
                    for g in 0..g_widths.len() {
                        let gw_val = get_breakable_advance(
                            g_widths,
                            g_prefix,
                            g,
                            profile.prefer_prefix_widths_for_breakable_runs,
                        );
                        if !has_content {
                            has_content = true;
                            line_start_seg = i;
                            line_start_graph = g;
                            line_end_seg = i;
                            line_end_graph = g + 1;
                            line_w = gw_val;
                        } else if line_w + gw_val > max_width + eps {
                            emit_line!();
                            has_content = true;
                            line_start_seg = i;
                            line_start_graph = g;
                            line_end_seg = i;
                            line_end_graph = g + 1;
                            line_w = gw_val;
                        } else {
                            line_w += gw_val;
                            line_end_seg = i;
                            line_end_graph = g + 1;
                        }
                    }
                    if has_content && line_end_seg == i && line_end_graph == g_widths.len() {
                        line_end_seg = i + 1;
                        line_end_graph = 0;
                    }
                    i += 1;
                    continue;
                }

                emit_line!();
                continue;
            }

            // Fits: append
            line_w += w;
            line_end_seg = i + 1;
            line_end_graph = 0;
            if can_break_after(kind) {
                let fit_adv = if kind == SegmentBreakKind::Tab {
                    0.0
                } else {
                    line_end_fit_advances[i]
                };
                let paint_adv = if kind == SegmentBreakKind::Tab {
                    w
                } else {
                    line_end_paint_advances[i]
                };
                pending_break_seg = Some(i + 1);
                pending_break_fit_w = line_w - w + fit_adv;
                pending_break_paint_w = line_w - w + paint_adv;
                pending_break_kind = Some(kind);
            }
            i += 1;
        }

        if has_content {
            let final_paint_w = if pending_break_seg == Some(chunk.consumed_end_segment_index) {
                pending_break_paint_w
            } else {
                line_w
            };
            emit_line!(chunk.consumed_end_segment_index, 0, final_paint_w);
        }
    }

    line_count
}

/// Optimal breaking for multi-chunk (hard-break) paragraphs.
///
/// Solves each chunk independently, buffering by-value [`InternalLayoutLine`]s
/// (no per-line clone) and replaying them only after every chunk has an
/// optimal solution; a single infeasible chunk falls back to greedy for the
/// whole paragraph. The buffer is capacity-reserved upfront so steady-state
/// walks perform at most one allocation.
fn walk_optimal_multi_chunk(
    prepared: &PreparedLineBreakData,
    max_width: f64,
    profile: &EngineProfile,
    params: &KnuthPlassParams,
    mut on_line: Option<&mut dyn FnMut(&InternalLayoutLine)>,
) -> usize {
    let mut total_lines = 0;
    // Do not invoke the caller until every chunk has an optimal solution.
    // A later fallback must restart greedily from the beginning.
    // Reserve ~2 lines per chunk (paragraph + overflow) to avoid regrowth.
    let mut pending_lines: Vec<InternalLayoutLine> =
        Vec::with_capacity(prepared.chunks.len().saturating_mul(2).max(2));

    for chunk in prepared.chunks.as_ref() {
        if chunk.start_segment_index == chunk.end_segment_index {
            // Empty chunk (hard break only).
            total_lines += 1;
            pending_lines.push(InternalLayoutLine {
                start_segment_index: chunk.start_segment_index,
                start_grapheme_index: 0,
                end_segment_index: chunk.consumed_end_segment_index,
                end_grapheme_index: 0,
                width: 0.0,
            });
            continue;
        }

        // Build a zero-copy view over this chunk's segment range.
        let chunk_start = chunk.start_segment_index;
        let chunk_end = chunk.end_segment_index;
        let sub_prepared = prepared.slice(chunk_start, chunk_end, chunk.consumed_end_segment_index);

        let items = build_kp_items(&sub_prepared, profile, params);

        let result = knuth_plass_chunk(&items, max_width, params, params.tolerance).or_else(|| {
            if params.looseness_recovery {
                // Retry with progressively larger tolerance.
                for extra in [2.0, 4.0, 8.0, 16.0] {
                    let result =
                        knuth_plass_chunk(&items, max_width, params, params.tolerance + extra);
                    if result.is_some() {
                        return result;
                    }
                }
            }
            None
        });

        match result {
            Some(mut breaks) => {
                let mut collect = |line: &InternalLayoutLine| pending_lines.push(*line);
                let mut on_pending_line: Option<&mut dyn FnMut(&InternalLayoutLine)> =
                    Some(&mut collect);
                // Remap chunk-local breakpoints to global coordinates in
                // place (no per-chunk allocation), then point the last break
                // at the chunk's consumed end.
                breaks.remap_segments(chunk_start);
                breaks.set_last_break((chunk.consumed_end_segment_index, 0));

                total_lines +=
                    breakpoints_to_lines(prepared, profile, &breaks, &mut on_pending_line);
            }
            None => {
                // Fall back to greedy for the entire paragraph.
                return walk_prepared_lines(prepared, max_width, profile, on_line);
            }
        }
    }

    if let Some(cb) = on_line.as_mut() {
        for line in &pending_lines {
            cb(line);
        }
    }
    total_lines
}

/// Run Knuth-Plass optimal line breaking on prepared data.
///
/// Returns the number of lines. If `on_line` is provided, calls it for each line.
/// Falls back to greedy if Knuth-Plass finds no feasible solution.
///
/// Strategy split: greedy counting lives in [`walk_prepared_lines`] (which
/// itself delegates to the `_simple` fast path when possible); this function
/// only runs the optimal solver, with per-chunk work in
/// `walk_optimal_multi_chunk` above.
pub fn walk_prepared_lines_optimal(
    prepared: &PreparedLineBreakData,
    max_width: f64,
    profile: &EngineProfile,
    params: &KnuthPlassParams,
    mut on_line: Option<&mut dyn FnMut(&InternalLayoutLine)>,
) -> usize {
    let widths = &prepared.widths;
    if widths.is_empty() {
        return 0;
    }

    // For multi-chunk data (hard breaks), process each chunk separately.
    if prepared.chunks.len() > 1 {
        return walk_optimal_multi_chunk(prepared, max_width, profile, params, on_line);
    }
    // Single chunk: run KP directly.
    let items = build_kp_items(prepared, profile, params);

    let result =
        knuth_plass_chunk(items.as_ref(), max_width, params, params.tolerance).or_else(|| {
            if params.looseness_recovery {
                for extra in [2.0, 4.0, 8.0, 16.0] {
                    let result = knuth_plass_chunk(
                        items.as_ref(),
                        max_width,
                        params,
                        params.tolerance + extra,
                    );
                    if result.is_some() {
                        return result;
                    }
                }
            }
            None
        });

    match result {
        Some(breaks) => breakpoints_to_lines(prepared, profile, &breaks, &mut on_line),
        None => {
            // Fall back to greedy.
            walk_prepared_lines(prepared, max_width, profile, on_line)
        }
    }
}
