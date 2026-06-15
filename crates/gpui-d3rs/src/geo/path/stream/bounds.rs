//! Bounds stream sink.
//!
//! Mirrors `d3-geo/src/path/bounds.js`: tracks the min/max x/y of every streamed
//! point.

use crate::geo::stream::Stream;

/// Stream sink that computes a projected bounding box.
pub struct BoundsStream {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl BoundsStream {
    pub fn new() -> Self {
        Self {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        }
    }

    pub fn result(&self) -> ((f64, f64), (f64, f64)) {
        if self.min_x == f64::INFINITY {
            ((f64::NAN, f64::NAN), (f64::NAN, f64::NAN))
        } else {
            ((self.min_x, self.min_y), (self.max_x, self.max_y))
        }
    }
}

impl Stream for BoundsStream {
    fn point(&mut self, x: f64, y: f64, _m: i32) {
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        if x < self.min_x {
            self.min_x = x;
        }
        if x > self.max_x {
            self.max_x = x;
        }
        if y < self.min_y {
            self.min_y = y;
        }
        if y > self.max_y {
            self.max_y = y;
        }
    }

    fn line_start(&mut self) {}
    fn line_end(&mut self) {}
    fn polygon_start(&mut self) {}
    fn polygon_end(&mut self) {}
    fn sphere(&mut self) {}
}
