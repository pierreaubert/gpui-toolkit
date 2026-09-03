//! Big-data downsampling for high-cardinality 1D series.
//!
//! Interactive line and scatter frames encode one GPU point per datum. Past
//! ~10k points the per-frame geometry dominates frame time while adding no
//! visible detail on typical viewports, so render paths decimate to a fixed
//! point budget before encoding geometry:
//!
//! - [`lttb_indices`] — Largest-Triangle-Three-Buckets downsampling for
//!   ordered line data. Preserves visual shape (peaks, valleys, slopes).
//! - [`min_max_indices`] — per-bucket min/max decimation for unordered
//!   scatter data. Preserves the vertical envelope of every bucket.
//! - [`decimate_line_points`] / [`decimate_scatter_points`] — thin wrappers
//!   applied by `LineChart::build` and `ScatterChart::build`. Inputs at or
//!   below [`DECIMATION_THRESHOLD`] are returned untouched (same `Arc`), so
//!   small charts pay no copy.
//!
//! Static SVG export (`to_svg`) keeps full resolution; decimation is a
//! render-time optimization only.

use d3rs::shape::{LinePoint, ScatterPoint};
use std::sync::Arc;

/// Point count above which interactive render paths decimate before encoding
/// geometry.
pub const DECIMATION_THRESHOLD: usize = 10_000;

/// Maximum points kept per series after decimation.
pub const DECIMATION_BUDGET: usize = 4_000;

/// Largest-Triangle-Three-Buckets downsampling.
///
/// Returns the sorted indices of at most `budget` points that best preserve
/// the polyline shape. The first and last points are always kept. Requires
/// `x.len() == y.len()`; mismatched or empty inputs yield an empty vector.
/// Non-finite coordinates contribute zero triangle area, so they are never
/// *preferred* but endpoints are still preserved.
///
/// Falls back to uniform stride when `budget < 3`.
pub fn lttb_indices(x: &[f64], y: &[f64], budget: usize) -> Vec<usize> {
    let count = x.len().min(y.len());
    if count == 0 || x.len() != y.len() {
        return Vec::new();
    }
    if budget >= count {
        return (0..count).collect();
    }
    if budget < 3 {
        return uniform_stride_indices(count, budget);
    }

    let bucket_width = (count - 2) as f64 / (budget - 2) as f64;
    let mut selected = Vec::with_capacity(budget);
    selected.push(0);
    let mut prev = 0usize;

    for bucket in 0..budget - 2 {
        let range_start = 1 + (bucket as f64 * bucket_width).floor() as usize;
        let range_end = (1 + ((bucket + 1) as f64 * bucket_width).floor() as usize)
            .max(range_start + 1)
            .min(count);

        // Average of the *next* bucket anchors the triangle base.
        let next_start = 1 + ((bucket + 1) as f64 * bucket_width).floor() as usize;
        let next_end = (1 + ((bucket + 2) as f64 * bucket_width).floor() as usize)
            .max(next_start + 1)
            .min(count);
        let mut avg_x = 0.0;
        let mut avg_y = 0.0;
        let mut avg_n = 0usize;
        for index in next_start..next_end {
            avg_x += x[index];
            avg_y += y[index];
            avg_n += 1;
        }
        if avg_n > 0 {
            avg_x /= avg_n as f64;
            avg_y /= avg_n as f64;
        } else {
            avg_x = x[count - 1];
            avg_y = y[count - 1];
        }

        let (prev_x, prev_y) = (x[prev], y[prev]);
        let mut best = range_start;
        let mut best_area = -1.0f64;
        for index in range_start..range_end {
            let area = ((prev_x - avg_x) * (y[index] - prev_y)
                - (prev_x - x[index]) * (avg_y - prev_y))
                .abs()
                * 0.5;
            let area = if area.is_finite() { area } else { 0.0 };
            if area > best_area {
                best_area = area;
                best = index;
            }
        }
        selected.push(best);
        prev = best;
    }
    selected.push(count - 1);
    selected
}

/// Per-bucket min/max decimation for unordered (scatter) data.
///
/// Splits `y` into `budget / 2` buckets and keeps the min and max sample of
/// each bucket, preserving the vertical envelope. Returns sorted, deduplicated
/// indices with at most `budget` entries. Empty input yields an empty vector.
pub fn min_max_indices(y: &[f64], budget: usize) -> Vec<usize> {
    let count = y.len();
    if count == 0 || budget == 0 {
        return Vec::new();
    }
    if budget >= count {
        return (0..count).collect();
    }
    if budget < 2 {
        return uniform_stride_indices(count, budget);
    }

    let buckets = budget / 2;
    let bucket_width = count as f64 / buckets as f64;
    let mut selected = Vec::with_capacity(budget);
    for bucket in 0..buckets {
        let start = (bucket as f64 * bucket_width).floor() as usize;
        let end =
            ((((bucket + 1) as f64 * bucket_width).floor() as usize).max(start + 1)).min(count);
        let mut min_index = start;
        let mut max_index = start;
        for index in start..end {
            // NaN never wins a comparison, so non-finite samples are skipped
            // in favor of finite extremes when a bucket has any.
            if y[index].is_finite() {
                if !y[min_index].is_finite() || y[index] < y[min_index] {
                    min_index = index;
                }
                if !y[max_index].is_finite() || y[index] > y[max_index] {
                    max_index = index;
                }
            }
        }
        selected.push(min_index);
        if max_index != min_index {
            selected.push(max_index);
        }
    }
    selected.sort_unstable();
    selected.dedup();
    selected.truncate(budget);
    selected
}

/// Uniform-stride fallback used for degenerate budgets.
fn uniform_stride_indices(count: usize, budget: usize) -> Vec<usize> {
    if count == 0 || budget == 0 {
        return Vec::new();
    }
    if budget >= count {
        return (0..count).collect();
    }
    if budget == 1 {
        return vec![0];
    }
    let step = (count - 1) as f64 / (budget - 1) as f64;
    (0..budget)
        .map(|slot| ((slot as f64 * step).round() as usize).min(count - 1))
        .collect()
}

/// Decimate cached line points to [`DECIMATION_BUDGET`] when the series
/// exceeds [`DECIMATION_THRESHOLD`]. Smaller inputs return the same `Arc`
/// without copying.
pub fn decimate_line_points(points: &Arc<[LinePoint]>) -> Arc<[LinePoint]> {
    if points.len() <= DECIMATION_THRESHOLD {
        return Arc::clone(points);
    }
    let x: Vec<f64> = points.iter().map(|point| point.x).collect();
    let y: Vec<f64> = points.iter().map(|point| point.y).collect();
    lttb_indices(&x, &y, DECIMATION_BUDGET)
        .into_iter()
        .map(|index| points[index])
        .collect::<Vec<_>>()
        .into()
}

/// Decimate cached scatter points to [`DECIMATION_BUDGET`] when the series
/// exceeds [`DECIMATION_THRESHOLD`]. Smaller inputs return the same `Arc`
/// without copying.
pub fn decimate_scatter_points(points: &Arc<[ScatterPoint]>) -> Arc<[ScatterPoint]> {
    if points.len() <= DECIMATION_THRESHOLD {
        return Arc::clone(points);
    }
    let y: Vec<f64> = points.iter().map(|point| point.y).collect();
    min_max_indices(&y, DECIMATION_BUDGET)
        .into_iter()
        .map(|index| points[index])
        .collect::<Vec<_>>()
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lttb_preserves_endpoints_and_budget_on_large_data() {
        let x: Vec<f64> = (0..20_000).map(|value| value as f64).collect();
        let y: Vec<f64> = x.iter().map(|value| value.sin()).collect();
        let selected = lttb_indices(&x, &y, DECIMATION_BUDGET);
        assert_eq!(selected.len(), DECIMATION_BUDGET);
        assert_eq!(selected[0], 0);
        assert_eq!(selected[selected.len() - 1], x.len() - 1);
        assert!(selected.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn lttb_keeps_spikes_that_stride_would_drop() {
        // A single impulse in flat data must survive decimation.
        let count = 1_000;
        let x: Vec<f64> = (0..count).map(|value| value as f64).collect();
        let mut y = vec![0.0; count];
        y[count / 2] = 100.0;
        let selected = lttb_indices(&x, &y, 10);
        assert!(selected.contains(&(count / 2)));
    }

    #[test]
    fn lttb_handles_degenerate_inputs() {
        assert!(lttb_indices(&[], &[], 100).is_empty());
        assert!(lttb_indices(&[1.0], &[1.0, 2.0], 100).is_empty());
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0, 3.0];
        assert_eq!(lttb_indices(&x, &y, 100), vec![0, 1, 2]);
        assert!(lttb_indices(&x, &y, 0).is_empty());
        assert_eq!(lttb_indices(&x, &y, 1), vec![0]);
        assert_eq!(lttb_indices(&x, &y, 2).len(), 2);
    }

    #[test]
    fn min_max_preserves_envelope_per_bucket() {
        let count = 20_000;
        let y: Vec<f64> = (0..count)
            .map(|index| if index % 2 == 0 { 100.0 } else { -100.0 })
            .collect();
        let selected = min_max_indices(&y, DECIMATION_BUDGET);
        assert!(selected.len() <= DECIMATION_BUDGET);
        assert!(selected.windows(2).all(|pair| pair[0] < pair[1]));
        // Every bucket contributes both extremes, so both survive globally.
        assert!(selected.iter().any(|&index| y[index] == 100.0));
        assert!(selected.iter().any(|&index| y[index] == -100.0));
    }

    #[test]
    fn min_max_handles_degenerate_inputs() {
        assert!(min_max_indices(&[], 100).is_empty());
        assert!(min_max_indices(&[1.0, 2.0], 0).is_empty());
        assert_eq!(min_max_indices(&[1.0, 2.0, 3.0], 100), vec![0, 1, 2]);
        assert_eq!(min_max_indices(&[1.0, 2.0, 3.0], 1).len(), 1);
    }

    #[test]
    fn point_wrappers_are_identity_below_threshold_and_bounded_above() {
        let small: Arc<[LinePoint]> = (0..100)
            .map(|value| LinePoint::new(value as f64, value as f64))
            .collect::<Vec<_>>()
            .into();
        assert!(Arc::ptr_eq(&decimate_line_points(&small), &small));

        let big: Arc<[LinePoint]> = (0..DECIMATION_THRESHOLD + 1)
            .map(|value| LinePoint::new(value as f64, (value as f64).sin()))
            .collect::<Vec<_>>()
            .into();
        let decimated = decimate_line_points(&big);
        assert_eq!(decimated.len(), DECIMATION_BUDGET);
        assert_eq!(decimated[0].x, 0.0);

        let small_scatter: Arc<[ScatterPoint]> = (0..100)
            .map(|value| ScatterPoint::new(value as f64, value as f64))
            .collect::<Vec<_>>()
            .into();
        assert!(Arc::ptr_eq(
            &decimate_scatter_points(&small_scatter),
            &small_scatter
        ));

        let big_scatter: Arc<[ScatterPoint]> = (0..DECIMATION_THRESHOLD + 1)
            .map(|value| ScatterPoint::new(value as f64, (value % 7) as f64))
            .collect::<Vec<_>>()
            .into();
        assert!(decimate_scatter_points(&big_scatter).len() <= DECIMATION_BUDGET);
    }
}
