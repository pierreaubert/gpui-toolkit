//! Bezier curve utilities for connection rendering

use super::state::Position;

/// Flatten a cubic bezier curve into line segments
///
/// Uses de Casteljau subdivision to approximate the curve with line segments
/// within the given tolerance.
#[cfg_attr(not(test), allow(dead_code))]
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
    let mut points = Vec::new();
    connection_path_into(from, to, tolerance, &mut points);
    points
}

/// Flatten a connection into caller-owned storage. Hot hit-testing paths keep
/// this buffer around, avoiding a fresh polyline allocation for every click.
pub fn connection_path_into(
    from: Position,
    to: Position,
    tolerance: f32,
    points: &mut Vec<Position>,
) {
    let (p0, p1, p2, p3) = horizontal_bezier(from, to);
    points.clear();
    points.push(p0);
    flatten_cubic_recursive(&p0, &p1, &p2, &p3, tolerance, points);
}

/// Rectangle representing a node bounding box used as a routing obstacle.
#[derive(Debug, Clone, Copy)]
pub struct ObstacleRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl ObstacleRect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    fn right(&self) -> f32 {
        self.x + self.w
    }

    fn bottom(&self) -> f32 {
        self.y + self.h
    }

    fn contains(&self, p: &Position) -> bool {
        p.x > self.x && p.x < self.right() && p.y > self.y && p.y < self.bottom()
    }
}

/// Check whether any sampled point of a polyline falls inside a rect.
fn path_hits_rect(path: &[Position], rect: &ObstacleRect) -> bool {
    path.iter().any(|p| rect.contains(p))
}

/// Generate a connection path that detours around obstacle nodes.
///
/// Falls back to the direct horizontal bezier when no obstacle is in the way.
/// The algorithm routes above or below the combined bounding box of all
/// blocking obstacles using three segments: a bezier into the detour altitude,
/// a straight horizontal segment past the obstacles, and a bezier back to the
/// target port.
pub fn connection_path_avoiding(
    from: Position,
    to: Position,
    obstacles: &[ObstacleRect],
    margin: f32,
    tolerance: f32,
) -> Vec<Position> {
    if obstacles.is_empty() {
        return connection_path(from, to, tolerance);
    }

    // Try the direct path and check for collisions
    let direct = connection_path(from, to, tolerance);
    let blocking: Vec<&ObstacleRect> = obstacles
        .iter()
        .filter(|o| path_hits_rect(&direct, o))
        .collect();

    if blocking.is_empty() {
        return direct;
    }

    // Combined bounding box of all blocking obstacles
    let obs_top = blocking.iter().map(|o| o.y).fold(f32::MAX, f32::min);
    let obs_bottom = blocking.iter().map(|o| o.bottom()).fold(f32::MIN, f32::max);
    let obs_left = blocking.iter().map(|o| o.x).fold(f32::MAX, f32::min);
    let obs_right = blocking.iter().map(|o| o.right()).fold(f32::MIN, f32::max);

    // Pick the closer side (above or below)
    let avg_y = (from.y + to.y) * 0.5;
    let above_y = obs_top - margin;
    let below_y = obs_bottom + margin;
    let route_y = if (avg_y - above_y).abs() <= (avg_y - below_y).abs() {
        above_y
    } else {
        below_y
    };

    // Waypoint X: enter before first obstacle, exit after last
    let enter_x = (obs_left - margin).max(from.x);
    let exit_x = (obs_right + margin).min(to.x);

    // Degenerate case — fall back to direct path
    if enter_x >= exit_x {
        return direct;
    }

    let wp1 = Position::new(enter_x, route_y);
    let wp2 = Position::new(exit_x, route_y);

    // Segment 1: from → wp1 (horizontal bezier curving to detour altitude)
    let mut path = connection_path(from, wp1, tolerance);

    // Segment 2: straight horizontal at route_y
    if (exit_x - enter_x).abs() > 1.0 {
        path.push(wp2);
    }

    // Segment 3: wp2 → to (horizontal bezier curving back to target)
    let seg_end = connection_path(wp2, to, tolerance);
    if seg_end.len() > 1 {
        path.extend_from_slice(&seg_end[1..]); // skip duplicate point
    }

    path
}

#[cfg(test)]
mod tests {
    use super::super::Position;
    use super::{connection_path, connection_path_into, flatten_cubic_bezier, horizontal_bezier};

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

    #[test]
    fn reused_connection_path_buffer_retains_capacity() {
        let from = Position::new(0.0, 50.0);
        let to = Position::new(200.0, 150.0);
        let mut path = Vec::new();
        connection_path_into(from, to, 1.0, &mut path);
        let capacity = path.capacity();
        for _ in 0..1_000 {
            connection_path_into(from, to, 1.0, &mut path);
            assert_eq!(path.capacity(), capacity);
        }
    }

    use super::{ObstacleRect, connection_path_avoiding, lerp, path_hits_rect};

    #[test]
    fn test_lerp_and_distance_to_line() {
        let a = Position::new(0.0, 0.0);
        let b = Position::new(10.0, 0.0);
        let mid = lerp(&a, &b, 0.5);
        assert_eq!(mid, Position::new(5.0, 0.0));

        let from = Position::new(0.0, 50.0);
        let to = Position::new(200.0, 50.0);
        let path = connection_path(from, to, 1.0);
        assert!(path.len() >= 2);
    }

    #[test]
    fn test_connection_path_avoiding_empty_obstacles() {
        let from = Position::new(0.0, 50.0);
        let to = Position::new(200.0, 50.0);
        let path = connection_path_avoiding(from, to, &[], 10.0, 1.0);
        assert!(path.len() >= 2);
        assert!((path[0].x - from.x).abs() < 0.1);
        assert!((path.last().unwrap().x - to.x).abs() < 0.1);
    }

    #[test]
    fn test_connection_path_avoiding_detours() {
        let from = Position::new(0.0, 50.0);
        let to = Position::new(200.0, 50.0);
        let obstacle = ObstacleRect::new(80.0, 30.0, 40.0, 40.0);
        let path = connection_path_avoiding(from, to, &[obstacle], 10.0, 1.0);
        // Path should not pass through the obstacle's interior
        assert!(!path_hits_rect(&path, &obstacle));
        assert!((path.last().unwrap().x - to.x).abs() < 0.1);
    }

    #[test]
    fn test_connection_path_avoiding_degenerate_fallback() {
        let from = Position::new(0.0, 50.0);
        let to = Position::new(50.0, 50.0);
        // Obstacle wider than the available gap, forcing enter_x >= exit_x
        let obstacle = ObstacleRect::new(10.0, 30.0, 30.0, 40.0);
        let path = connection_path_avoiding(from, to, &[obstacle], 100.0, 1.0);
        assert!(path.len() >= 2);
    }

    #[test]
    fn test_obstacle_rect_contains() {
        let rect = ObstacleRect::new(10.0, 10.0, 20.0, 20.0);
        assert!(rect.contains(&Position::new(15.0, 15.0)));
        assert!(!rect.contains(&Position::new(5.0, 15.0)));
        assert!(!rect.contains(&Position::new(35.0, 15.0)));
    }
}
