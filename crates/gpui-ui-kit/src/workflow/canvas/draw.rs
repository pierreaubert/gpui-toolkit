use super::super::bezier::{ObstacleRect, connection_path, connection_path_avoiding};
use super::super::state::Position;
use gpui::{PathBuilder, Rgba, Window, point, px};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

const CONNECTION_PATH_CACHE_CAPACITY: usize = 512;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ConnectionPathCacheKey {
    from_x: u32,
    from_y: u32,
    to_x: u32,
    to_y: u32,
    margin: u32,
    tolerance: u32,
    obstacles_hash: u64,
    obstacles_len: usize,
}

thread_local! {
    /// Bounded per-UI-thread cache: paths normally stay identical across drag
    /// repaint frames and only change with endpoints or routing obstacles.
    static CONNECTION_PATH_CACHE: RefCell<HashMap<ConnectionPathCacheKey, Arc<[Position]>>> =
        RefCell::new(HashMap::new());
}

fn obstacle_signature(obstacles: &[ObstacleRect]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for obstacle in obstacles {
        for part in [obstacle.x, obstacle.y, obstacle.w, obstacle.h] {
            hash ^= u64::from(part.to_bits());
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
    }
    hash
}

fn cached_connection_path(
    from: Position,
    to: Position,
    obstacles: &[ObstacleRect],
    margin: f32,
    tolerance: f32,
) -> Arc<[Position]> {
    let key = ConnectionPathCacheKey {
        from_x: from.x.to_bits(),
        from_y: from.y.to_bits(),
        to_x: to.x.to_bits(),
        to_y: to.y.to_bits(),
        margin: margin.to_bits(),
        tolerance: tolerance.to_bits(),
        obstacles_hash: obstacle_signature(obstacles),
        obstacles_len: obstacles.len(),
    };
    CONNECTION_PATH_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(path) = cache.get(&key) {
            return Arc::clone(path);
        }
        if cache.len() >= CONNECTION_PATH_CACHE_CAPACITY {
            cache.clear();
        }
        let path: Arc<[Position]> =
            connection_path_avoiding(from, to, obstacles, margin, tolerance).into();
        cache.insert(key, Arc::clone(&path));
        path
    })
}

/// Draw a connection line between two ports, shortened at both ends by port_radius.
/// Routes around `obstacles` (other node bounding rects) when necessary.
#[allow(
    clippy::too_many_arguments,
    reason = "canvas drawing primitive takes geometry, style, and viewport offsets explicitly"
)]
pub(super) fn draw_connection(
    window: &mut Window,
    from: Position,
    to: Position,
    color: Rgba,
    width: f32,
    port_radius: f32,
    obstacles: &[ObstacleRect],
    offset_x: f32,
    offset_y: f32,
) {
    // Shorten the line at both ends so it doesn't overlap with ports
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let length = (dx * dx + dy * dy).sqrt();

    if length < port_radius * 2.5 {
        return; // Too short to draw
    }

    // Normalize direction
    let nx = dx / length;
    let ny = dy / length;

    // Shorten both ends by port_radius
    let shortened_from = Position::new(from.x + nx * port_radius, from.y + ny * port_radius);
    let shortened_to = Position::new(to.x - nx * port_radius, to.y - ny * port_radius);

    let margin = 15.0;
    let path_points = cached_connection_path(shortened_from, shortened_to, obstacles, margin, 2.0);

    if path_points.len() < 2 {
        return;
    }

    let mut builder = PathBuilder::stroke(px(width));

    // Move to first point
    builder.move_to(point(
        px(path_points[0].x + offset_x),
        px(path_points[0].y + offset_y),
    ));

    // Line to remaining points
    for point_pos in path_points.iter().skip(1) {
        builder.line_to(point(
            px(point_pos.x + offset_x),
            px(point_pos.y + offset_y),
        ));
    }

    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

/// Draw a connection preview line, shortened only at the port end
#[allow(
    clippy::too_many_arguments,
    reason = "canvas drawing primitive takes geometry, style, and viewport offsets explicitly"
)]
pub(super) fn draw_connection_preview(
    window: &mut Window,
    from: Position,
    to: Position,
    color: Rgba,
    width: f32,
    port_radius: f32,
    from_is_port: bool, // true if 'from' is the port, false if 'to' is the port
    offset_x: f32,
    offset_y: f32,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let length = (dx * dx + dy * dy).sqrt();

    if length < port_radius * 1.5 {
        return; // Too short to draw
    }

    // Normalize direction
    let nx = dx / length;
    let ny = dy / length;

    // Shorten only the port end
    let (shortened_from, shortened_to) = if from_is_port {
        (
            Position::new(from.x + nx * port_radius, from.y + ny * port_radius),
            to,
        )
    } else {
        (
            from,
            Position::new(to.x - nx * port_radius, to.y - ny * port_radius),
        )
    };

    let path_points = connection_path(shortened_from, shortened_to, 2.0);

    if path_points.len() < 2 {
        return;
    }

    let mut builder = PathBuilder::stroke(px(width));

    builder.move_to(point(
        px(path_points[0].x + offset_x),
        px(path_points[0].y + offset_y),
    ));

    for point_pos in path_points.iter().skip(1) {
        builder.line_to(point(
            px(point_pos.x + offset_x),
            px(point_pos.y + offset_y),
        ));
    }

    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}
