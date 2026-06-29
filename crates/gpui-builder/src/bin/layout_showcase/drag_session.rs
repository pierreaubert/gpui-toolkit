use super::types::DragTarget;
use gpui::{Pixels, Point};
use gpui_builder::Axis;

#[derive(Clone, Copy)]
pub(super) struct DragSession {
    pub(super) target: DragTarget,
    pub(super) axis: Axis,
    pub(super) start_pos: f32,
    pub(super) start_ratio: f32,
    pub(super) extent: f32,
}

impl DragSession {
    pub(super) fn axis_position(&self, position: Point<Pixels>) -> f32 {
        match self.axis {
            Axis::Horizontal => position.x.into(),
            Axis::Vertical => position.y.into(),
        }
    }
}
