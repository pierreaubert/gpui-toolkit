#[cfg(not(feature = "vello-gpui"))]
use super::super::primitives::Color4;

#[cfg(not(feature = "vello-gpui"))]
pub(super) struct ContourDrawData {
    pub(super) rings: Vec<Vec<(f32, f32)>>,
    pub(super) stroke_color: Color4,
    pub(super) fill_color: Color4,
}

#[cfg(not(feature = "vello-gpui"))]
pub(super) struct BandDrawData {
    pub(super) polygons: Vec<Vec<(f32, f32)>>,
    pub(super) fill_color: Color4,
}

#[cfg(not(feature = "vello-gpui"))]
pub(super) struct CellDrawData {
    pub(super) x_min: f32,
    pub(super) x_max: f32,
    pub(super) y_min: f32,
    pub(super) y_max: f32,
    pub(super) fill_color: Color4,
}
