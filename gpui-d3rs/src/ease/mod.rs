//! Easing functions (d3-ease)
//!
//! This module provides easing functions for smooth animations.
//! Easing functions take a normalized time t in [0, 1] and return
//! a value typically in [0, 1] representing the progress of the animation.
//!
//! Each easing type has three variants:
//! - `ease_in_*`: Slow at the start
//! - `ease_out_*`: Slow at the end
//! - `ease_in_out_*`: Slow at both ends
//!
//! # Example
//!
//! ```
//! use d3rs::ease::{ease_cubic_in_out, ease_bounce_out};
//!
//! // Smooth cubic easing
//! assert!((ease_cubic_in_out(0.0) - 0.0).abs() < 1e-6);
//! assert!((ease_cubic_in_out(0.5) - 0.5).abs() < 1e-6);
//! assert!((ease_cubic_in_out(1.0) - 1.0).abs() < 1e-6);
//!
//! // Bouncy ending
//! assert!((ease_bounce_out(1.0) - 1.0).abs() < 1e-6);
//! ```

use std::f64::consts::PI;

// ============================================================================
// LINEAR
// ============================================================================

/// Linear easing (no easing, constant speed)
pub fn ease_linear(t: f64) -> f64 {
    t
}

// ============================================================================
// QUADRATIC (t^2)
// ============================================================================

/// Quadratic ease-in
pub fn ease_quad_in(t: f64) -> f64 {
    t * t
}

/// Quadratic ease-out
pub fn ease_quad_out(t: f64) -> f64 {
    t * (2.0 - t)
}

/// Quadratic ease-in-out
pub fn ease_quad_in_out(t: f64) -> f64 {
    let t = t * 2.0;
    if t <= 1.0 {
        t * t / 2.0
    } else {
        let t = t - 1.0;
        (t * (2.0 - t) + 1.0) / 2.0
    }
}

// ============================================================================
// CUBIC (t^3)
// ============================================================================

/// Cubic ease-in
pub fn ease_cubic_in(t: f64) -> f64 {
    t * t * t
}

/// Cubic ease-out
pub fn ease_cubic_out(t: f64) -> f64 {
    let t = t - 1.0;
    t * t * t + 1.0
}

/// Cubic ease-in-out
pub fn ease_cubic_in_out(t: f64) -> f64 {
    let t = t * 2.0;
    if t <= 1.0 {
        t * t * t / 2.0
    } else {
        let t = t - 2.0;
        (t * t * t + 2.0) / 2.0
    }
}

// ============================================================================
// POLYNOMIAL (t^e)
// ============================================================================

/// Create a polynomial ease-in function with custom exponent
pub fn ease_poly_in(exponent: f64) -> impl Fn(f64) -> f64 {
    move |t: f64| t.powf(exponent)
}

/// Create a polynomial ease-out function with custom exponent
pub fn ease_poly_out(exponent: f64) -> impl Fn(f64) -> f64 {
    move |t: f64| 1.0 - (1.0 - t).powf(exponent)
}

/// Create a polynomial ease-in-out function with custom exponent
pub fn ease_poly_in_out(exponent: f64) -> impl Fn(f64) -> f64 {
    move |t: f64| {
        let t = t * 2.0;
        if t <= 1.0 {
            t.powf(exponent) / 2.0
        } else {
            (2.0 - (2.0 - t).powf(exponent)) / 2.0
        }
    }
}

// ============================================================================
// SINUSOIDAL
// ============================================================================

/// Sinusoidal ease-in
pub fn ease_sin_in(t: f64) -> f64 {
    1.0 - (t * PI / 2.0).cos()
}

/// Sinusoidal ease-out
pub fn ease_sin_out(t: f64) -> f64 {
    (t * PI / 2.0).sin()
}

/// Sinusoidal ease-in-out
pub fn ease_sin_in_out(t: f64) -> f64 {
    (1.0 - (t * PI).cos()) / 2.0
}

// ============================================================================
// EXPONENTIAL
// ============================================================================

/// Exponential ease-in
pub fn ease_exp_in(t: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else {
        2.0_f64.powf(10.0 * t - 10.0)
    }
}

/// Exponential ease-out
pub fn ease_exp_out(t: f64) -> f64 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - 2.0_f64.powf(-10.0 * t)
    }
}

/// Exponential ease-in-out
pub fn ease_exp_in_out(t: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let t = t * 2.0;
    if t <= 1.0 {
        2.0_f64.powf(10.0 * t - 10.0) / 2.0
    } else {
        (2.0 - 2.0_f64.powf(10.0 - 10.0 * t)) / 2.0
    }
}

// ============================================================================
// CIRCULAR
// ============================================================================

/// Circular ease-in
pub fn ease_circle_in(t: f64) -> f64 {
    1.0 - (1.0 - t * t).sqrt()
}

/// Circular ease-out
pub fn ease_circle_out(t: f64) -> f64 {
    let t = t - 1.0;
    (1.0 - t * t).sqrt()
}

/// Circular ease-in-out
pub fn ease_circle_in_out(t: f64) -> f64 {
    let t = t * 2.0;
    if t <= 1.0 {
        (1.0 - (1.0 - t * t).sqrt()) / 2.0
    } else {
        let t = t - 2.0;
        ((1.0 - t * t).sqrt() + 1.0) / 2.0
    }
}

// ============================================================================
// ELASTIC
// ============================================================================

const TAU: f64 = 2.0 * PI;

/// Create an elastic ease-in with configurable amplitude and period
pub fn ease_elastic_in_with(amplitude: f64, period: f64) -> impl Fn(f64) -> f64 {
    let a = amplitude.max(1.0);
    let p = period / TAU;
    let s = (1.0 / a).asin() * p;
    move |t: f64| {
        if t <= 0.0 {
            return 0.0;
        }
        if t >= 1.0 {
            return 1.0;
        }
        a * 2.0_f64.powf(10.0 * t - 10.0) * ((s - t + 1.0) / p).sin()
    }
}

/// Create an elastic ease-out with configurable amplitude and period
pub fn ease_elastic_out_with(amplitude: f64, period: f64) -> impl Fn(f64) -> f64 {
    let a = amplitude.max(1.0);
    let p = period / TAU;
    let s = (1.0 / a).asin() * p;
    move |t: f64| {
        if t <= 0.0 {
            return 0.0;
        }
        if t >= 1.0 {
            return 1.0;
        }
        1.0 - a * 2.0_f64.powf(-10.0 * t) * ((t + s) / p).sin()
    }
}

/// Elastic ease-in with default parameters
pub fn ease_elastic_in(t: f64) -> f64 {
    ease_elastic_in_with(1.0, 0.3)(t)
}

/// Elastic ease-out with default parameters
pub fn ease_elastic_out(t: f64) -> f64 {
    ease_elastic_out_with(1.0, 0.3)(t)
}

/// Elastic ease-in-out with default parameters
pub fn ease_elastic_in_out(t: f64) -> f64 {
    let t = t * 2.0;
    if t <= 1.0 {
        ease_elastic_in(t) / 2.0
    } else {
        (ease_elastic_out(t - 1.0) + 1.0) / 2.0
    }
}

// ============================================================================
// BACK (overshoot)
// ============================================================================

const BACK_S: f64 = 1.70158;

/// Create a back ease-in with configurable overshoot
pub fn ease_back_in_with(s: f64) -> impl Fn(f64) -> f64 {
    move |t: f64| t * t * ((s + 1.0) * t - s)
}

/// Create a back ease-out with configurable overshoot
pub fn ease_back_out_with(s: f64) -> impl Fn(f64) -> f64 {
    move |t: f64| {
        let t = t - 1.0;
        t * t * ((s + 1.0) * t + s) + 1.0
    }
}

/// Create a back ease-in-out with configurable overshoot
pub fn ease_back_in_out_with(s: f64) -> impl Fn(f64) -> f64 {
    move |t: f64| {
        let t = t * 2.0;
        if t < 1.0 {
            t * t * ((s + 1.0) * t - s) / 2.0
        } else {
            let t = t - 2.0;
            (t * t * ((s + 1.0) * t + s) + 2.0) / 2.0
        }
    }
}

/// Back ease-in with default overshoot
pub fn ease_back_in(t: f64) -> f64 {
    ease_back_in_with(BACK_S)(t)
}

/// Back ease-out with default overshoot
pub fn ease_back_out(t: f64) -> f64 {
    ease_back_out_with(BACK_S)(t)
}

/// Back ease-in-out with default overshoot
pub fn ease_back_in_out(t: f64) -> f64 {
    ease_back_in_out_with(BACK_S)(t)
}

// ============================================================================
// BOUNCE
// ============================================================================

const B1: f64 = 4.0 / 11.0;
const B2: f64 = 6.0 / 11.0;
const B3: f64 = 8.0 / 11.0;
const B4: f64 = 3.0 / 4.0;
const B5: f64 = 9.0 / 11.0;
const B6: f64 = 10.0 / 11.0;
const B7: f64 = 15.0 / 16.0;
const B8: f64 = 21.0 / 22.0;
const B9: f64 = 63.0 / 64.0;
const B0: f64 = 1.0 / B1 / B1;

/// Bounce ease-out
pub fn ease_bounce_out(t: f64) -> f64 {
    if t < B1 {
        B0 * t * t
    } else if t < B3 {
        let t = t - B2;
        B0 * t * t + B4
    } else if t < B6 {
        let t = t - B5;
        B0 * t * t + B7
    } else {
        let t = t - B8;
        B0 * t * t + B9
    }
}

/// Bounce ease-in
pub fn ease_bounce_in(t: f64) -> f64 {
    1.0 - ease_bounce_out(1.0 - t)
}

/// Bounce ease-in-out
pub fn ease_bounce_in_out(t: f64) -> f64 {
    let t = t * 2.0;
    if t <= 1.0 {
        (1.0 - ease_bounce_out(1.0 - t)) / 2.0
    } else {
        (ease_bounce_out(t - 1.0) + 1.0) / 2.0
    }
}

// ============================================================================
// EASING TYPE ENUM
// ============================================================================

/// Enumeration of all easing types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EaseType {
    Linear,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    SinIn,
    SinOut,
    SinInOut,
    ExpIn,
    ExpOut,
    ExpInOut,
    CircleIn,
    CircleOut,
    CircleInOut,
    ElasticIn,
    ElasticOut,
    ElasticInOut,
    BackIn,
    BackOut,
    BackInOut,
    BounceIn,
    BounceOut,
    BounceInOut,
}

impl EaseType {
    /// Apply the easing function to a value
    pub fn ease(&self, t: f64) -> f64 {
        match self {
            EaseType::Linear => ease_linear(t),
            EaseType::QuadIn => ease_quad_in(t),
            EaseType::QuadOut => ease_quad_out(t),
            EaseType::QuadInOut => ease_quad_in_out(t),
            EaseType::CubicIn => ease_cubic_in(t),
            EaseType::CubicOut => ease_cubic_out(t),
            EaseType::CubicInOut => ease_cubic_in_out(t),
            EaseType::SinIn => ease_sin_in(t),
            EaseType::SinOut => ease_sin_out(t),
            EaseType::SinInOut => ease_sin_in_out(t),
            EaseType::ExpIn => ease_exp_in(t),
            EaseType::ExpOut => ease_exp_out(t),
            EaseType::ExpInOut => ease_exp_in_out(t),
            EaseType::CircleIn => ease_circle_in(t),
            EaseType::CircleOut => ease_circle_out(t),
            EaseType::CircleInOut => ease_circle_in_out(t),
            EaseType::ElasticIn => ease_elastic_in(t),
            EaseType::ElasticOut => ease_elastic_out(t),
            EaseType::ElasticInOut => ease_elastic_in_out(t),
            EaseType::BackIn => ease_back_in(t),
            EaseType::BackOut => ease_back_out(t),
            EaseType::BackInOut => ease_back_in_out(t),
            EaseType::BounceIn => ease_bounce_in(t),
            EaseType::BounceOut => ease_bounce_out(t),
            EaseType::BounceInOut => ease_bounce_in_out(t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn test_linear() {
        assert!(approx_eq(ease_linear(0.0), 0.0));
        assert!(approx_eq(ease_linear(0.5), 0.5));
        assert!(approx_eq(ease_linear(1.0), 1.0));
    }

    #[test]
    fn test_quad() {
        assert!(approx_eq(ease_quad_in(0.0), 0.0));
        assert!(approx_eq(ease_quad_in(1.0), 1.0));
        assert!(approx_eq(ease_quad_out(0.0), 0.0));
        assert!(approx_eq(ease_quad_out(1.0), 1.0));
        assert!(approx_eq(ease_quad_in_out(0.0), 0.0));
        assert!(approx_eq(ease_quad_in_out(0.5), 0.5));
        assert!(approx_eq(ease_quad_in_out(1.0), 1.0));
    }

    #[test]
    fn test_cubic() {
        assert!(approx_eq(ease_cubic_in(0.0), 0.0));
        assert!(approx_eq(ease_cubic_in(1.0), 1.0));
        assert!(approx_eq(ease_cubic_out(0.0), 0.0));
        assert!(approx_eq(ease_cubic_out(1.0), 1.0));
        assert!(approx_eq(ease_cubic_in_out(0.0), 0.0));
        assert!(approx_eq(ease_cubic_in_out(0.5), 0.5));
        assert!(approx_eq(ease_cubic_in_out(1.0), 1.0));
    }

    #[test]
    fn test_sin() {
        assert!(approx_eq(ease_sin_in(0.0), 0.0));
        assert!(approx_eq(ease_sin_in(1.0), 1.0));
        assert!(approx_eq(ease_sin_out(0.0), 0.0));
        assert!(approx_eq(ease_sin_out(1.0), 1.0));
        assert!(approx_eq(ease_sin_in_out(0.0), 0.0));
        assert!(approx_eq(ease_sin_in_out(0.5), 0.5));
        assert!(approx_eq(ease_sin_in_out(1.0), 1.0));
    }

    #[test]
    fn test_exp() {
        assert!(approx_eq(ease_exp_in(0.0), 0.0));
        assert!(approx_eq(ease_exp_in(1.0), 1.0));
        assert!(approx_eq(ease_exp_out(0.0), 0.0));
        assert!(approx_eq(ease_exp_out(1.0), 1.0));
        assert!(approx_eq(ease_exp_in_out(0.0), 0.0));
        assert!(approx_eq(ease_exp_in_out(1.0), 1.0));
    }

    #[test]
    fn test_circle() {
        assert!(approx_eq(ease_circle_in(0.0), 0.0));
        assert!(approx_eq(ease_circle_in(1.0), 1.0));
        assert!(approx_eq(ease_circle_out(0.0), 0.0));
        assert!(approx_eq(ease_circle_out(1.0), 1.0));
        assert!(approx_eq(ease_circle_in_out(0.0), 0.0));
        assert!(approx_eq(ease_circle_in_out(0.5), 0.5));
        assert!(approx_eq(ease_circle_in_out(1.0), 1.0));
    }

    #[test]
    fn test_elastic() {
        assert!(approx_eq(ease_elastic_in(0.0), 0.0));
        assert!(approx_eq(ease_elastic_in(1.0), 1.0));
        assert!(approx_eq(ease_elastic_out(0.0), 0.0));
        assert!(approx_eq(ease_elastic_out(1.0), 1.0));
    }

    #[test]
    fn test_back() {
        assert!(approx_eq(ease_back_in(0.0), 0.0));
        assert!(approx_eq(ease_back_in(1.0), 1.0));
        assert!(approx_eq(ease_back_out(0.0), 0.0));
        assert!(approx_eq(ease_back_out(1.0), 1.0));
        // Back goes outside [0, 1] during animation
        assert!(ease_back_in(0.5) < 0.0);
        assert!(ease_back_out(0.5) > 1.0);
    }

    #[test]
    fn test_bounce() {
        assert!(approx_eq(ease_bounce_in(0.0), 0.0));
        assert!(approx_eq(ease_bounce_in(1.0), 1.0));
        assert!(approx_eq(ease_bounce_out(0.0), 0.0));
        assert!(approx_eq(ease_bounce_out(1.0), 1.0));
        assert!(approx_eq(ease_bounce_in_out(0.0), 0.0));
        assert!(approx_eq(ease_bounce_in_out(1.0), 1.0));
    }

    #[test]
    fn test_poly() {
        let poly3_in = ease_poly_in(3.0);
        assert!(approx_eq(poly3_in(0.0), 0.0));
        assert!(approx_eq(poly3_in(1.0), 1.0));
        assert!(approx_eq(poly3_in(0.5), ease_cubic_in(0.5)));
    }

    #[test]
    fn test_ease_type_enum() {
        assert!(approx_eq(EaseType::Linear.ease(0.5), 0.5));
        assert!(approx_eq(EaseType::CubicInOut.ease(0.0), 0.0));
        assert!(approx_eq(EaseType::CubicInOut.ease(1.0), 1.0));
    }
}
