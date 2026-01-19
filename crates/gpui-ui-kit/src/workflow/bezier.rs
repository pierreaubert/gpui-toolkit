//! Bezier curve utilities for connection rendering

use super::state::Position;

/// Flatten a cubic bezier curve into line segments
///
/// Uses de Casteljau subdivision to approximate the curve with line segments
/// within the given tolerance.
pub fn flatten_cubic_bezier(
    p0: Position,
    p1: Position,
    p2: Position,
    p3: Position,
    tolerance: f32,
) -> Vec<Position> {
    let mut points = vec![p0];
    flatten_cubic_recursive(&p0, &p1, &p2, &p3, tolerance, &mut points);
    points
}

fn flatten_cubic_recursive(
    p0: &Position,
    p1: &Position,
    p2: &Position,
    p3: &Position,
    tolerance: f32,
    points: &mut Vec<Position>,
) {
    // Check if the curve is flat enough
    let d1 = distance_to_line(p1, p0, p3);
    let d2 = distance_to_line(p2, p0, p3);

    if d1 + d2 < tolerance {
        points.push(*p3);
    } else {
        // Subdivide using de Casteljau's algorithm
        let p01 = lerp(p0, p1, 0.5);
        let p12 = lerp(p1, p2, 0.5);
        let p23 = lerp(p2, p3, 0.5);
        let p012 = lerp(&p01, &p12, 0.5);
        let p123 = lerp(&p12, &p23, 0.5);
        let p0123 = lerp(&p012, &p123, 0.5);

        flatten_cubic_recursive(p0, &p01, &p012, &p0123, tolerance, points);
        flatten_cubic_recursive(&p0123, &p123, &p23, p3, tolerance, points);
    }
}

/// Linear interpolation between two points
fn lerp(a: &Position, b: &Position, t: f32) -> Position {
    Position::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// Calculate perpendicular distance from a point to a line
fn distance_to_line(point: &Position, line_start: &Position, line_end: &Position) -> f32 {
    let dx = line_end.x - line_start.x;
    let dy = line_end.y - line_start.y;
    let length_sq = dx * dx + dy * dy;

    if length_sq < 1e-10 {
        return point.distance(line_start);
    }

    let cross = (point.x - line_start.x) * dy - (point.y - line_start.y) * dx;
    cross.abs() / length_sq.sqrt()
}

/// Generate a horizontal bezier curve for connecting nodes
///
/// Creates a smooth S-curve that starts horizontal from the source
/// and ends horizontal at the target (like ReactFlow).
pub fn horizontal_bezier(from: Position, to: Position) -> (Position, Position, Position, Position) {
    let mid_x = (from.x + to.x) / 2.0;

    // Control points create a horizontal S-curve
    let p0 = from;
    let p1 = Position::new(mid_x, from.y);
    let p2 = Position::new(mid_x, to.y);
    let p3 = to;

    (p0, p1, p2, p3)
}

/// Generate points for rendering a horizontal bezier connection
pub fn connection_path(from: Position, to: Position, tolerance: f32) -> Vec<Position> {
    let (p0, p1, p2, p3) = horizontal_bezier(from, to);
    flatten_cubic_bezier(p0, p1, p2, p3, tolerance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatten_straight_line() {
        let from = Position::new(0.0, 0.0);
        let to = Position::new(100.0, 0.0);

        // A straight line should flatten to just start and end
        let points = flatten_cubic_bezier(
            from,
            Position::new(33.0, 0.0),
            Position::new(66.0, 0.0),
            to,
            1.0,
        );

        assert!(points.len() >= 2);
        assert!((points[0].x - 0.0).abs() < 0.1);
        assert!((points.last().unwrap().x - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_horizontal_bezier() {
        let from = Position::new(0.0, 50.0);
        let to = Position::new(200.0, 150.0);

        let (p0, p1, p2, p3) = horizontal_bezier(from, to);

        // Start point matches
        assert_eq!(p0, from);
        // End point matches
        assert_eq!(p3, to);
        // Control points are at midpoint X
        assert_eq!(p1.x, 100.0);
        assert_eq!(p2.x, 100.0);
        // Control points have same Y as their respective endpoints
        assert_eq!(p1.y, from.y);
        assert_eq!(p2.y, to.y);
    }

    #[test]
    fn test_connection_path() {
        let from = Position::new(0.0, 50.0);
        let to = Position::new(200.0, 150.0);

        let points = connection_path(from, to, 1.0);

        // Should have multiple points for a curved path
        assert!(points.len() > 2);
        // First point should be near start
        assert!((points[0].x - from.x).abs() < 0.1);
        // Last point should be near end
        assert!((points.last().unwrap().x - to.x).abs() < 0.1);
    }
}
