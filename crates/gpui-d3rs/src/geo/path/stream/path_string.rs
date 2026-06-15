//! Path string sink.
//!
//! Mirrors `d3-geo/src/path/string.js`: accumulates streamed points into an SVG
//! path data string.

use std::fmt::Write;

use crate::geo::stream::Stream;

/// Stream sink that builds an SVG path data string.
pub struct PathString {
    buf: String,
    digits: usize,
    point_radius: f64,
    /// 0 = first point of a line, 1 = subsequent point of a line,
    /// NaN = outside a line (used for point features).
    point_state: PointState,
    /// 0 = inside a polygon, NaN = outside a polygon.
    line_state: LineState,
}

#[derive(Clone, Copy)]
enum PointState {
    First,
    Cont,
    Feature,
}

#[derive(Clone, Copy)]
enum LineState {
    Polygon,
    Other,
}

impl PathString {
    pub fn new(digits: usize, point_radius: f64) -> Self {
        Self {
            buf: String::new(),
            digits,
            point_radius,
            point_state: PointState::Feature,
            line_state: LineState::Other,
        }
    }

    pub fn result(&mut self) -> String {
        std::mem::take(&mut self.buf)
    }

    fn append(&mut self, x: f64, y: f64) {
        if self.digits == 0 {
            write!(self.buf, "{},{}", x, y).unwrap();
        } else {
            let k = 10f64.powi(self.digits as i32);
            let xr = (x * k).round() / k;
            let yr = (y * k).round() / k;
            write!(self.buf, "{},{}", xr, yr).unwrap();
        }
    }

    fn append_circle(&mut self) {
        let r = self.point_radius;
        let two_r = -2.0 * r;
        write!(
            self.buf,
            "m0,{r}a{r},{r} 0 1,1 0,{two_r}a{r},{r} 0 1,1 0,{neg_two_r}z",
            r = r,
            two_r = two_r,
            neg_two_r = -two_r
        )
        .unwrap();
    }
}

impl Stream for PathString {
    fn point(&mut self, x: f64, y: f64, _m: i32) {
        match self.point_state {
            PointState::First => {
                self.buf.push('M');
                self.append(x, y);
                self.point_state = PointState::Cont;
            }
            PointState::Cont => {
                self.buf.push('L');
                self.append(x, y);
            }
            PointState::Feature => {
                self.buf.push('M');
                self.append(x, y);
                self.append_circle();
            }
        }
    }

    fn line_start(&mut self) {
        self.point_state = PointState::First;
    }

    fn line_end(&mut self) {
        if matches!(self.line_state, LineState::Polygon) {
            self.buf.push('Z');
        }
        self.point_state = PointState::Feature;
    }

    fn polygon_start(&mut self) {
        self.line_state = LineState::Polygon;
    }

    fn polygon_end(&mut self) {
        self.line_state = LineState::Other;
    }

    fn sphere(&mut self) {
        // Not used for path strings in the current scope.
    }
}
