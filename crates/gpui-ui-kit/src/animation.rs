//! Animation Primitives
//!
//! Provides animation utilities including easing functions, timing configuration,
//! and spring physics for natural motion.
//!
//! # Usage
//!
//! ```ignore
//! use gpui_ui_kit::animation::{Animation, Easing, ease};
//!
//! // Create an animation configuration
//! let anim = Animation::new()
//!     .duration_ms(300)
//!     .easing(Easing::EaseOutCubic)
//!     .delay_ms(100);
//!
//! // Use easing functions directly
//! let progress = 0.5; // 0.0 to 1.0
//! let eased = ease(Easing::EaseInOutQuad, progress);
//!
//! // Spring physics for natural motion
//! let spring = Spring::default();
//! let (position, velocity) = spring.step(current_pos, target_pos, current_vel, dt);
//! ```
//!
//! # Easing Functions
//!
//! All standard easing functions are provided:
//! - Linear
//! - Quad (In, Out, InOut)
//! - Cubic (In, Out, InOut)
//! - Quart (In, Out, InOut)
//! - Quint (In, Out, InOut)
//! - Sine (In, Out, InOut)
//! - Expo (In, Out, InOut)
//! - Circ (In, Out, InOut)
//! - Back (In, Out, InOut)
//! - Elastic (In, Out, InOut)
//! - Bounce (In, Out, InOut)

use std::f32::consts::PI;
use std::time::Duration;

/// Animation timing and easing configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Animation {
    /// Duration of the animation
    pub duration: Duration,
    /// Easing function to use
    pub easing: Easing,
    /// Delay before animation starts
    pub delay: Duration,
    /// Number of times to repeat (0 = play once, 1 = repeat once, etc.)
    pub repeat: u32,
    /// Whether to reverse on each repeat (ping-pong)
    pub alternate: bool,
}

impl Animation {
    /// Create a new animation with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the duration in milliseconds
    pub fn duration_ms(mut self, ms: u64) -> Self {
        self.duration = Duration::from_millis(ms);
        self
    }

    /// Set the duration
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set the easing function
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Set the delay in milliseconds
    pub fn delay_ms(mut self, ms: u64) -> Self {
        self.delay = Duration::from_millis(ms);
        self
    }

    /// Set the delay
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Set the number of repeats
    pub fn repeat(mut self, count: u32) -> Self {
        self.repeat = count;
        self
    }

    /// Enable ping-pong alternating on repeat
    pub fn alternate(mut self, alternate: bool) -> Self {
        self.alternate = alternate;
        self
    }

    /// Create a quick animation (150ms)
    pub fn quick() -> Self {
        Self::new().duration_ms(150).easing(Easing::EaseOutQuad)
    }

    /// Create a standard animation (250ms)
    pub fn standard() -> Self {
        Self::new().duration_ms(250).easing(Easing::EaseOutCubic)
    }

    /// Create a slow animation (400ms)
    pub fn slow() -> Self {
        Self::new().duration_ms(400).easing(Easing::EaseInOutCubic)
    }

    /// Create an emphasis animation with overshoot
    pub fn emphasis() -> Self {
        Self::new().duration_ms(300).easing(Easing::EaseOutBack)
    }

    /// Create a bouncy animation
    pub fn bouncy() -> Self {
        Self::new().duration_ms(500).easing(Easing::EaseOutBounce)
    }

    /// Calculate the eased progress for a given time
    ///
    /// Returns a value between 0.0 and 1.0, where 0.0 is the start
    /// and 1.0 is the end of the animation.
    pub fn progress(&self, elapsed: Duration) -> f32 {
        if elapsed < self.delay {
            return 0.0;
        }

        let effective_elapsed = elapsed - self.delay;
        let t = if self.duration.is_zero() {
            1.0
        } else {
            (effective_elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0)
        };

        ease(self.easing, t)
    }

    /// Check if the animation is complete
    pub fn is_complete(&self, elapsed: Duration) -> bool {
        elapsed >= self.delay + self.duration
    }

    /// Get the total duration including delay
    pub fn total_duration(&self) -> Duration {
        self.delay + self.duration
    }
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            duration: Duration::from_millis(200),
            easing: Easing::EaseOutQuad,
            delay: Duration::ZERO,
            repeat: 0,
            alternate: false,
        }
    }
}

/// Easing function types
///
/// Each easing function transforms linear progress (0.0 to 1.0) into
/// eased progress for natural-feeling animations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Easing {
    /// Linear interpolation (no easing)
    Linear,

    // Quadratic
    /// Quadratic ease in (accelerating)
    EaseInQuad,
    /// Quadratic ease out (decelerating) - good default
    #[default]
    EaseOutQuad,
    /// Quadratic ease in-out
    EaseInOutQuad,

    // Cubic
    /// Cubic ease in
    EaseInCubic,
    /// Cubic ease out - smooth deceleration
    EaseOutCubic,
    /// Cubic ease in-out
    EaseInOutCubic,

    // Quartic
    /// Quartic ease in
    EaseInQuart,
    /// Quartic ease out
    EaseOutQuart,
    /// Quartic ease in-out
    EaseInOutQuart,

    // Quintic
    /// Quintic ease in
    EaseInQuint,
    /// Quintic ease out
    EaseOutQuint,
    /// Quintic ease in-out
    EaseInOutQuint,

    // Sine
    /// Sine ease in
    EaseInSine,
    /// Sine ease out
    EaseOutSine,
    /// Sine ease in-out
    EaseInOutSine,

    // Exponential
    /// Exponential ease in
    EaseInExpo,
    /// Exponential ease out
    EaseOutExpo,
    /// Exponential ease in-out
    EaseInOutExpo,

    // Circular
    /// Circular ease in
    EaseInCirc,
    /// Circular ease out
    EaseOutCirc,
    /// Circular ease in-out
    EaseInOutCirc,

    // Back (overshoot)
    /// Back ease in (anticipation)
    EaseInBack,
    /// Back ease out (overshoot) - good for emphasis
    EaseOutBack,
    /// Back ease in-out
    EaseInOutBack,

    // Elastic
    /// Elastic ease in
    EaseInElastic,
    /// Elastic ease out - springy feel
    EaseOutElastic,
    /// Elastic ease in-out
    EaseInOutElastic,

    // Bounce
    /// Bounce ease in
    EaseInBounce,
    /// Bounce ease out - ball dropping effect
    EaseOutBounce,
    /// Bounce ease in-out
    EaseInOutBounce,
}

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

/// Spring physics simulation for natural motion
///
/// Simulates a damped spring for smooth, natural animations.
/// Good for drag-and-drop, pull-to-refresh, and other interactive animations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    /// Spring stiffness (higher = faster oscillation)
    pub stiffness: f32,
    /// Damping ratio (1.0 = critically damped, <1.0 = oscillates, >1.0 = overdamped)
    pub damping: f32,
    /// Mass of the object
    pub mass: f32,
}

impl Spring {
    /// Create a new spring with custom parameters
    pub fn new(stiffness: f32, damping: f32, mass: f32) -> Self {
        Self {
            stiffness,
            damping,
            mass,
        }
    }

    /// Create a gentle spring (slow, smooth motion)
    pub fn gentle() -> Self {
        Self {
            stiffness: 100.0,
            damping: 15.0,
            mass: 1.0,
        }
    }

    /// Create a wobbly spring (bouncy, playful motion)
    pub fn wobbly() -> Self {
        Self {
            stiffness: 180.0,
            damping: 12.0,
            mass: 1.0,
        }
    }

    /// Create a stiff spring (quick, snappy motion)
    pub fn stiff() -> Self {
        Self {
            stiffness: 400.0,
            damping: 30.0,
            mass: 1.0,
        }
    }

    /// Create a slow spring (deliberate motion)
    pub fn slow() -> Self {
        Self {
            stiffness: 50.0,
            damping: 20.0,
            mass: 1.0,
        }
    }

    /// Calculate the spring force at a given displacement
    pub fn force(&self, displacement: f32, velocity: f32) -> f32 {
        let spring_force = -self.stiffness * displacement;
        let damping_force = -self.damping * velocity;
        (spring_force + damping_force) / self.mass
    }

    /// Step the spring simulation forward by dt seconds
    ///
    /// Returns the new (position, velocity) tuple
    pub fn step(&self, current: f32, target: f32, velocity: f32, dt: f32) -> (f32, f32) {
        let displacement = current - target;
        let acceleration = self.force(displacement, velocity);
        let new_velocity = velocity + acceleration * dt;
        let new_position = current + new_velocity * dt;
        (new_position, new_velocity)
    }

    /// Check if the spring has settled (velocity and displacement are negligible)
    pub fn is_settled(&self, current: f32, target: f32, velocity: f32, threshold: f32) -> bool {
        (current - target).abs() < threshold && velocity.abs() < threshold
    }
}

impl Default for Spring {
    fn default() -> Self {
        Self {
            stiffness: 170.0,
            damping: 26.0,
            mass: 1.0,
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

/// Keyframe for multi-step animations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Keyframe<T> {
    /// Progress point (0.0 to 1.0)
    pub at: f32,
    /// Value at this keyframe
    pub value: T,
    /// Easing to use when interpolating TO this keyframe
    pub easing: Easing,
}

impl<T> Keyframe<T> {
    /// Create a new keyframe
    pub fn new(at: f32, value: T) -> Self {
        Self {
            at: at.clamp(0.0, 1.0),
            value,
            easing: Easing::Linear,
        }
    }

    /// Set the easing function for interpolation to this keyframe
    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}

/// A sequence of keyframes for complex animations
#[derive(Debug, Clone)]
pub struct KeyframeAnimation<T: Clone> {
    keyframes: Vec<Keyframe<T>>,
}

impl<T: Clone> KeyframeAnimation<T> {
    /// Create a new keyframe animation
    pub fn new() -> Self {
        Self {
            keyframes: Vec::new(),
        }
    }

    /// Add a keyframe
    pub fn keyframe(mut self, keyframe: Keyframe<T>) -> Self {
        self.keyframes.push(keyframe);
        // Keep sorted by position
        self.keyframes
            .sort_by(|a, b| a.at.partial_cmp(&b.at).unwrap());
        self
    }

    /// Add a keyframe at a specific position
    pub fn at(self, position: f32, value: T) -> Self {
        self.keyframe(Keyframe::new(position, value))
    }

    /// Get the keyframes surrounding a given progress value
    pub fn get_surrounding(&self, t: f32) -> Option<(&Keyframe<T>, &Keyframe<T>, f32)> {
        if self.keyframes.is_empty() {
            return None;
        }

        let t = t.clamp(0.0, 1.0);

        // Find the two keyframes surrounding t
        let mut prev_idx = 0;
        for (i, kf) in self.keyframes.iter().enumerate() {
            if kf.at <= t {
                prev_idx = i;
            } else {
                break;
            }
        }

        let next_idx = (prev_idx + 1).min(self.keyframes.len() - 1);
        let prev = &self.keyframes[prev_idx];
        let next = &self.keyframes[next_idx];

        // Calculate local progress between the two keyframes
        let local_t = if prev_idx == next_idx {
            1.0
        } else {
            let range = next.at - prev.at;
            if range == 0.0 {
                1.0
            } else {
                (t - prev.at) / range
            }
        };

        Some((prev, next, local_t))
    }
}

impl<T: Clone> Default for KeyframeAnimation<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Evaluate a keyframe animation at a given progress, using a custom interpolation function
pub fn evaluate_keyframes<T: Clone>(
    animation: &KeyframeAnimation<T>,
    t: f32,
    interpolate_fn: impl Fn(&T, &T, f32) -> T,
) -> Option<T> {
    animation.get_surrounding(t).map(|(prev, next, local_t)| {
        let eased_t = ease(next.easing, local_t);
        interpolate_fn(&prev.value, &next.value, eased_t)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_easing() {
        assert_eq!(ease(Easing::Linear, 0.0), 0.0);
        assert_eq!(ease(Easing::Linear, 0.5), 0.5);
        assert_eq!(ease(Easing::Linear, 1.0), 1.0);
    }

    #[test]
    fn test_ease_in_out_bounds() {
        for easing in [
            Easing::EaseInQuad,
            Easing::EaseOutQuad,
            Easing::EaseInOutQuad,
            Easing::EaseInCubic,
            Easing::EaseOutCubic,
            Easing::EaseInOutCubic,
        ] {
            let start = ease(easing, 0.0);
            let end = ease(easing, 1.0);
            assert!(
                (start - 0.0).abs() < 0.001,
                "{:?} at t=0 should be ~0, got {}",
                easing,
                start
            );
            assert!(
                (end - 1.0).abs() < 0.001,
                "{:?} at t=1 should be ~1, got {}",
                easing,
                end
            );
        }
    }

    #[test]
    fn test_ease_out_faster_start() {
        // EaseOut should have faster progress at the start
        let ease_in = ease(Easing::EaseInQuad, 0.5);
        let ease_out = ease(Easing::EaseOutQuad, 0.5);
        assert!(ease_out > ease_in, "EaseOut should be ahead at midpoint");
    }

    #[test]
    fn test_back_overshoot() {
        // EaseOutBack should overshoot past 1.0 before settling
        let peak = ease(Easing::EaseOutBack, 0.7);
        assert!(peak > 1.0, "EaseOutBack should overshoot");
    }

    #[test]
    fn test_animation_progress() {
        let anim = Animation::new().duration_ms(1000);

        assert_eq!(anim.progress(Duration::from_millis(0)), 0.0);
        // At 50% time with EaseOutQuad, progress is 0.75 (faster at start)
        let mid_progress = anim.progress(Duration::from_millis(500));
        assert!(mid_progress > 0.5, "EaseOut should be past 50% at midpoint");
        assert!(mid_progress < 0.9, "But not too far");
        assert_eq!(anim.progress(Duration::from_millis(1000)), 1.0);
        assert_eq!(anim.progress(Duration::from_millis(2000)), 1.0);
    }

    #[test]
    fn test_animation_with_delay() {
        let anim = Animation::new().duration_ms(1000).delay_ms(500);

        assert_eq!(anim.progress(Duration::from_millis(0)), 0.0);
        assert_eq!(anim.progress(Duration::from_millis(500)), 0.0);
        assert!(anim.progress(Duration::from_millis(1000)) > 0.4);
        assert_eq!(anim.progress(Duration::from_millis(1500)), 1.0);
    }

    #[test]
    fn test_animation_is_complete() {
        let anim = Animation::new().duration_ms(1000).delay_ms(500);

        assert!(!anim.is_complete(Duration::from_millis(0)));
        assert!(!anim.is_complete(Duration::from_millis(1000)));
        assert!(anim.is_complete(Duration::from_millis(1500)));
        assert!(anim.is_complete(Duration::from_millis(2000)));
    }

    #[test]
    fn test_spring_settling() {
        let spring = Spring::default();
        let mut pos = 0.0;
        let mut vel = 0.0;
        let target = 100.0;
        let dt = 1.0 / 60.0; // 60 FPS

        // Simulate for 2 seconds
        for _ in 0..120 {
            (pos, vel) = spring.step(pos, target, vel, dt);
        }

        // Should be close to target
        assert!(
            (pos - target).abs() < 1.0,
            "Spring should settle near target"
        );
        assert!(vel.abs() < 1.0, "Spring velocity should be near zero");
    }

    #[test]
    fn test_interpolate() {
        let result = interpolate(0.0, 100.0, Easing::Linear, 0.5);
        assert_eq!(result, 50.0);

        let result = interpolate(0.0, 100.0, Easing::Linear, 0.0);
        assert_eq!(result, 0.0);

        let result = interpolate(0.0, 100.0, Easing::Linear, 1.0);
        assert_eq!(result, 100.0);
    }

    #[test]
    fn test_keyframe_animation() {
        let anim = KeyframeAnimation::new()
            .at(0.0, 0.0_f32)
            .at(0.5, 50.0)
            .at(1.0, 100.0);

        let result = evaluate_keyframes(&anim, 0.25, |a, b, t| a + (b - a) * t);
        assert!(result.is_some());
        assert!((result.unwrap() - 25.0).abs() < 0.1);

        let result = evaluate_keyframes(&anim, 0.75, |a, b, t| a + (b - a) * t);
        assert!(result.is_some());
        assert!((result.unwrap() - 75.0).abs() < 0.1);
    }
}
