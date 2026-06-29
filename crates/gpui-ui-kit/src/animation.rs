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

use std::time::Duration;

mod easing;
mod keyframe;
mod keyframe_animation;
mod spring;
#[cfg(test)]
mod tests;
mod types;

pub use easing::{ease, interpolate, interpolate_color};
pub use keyframe::Keyframe;
pub use keyframe_animation::{KeyframeAnimation, evaluate_keyframes};
pub use spring::Spring;
pub use types::Easing;
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
