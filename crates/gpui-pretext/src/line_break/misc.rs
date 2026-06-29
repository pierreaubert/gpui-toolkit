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

pub(super) fn can_break_after(kind: SegmentBreakKind) -> bool {
    matches!(
        kind,
        SegmentBreakKind::Space
            | SegmentBreakKind::PreservedSpace
            | SegmentBreakKind::Tab
            | SegmentBreakKind::ZeroWidthBreak
            | SegmentBreakKind::SoftHyphen
    )
}

pub(super) fn is_simple_collapsible_space(kind: SegmentBreakKind) -> bool {
    kind == SegmentBreakKind::Space
}

pub(super) fn fit_soft_hyphen_break(
    grapheme_widths: &[f64],
    initial_width: f64,
    max_width: f64,
    line_fit_epsilon: f64,
    discretionary_hyphen_width: f64,
    cumulative_widths: bool,
) -> (usize, f64) {
    let mut fit_count = 0;
    let mut fitted_width = initial_width;

    while fit_count < grapheme_widths.len() {
        let next_width = if cumulative_widths {
            initial_width + grapheme_widths[fit_count]
        } else {
            fitted_width + grapheme_widths[fit_count]
        };
        let next_line_width = if fit_count + 1 < grapheme_widths.len() {
            next_width + discretionary_hyphen_width
        } else {
            next_width
        };
        if next_line_width > max_width + line_fit_epsilon {
            break;
        }
        fitted_width = next_width;
        fit_count += 1;
    }

    (fit_count, fitted_width)
}

/// Compute badness from adjustment ratio, following Knuth-Plass.
/// badness = 100 * |r|^3, capped at 10000 for infeasible.
pub(super) fn badness(ratio: f64) -> f64 {
    if ratio.is_infinite() {
        return 10000.0;
    }
    let b = 100.0 * ratio.abs().powi(3);
    b.min(10000.0)
}
