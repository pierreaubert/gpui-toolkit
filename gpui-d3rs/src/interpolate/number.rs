//! Numeric interpolation functions
//!
//! Provides interpolation functions for numeric values.

use std::ops::{Add, Mul, Sub};

/// Creates an interpolator between two f64 values.
///
/// Returns a function that takes a parameter t in [0, 1] and returns
/// the interpolated value.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate;
///
/// let lerp = interpolate(0.0, 100.0);
/// assert_eq!(lerp(0.0), 0.0);
/// assert_eq!(lerp(0.5), 50.0);
/// assert_eq!(lerp(1.0), 100.0);
/// ```
pub fn interpolate(a: f64, b: f64) -> impl Fn(f64) -> f64 {
    move |t| a + (b - a) * t
}

/// Creates an interpolator between two f32 values.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate_f32;
///
/// let lerp = interpolate_f32(0.0, 100.0);
/// assert_eq!(lerp(0.5), 50.0);
/// ```
pub fn interpolate_f32(a: f32, b: f32) -> impl Fn(f32) -> f32 {
    move |t| a + (b - a) * t
}

/// Creates an interpolator that rounds the result to the nearest integer.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate_round;
///
/// let lerp = interpolate_round(0, 10);
/// assert_eq!(lerp(0.0), 0);
/// assert_eq!(lerp(0.25), 3);
/// assert_eq!(lerp(0.5), 5);
/// assert_eq!(lerp(1.0), 10);
/// ```
pub fn interpolate_round(a: i64, b: i64) -> impl Fn(f64) -> i64 {
    let a_f = a as f64;
    let b_f = b as f64;
    move |t| (a_f + (b_f - a_f) * t).round() as i64
}

/// Creates an interpolator that rounds the result to the nearest i32.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate_round_i32;
///
/// let lerp = interpolate_round_i32(0, 100);
/// assert_eq!(lerp(0.5), 50);
/// ```
pub fn interpolate_round_i32(a: i32, b: i32) -> impl Fn(f64) -> i32 {
    let a_f = a as f64;
    let b_f = b as f64;
    move |t| (a_f + (b_f - a_f) * t).round() as i32
}

/// Interpolation trait for generic types.
pub trait Interpolate: Clone {
    /// Interpolate between self and other at parameter t.
    fn interpolate(&self, other: &Self, t: f64) -> Self;
}

impl Interpolate for f64 {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        self + (other - self) * t
    }
}

impl Interpolate for f32 {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        self + (other - self) * t as f32
    }
}

impl Interpolate for i64 {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        ((*self as f64) + ((*other - *self) as f64) * t).round() as i64
    }
}

impl Interpolate for i32 {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        ((*self as f64) + ((*other - *self) as f64) * t).round() as i32
    }
}

/// Generic linear interpolation (lerp) function.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::lerp;
///
/// assert_eq!(lerp(0.0_f64, 100.0_f64, 0.5), 50.0);
/// ```
pub fn lerp<T>(a: T, b: T, t: f64) -> T
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<f64, Output = T>,
{
    a + (b - a) * t
}

/// Clamp a value to the range [0, 1].
///
/// # Example
///
/// ```
/// use d3rs::interpolate::clamp01;
///
/// assert_eq!(clamp01(-0.5), 0.0);
/// assert_eq!(clamp01(0.5), 0.5);
/// assert_eq!(clamp01(1.5), 1.0);
/// ```
pub fn clamp01(t: f64) -> f64 {
    t.clamp(0.0, 1.0)
}

/// Creates an interpolator with clamping to [0, 1].
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate_clamped;
///
/// let lerp = interpolate_clamped(0.0, 100.0);
/// assert_eq!(lerp(-0.5), 0.0);  // Clamped to 0
/// assert_eq!(lerp(0.5), 50.0);
/// assert_eq!(lerp(1.5), 100.0); // Clamped to 1
/// ```
pub fn interpolate_clamped(a: f64, b: f64) -> impl Fn(f64) -> f64 {
    move |t| {
        let t = t.clamp(0.0, 1.0);
        a + (b - a) * t
    }
}

/// Creates a basis spline interpolator (smooth interpolation through points).
///
/// The basis function provides smooth C2-continuous interpolation.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate_basis;
///
/// let values = vec![0.0, 10.0, 20.0, 10.0, 0.0];
/// let interp = interpolate_basis(&values);
/// let mid = interp(0.5);
/// assert!(mid > 0.0 && mid < 20.0); // Smooth curve through points
/// ```
pub fn interpolate_basis(values: &[f64]) -> impl Fn(f64) -> f64 + '_ {
    let n = values.len();
    move |t| {
        if n == 0 {
            return 0.0;
        }
        if n == 1 {
            return values[0];
        }

        let t = t.clamp(0.0, 1.0);
        let scaled = t * (n - 1) as f64;
        let i = (scaled.floor() as usize).min(n - 2);
        let local_t = scaled - i as f64;

        basis_point(
            if i > 0 {
                values[i - 1]
            } else {
                2.0 * values[0] - values[1]
            },
            values[i],
            values[i + 1],
            if i < n - 2 {
                values[i + 2]
            } else {
                2.0 * values[n - 1] - values[n - 2]
            },
            local_t,
        )
    }
}

/// Compute a basis spline point.
fn basis_point(v0: f64, v1: f64, v2: f64, v3: f64, t: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;
    ((1.0 - 3.0 * t + 3.0 * t2 - t3) * v0
        + (4.0 - 6.0 * t2 + 3.0 * t3) * v1
        + (1.0 + 3.0 * t + 3.0 * t2 - 3.0 * t3) * v2
        + t3 * v3)
        / 6.0
}

/// Creates a closed basis spline interpolator (loops back to start).
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate_basis_closed;
///
/// let values = vec![0.0, 10.0, 20.0, 10.0];
/// let interp = interpolate_basis_closed(&values);
/// let start = interp(0.0);
/// let end = interp(1.0);
/// // For closed spline, start and end should be similar
/// assert!((start - end).abs() < 1.0);
/// ```
pub fn interpolate_basis_closed(values: &[f64]) -> impl Fn(f64) -> f64 + '_ {
    let n = values.len();
    move |t| {
        if n == 0 {
            return 0.0;
        }
        if n == 1 {
            return values[0];
        }

        let t = t - t.floor(); // Wrap to [0, 1)
        let scaled = t * n as f64;
        let i = scaled.floor() as usize % n;
        let local_t = scaled - scaled.floor();

        basis_point(
            values[(i + n - 1) % n],
            values[i],
            values[(i + 1) % n],
            values[(i + 2) % n],
            local_t,
        )
    }
}

/// Exponential interpolation (for geometric progressions).
///
/// Useful for scales that should grow/shrink exponentially.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate_exp;
///
/// let interp = interpolate_exp(1.0, 100.0);
/// assert!((interp(0.0) - 1.0).abs() < 0.001);
/// assert!((interp(1.0) - 100.0).abs() < 0.001);
/// // Midpoint should be geometric mean: sqrt(1 * 100) = 10
/// assert!((interp(0.5) - 10.0).abs() < 0.001);
/// ```
pub fn interpolate_exp(a: f64, b: f64) -> impl Fn(f64) -> f64 {
    let log_a = a.ln();
    let log_b = b.ln();
    move |t| (log_a + (log_b - log_a) * t).exp()
}

/// Discrete interpolation (step function).
///
/// Returns values from a discrete set based on t.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate_discrete;
///
/// let values = vec!["a", "b", "c", "d"];
/// let interp = interpolate_discrete(&values);
/// assert_eq!(interp(0.0), "a");
/// assert_eq!(interp(0.25), "b");
/// assert_eq!(interp(0.5), "c");
/// assert_eq!(interp(0.75), "d");
/// ```
pub fn interpolate_discrete<T: Clone>(values: &[T]) -> impl Fn(f64) -> T + '_ {
    let n = values.len();
    move |t| {
        if n == 0 {
            panic!("interpolate_discrete requires at least one value");
        }
        let t = t.clamp(0.0, 1.0);
        let i = ((t * n as f64).floor() as usize).min(n - 1);
        values[i].clone()
    }
}

/// Quantize interpolation to n discrete levels.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate_quantize;
///
/// let interp = interpolate_quantize(0.0, 100.0, 5);
/// assert_eq!(interp(0.0), 0.0);
/// assert_eq!(interp(0.15), 0.0);  // Still in first bucket
/// assert_eq!(interp(0.25), 25.0); // Second bucket
/// assert_eq!(interp(0.5), 50.0);
/// ```
pub fn interpolate_quantize(a: f64, b: f64, n: usize) -> impl Fn(f64) -> f64 {
    move |t| {
        if n <= 1 {
            return a;
        }
        let t = t.clamp(0.0, 1.0);
        let step = 1.0 / n as f64;
        let bucket = (t / step).floor() as usize;
        let bucket = bucket.min(n - 1);
        a + (b - a) * (bucket as f64 / (n - 1) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate() {
        let lerp = interpolate(0.0, 100.0);
        assert_eq!(lerp(0.0), 0.0);
        assert_eq!(lerp(0.5), 50.0);
        assert_eq!(lerp(1.0), 100.0);
    }

    #[test]
    fn test_interpolate_round() {
        let lerp = interpolate_round(0, 10);
        assert_eq!(lerp(0.0), 0);
        assert_eq!(lerp(0.25), 3);
        assert_eq!(lerp(0.5), 5);
        assert_eq!(lerp(1.0), 10);
    }

    #[test]
    fn test_interpolate_trait() {
        let a = 0.0_f64;
        let b = 100.0_f64;
        assert_eq!(a.interpolate(&b, 0.5), 50.0);
    }

    #[test]
    fn test_interpolate_clamped() {
        let lerp = interpolate_clamped(0.0, 100.0);
        assert_eq!(lerp(-0.5), 0.0);
        assert_eq!(lerp(1.5), 100.0);
    }

    #[test]
    fn test_interpolate_basis() {
        let values = vec![0.0, 10.0, 20.0];
        let interp = interpolate_basis(&values);
        let mid = interp(0.5);
        assert!(mid > 5.0 && mid < 15.0);
    }

    #[test]
    fn test_interpolate_exp() {
        let interp = interpolate_exp(1.0, 100.0);
        assert!((interp(0.0) - 1.0).abs() < 0.001);
        assert!((interp(1.0) - 100.0).abs() < 0.001);
        assert!((interp(0.5) - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_interpolate_discrete() {
        let values = vec![1, 2, 3, 4];
        let interp = interpolate_discrete(&values);
        assert_eq!(interp(0.0), 1);
        assert_eq!(interp(0.99), 4);
    }

    #[test]
    fn test_interpolate_quantize() {
        let interp = interpolate_quantize(0.0, 100.0, 5);
        assert_eq!(interp(0.0), 0.0);
        assert_eq!(interp(0.25), 25.0);
        assert_eq!(interp(0.5), 50.0);
        assert_eq!(interp(0.75), 75.0);
        assert_eq!(interp(1.0), 100.0);
    }
}
