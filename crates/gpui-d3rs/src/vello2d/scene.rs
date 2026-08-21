//! Backend-neutral chart scene: kurbo geometry + peniko brushes.

use kurbo::{BezPath, Circle, PathEl, Rect, RoundedRect, Shape, Stroke};
use peniko::{Brush, Fill};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SCENE_REVISION: AtomicU64 = AtomicU64::new(1);

/// One draw command in a [`ChartScene`].
#[derive(Clone, Debug)]
pub enum ChartCmd {
    /// Fill `path` with `brush` using `fill` rule.
    Fill {
        path: BezPath,
        fill: Fill,
        brush: Brush,
    },
    /// Stroke `path` with `stroke` style and `brush`.
    Stroke {
        path: BezPath,
        stroke: Stroke,
        brush: Brush,
    },
}

/// Ordered list of chart draw commands, replayed by GPU or CPU backends.
#[derive(Clone, Debug)]
pub struct ChartScene {
    cmds: Vec<ChartCmd>,
    /// Monotonic revision for persistent painters. A freshly-built scene
    /// starts at zero; every mutating command advances the revision so a
    /// retained element can avoid resubmitting identical geometry.
    revision: u64,
}

impl Default for ChartScene {
    fn default() -> Self {
        Self {
            cmds: Vec::new(),
            revision: NEXT_SCENE_REVISION.fetch_add(1, Ordering::Relaxed),
        }
    }
}

impl ChartScene {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fill an arbitrary path with the non-zero winding rule.
    pub fn fill_path(&mut self, path: BezPath, brush: Brush) {
        self.cmds.push(ChartCmd::Fill {
            path,
            fill: Fill::NonZero,
            brush,
        });
        self.revision = self.revision.wrapping_add(1);
    }

    /// Fill a circle (the scatter-marker primitive).
    pub fn fill_circle(&mut self, cx: f64, cy: f64, radius: f64, brush: Brush) {
        if radius <= 0.0 {
            return;
        }
        self.fill_path(Circle::new((cx, cy), radius).to_path(0.1), brush);
    }

    /// Fill an axis-aligned rectangle (the bar primitive).
    pub fn fill_rect(&mut self, rect: Rect, brush: Brush) {
        self.fill_path(rect.to_path(0.1), brush);
    }

    /// Fill a rounded rectangle. Keeping this operation in the scene IR
    /// avoids every producer having to hand-roll corner geometry.
    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: f64, brush: Brush) {
        if !radius.is_finite() || radius <= 0.0 {
            self.fill_rect(rect, brush);
            return;
        }
        let radius = radius
            .min(rect.width().abs() * 0.5)
            .min(rect.height().abs() * 0.5);
        self.fill_path(RoundedRect::from_rect(rect, radius).to_path(0.1), brush);
    }

    /// Fill a circular wedge (including its center point).
    pub fn fill_wedge(
        &mut self,
        cx: f64,
        cy: f64,
        radius: f64,
        start_angle: f64,
        sweep_angle: f64,
        brush: Brush,
    ) {
        let Some(path) = arc_path(cx, cy, radius, start_angle, sweep_angle, true) else {
            return;
        };
        self.fill_path(path, brush);
    }

    /// Stroke a circular arc. Angles are measured in radians.
    pub fn stroke_arc(
        &mut self,
        cx: f64,
        cy: f64,
        radius: f64,
        start_angle: f64,
        sweep_angle: f64,
        stroke: Stroke,
        brush: Brush,
    ) {
        let Some(path) = arc_path(cx, cy, radius, start_angle, sweep_angle, false) else {
            return;
        };
        self.stroke_path(path, stroke, brush);
    }

    /// Stroke an arbitrary path.
    pub fn stroke_path(&mut self, path: BezPath, stroke: Stroke, brush: Brush) {
        self.cmds.push(ChartCmd::Stroke {
            path,
            stroke,
            brush,
        });
        self.revision = self.revision.wrapping_add(1);
    }

    /// Stroke a polyline (the line-chart primitive). Fewer than two points
    /// emit nothing.
    pub fn stroke_polyline(&mut self, points: &[(f64, f64)], stroke: Stroke, brush: Brush) {
        if points.len() < 2 {
            return;
        }
        let mut path = BezPath::new();
        path.push(PathEl::MoveTo(points[0].into()));
        for &p in &points[1..] {
            path.push(PathEl::LineTo(p.into()));
        }
        self.stroke_path(path, stroke, brush);
    }

    pub fn commands(&self) -> &[ChartCmd] {
        &self.cmds
    }

    pub fn len(&self) -> usize {
        self.cmds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }

    /// Revision of the scene's command stream.
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

/// Build a deterministic polyline approximation of a circular arc. Vello
/// and `vello_cpu` consume the same path, so fixture and GPU geometry agree.
fn arc_path(
    cx: f64,
    cy: f64,
    radius: f64,
    start_angle: f64,
    sweep_angle: f64,
    close_to_center: bool,
) -> Option<BezPath> {
    if !cx.is_finite()
        || !cy.is_finite()
        || !radius.is_finite()
        || radius <= 0.0
        || !start_angle.is_finite()
        || !sweep_angle.is_finite()
        || sweep_angle.abs() <= f64::EPSILON
    {
        return None;
    }

    let segments =
        ((sweep_angle.abs() / std::f64::consts::FRAC_PI_8).ceil() as usize).clamp(1, 256);
    let mut path = BezPath::new();
    let first = start_angle;
    let first_point = (cx + radius * first.cos(), cy + radius * first.sin());
    if close_to_center {
        path.push(PathEl::MoveTo((cx, cy).into()));
        path.push(PathEl::LineTo(first_point.into()));
    } else {
        path.push(PathEl::MoveTo(first_point.into()));
    }
    for index in 1..=segments {
        let angle = start_angle + sweep_angle * index as f64 / segments as f64;
        path.push(PathEl::LineTo(
            (cx + radius * angle.cos(), cy + radius * angle.sin()).into(),
        ));
    }
    if close_to_center {
        path.push(PathEl::ClosePath);
    }
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use peniko::Color;

    #[test]
    fn revisions_advance_only_when_commands_are_emitted() {
        let mut scene = ChartScene::new();
        let initial = scene.revision();
        scene.fill_circle(1.0, 1.0, 0.0, Brush::Solid(Color::WHITE));
        assert_eq!(scene.revision(), initial);
        scene.fill_rect(Rect::new(0.0, 0.0, 2.0, 2.0), Brush::Solid(Color::WHITE));
        assert_eq!(scene.revision(), initial + 1);
        scene.stroke_polyline(&[(0.0, 0.0)], Stroke::new(1.0), Brush::Solid(Color::WHITE));
        assert_eq!(scene.revision(), initial + 1);
        scene.stroke_polyline(
            &[(0.0, 0.0), (1.0, 1.0)],
            Stroke::new(1.0),
            Brush::Solid(Color::WHITE),
        );
        assert_eq!(scene.revision(), initial + 2);
    }

    #[test]
    fn fresh_scenes_have_distinct_revisions_for_live_painters() {
        let first = ChartScene::new();
        let second = ChartScene::new();
        assert_ne!(first.revision(), second.revision());
    }

    #[test]
    fn clone_preserves_revision_for_retained_painters() {
        let mut scene = ChartScene::new();
        scene.fill_rect(Rect::new(0.0, 0.0, 2.0, 2.0), Brush::Solid(Color::WHITE));
        let clone = scene.clone();
        assert_eq!(clone.revision(), scene.revision());
        assert_eq!(clone.len(), scene.len());
    }

    #[test]
    fn wedge_and_arc_emit_deterministic_paths() {
        let mut scene = ChartScene::new();
        let initial = scene.revision();
        scene.fill_wedge(
            10.0,
            10.0,
            5.0,
            0.0,
            std::f64::consts::FRAC_PI_2,
            Brush::Solid(Color::WHITE),
        );
        scene.stroke_arc(
            10.0,
            10.0,
            4.0,
            0.0,
            std::f64::consts::PI,
            Stroke::new(1.0),
            Brush::Solid(Color::WHITE),
        );
        assert_eq!(scene.len(), 2);
        assert_eq!(scene.revision(), initial + 2);
        assert!(matches!(scene.commands()[0], ChartCmd::Fill { .. }));
        assert!(matches!(scene.commands()[1], ChartCmd::Stroke { .. }));
    }

    #[test]
    fn invalid_arc_is_a_noop() {
        let mut scene = ChartScene::new();
        let revision = scene.revision();
        scene.fill_wedge(0.0, 0.0, 0.0, 0.0, 1.0, Brush::Solid(Color::WHITE));
        scene.stroke_arc(
            0.0,
            0.0,
            1.0,
            0.0,
            f64::NAN,
            Stroke::new(1.0),
            Brush::Solid(Color::WHITE),
        );
        assert_eq!(scene.revision(), revision);
        assert!(scene.is_empty());
    }
}
