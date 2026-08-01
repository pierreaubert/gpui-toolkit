use gpui::{Bounds, Path, PathBuilder, Pixels, point, px};
use std::f32::consts::{PI, TAU};

/// Build a stroked circular arc in window coordinates.
pub(crate) fn arc_path(
    bounds: Bounds<Pixels>,
    thickness: Pixels,
    start_angle: f32,
    sweep_angle: f32,
) -> Option<Path<Pixels>> {
    let width: f32 = bounds.size.width.into();
    let height: f32 = bounds.size.height.into();
    let stroke: f32 = thickness.into();
    let radius = (width.min(height) / 2.0 - stroke / 2.0).max(0.5);
    let origin_x: f32 = bounds.origin.x.into();
    let origin_y: f32 = bounds.origin.y.into();
    let center_x = origin_x + width / 2.0;
    let center_y = origin_y + height / 2.0;
    let sweep = sweep_angle.clamp(-TAU, TAU);

    if sweep.abs() <= f32::EPSILON {
        return None;
    }

    let point_at = |angle: f32| {
        point(
            px(center_x + radius * angle.cos()),
            px(center_y + radius * angle.sin()),
        )
    };

    let mut path = PathBuilder::stroke(thickness);
    let mut current_angle = start_angle;
    let mut remaining = sweep.abs();
    let direction = sweep.is_sign_positive();
    path.move_to(point_at(current_angle));

    while remaining > f32::EPSILON {
        let segment = remaining.min(PI);
        let next_angle = if direction {
            current_angle + segment
        } else {
            current_angle - segment
        };
        path.arc_to(
            point(px(radius), px(radius)),
            px(0.0),
            segment > PI,
            direction,
            point_at(next_angle),
        );
        current_angle = next_angle;
        remaining -= segment;
    }

    path.build().ok()
}

#[cfg(test)]
mod tests {
    use super::arc_path;
    use gpui::{Bounds, point, px, size};

    #[test]
    fn arc_path_rejects_empty_sweeps() {
        let bounds = Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(32.0), px(32.0)),
        };
        assert!(arc_path(bounds, px(2.0), 0.0, 0.0).is_none());
    }

    #[test]
    fn arc_path_builds_partial_and_full_circles() {
        let bounds = Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(32.0), px(32.0)),
        };
        assert!(arc_path(bounds, px(2.0), 0.0, std::f32::consts::PI).is_some());
        assert!(arc_path(bounds, px(2.0), 0.0, std::f32::consts::TAU).is_some());
    }
}
