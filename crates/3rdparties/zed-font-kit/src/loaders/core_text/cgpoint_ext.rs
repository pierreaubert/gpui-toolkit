use core_graphics :: geometry :: { CGPoint } ;
use pathfinder_geometry::vector::Vector2F;
use std::f32;

pub(super) trait CGPointExt {
    fn to_vector(&self) -> Vector2F;
}

impl CGPointExt for CGPoint {
    #[inline]
    fn to_vector(&self) -> Vector2F {
        Vector2F::new(self.x as f32, self.y as f32)
    }
}
