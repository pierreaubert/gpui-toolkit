use super::types::Easing;
use std::f32::consts::PI;

/// Apply an easing function to a progress value
///
/// # Arguments
/// * `easing` - The easing function to apply
/// * `t` - Progress value from 0.0 to 1.0
///
/// # Returns
/// Eased progress value (may exceed 0.0-1.0 range for overshoot easings)
pub fn ease(easing: Easing, t: f32) -> f32 {
    match easing {
        Easing::Linear => t,

        // Quadratic
        Easing::EaseInQuad => t * t,
        Easing::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
        Easing::EaseInOutQuad => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }

        // Cubic
        Easing::EaseInCubic => t * t * t,
        Easing::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
        Easing::EaseInOutCubic => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
            }
        }

        // Quartic
        Easing::EaseInQuart => t * t * t * t,
        Easing::EaseOutQuart => 1.0 - (1.0 - t).powi(4),
        Easing::EaseInOutQuart => {
            if t < 0.5 {
                8.0 * t * t * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
            }
        }

        // Quintic
        Easing::EaseInQuint => t * t * t * t * t,
        Easing::EaseOutQuint => 1.0 - (1.0 - t).powi(5),
        Easing::EaseInOutQuint => {
            if t < 0.5 {
                16.0 * t * t * t * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
            }
        }

        // Sine
        Easing::EaseInSine => 1.0 - (t * PI / 2.0).cos(),
        Easing::EaseOutSine => (t * PI / 2.0).sin(),
        Easing::EaseInOutSine => -(((t * PI).cos() - 1.0) / 2.0),

        // Exponential
        Easing::EaseInExpo => {
            if t == 0.0 {
                0.0
            } else {
                2.0_f32.powf(10.0 * t - 10.0)
            }
        }
        Easing::EaseOutExpo => {
            if t == 1.0 {
                1.0
            } else {
                1.0 - 2.0_f32.powf(-10.0 * t)
            }
        }
        Easing::EaseInOutExpo => {
            if t == 0.0 {
                0.0
            } else if t == 1.0 {
                1.0
            } else if t < 0.5 {
                2.0_f32.powf(20.0 * t - 10.0) / 2.0
            } else {
                (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) / 2.0
            }
        }

        // Circular
        Easing::EaseInCirc => 1.0 - (1.0 - t * t).sqrt(),
        Easing::EaseOutCirc => (1.0 - (t - 1.0).powi(2)).sqrt(),
        Easing::EaseInOutCirc => {
            if t < 0.5 {
                (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
            } else {
                ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
            }
        }

        // Back (with overshoot)
        Easing::EaseInBack => {
            let c1 = 1.70158;
            let c3 = c1 + 1.0;
            c3 * t * t * t - c1 * t * t
        }
        Easing::EaseOutBack => {
            let c1 = 1.70158;
            let c3 = c1 + 1.0;
            1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
        }
        Easing::EaseInOutBack => {
            let c1 = 1.70158;
            let c2 = c1 * 1.525;
            if t < 0.5 {
                ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
            } else {
                ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
            }
        }

        // Elastic
        Easing::EaseInElastic => {
            let c4 = (2.0 * PI) / 3.0;
            if t == 0.0 {
                0.0
            } else if t == 1.0 {
                1.0
            } else {
                -(2.0_f32.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * c4).sin()
            }
        }
        Easing::EaseOutElastic => {
            let c4 = (2.0 * PI) / 3.0;
            if t == 0.0 {
                0.0
            } else if t == 1.0 {
                1.0
            } else {
                2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
            }
        }
        Easing::EaseInOutElastic => {
            let c5 = (2.0 * PI) / 4.5;
            if t == 0.0 {
                0.0
            } else if t == 1.0 {
                1.0
            } else if t < 0.5 {
                -(2.0_f32.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c5).sin()) / 2.0
            } else {
                (2.0_f32.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c5).sin()) / 2.0 + 1.0
            }
        }

        // Bounce
        Easing::EaseInBounce => 1.0 - ease(Easing::EaseOutBounce, 1.0 - t),
        Easing::EaseOutBounce => {
            let n1 = 7.5625;
            let d1 = 2.75;
            if t < 1.0 / d1 {
                n1 * t * t
            } else if t < 2.0 / d1 {
                let t = t - 1.5 / d1;
                n1 * t * t + 0.75
            } else if t < 2.5 / d1 {
                let t = t - 2.25 / d1;
                n1 * t * t + 0.9375
            } else {
                let t = t - 2.625 / d1;
                n1 * t * t + 0.984375
            }
        }
        Easing::EaseInOutBounce => {
            if t < 0.5 {
                (1.0 - ease(Easing::EaseOutBounce, 1.0 - 2.0 * t)) / 2.0
            } else {
                (1.0 + ease(Easing::EaseOutBounce, 2.0 * t - 1.0)) / 2.0
            }
        }
    }
}

/// Interpolate between two values using an easing function
pub fn interpolate(from: f32, to: f32, easing: Easing, t: f32) -> f32 {
    let eased = ease(easing, t);
    from + (to - from) * eased
}

/// Interpolate a color between two values
pub fn interpolate_color(from: gpui::Rgba, to: gpui::Rgba, easing: Easing, t: f32) -> gpui::Rgba {
    let eased = ease(easing, t);
    gpui::Rgba {
        r: from.r + (to.r - from.r) * eased,
        g: from.g + (to.g - from.g) * eased,
        b: from.b + (to.b - from.b) * eased,
        a: from.a + (to.a - from.a) * eased,
    }
}
