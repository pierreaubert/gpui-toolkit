use super::easing::ease;
use super::keyframe::Keyframe;

/// A sequence of keyframes for complex animations
#[derive(Debug, Clone)]
pub struct KeyframeAnimation<T: Clone> {
    pub(super) keyframes: Vec<Keyframe<T>>,
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
        // Keep sorted by position - handle NaN by treating it as 0.0 to avoid panic
        self.keyframes
            .sort_by(|a, b| a.at.partial_cmp(&b.at).unwrap_or(std::cmp::Ordering::Equal));
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
