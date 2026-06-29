use super::super::data::SurfaceData;
use super::linear::linear_subdivision_ticks;
use super::misc::default_frequency_ticks;
use super::misc::log_frequency_minor_ticks;
use super::misc::remove_positions;
use super::misc::sanitize_axis_positions;
use super::normalized::normalized_x_positions;
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CartesianGridLineKind {
    Minor,
    Major,
    Border,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CartesianGridLine {
    pub(super) start: Vec3,
    pub(super) end: Vec3,
    pub(super) kind: CartesianGridLineKind,
}

#[derive(Debug, Clone, Default)]
pub(super) struct AxisGridTicks {
    pub(super) major: Vec<f32>,
    pub(super) minor: Vec<f32>,
}

pub(super) fn frequency_grid_ticks(data: &SurfaceData) -> AxisGridTicks {
    let mut major = normalized_x_positions(
        data.x_ticks
            .clone()
            .unwrap_or_else(default_frequency_ticks)
            .into_iter(),
        data,
    );
    let mut minor = if data.x_log {
        normalized_x_positions(log_frequency_minor_ticks(data).into_iter(), data)
    } else {
        normalized_x_positions(
            linear_subdivision_ticks(data.x_min, data.x_max, 25).into_iter(),
            data,
        )
    };
    sanitize_axis_positions(&mut major, -1.0, 1.0);
    sanitize_axis_positions(&mut minor, -1.0, 1.0);
    remove_positions(&mut minor, &major);
    AxisGridTicks { major, minor }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartesianGridLineDebugKind {
    Minor,
    Major,
    Border,
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CartesianGridLineDebug {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub kind: CartesianGridLineDebugKind,
}
