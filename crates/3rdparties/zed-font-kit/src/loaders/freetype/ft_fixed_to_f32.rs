use pathfinder_geometry::rect::{RectF, RectI};
use pathfinder_geometry::vector::{Vector2F, Vector2I};

pub(super) trait FtFixedToF32 {
    type Output;
    fn ft_fixed_26_6_to_f32(self) -> Self::Output;
}

impl FtFixedToF32 for Vector2I {
    type Output = Vector2F;
    #[inline]
    fn ft_fixed_26_6_to_f32(self) -> Vector2F {
        (self.to_f32() * (1.0 / 64.0)).round()
    }
}

impl FtFixedToF32 for RectI {
    type Output = RectF;
    #[inline]
    fn ft_fixed_26_6_to_f32(self) -> RectF {
        self.to_f32() * (1.0 / 64.0)
    }
}
