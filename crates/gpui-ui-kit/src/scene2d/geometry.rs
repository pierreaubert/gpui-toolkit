//! Logical view-box fitting, hit testing, and ordered grid traversal.

// Rust guideline compliant 2026-02-21

use super::types::{
    Scene2DGrid, Scene2DGridCell, Scene2DNode, Scene2DNodeKind, Scene2DScene, ScenePoint, SceneRect,
};

/// A contain-fit transform shared by native painting and pointer picking.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scene2DViewTransform {
    view_box: SceneRect,
    viewport_width: f32,
    viewport_height: f32,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
}

impl Scene2DViewTransform {
    /// Fits a logical view box inside a pixel-sized viewport without cropping.
    ///
    /// # Examples
    ///
    /// ```
    /// use gpui_ui_kit::scene2d::{Scene2DViewTransform, ScenePoint, SceneRect};
    ///
    /// let transform = Scene2DViewTransform::contain(
    ///     SceneRect::new(0.0, 0.0, 100.0, 50.0),
    ///     200.0,
    ///     200.0,
    /// )
    /// .unwrap();
    /// assert_eq!(transform.scene_to_viewport(ScenePoint::new(0.0, 0.0)), ScenePoint::new(0.0, 50.0));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `None` when the view box or viewport has invalid dimensions.
    pub fn contain(view_box: SceneRect, viewport_width: f32, viewport_height: f32) -> Option<Self> {
        if !view_box.is_finite()
            || view_box.width <= 0.0
            || view_box.height <= 0.0
            || !viewport_width.is_finite()
            || !viewport_height.is_finite()
            || viewport_width <= 0.0
            || viewport_height <= 0.0
        {
            return None;
        }
        let scale = (viewport_width / view_box.width).min(viewport_height / view_box.height);
        if !scale.is_finite() || scale <= 0.0 {
            return None;
        }
        let fitted_width = view_box.width * scale;
        let fitted_height = view_box.height * scale;
        Some(Self {
            view_box,
            viewport_width,
            viewport_height,
            scale,
            offset_x: (viewport_width - fitted_width) * 0.5,
            offset_y: (viewport_height - fitted_height) * 0.5,
        })
    }

    /// Returns the uniform logical-to-viewport scale.
    pub fn scale(self) -> f32 {
        self.scale
    }

    /// Returns the fitted content rectangle within the viewport.
    pub fn content_bounds(self) -> SceneRect {
        SceneRect::new(
            self.offset_x,
            self.offset_y,
            self.view_box.width * self.scale,
            self.view_box.height * self.scale,
        )
    }

    /// Maps scene coordinates to viewport coordinates.
    pub fn scene_to_viewport(self, point: ScenePoint) -> ScenePoint {
        ScenePoint::new(
            self.offset_x + (point.x - self.view_box.x) * self.scale,
            self.offset_y + (point.y - self.view_box.y) * self.scale,
        )
    }

    /// Maps viewport coordinates into the view box, returning `None` in letterbox bars.
    pub fn viewport_to_scene(self, point: ScenePoint) -> Option<ScenePoint> {
        let bounds = self.content_bounds();
        if !bounds.contains(point) {
            return None;
        }
        Some(self.viewport_to_scene_unclamped(point))
    }

    /// Maps any viewport coordinate into the view box, including outside its fitted bounds.
    pub fn viewport_to_scene_unclamped(self, point: ScenePoint) -> ScenePoint {
        ScenePoint::new(
            self.view_box.x + (point.x - self.offset_x) / self.scale,
            self.view_box.y + (point.y - self.offset_y) / self.scale,
        )
    }

    /// Returns true when viewport coordinates are inside the fitted view box.
    pub fn contains_viewport_point(self, point: ScenePoint) -> bool {
        self.content_bounds().contains(point)
    }

    /// Returns the viewport dimensions used to create this transform.
    pub fn viewport_size(self) -> (f32, f32) {
        (self.viewport_width, self.viewport_height)
    }
}

/// Semantic result from hit-testing a scene point.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scene2DHit {
    /// Stable hit target identifier, when a drawn object was hit.
    pub hit_id: Option<String>,
    /// Grid cell identity, when the point is over a configured cell.
    pub cell: Option<Scene2DGridCell>,
}

impl Scene2DScene {
    /// Resolves the topmost interactive object and optional grid cell at a point.
    ///
    /// Drawn objects are checked from front to back. The regular grid is used
    /// when no object with an explicit hit identifier covers the point.
    pub fn hit_test(&self, point: ScenePoint) -> Scene2DHit {
        let hit_id = find_hit_id(&self.nodes, point).map(str::to_owned);
        let cell = self.grid.as_ref().and_then(|grid| grid.hit_cell(point));
        Scene2DHit { hit_id, cell }
    }
}

fn find_hit_id(nodes: &[Scene2DNode], point: ScenePoint) -> Option<&str> {
    nodes
        .iter()
        .rev()
        .find_map(|node| find_node_hit_id(node, point))
}

fn find_node_hit_id(node: &Scene2DNode, point: ScenePoint) -> Option<&str> {
    let local = inverse_transform_point(node, point);
    if let Some(clip) = node.clip
        && !clip.contains(local)
    {
        return None;
    }
    if let Scene2DNodeKind::Group { children } = &node.kind
        && let Some(hit_id) = find_hit_id(children, local)
    {
        return Some(hit_id);
    }
    (node.hit_id.is_some() && node_contains_local(node, local))
        .then(|| node.hit_id.as_deref())
        .flatten()
}

impl Scene2DGrid {
    /// Lists each grid cell crossed from one point to another in path order.
    ///
    /// Exact diagonal corner crossings include both touching side cells before
    /// the diagonal cell. Gaps between cells do not interrupt the logical path.
    pub fn crossed_cells(&self, start: ScenePoint, end: ScenePoint) -> Vec<Scene2DGridCell> {
        if self.rows == 0
            || self.columns == 0
            || !start.is_finite()
            || !end.is_finite()
            || self.cell_width <= 0.0
            || self.cell_height <= 0.0
        {
            return Vec::new();
        }
        let pitch_x = self.cell_width + self.gap;
        let pitch_y = self.cell_height + self.gap;
        if pitch_x <= 0.0 || pitch_y <= 0.0 {
            return Vec::new();
        }
        let Some((mut column, mut row)) = self.logical_cell(start, pitch_x, pitch_y) else {
            return Vec::new();
        };
        let Some((end_column, end_row)) = self.logical_cell(end, pitch_x, pitch_y) else {
            return Vec::new();
        };
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let step_x = if dx > 0.0 {
            1_i64
        } else if dx < 0.0 {
            -1_i64
        } else {
            0_i64
        };
        let step_y = if dy > 0.0 {
            1_i64
        } else if dy < 0.0 {
            -1_i64
        } else {
            0_i64
        };
        let mut t_delta_x = if dx == 0.0 {
            f32::INFINITY
        } else {
            pitch_x / dx.abs()
        };
        let mut t_delta_y = if dy == 0.0 {
            f32::INFINITY
        } else {
            pitch_y / dy.abs()
        };
        if !t_delta_x.is_finite() {
            t_delta_x = f32::INFINITY;
        }
        if !t_delta_y.is_finite() {
            t_delta_y = f32::INFINITY;
        }
        let next_x = self.x + (column + i64::from(step_x > 0)) as f32 * pitch_x;
        let next_y = self.y + (row + i64::from(step_y > 0)) as f32 * pitch_y;
        let mut t_max_x = if dx == 0.0 {
            f32::INFINITY
        } else {
            (next_x - start.x) / dx
        };
        let mut t_max_y = if dy == 0.0 {
            f32::INFINITY
        } else {
            (next_y - start.y) / dy
        };
        let mut cells = Vec::new();
        append_cell(self, column, row, &mut cells);
        let max_steps = self.rows as usize + self.columns as usize + 2;
        for _ in 0..max_steps {
            if column == end_column && row == end_row {
                break;
            }
            if (t_max_x - t_max_y).abs() <= 1e-6 {
                if step_x != 0 {
                    append_cell(self, column + step_x, row, &mut cells);
                }
                if step_y != 0 {
                    append_cell(self, column, row + step_y, &mut cells);
                }
                column += step_x;
                row += step_y;
                t_max_x += t_delta_x;
                t_max_y += t_delta_y;
                append_cell(self, column, row, &mut cells);
            } else if t_max_x < t_max_y {
                column += step_x;
                t_max_x += t_delta_x;
                append_cell(self, column, row, &mut cells);
            } else {
                row += step_y;
                t_max_y += t_delta_y;
                append_cell(self, column, row, &mut cells);
            }
        }
        cells
    }

    fn logical_cell(&self, point: ScenePoint, pitch_x: f32, pitch_y: f32) -> Option<(i64, i64)> {
        let column = ((point.x - self.x) / pitch_x).floor() as i64;
        let row = ((point.y - self.y) / pitch_y).floor() as i64;
        (column >= 0 && row >= 0 && column < i64::from(self.columns) && row < i64::from(self.rows))
            .then_some((column, row))
    }
}

fn append_cell(grid: &Scene2DGrid, column: i64, row: i64, cells: &mut Vec<Scene2DGridCell>) {
    if column < 0 || row < 0 || column >= i64::from(grid.columns) || row >= i64::from(grid.rows) {
        return;
    }
    let index = row as u32 * grid.columns + column as u32;
    if cells.last().is_some_and(|cell| cell.index == index) {
        return;
    }
    cells.push(Scene2DGridCell {
        row: row as u32,
        column: column as u32,
        index,
        id: format!("r{row}c{column}"),
    });
}

fn node_contains_local(node: &Scene2DNode, local: ScenePoint) -> bool {
    if let Some(bounds) = node.hit_bounds {
        return bounds.contains(local);
    }
    match &node.kind {
        Scene2DNodeKind::Rect { rect, .. } | Scene2DNodeKind::RoundedRect { rect, .. } => {
            rect.contains(local)
        }
        Scene2DNodeKind::Circle {
            center,
            radius,
            stroke,
            ..
        } => {
            let edge = stroke.map_or(0.0, |stroke| stroke.width * 0.5);
            distance_squared(local, *center) <= (radius + edge).powi(2)
        }
        Scene2DNodeKind::Line { start, end, stroke } => {
            distance_to_segment(local, *start, *end) <= stroke.width * 0.5
        }
        Scene2DNodeKind::Path {
            commands, stroke, ..
        } => {
            let Some(bounds) = path_bounds(commands) else {
                return false;
            };
            let Some(stroke) = stroke else {
                return bounds.contains(local);
            };
            let expanded = SceneRect::new(
                bounds.x - stroke.width * 0.5,
                bounds.y - stroke.width * 0.5,
                bounds.width + stroke.width,
                bounds.height + stroke.width,
            );
            expanded.contains(local)
        }
        Scene2DNodeKind::Text {
            origin,
            content,
            size,
            ..
        } => {
            let estimated_width = content.chars().count() as f32 * *size * 0.62;
            SceneRect::new(origin.x, origin.y, estimated_width, *size * 1.4).contains(local)
        }
        Scene2DNodeKind::Group { .. } => false,
    }
}

fn inverse_transform_point(node: &Scene2DNode, point: ScenePoint) -> ScenePoint {
    let transformed = ScenePoint::new(
        point.x - node.transform.translate_x,
        point.y - node.transform.translate_y,
    );
    let [a, b, c, d] = node.transform.linear();
    let determinant = a * d - b * c;
    ScenePoint::new(
        (d * transformed.x - b * transformed.y) / determinant,
        (-c * transformed.x + a * transformed.y) / determinant,
    )
}

fn distance_squared(left: ScenePoint, right: ScenePoint) -> f32 {
    (left.x - right.x).powi(2) + (left.y - right.y).powi(2)
}

fn distance_to_segment(point: ScenePoint, start: ScenePoint, end: ScenePoint) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f32::EPSILON {
        return distance_squared(point, start).sqrt();
    }
    let t =
        (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared).clamp(0.0, 1.0);
    let closest = ScenePoint::new(start.x + t * dx, start.y + t * dy);
    distance_squared(point, closest).sqrt()
}

fn path_bounds(commands: &[super::types::Scene2DPathCommand]) -> Option<SceneRect> {
    use super::types::Scene2DPathCommand as Command;
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut add = |point: ScenePoint| {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    };
    for command in commands {
        match command {
            Command::MoveTo { point } | Command::LineTo { point } => add(*point),
            Command::QuadraticTo { control, point } => {
                add(*control);
                add(*point);
            }
            Command::CubicTo {
                control_a,
                control_b,
                point,
            } => {
                add(*control_a);
                add(*control_b);
                add(*point);
            }
            Command::Close => {}
        }
    }
    (min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite())
        .then_some(SceneRect::new(min_x, min_y, max_x - min_x, max_y - min_y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene2d::{Scene2DBrush, Scene2DColor, Scene2DNodeKind, Scene2DStroke};

    fn cell_node(id: &str, hit_id: &str, x: f32) -> Scene2DNode {
        Scene2DNode {
            id: id.to_owned(),
            hit_id: Some(hit_id.to_owned()),
            hit_bounds: None,
            clip: None,
            shadow: None,
            semantic: None,
            transform: Default::default(),
            opacity: 1.0,
            transition: None,
            kind: Scene2DNodeKind::Rect {
                rect: SceneRect::new(x, 0.0, 10.0, 10.0),
                fill: Some(Scene2DBrush::solid(Scene2DColor::rgb(0.2, 0.4, 0.6))),
                stroke: Some(Scene2DStroke {
                    width: 1.0,
                    color: Scene2DColor::rgb(0.0, 0.0, 0.0),
                }),
            },
        }
    }

    #[test]
    fn contain_transform_round_trips_scene_points_and_rejects_letterbox_bars() {
        let transform =
            Scene2DViewTransform::contain(SceneRect::new(10.0, 20.0, 100.0, 50.0), 200.0, 200.0)
                .unwrap();
        let point = ScenePoint::new(75.0, 42.0);
        let viewport = transform.scene_to_viewport(point);
        assert_eq!(transform.viewport_to_scene(viewport), Some(point));
        assert_eq!(
            transform.viewport_to_scene(ScenePoint::new(100.0, 10.0)),
            None
        );
    }

    #[test]
    fn picking_returns_the_frontmost_object_and_the_overlaid_grid_cell() {
        let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 20.0, 10.0));
        scene.grid = Some(Scene2DGrid {
            rows: 1,
            columns: 2,
            x: 0.0,
            y: 0.0,
            cell_width: 10.0,
            cell_height: 10.0,
            gap: 0.0,
            row_labels: Vec::new(),
            column_labels: Vec::new(),
        });
        scene.nodes.push(cell_node("bottom", "first", 0.0));
        scene.nodes.push(cell_node("top", "second", 0.0));

        let hit = scene.hit_test(ScenePoint::new(5.0, 5.0));
        assert_eq!(hit.hit_id.as_deref(), Some("second"));
        assert_eq!(hit.cell.as_ref().map(|cell| cell.id.as_str()), Some("r0c0"));
    }

    #[test]
    fn exact_diagonal_traversal_reports_touching_cells_in_path_order() {
        let grid = Scene2DGrid {
            rows: 3,
            columns: 3,
            x: 0.0,
            y: 0.0,
            cell_width: 10.0,
            cell_height: 10.0,
            gap: 0.0,
            row_labels: Vec::new(),
            column_labels: Vec::new(),
        };
        let ids = grid
            .crossed_cells(ScenePoint::new(5.0, 5.0), ScenePoint::new(25.0, 25.0))
            .into_iter()
            .map(|cell| cell.id)
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            ["r0c0", "r0c1", "r1c0", "r1c1", "r1c2", "r2c1", "r2c2"]
        );
    }
}
