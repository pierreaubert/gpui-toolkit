use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use super::compute::compute_adjustment_ratio;
use super::compute::compute_demerits;
use super::fitness_class::FitnessClass;
use super::get::get_breakable_advance;
use super::get::get_tab_advance;
use super::knuth_plass_params::KnuthPlassParams;
use super::misc::badness;
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
use crate::layout::KpItemCacheKey;
use crate::measurement::EngineProfile;

thread_local! {
    /// Reusable scratch buffer for Knuth-Plass active nodes.
    static KP_NODES_SCRATCH: RefCell<Vec<ActiveNode>> = const { RefCell::new(Vec::new()) };
    /// Reusable scratch buffer for the active-node index set in Knuth-Plass.
    static KP_ACTIVE_SCRATCH: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    /// Reusable scratch buffer for Knuth-Plass breakpoints.
    static KP_BREAKS_SCRATCH: RefCell<Vec<(usize, usize)>> = const { RefCell::new(Vec::new()) };
    /// Reusable scratch buffer for indices deactivated during a Knuth-Plass pass.
    static KP_DEACTIVATE_SCRATCH: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

pub(crate) type KpItemCache<'a> = Option<&'a RefCell<HashMap<KpItemCacheKey, Arc<[KPItem]>>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineBreakCursor {
    pub segment_index: usize,
    pub grapheme_index: usize,
}

/// Zero-copy view over prepared line-break data.
///
/// Uses `Cow` so production paths can borrow from the prepared-text core
/// while tests can construct owned fixtures cheaply.
#[derive(Debug, Clone)]
pub struct PreparedLineBreakData<'a> {
    pub widths: Cow<'a, [f64]>,
    pub line_end_fit_advances: Cow<'a, [f64]>,
    pub line_end_paint_advances: Cow<'a, [f64]>,
    pub kinds: Cow<'a, [SegmentBreakKind]>,
    pub simple_line_walk_fast_path: bool,
    pub breakable_widths: Cow<'a, [Option<Arc<[f64]>>]>,
    pub breakable_prefix_widths: Cow<'a, [Option<Arc<[f64]>>]>,
    pub discretionary_hyphen_width: f64,
    pub tab_stop_advance: f64,
    pub chunks: Cow<'a, [PreparedLineChunk]>,
    /// Global segment range that this view covers in the underlying
    /// the prepared-text core. Used to key caches.
    pub(crate) start_segment: usize,
    pub(crate) end_segment: usize,
    /// Optional cache for Knuth-Plass breakpoint candidate lists. This is
    /// populated by production paths and left as `None` in manually-constructed
    /// test fixtures.
    pub(crate) kp_item_cache: KpItemCache<'a>,
}

impl<'a> PreparedLineBreakData<'a> {
    /// Return a view over a sub-range of segments, remapping the chunk list to a
    /// single chunk covering the range.
    ///
    /// When the slice covers the whole view, the existing chunk list is already
    /// in the correct local coordinate space and can be borrowed without a fresh
    /// `Vec` allocation.
    pub(super) fn slice(
        &self,
        start: usize,
        end: usize,
        consumed_end: usize,
    ) -> PreparedLineBreakData<'_> {
        // When the slice covers the entire view, the existing chunk list is
        // already in the correct local coordinate space and can be borrowed.
        let chunks: Cow<'_, [PreparedLineChunk]> =
            if start == 0 && end == self.widths.len() && consumed_end == self.end_segment {
                Cow::Borrowed(&self.chunks)
            } else {
                Cow::Owned(vec![PreparedLineChunk {
                    start_segment_index: 0,
                    end_segment_index: end - start,
                    consumed_end_segment_index: consumed_end - start,
                }])
            };

        PreparedLineBreakData {
            widths: Cow::Borrowed(&self.widths[start..end]),
            line_end_fit_advances: Cow::Borrowed(&self.line_end_fit_advances[start..end]),
            line_end_paint_advances: Cow::Borrowed(&self.line_end_paint_advances[start..end]),
            kinds: Cow::Borrowed(&self.kinds[start..end]),
            simple_line_walk_fast_path: false,
            breakable_widths: Cow::Borrowed(&self.breakable_widths[start..end]),
            breakable_prefix_widths: Cow::Borrowed(&self.breakable_prefix_widths[start..end]),
            discretionary_hyphen_width: self.discretionary_hyphen_width,
            tab_stop_advance: self.tab_stop_advance,
            chunks,
            start_segment: self.start_segment + start,
            end_segment: self.start_segment + end,
            kp_item_cache: self.kp_item_cache,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreparedLineChunk {
    pub start_segment_index: usize,
    pub end_segment_index: usize,
    pub consumed_end_segment_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InternalLayoutLine {
    pub start_segment_index: usize,
    pub start_grapheme_index: usize,
    pub end_segment_index: usize,
    pub end_grapheme_index: usize,
    pub width: f64,
}

pub(super) fn find_chunk_index_for_start(
    prepared: &PreparedLineBreakData,
    segment_index: usize,
) -> Option<usize> {
    prepared
        .chunks
        .iter()
        .position(|chunk| segment_index < chunk.consumed_end_segment_index)
}

pub fn normalize_line_start(
    prepared: &PreparedLineBreakData,
    start: LineBreakCursor,
) -> Option<LineBreakCursor> {
    let mut seg_idx = start.segment_index;

    if seg_idx >= prepared.widths.len() {
        return None;
    }
    if start.grapheme_index > 0 {
        return Some(start);
    }

    let chunk_idx = find_chunk_index_for_start(prepared, seg_idx)?;
    let chunk = &prepared.chunks[chunk_idx];

    if chunk.start_segment_index == chunk.end_segment_index && seg_idx == chunk.start_segment_index
    {
        return Some(LineBreakCursor {
            segment_index: seg_idx,
            grapheme_index: 0,
        });
    }

    if seg_idx < chunk.start_segment_index {
        seg_idx = chunk.start_segment_index;
    }

    while seg_idx < chunk.end_segment_index {
        let kind = prepared.kinds[seg_idx];
        if kind != SegmentBreakKind::Space
            && kind != SegmentBreakKind::ZeroWidthBreak
            && kind != SegmentBreakKind::SoftHyphen
        {
            return Some(LineBreakCursor {
                segment_index: seg_idx,
                grapheme_index: 0,
            });
        }
        seg_idx += 1;
    }

    if chunk.consumed_end_segment_index >= prepared.widths.len() {
        return None;
    }
    Some(LineBreakCursor {
        segment_index: chunk.consumed_end_segment_index,
        grapheme_index: 0,
    })
}

/// Strategy for line breaking.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum LineBreakStrategy {
    /// Greedy first-fit algorithm (fast, local decisions).
    #[default]
    Greedy,
    /// Knuth-Plass optimal algorithm (minimizes total demerits across the
    /// entire paragraph). Falls back to greedy if no feasible solution exists.
    Optimal,
}

/// A candidate breakpoint in the Knuth-Plass item list.
#[derive(Debug, Clone)]
pub(crate) struct KPItem {
    /// Segment index in the PreparedLineBreakData.
    pub(super) segment_index: usize,
    /// Grapheme index within the segment (0 for segment-level breaks).
    pub(super) grapheme_index: usize,
    /// Cumulative width of all content up to (not including) this break.
    pub(super) total_width: f64,
    /// Cumulative stretch (glue stretchability) up to this break.
    pub(super) total_stretch: f64,
    /// Cumulative shrink (glue shrinkability) up to this break.
    pub(super) total_shrink: f64,
    /// Penalty for breaking here. f64::INFINITY means "must not break",
    /// f64::NEG_INFINITY means "must break" (hard break / end of paragraph).
    pub(super) penalty: f64,
    /// Whether this break is "flagged" (e.g., a hyphenation break).
    pub(super) flagged: bool,
    /// Width added at this break position (e.g., hyphen width for soft hyphen).
    pub(super) break_width: f64,
    /// Width of trailing glue at this breakpoint (e.g., space width).
    /// Subtracted from the line width because trailing spaces are not
    /// rendered in CSS-like text layout.
    #[allow(dead_code)]
    pub(super) glue_width: f64,
    /// Stretch of the trailing glue at this breakpoint.
    #[allow(dead_code)]
    pub(super) glue_stretch: f64,
    /// Shrink of the trailing glue at this breakpoint.
    #[allow(dead_code)]
    pub(super) glue_shrink: f64,
}

/// An active node in the Knuth-Plass DP.
#[derive(Debug, Clone)]
pub(super) struct ActiveNode {
    /// Index into the breakpoint candidate list.
    pub(super) break_index: usize,
    /// Line number (0-based: this node starts line `line`).
    pub(super) line: usize,
    /// Fitness class of the line ending at this breakpoint.
    pub(super) fitness: FitnessClass,
    /// Total accumulated demerits up to this breakpoint.
    pub(super) total_demerits: f64,
    /// Index of the previous node in the optimal path (into `nodes` vec).
    pub(super) prev_node: Option<usize>,
}

/// Build the list of Knuth-Plass items (breakpoint candidates) from prepared data.
///
/// Each item represents a legal breakpoint with cumulative width/stretch/shrink.
///
/// The result is returned as an `Arc<[KPItem]>` and is cached on the
/// the prepared-text core when a cache is available,
/// avoiding recomputation on repeated optimal layouts of the same text with the
/// same parameters.
pub(super) fn build_kp_items(
    prepared: &PreparedLineBreakData,
    profile: &EngineProfile,
    params: &KnuthPlassParams,
) -> Arc<[KPItem]> {
    let widths = &prepared.widths;
    let kinds = &prepared.kinds;
    let breakable_widths = &prepared.breakable_widths;
    let breakable_prefix_widths = &prepared.breakable_prefix_widths;

    if widths.is_empty() {
        return Arc::new([]);
    }

    let cache_key = KpItemCacheKey {
        start_segment: prepared.start_segment,
        end_segment: prepared.end_segment,
        line_penalty: params.line_penalty.to_bits(),
        hyphen_penalty: params.hyphen_penalty.to_bits(),
        prefer_prefix_widths: profile.prefer_prefix_widths_for_breakable_runs,
    };

    if let Some(cache) = prepared.kp_item_cache
        && let Some(cached) = cache.borrow().get(&cache_key)
    {
        return Arc::clone(cached);
    }

    let mut items = Vec::new();

    // Start-of-paragraph sentinel (breakpoint at position 0).
    items.push(KPItem {
        segment_index: 0,
        grapheme_index: 0,
        total_width: 0.0,
        total_stretch: 0.0,
        total_shrink: 0.0,
        penalty: f64::NEG_INFINITY, // forced break (start)
        flagged: false,
        break_width: 0.0,
        glue_width: 0.0,
        glue_stretch: 0.0,
        glue_shrink: 0.0,
    });

    let mut cum_width = 0.0;
    let mut cum_stretch = 0.0;
    let mut cum_shrink = 0.0;
    let mut line_start_width = 0.0;
    let mut last_glue_width = 0.0;
    let mut last_glue_stretch = 0.0;
    let mut last_glue_shrink = 0.0;

    for i in 0..widths.len() {
        let kind = kinds[i];
        let w = if kind == SegmentBreakKind::Tab {
            // Use line-relative width rather than paragraph cumulative width
            // so that tabs on later lines advance correctly.
            get_tab_advance(cum_width - line_start_width, prepared.tab_stop_advance)
        } else {
            widths[i]
        };

        match kind {
            SegmentBreakKind::HardBreak => {
                // Must break here.
                items.push(KPItem {
                    segment_index: i + 1,
                    grapheme_index: 0,
                    total_width: cum_width,
                    total_stretch: cum_stretch,
                    total_shrink: cum_shrink,
                    penalty: f64::NEG_INFINITY,
                    flagged: false,
                    break_width: 0.0,
                    glue_width: 0.0,
                    glue_stretch: 0.0,
                    glue_shrink: 0.0,
                });
                line_start_width = cum_width;
                last_glue_width = 0.0;
                last_glue_stretch = 0.0;
                last_glue_shrink = 0.0;
            }
            SegmentBreakKind::Space | SegmentBreakKind::PreservedSpace => {
                // Glue: the space has width, stretch, and shrink.
                // Standard Knuth-Plass model: space_width with ±stretch/shrink.
                // We use 1/2 of space width for stretch and 1/3 for shrink
                // (approximation of TeX's interword spacing model).
                let stretch = w * 0.5;
                let shrink = w * 0.333;

                cum_width += w;
                cum_stretch += stretch;
                cum_shrink += shrink;

                // Break is legal *after* the space.
                items.push(KPItem {
                    segment_index: i + 1,
                    grapheme_index: 0,
                    total_width: cum_width,
                    total_stretch: cum_stretch,
                    total_shrink: cum_shrink,
                    penalty: params.line_penalty,
                    flagged: false,
                    break_width: 0.0,
                    glue_width: w,
                    glue_stretch: stretch,
                    glue_shrink: shrink,
                });
                last_glue_width = w;
                last_glue_stretch = stretch;
                last_glue_shrink = shrink;
            }
            SegmentBreakKind::ZeroWidthBreak => {
                // Zero-width break opportunity.
                items.push(KPItem {
                    segment_index: i + 1,
                    grapheme_index: 0,
                    total_width: cum_width,
                    total_stretch: cum_stretch,
                    total_shrink: cum_shrink,
                    penalty: params.line_penalty,
                    flagged: false,
                    break_width: 0.0,
                    glue_width: 0.0,
                    glue_stretch: 0.0,
                    glue_shrink: 0.0,
                });
                last_glue_width = 0.0;
                last_glue_stretch = 0.0;
                last_glue_shrink = 0.0;
            }
            SegmentBreakKind::SoftHyphen => {
                // Discretionary hyphen: penalty for hyphenation, flagged.
                items.push(KPItem {
                    segment_index: i + 1,
                    grapheme_index: 0,
                    total_width: cum_width,
                    total_stretch: cum_stretch,
                    total_shrink: cum_shrink,
                    penalty: params.hyphen_penalty,
                    flagged: true,
                    break_width: prepared.discretionary_hyphen_width,
                    glue_width: 0.0,
                    glue_stretch: 0.0,
                    glue_shrink: 0.0,
                });
                last_glue_width = 0.0;
                last_glue_stretch = 0.0;
                last_glue_shrink = 0.0;
            }
            SegmentBreakKind::Tab => {
                cum_width += w;
                let stretch = w * 0.5;
                let shrink = w * 0.333;
                cum_stretch += stretch;
                cum_shrink += shrink;

                // Breakable after tab.
                items.push(KPItem {
                    segment_index: i + 1,
                    grapheme_index: 0,
                    total_width: cum_width,
                    total_stretch: cum_stretch,
                    total_shrink: cum_shrink,
                    penalty: params.line_penalty,
                    flagged: false,
                    break_width: 0.0,
                    glue_width: w,
                    glue_stretch: stretch,
                    glue_shrink: shrink,
                });
                last_glue_width = w;
                last_glue_stretch = stretch;
                last_glue_shrink = shrink;
            }
            SegmentBreakKind::Text | SegmentBreakKind::Glue => {
                // For CJK text segments that were split into per-grapheme units
                // during preparation, each unit is a separate segment, so
                // breakpoints between CJK characters are already handled as
                // segment boundaries with Text kind. We add a breakpoint
                // candidate after each such segment (CJK characters can break
                // between any pair).

                // Check if this segment has grapheme-level breakability
                // (overflow-wrap: break-word for long words).
                if let Some(ref gw) = breakable_widths[i] {
                    let gp = breakable_prefix_widths[i].as_deref();
                    for g in 0..gw.len() {
                        let gw_val = get_breakable_advance(
                            gw,
                            gp,
                            g,
                            profile.prefer_prefix_widths_for_breakable_runs,
                        );
                        cum_width += gw_val;

                        // Each grapheme boundary within a breakable word is a
                        // break candidate with high penalty (emergency breaks).
                        if g + 1 < gw.len() {
                            items.push(KPItem {
                                segment_index: i,
                                grapheme_index: g + 1,
                                total_width: cum_width,
                                total_stretch: cum_stretch,
                                total_shrink: cum_shrink,
                                penalty: 1000.0, // very high penalty: emergency only
                                flagged: false,
                                break_width: 0.0,
                                glue_width: 0.0,
                                glue_stretch: 0.0,
                                glue_shrink: 0.0,
                            });
                        }
                    }
                } else {
                    cum_width += w;

                    // Add a segment-level breakpoint for adjacent Text segments.
                    // This is needed for CJK text which is split into per-grapheme
                    // units during prepare — without these breakpoints, KP cannot
                    // break between CJK characters.
                    if kind == SegmentBreakKind::Text
                        && i + 1 < widths.len()
                        && kinds[i + 1] == SegmentBreakKind::Text
                    {
                        items.push(KPItem {
                            segment_index: i + 1,
                            grapheme_index: 0,
                            total_width: cum_width,
                            total_stretch: cum_stretch,
                            total_shrink: cum_shrink,
                            penalty: 100.0, // moderate penalty: prefer word breaks
                            flagged: false,
                            break_width: 0.0,
                            glue_width: 0.0,
                            glue_stretch: 0.0,
                            glue_shrink: 0.0,
                        });
                    }
                }

                last_glue_width = 0.0;
                last_glue_stretch = 0.0;
                last_glue_shrink = 0.0;
            }
        }
    }

    // End-of-paragraph: forced break at the end.
    items.push(KPItem {
        segment_index: widths.len(),
        grapheme_index: 0,
        total_width: cum_width,
        total_stretch: cum_stretch,
        total_shrink: cum_shrink,
        penalty: f64::NEG_INFINITY,
        flagged: false,
        break_width: 0.0,
        glue_width: last_glue_width,
        glue_stretch: last_glue_stretch,
        glue_shrink: last_glue_shrink,
    });

    let items: Arc<[KPItem]> = items.into();
    if let Some(cache) = prepared.kp_item_cache {
        cache.borrow_mut().insert(cache_key, Arc::clone(&items));
    }
    items
}

/// Run the Knuth-Plass algorithm on a single chunk of prepared data.
///
/// Returns the chosen breakpoints as (segment_index, grapheme_index) pairs,
/// or None if no feasible solution exists.
pub(super) fn knuth_plass_chunk(
    items: &[KPItem],
    max_width: f64,
    params: &KnuthPlassParams,
    tolerance: f64,
) -> Option<Vec<(usize, usize)>> {
    if items.len() < 2 {
        return Some(Vec::new());
    }

    KP_NODES_SCRATCH.with(|nodes_scratch| {
        KP_ACTIVE_SCRATCH.with(|active_scratch| {
            KP_BREAKS_SCRATCH.with(|breaks_scratch| {
                KP_DEACTIVATE_SCRATCH.with(|deactivate_scratch| {
                    let mut nodes = nodes_scratch.borrow_mut();
                    let mut active = active_scratch.borrow_mut();
                    let mut breaks = breaks_scratch.borrow_mut();
                    let mut to_deactivate = deactivate_scratch.borrow_mut();

                    nodes.clear();
                    active.clear();
                    breaks.clear();
                    to_deactivate.clear();

                    // Initialize with the start-of-paragraph node (items[0]).
                    nodes.push(ActiveNode {
                        break_index: 0,
                        line: 0,
                        fitness: FitnessClass::Normal,
                        total_demerits: 0.0,
                        prev_node: None,
                    });
                    active.push(0);

                    // Process each candidate breakpoint.
                    for b in 1..items.len() {
                        let item = &items[b];

                        // Skip non-breakable items (infinite penalty, not forced).
                        if item.penalty == f64::INFINITY {
                            continue;
                        }

                        // Best candidates for each fitness class at this breakpoint.
                        let mut best: [Option<(f64, usize)>; 4] = [None; 4]; // (demerits, node_index)

                        to_deactivate.clear();

                        for (ai, &node_idx) in active.iter().enumerate() {
                            let node = &nodes[node_idx];

                            let ratio = compute_adjustment_ratio(
                                items,
                                node.break_index,
                                b,
                                max_width,
                                item.break_width,
                            );

                            // If the line is too long even with maximum shrink, deactivate.
                            if ratio < -1.0 {
                                to_deactivate.push(ai);
                            }

                            // Check feasibility.
                            if ratio < -1.0 || ratio > tolerance {
                                // For forced breaks (neg infinity penalty), we must still
                                // consider even if ratio exceeds tolerance — the line might
                                // just be short (positive ratio), which is acceptable for
                                // paragraph-ending lines.
                                if item.penalty == f64::NEG_INFINITY && ratio >= -1.0 {
                                    // Allow forced breaks with short lines.
                                } else {
                                    continue;
                                }
                            }

                            // Compute demerits.
                            let bad = badness(ratio);
                            let fitness = FitnessClass::from_ratio(ratio);
                            let demerits = compute_demerits(
                                params,
                                item.penalty,
                                bad,
                                items[node.break_index].flagged,
                                item.flagged,
                                node.fitness,
                                fitness,
                            );
                            let total_d = node.total_demerits + demerits;

                            let class_idx = fitness as usize;
                            match best[class_idx] {
                                Some((best_d, _)) if total_d >= best_d => {}
                                _ => {
                                    best[class_idx] = Some((total_d, node_idx));
                                }
                            }
                        }

                        // Remove deactivated nodes (iterate in reverse to preserve indices).
                        for &ai in to_deactivate.iter().rev() {
                            active.swap_remove(ai);
                        }

                        // Add new active nodes for the best candidates.
                        for (class_idx, slot) in best.iter().enumerate() {
                            if let Some((total_d, prev_idx)) = *slot {
                                let fitness = match class_idx {
                                    0 => FitnessClass::Tight,
                                    1 => FitnessClass::Normal,
                                    2 => FitnessClass::Loose,
                                    _ => FitnessClass::VeryLoose,
                                };
                                let new_idx = nodes.len();
                                let prev_line = nodes[prev_idx].line;
                                nodes.push(ActiveNode {
                                    break_index: b,
                                    line: prev_line + 1,
                                    fitness,
                                    total_demerits: total_d,
                                    prev_node: Some(prev_idx),
                                });
                                active.push(new_idx);
                            }
                        }

                        // If active set is empty, no feasible solution found.
                        if active.is_empty() {
                            return None;
                        }
                    }

                    // Find the best node at the end of paragraph (last item).
                    // Only nodes that reach the end-of-paragraph are valid; otherwise
                    // trailing text would be silently dropped.
                    let last_item_idx = items.len() - 1;
                    let best_node_idx = active
                        .iter()
                        .filter(|&&idx| nodes[idx].break_index == last_item_idx)
                        .min_by(|&&a, &&b| {
                            nodes[a]
                                .total_demerits
                                .partial_cmp(&nodes[b].total_demerits)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .copied()?;

                    // Trace back the path to collect breakpoints.
                    let mut cur = Some(best_node_idx);
                    while let Some(idx) = cur {
                        let node = &nodes[idx];
                        let item = &items[node.break_index];
                        breaks.push((item.segment_index, item.grapheme_index));
                        cur = node.prev_node;
                    }
                    breaks.reverse();

                    // The first break is the start-of-paragraph (segment 0, grapheme 0),
                    // and the last is end-of-paragraph. We include both — callers interpret
                    // consecutive pairs as line ranges.
                    Some(breaks.clone())
                })
            })
        })
    })
}

/// Convert Knuth-Plass breakpoints into InternalLayoutLine entries.
///
/// `breaks` contains (segment_index, grapheme_index) pairs representing
/// the start of each line and the end of the last line.
pub(super) fn breakpoints_to_lines(
    prepared: &PreparedLineBreakData,
    profile: &EngineProfile,
    breaks: &[(usize, usize)],
    on_line: &mut Option<&mut dyn FnMut(&InternalLayoutLine)>,
) -> usize {
    if breaks.len() < 2 {
        return 0;
    }

    let widths = &prepared.widths;
    let kinds = &prepared.kinds;
    let line_end_paint_advances = &prepared.line_end_paint_advances;
    let breakable_widths = &prepared.breakable_widths;
    let breakable_prefix_widths = &prepared.breakable_prefix_widths;
    let mut line_count = 0;

    for pair in breaks.windows(2) {
        let (start_seg, start_graph) = pair[0];
        let (end_seg, end_graph) = pair[1];

        // Skip empty ranges at the end.
        if start_seg >= widths.len() && end_seg >= widths.len() {
            continue;
        }

        // Normalize start: skip leading spaces.
        let mut actual_start_seg = start_seg;
        let mut actual_start_graph = start_graph;
        if actual_start_graph == 0 {
            while actual_start_seg < end_seg.min(widths.len()) {
                let k = kinds[actual_start_seg];
                if k == SegmentBreakKind::Space
                    || k == SegmentBreakKind::ZeroWidthBreak
                    || k == SegmentBreakKind::SoftHyphen
                {
                    actual_start_seg += 1;
                } else {
                    break;
                }
            }
        }

        // Compute line width.
        let mut line_w = 0.0;
        let mut seg = actual_start_seg;
        let g_start = actual_start_graph;

        while seg < end_seg.min(widths.len()) {
            let kind = kinds[seg];
            // Partial end segments are handled after the loop.
            if kind == SegmentBreakKind::SoftHyphen || kind == SegmentBreakKind::HardBreak {
                seg += 1;
                continue;
            }

            if seg == actual_start_seg && g_start > 0 {
                // Partial segment from grapheme g_start.
                if let Some(ref gw) = breakable_widths[seg] {
                    let gp = breakable_prefix_widths[seg].as_deref();
                    for g in g_start..gw.len() {
                        line_w += get_breakable_advance(
                            gw,
                            gp,
                            g,
                            profile.prefer_prefix_widths_for_breakable_runs,
                        );
                    }
                } else {
                    line_w += widths[seg];
                }
            } else {
                // Check if this is a trailing space — use paint advance.
                if (kind == SegmentBreakKind::Space || kind == SegmentBreakKind::PreservedSpace)
                    && seg + 1 >= end_seg
                {
                    line_w += line_end_paint_advances[seg];
                } else {
                    line_w += widths[seg];
                }
            }
            seg += 1;
        }

        // Add partial end segment (grapheme-level break).
        if end_graph > 0
            && end_seg < widths.len()
            && let Some(ref gw) = breakable_widths[end_seg]
        {
            let gp = breakable_prefix_widths[end_seg].as_deref();
            let skip = if actual_start_seg == end_seg {
                actual_start_graph
            } else {
                0
            };
            for g in skip..end_graph {
                line_w += get_breakable_advance(
                    gw,
                    gp,
                    g,
                    profile.prefer_prefix_widths_for_breakable_runs,
                );
            }
        }

        // Check for discretionary hyphen at line end.
        if end_seg > 0
            && end_seg <= widths.len()
            && end_graph == 0
            && kinds[end_seg - 1] == SegmentBreakKind::SoftHyphen
        {
            line_w += prepared.discretionary_hyphen_width;
        }

        line_count += 1;
        if let Some(cb) = on_line.as_mut() {
            cb(&InternalLayoutLine {
                start_segment_index: actual_start_seg,
                start_grapheme_index: actual_start_graph,
                end_segment_index: end_seg,
                end_grapheme_index: end_graph,
                width: line_w,
            });
        }
    }

    line_count
}

#[cfg(test)]
pub(super) fn knuth_plass_chunk_breaks_scratch_capacity() -> usize {
    KP_BREAKS_SCRATCH.with(|s| s.borrow().capacity())
}
