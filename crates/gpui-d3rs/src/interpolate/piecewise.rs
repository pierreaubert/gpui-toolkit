//! Piecewise interpolation
//!
//! Provides interpolation across multiple values using piecewise functions.

use super::number::Interpolate;

/// Create a piecewise interpolator across multiple values.
///
/// Given n values, the interpolator divides [0, 1] into n-1 segments
/// and interpolates linearly within each segment.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::piecewise;
///
/// let values = vec![0.0, 50.0, 100.0];
/// let interp = piecewise(&values);
///
/// assert_eq!(interp(0.0), 0.0);
/// assert_eq!(interp(0.25), 25.0);  // Halfway through first segment
/// assert_eq!(interp(0.5), 50.0);   // At second value
/// assert_eq!(interp(0.75), 75.0);  // Halfway through second segment
/// assert_eq!(interp(1.0), 100.0);
/// ```
pub fn piecewise<T: Interpolate>(values: &[T]) -> impl Fn(f64) -> T + '_ {
    let n = values.len();
    move |t| {
        if n == 0 {
            panic!("piecewise requires at least one value");
        }
        if n == 1 {
            return values[0].clone();
        }

        let t = t.clamp(0.0, 1.0);
        let segments = n - 1;
        let scaled = t * segments as f64;
        let i = (scaled.floor() as usize).min(segments - 1);
        let local_t = scaled - i as f64;

        values[i].interpolate(&values[i + 1], local_t)
    }
}

/// Create a piecewise interpolator with custom interpolation function.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::piecewise_with;
///
/// let values = vec![0.0_f64, 100.0_f64, 200.0_f64];
/// let interp = piecewise_with(&values, |a: &f64, b: &f64, t: f64| {
///     // Linear interpolation
///     a + (b - a) * t
/// });
///
/// let mid = interp(0.5);
/// assert_eq!(mid, 100.0); // At the middle value
/// ```
pub fn piecewise_with<'a, T, F>(values: &'a [T], interpolator: F) -> impl Fn(f64) -> T + 'a
where
    T: Clone,
    F: Fn(&T, &T, f64) -> T + 'a,
{
    let n = values.len();
    move |t| {
        if n == 0 {
            panic!("piecewise requires at least one value");
        }
        if n == 1 {
            return values[0].clone();
        }

        let t = t.clamp(0.0, 1.0);
        let segments = n - 1;
        let scaled = t * segments as f64;
        let i = (scaled.floor() as usize).min(segments - 1);
        let local_t = scaled - i as f64;

        interpolator(&values[i], &values[i + 1], local_t)
    }
}

/// Create a piecewise interpolator with explicit domain positions.
///
/// Unlike `piecewise` which divides [0, 1] evenly, this allows
/// specifying the exact position of each value.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::piecewise_domain;
///
/// let positions = vec![0.0, 0.3, 1.0];  // Values positioned unevenly
/// let values = vec![0.0, 100.0, 200.0];
///
/// let interp = piecewise_domain(&positions, &values);
///
/// assert_eq!(interp(0.0), 0.0);
/// assert_eq!(interp(0.3), 100.0);  // Exactly at position
/// assert_eq!(interp(1.0), 200.0);
/// ```
pub fn piecewise_domain<'a, T: Interpolate>(
    positions: &'a [f64],
    values: &'a [T],
) -> impl Fn(f64) -> T + 'a {
    let n = positions.len().min(values.len());
    move |t| {
        if n == 0 {
            panic!("piecewise_domain requires at least one value");
        }
        if n == 1 {
            return values[0].clone();
        }

        let t = t.clamp(positions[0], positions[n - 1]);

        // Find the segment containing t
        let mut i = 0;
        while i < n - 1 && positions[i + 1] < t {
            i += 1;
        }
        i = i.min(n - 2);

        // Calculate local t within segment
        let segment_start = positions[i];
        let segment_end = positions[i + 1];
        let segment_length = segment_end - segment_start;

        let local_t = if segment_length.abs() < f64::EPSILON {
            0.0
        } else {
            (t - segment_start) / segment_length
        };

        values[i].interpolate(&values[i + 1], local_t)
    }
}

/// Quantize interpolation into discrete steps.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::quantize;
///
/// let values = vec!["A", "B", "C", "D"];
/// let interp = quantize(&values);
///
/// assert_eq!(interp(0.0), "A");
/// assert_eq!(interp(0.24), "A");
/// assert_eq!(interp(0.25), "B");
/// assert_eq!(interp(0.5), "C");
/// assert_eq!(interp(0.75), "D");
/// assert_eq!(interp(1.0), "D");
/// ```
pub fn quantize<T: Clone>(values: &[T]) -> impl Fn(f64) -> T + '_ {
    let n = values.len();
    move |t| {
        if n == 0 {
            panic!("quantize requires at least one value");
        }

        let t = t.clamp(0.0, 1.0);
        let i = ((t * n as f64).floor() as usize).min(n - 1);
        values[i].clone()
    }
}

/// Create an eased interpolator using common easing functions.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::{interpolate_ease, EaseFunction};
///
/// let interp = interpolate_ease(0.0, 100.0, EaseFunction::QuadInOut);
///
/// let start = interp(0.0);
/// let mid = interp(0.5);
/// let end = interp(1.0);
///
/// assert!((start - 0.0).abs() < 0.001);
/// assert!((end - 100.0).abs() < 0.001);
/// ```
pub fn interpolate_ease(a: f64, b: f64, ease: EaseFunction) -> impl Fn(f64) -> f64 {
    move |t| {
        let t = ease.apply(t);
        a + (b - a) * t
    }
}

/// Common easing functions.
#[derive(Debug, Clone, Copy)]
pub enum EaseFunction {
    /// Linear (no easing)
    Linear,
    /// Quadratic ease in
    QuadIn,
    /// Quadratic ease out
    QuadOut,
    /// Quadratic ease in-out
    QuadInOut,
    /// Cubic ease in
    CubicIn,
    /// Cubic ease out
    CubicOut,
    /// Cubic ease in-out
    CubicInOut,
    /// Sinusoidal ease in
    SinIn,
    /// Sinusoidal ease out
    SinOut,
    /// Sinusoidal ease in-out
    SinInOut,
    /// Exponential ease in
    ExpIn,
    /// Exponential ease out
    ExpOut,
    /// Exponential ease in-out
    ExpInOut,
    /// Circular ease in
    CircleIn,
    /// Circular ease out
    CircleOut,
    /// Circular ease in-out
    CircleInOut,
    /// Elastic ease in
    ElasticIn,
    /// Elastic ease out
    ElasticOut,
    /// Bounce ease out
    BounceOut,
    /// Back ease in (overshoots)
    BackIn,
    /// Back ease out (overshoots)
    BackOut,
    /// Back ease in-out (overshoots)
    BackInOut,
}

impl EaseFunction {
    /// Apply the easing function to a value t in [0, 1].
    pub fn apply(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            EaseFunction::Linear => t,
            EaseFunction::QuadIn => t * t,
            EaseFunction::QuadOut => t * (2.0 - t),
            EaseFunction::QuadInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            EaseFunction::CubicIn => t * t * t,
            EaseFunction::CubicOut => {
                let t = t - 1.0;
                t * t * t + 1.0
            }
            EaseFunction::CubicInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    let t = t - 1.0;
                    (t * 2.0) * (t * 2.0) * (t * 2.0) / 2.0 + 1.0
                }
            }
            EaseFunction::SinIn => 1.0 - (t * std::f64::consts::FRAC_PI_2).cos(),
            EaseFunction::SinOut => (t * std::f64::consts::FRAC_PI_2).sin(),
            EaseFunction::SinInOut => -(std::f64::consts::PI * t).cos() / 2.0 + 0.5,
            EaseFunction::ExpIn => {
                if t == 0.0 {
                    0.0
                } else {
                    2.0_f64.powf(10.0 * (t - 1.0))
                }
            }
            EaseFunction::ExpOut => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - 2.0_f64.powf(-10.0 * t)
                }
            }
            EaseFunction::ExpInOut => {
                if t == 0.0 {
                    return 0.0;
                }
                if t == 1.0 {
                    return 1.0;
                }
                if t < 0.5 {
                    2.0_f64.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2.0_f64.powf(-20.0 * t + 10.0)) / 2.0
                }
            }
            EaseFunction::CircleIn => 1.0 - (1.0 - t * t).sqrt(),
            EaseFunction::CircleOut => (1.0 - (t - 1.0) * (t - 1.0)).sqrt(),
            EaseFunction::CircleInOut => {
                if t < 0.5 {
                    (1.0 - (1.0 - 4.0 * t * t).sqrt()) / 2.0
                } else {
                    ((1.0 - (2.0 * t - 2.0).powi(2)).sqrt() + 1.0) / 2.0
                }
            }
            EaseFunction::ElasticIn => {
                if t == 0.0 || t == 1.0 {
                    return t;
                }
                let p = 0.3;
                let s = p / 4.0;
                -(2.0_f64.powf(10.0 * (t - 1.0))
                    * ((t - 1.0 - s) * (2.0 * std::f64::consts::PI) / p).sin())
            }
            EaseFunction::ElasticOut => {
                if t == 0.0 || t == 1.0 {
                    return t;
                }
                let p = 0.3;
                let s = p / 4.0;
                2.0_f64.powf(-10.0 * t) * ((t - s) * (2.0 * std::f64::consts::PI) / p).sin() + 1.0
            }
            EaseFunction::BounceOut => {
                if t < 1.0 / 2.75 {
                    7.5625 * t * t
                } else if t < 2.0 / 2.75 {
                    let t = t - 1.5 / 2.75;
                    7.5625 * t * t + 0.75
                } else if t < 2.5 / 2.75 {
                    let t = t - 2.25 / 2.75;
                    7.5625 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / 2.75;
                    7.5625 * t * t + 0.984375
                }
            }
            EaseFunction::BackIn => {
                let s = 1.70158;
                t * t * ((s + 1.0) * t - s)
            }
            EaseFunction::BackOut => {
                let s = 1.70158;
                let t = t - 1.0;
                t * t * ((s + 1.0) * t + s) + 1.0
            }
            EaseFunction::BackInOut => {
                let s = 1.70158 * 1.525;
                if t < 0.5 {
                    let t = t * 2.0;
                    (t * t * ((s + 1.0) * t - s)) / 2.0
                } else {
                    let t = t * 2.0 - 2.0;
                    (t * t * ((s + 1.0) * t + s) + 2.0) / 2.0
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_piecewise() {
        let values = vec![0.0, 50.0, 100.0];
        let interp = piecewise(&values);

        assert_eq!(interp(0.0), 0.0);
        assert_eq!(interp(0.25), 25.0);
        assert_eq!(interp(0.5), 50.0);
        assert_eq!(interp(0.75), 75.0);
        assert_eq!(interp(1.0), 100.0);
    }

    #[test]
    fn test_piecewise_domain() {
        let positions = vec![0.0, 0.3, 1.0];
        let values = vec![0.0, 100.0, 200.0];

        let interp = piecewise_domain(&positions, &values);

        assert_eq!(interp(0.0), 0.0);
        assert_eq!(interp(0.3), 100.0);
        assert_eq!(interp(1.0), 200.0);
    }

    #[test]
    fn test_quantize() {
        let values = vec!["A", "B", "C", "D"];
        let interp = quantize(&values);

        assert_eq!(interp(0.0), "A");
        assert_eq!(interp(0.24), "A");
        assert_eq!(interp(0.25), "B");
        assert_eq!(interp(0.5), "C");
        assert_eq!(interp(0.75), "D");
    }

    #[test]
    fn test_ease_functions() {
        // All easing functions should map 0 -> 0 and 1 -> 1
        let functions = vec![
            EaseFunction::Linear,
            EaseFunction::QuadIn,
            EaseFunction::QuadOut,
            EaseFunction::QuadInOut,
            EaseFunction::CubicIn,
            EaseFunction::CubicOut,
            EaseFunction::SinIn,
            EaseFunction::SinOut,
            EaseFunction::ExpIn,
            EaseFunction::ExpOut,
            EaseFunction::CircleIn,
            EaseFunction::CircleOut,
        ];

        for ease in functions {
            assert!(
                (ease.apply(0.0)).abs() < 0.01,
                "Ease {:?} failed at 0",
                ease
            );
            assert!(
                (ease.apply(1.0) - 1.0).abs() < 0.01,
                "Ease {:?} failed at 1",
                ease
            );
        }
    }

    #[test]
    fn test_interpolate_ease() {
        let interp = interpolate_ease(0.0, 100.0, EaseFunction::QuadInOut);

        assert!((interp(0.0) - 0.0).abs() < 0.001);
        assert!((interp(1.0) - 100.0).abs() < 0.001);

        // Midpoint should be 50 for symmetric easing
        assert!((interp(0.5) - 50.0).abs() < 0.001);
    }
}
