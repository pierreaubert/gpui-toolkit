use super::super::projection::{
    IsometricProjection, ObliqueProjection, OrthographicProjection, PerspectiveProjection, Point2D,
    Projection,
};

/// Enum wrapper for projections to enable dynamic dispatch without trait objects.
/// This is needed because the Projection trait has Clone bound which makes it not dyn-compatible.
#[derive(Clone)]
pub(super) enum ProjectionImpl {
    Isometric(IsometricProjection),
    Oblique(ObliqueProjection),
    Orthographic(OrthographicProjection),
    Perspective(PerspectiveProjection),
}

impl ProjectionImpl {
    pub(super) fn project(&self, x: f64, y: f64, z: f64) -> Point2D {
        match self {
            ProjectionImpl::Isometric(p) => p.project(x, y, z),
            ProjectionImpl::Oblique(p) => p.project(x, y, z),
            ProjectionImpl::Orthographic(p) => p.project(x, y, z),
            ProjectionImpl::Perspective(p) => p.project(x, y, z),
        }
    }

    pub(super) fn depth(&self, x: f64, y: f64, z: f64) -> f64 {
        match self {
            ProjectionImpl::Isometric(p) => p.depth(x, y, z),
            ProjectionImpl::Oblique(p) => p.depth(x, y, z),
            ProjectionImpl::Orthographic(p) => p.depth(x, y, z),
            ProjectionImpl::Perspective(p) => p.depth(x, y, z),
        }
    }

    pub(super) fn point_depth(&self, p: &super::super::data::SurfacePoint3D) -> f64 {
        self.depth(p.x, p.y, p.z)
    }
}
