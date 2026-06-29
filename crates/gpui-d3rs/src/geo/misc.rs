use std::f64::consts::PI;

/// Converts degrees to radians
#[inline]
pub fn radians(degrees: f64) -> f64 {
    degrees * PI / 180.0
}

/// Converts radians to degrees
#[inline]
pub fn degrees(radians: f64) -> f64 {
    radians * 180.0 / PI
}
