use super::super::data::SurfaceData;
use super::linear::{linear_nice_step, linear_nice_ticks, linear_step_ticks};
use super::misc::remove_positions;
use super::misc::sanitize_axis_positions;
use super::normalized::normalized_z_positions;
use super::types::AxisGridTicks;

pub(super) fn spl_grid_ticks(data: &SurfaceData, major_ticks: &[f64]) -> AxisGridTicks {
    let mut major = normalized_z_positions(major_ticks.iter().copied(), data);
    let mut minor = if data.z_ticks.is_some() {
        Vec::new()
    } else {
        // Nice ticks are uniform: subdivide the leading interval.
        let major_step = if major_ticks.len() >= 2 {
            (major_ticks[1] - major_ticks[0]).abs()
        } else {
            1.0
        };
        normalized_z_positions(
            linear_step_ticks(data.z_min, data.z_max, major_step / 5.0).into_iter(),
            data,
        )
    };
    sanitize_axis_positions(&mut major, -0.5, 0.5);
    sanitize_axis_positions(&mut minor, -0.5, 0.5);
    remove_positions(&mut minor, &major);
    AxisGridTicks { major, minor }
}

pub(super) fn spl_major_ticks(data: &SurfaceData) -> (Vec<f64>, f64) {
    if let Some(ticks) = data.z_ticks.clone() {
        return (ticks, 1.0);
    }

    let step = linear_nice_step(data.z_min, data.z_max, 6);
    (linear_nice_ticks(data.z_min, data.z_max, 6), step)
}
