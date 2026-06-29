use super::super::data::SurfaceData;

pub(super) fn normalized_x_positions(
    ticks: impl Iterator<Item = f64>,
    data: &SurfaceData,
) -> Vec<f32> {
    ticks.map(|value| data.normalize_x(value)).collect()
}

pub(super) fn normalized_y_positions(
    ticks: impl Iterator<Item = f64>,
    data: &SurfaceData,
) -> Vec<f32> {
    ticks.map(|value| data.normalize_y(value)).collect()
}

pub(super) fn normalized_z_positions(
    ticks: impl Iterator<Item = f64>,
    data: &SurfaceData,
) -> Vec<f32> {
    ticks.map(|value| data.normalize_z(value) - 0.5).collect()
}
