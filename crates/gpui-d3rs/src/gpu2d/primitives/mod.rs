//! Primitive types for GPU 2D rendering

mod circle;
mod line;
mod rect;
mod triangle;

pub use circle::{CircleBatch, CircleVertex};
pub use line::{LineBatch, LineVertex};
pub use rect::{RectBatch, RectVertex};
pub use triangle::{TriangleBatch, TriangleVertex};

/// A color represented as RGBA floats [0.0, 1.0]
pub type Color4 = [f32; 4];

/// A 2D rectangle in pixel coordinates
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn min(&self) -> [f32; 2] {
        [self.x, self.y]
    }

    pub fn max(&self) -> [f32; 2] {
        [self.x + self.width, self.y + self.height]
    }
}
