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

mod consts;
mod ease_type;
#[cfg(test)]
mod tests;

pub use ease_type::*;

use consts::B0;
use consts::B1;
use consts::B2;
use consts::B3;
use consts::B4;
use consts::B5;
use consts::B6;
use consts::B7;
use consts::B8;
use consts::B9;
use consts::BACK_S;
use consts::TAU;

/// Linear easing (no easing, constant speed)
pub fn ease_linear(t: f64) -> f64 {
    t
}

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
