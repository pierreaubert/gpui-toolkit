//! Backend-neutral chart scene: kurbo geometry + peniko brushes.

use kurbo::{BezPath, Circle, PathEl, Rect, Shape, Stroke};
use peniko::{Brush, Fill};

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
#[derive(Clone, Debug, Default)]
pub struct ChartScene {
    cmds: Vec<ChartCmd>,
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

    /// Stroke an arbitrary path.
    pub fn stroke_path(&mut self, path: BezPath, stroke: Stroke, brush: Brush) {
        self.cmds.push(ChartCmd::Stroke {
            path,
            stroke,
            brush,
        });
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
}
