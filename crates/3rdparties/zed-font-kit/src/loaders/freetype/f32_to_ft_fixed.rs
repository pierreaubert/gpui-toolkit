use freetype_sys :: { FT_Fixed } ;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use std::f32;

pub(super) trait F32ToFtFixed {
    type Output;
    fn f32_to_ft_fixed_26_6(self) -> Self::Output;
}

impl F32ToFtFixed for Vector2F {
    type Output = Vector2I;
    #[inline]
    fn f32_to_ft_fixed_26_6(self) -> Vector2I {
        (self * 64.0).to_i32()
    }
}

impl F32ToFtFixed for f32 {
    type Output = FT_Fixed;
    #[inline]
    fn f32_to_ft_fixed_26_6(self) -> FT_Fixed {
        (self * 64.0) as FT_Fixed
    }
}
