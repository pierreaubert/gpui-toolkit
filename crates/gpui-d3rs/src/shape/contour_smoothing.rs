#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StrokePoint {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

impl StrokePoint {
    pub(crate) fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    fn distance_to(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

pub(crate) fn split_stroke_segments(
    points: &[StrokePoint],
    x_jump_threshold: f32,
    y_jump_threshold: f32,
) -> Vec<Vec<StrokePoint>> {
    let mut segments = Vec::new();
    let mut current: Vec<StrokePoint> = Vec::new();
    let distance_jump_threshold =
        adaptive_distance_jump_threshold(points, x_jump_threshold, y_jump_threshold);

    for point in points {
        if let Some(previous) = current.last().copied()
            && is_jump(
                previous,
                *point,
                x_jump_threshold,
                y_jump_threshold,
                distance_jump_threshold,
            )
        {
            if current.len() >= 2 {
                segments.push(current);
            }
            current = Vec::new();
        }
        current.push(*point);
    }

    if current.len() >= 2 {
        segments.push(current);
    }

    segments
}

pub(crate) fn smooth_stroke_segment(
    points: &[StrokePoint],
    closed: bool,
    smooth_strokes: bool,
    smoothing_iterations: usize,
    smoothing_max_deviation_px: f32,
) -> Vec<StrokePoint> {
    if !smooth_strokes || points.len() < 3 || smoothing_iterations == 0 {
        return points.to_vec();
    }

    let mut smoothed = points.to_vec();
    for _ in 0..smoothing_iterations {
        smoothed = chaikin_iteration(&smoothed, closed);
    }

    if smoothing_max_deviation_px > 0.0
        && max_polyline_deviation(&smoothed, points, closed) > smoothing_max_deviation_px
    {
        points.to_vec()
    } else {
        smoothed
    }
}

fn adaptive_distance_jump_threshold(
    points: &[StrokePoint],
    x_jump_threshold: f32,
    y_jump_threshold: f32,
) -> f32 {
    let mut distances: Vec<_> = points
        .windows(2)
        .map(|pair| pair[0].distance_to(pair[1]))
        .filter(|distance| distance.is_finite() && *distance > 0.0)
        .collect();
    if distances.is_empty() {
        return x_jump_threshold.hypot(y_jump_threshold);
    }

    distances.sort_by(|a, b| a.total_cmp(b));
    let median = distances[distances.len() / 2];
    let viewport_threshold = x_jump_threshold.hypot(y_jump_threshold);
    (median * 8.0).max(24.0).min(viewport_threshold.max(24.0))
}

fn is_jump(
    previous: StrokePoint,
    point: StrokePoint,
    x_jump_threshold: f32,
    y_jump_threshold: f32,
    distance_jump_threshold: f32,
) -> bool {
    (point.x - previous.x).abs() > x_jump_threshold
        || (point.y - previous.y).abs() > y_jump_threshold
        || previous.distance_to(point) > distance_jump_threshold
}

fn chaikin_iteration(points: &[StrokePoint], closed: bool) -> Vec<StrokePoint> {
    if points.len() < 2 {
        return points.to_vec();
    }

    let mut result = Vec::with_capacity(points.len() * 2);
    if !closed {
        result.push(points[0]);
    }

    let pair_count = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    for i in 0..pair_count {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        result.push(lerp_point(a, b, 0.25));
        result.push(lerp_point(a, b, 0.75));
    }

    if !closed {
        result.push(points[points.len() - 1]);
    }

    result
}

fn lerp_point(a: StrokePoint, b: StrokePoint, t: f32) -> StrokePoint {
    StrokePoint::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

fn max_polyline_deviation(
    candidate: &[StrokePoint],
    original: &[StrokePoint],
    closed: bool,
) -> f32 {
    let candidate_to_original = candidate
        .iter()
        .map(|point| point_to_polyline_distance(*point, original, closed))
        .fold(0.0, f32::max);
    let original_to_candidate = original
        .iter()
        .map(|point| point_to_polyline_distance(*point, candidate, closed))
        .fold(0.0, f32::max);

    candidate_to_original.max(original_to_candidate)
}

fn point_to_polyline_distance(point: StrokePoint, polyline: &[StrokePoint], closed: bool) -> f32 {
    if polyline.len() < 2 {
        return 0.0;
    }

    let mut min_distance = f32::INFINITY;
    for pair in polyline.windows(2) {
        min_distance = min_distance.min(point_to_segment_distance(point, pair[0], pair[1]));
    }

    if closed {
        min_distance = min_distance.min(point_to_segment_distance(
            point,
            polyline[polyline.len() - 1],
            polyline[0],
        ));
    }

    min_distance
}

fn point_to_segment_distance(point: StrokePoint, start: StrokePoint, end: StrokePoint) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq <= f32::EPSILON {
        let point_dx = point.x - start.x;
        let point_dy = point.y - start.y;
        return (point_dx * point_dx + point_dy * point_dy).sqrt();
    }

    let t = (((point.x - start.x) * dx + (point.y - start.y) * dy) / len_sq).clamp(0.0, 1.0);
    let nearest_x = start.x + dx * t;
    let nearest_y = start.y + dy * t;
    let nearest_dx = point.x - nearest_x;
    let nearest_dy = point.y - nearest_y;
    (nearest_dx * nearest_dx + nearest_dy * nearest_dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothing_rejects_excessive_corner_deviation() {
        let points = vec![
            StrokePoint::new(0.0, 0.0),
            StrokePoint::new(10.0, 10.0),
            StrokePoint::new(20.0, 0.0),
        ];

        let smoothed = smooth_stroke_segment(&points, false, true, 1, 1.0);

        assert_eq!(smoothed, points);
    }

    #[test]
    fn smoothing_accepts_when_corner_deviation_is_within_limit() {
        let points = vec![
            StrokePoint::new(0.0, 0.0),
            StrokePoint::new(10.0, 10.0),
            StrokePoint::new(20.0, 0.0),
        ];

        let smoothed = smooth_stroke_segment(&points, false, true, 1, 3.0);

        assert!(smoothed.len() > points.len());
    }

    #[test]
    fn smoothing_disabled_preserves_original_points() {
        let points = vec![
            StrokePoint::new(0.0, 0.0),
            StrokePoint::new(10.0, 10.0),
            StrokePoint::new(20.0, 0.0),
        ];

        let smoothed = smooth_stroke_segment(&points, false, false, 1, 3.0);

        assert_eq!(smoothed, points);
    }

    #[test]
    fn split_stroke_segments_breaks_axis_jumps() {
        let points = vec![
            StrokePoint::new(0.0, 0.0),
            StrokePoint::new(5.0, 1.0),
            StrokePoint::new(200.0, 2.0),
            StrokePoint::new(205.0, 3.0),
        ];

        let segments = split_stroke_segments(&points, 50.0, 50.0);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], points[..2]);
        assert_eq!(segments[1], points[2..]);
    }

    #[test]
    fn split_stroke_segments_breaks_adaptive_distance_jumps() {
        let points = vec![
            StrokePoint::new(0.0, 0.0),
            StrokePoint::new(5.0, 0.0),
            StrokePoint::new(10.0, 0.0),
            StrokePoint::new(40.0, 30.0),
            StrokePoint::new(45.0, 30.0),
        ];

        let segments = split_stroke_segments(&points, 100.0, 100.0);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], points[..3]);
        assert_eq!(segments[1], points[3..]);
    }
}
