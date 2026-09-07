use super::super::data::SurfaceData;
use super::linear::linear_step_ticks;
use super::misc::remove_positions;
use super::misc::sanitize_axis_positions;
use super::normalized::normalized_y_positions;
use super::types::AxisGridTicks;

pub(super) fn angle_grid_ticks(data: &SurfaceData, major_ticks: &[f64]) -> AxisGridTicks {
    let mut major = normalized_y_positions(major_ticks.iter().copied(), data);
    let mut minor = normalized_y_positions(
        linear_step_ticks(data.y_min, data.y_max, 10.0).into_iter(),
        data,
    );
    sanitize_axis_positions(&mut major, -1.0, 1.0);
    sanitize_axis_positions(&mut minor, -1.0, 1.0);
    remove_positions(&mut minor, &major);
    AxisGridTicks { major, minor }
}
