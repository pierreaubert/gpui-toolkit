//! # d3-transition - Animation Transitions for GPUI
//!
//! This module provides smooth transitions for animating values over time,
//! inspired by D3.js's d3-transition module but designed for GPUI integration.
//!
//! ## Features
//!
//! - Animate numeric values with easing functions
//! - Lifecycle callbacks (on_start, on_end, on_interrupt)
//! - Chained transitions with delays
//! - Named transitions for interruption control
//!
//! ## Example
//!
//! ```rust,no_run
//! use d3rs::transition::{Transition, TransitionConfig};
//! use d3rs::ease::ease_cubic_in_out;
//!
//! // Create a transition from 0.0 to 100.0 over 1 second
//! let mut transition = Transition::new()
//!     .duration(1000.0)
//!     .ease(ease_cubic_in_out)
//!     .from_to(0.0, 100.0);
//!
//! // Update with delta time
//! let value = transition.tick(16.0); // 16ms frame
//! ```

use crate::ease::ease_linear;
use crate::timer::now;
use std::sync::Arc;

/// Easing function type: takes t in [0,1], returns eased value
pub type EaseFn = fn(f64) -> f64;

/// Lifecycle callback types
pub type OnStartFn = Arc<dyn Fn() + Send + Sync>;
pub type OnEndFn = Arc<dyn Fn() + Send + Sync>;
pub type OnInterruptFn = Arc<dyn Fn() + Send + Sync>;

/// Transition state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionState {
    /// Transition hasn't started yet (during delay)
    Pending,
    /// Transition is actively running
    Active,
    /// Transition completed successfully
    Ended,
    /// Transition was interrupted before completion
    Interrupted,
}

/// Configuration for a transition
#[derive(Clone)]
pub struct TransitionConfig {
    /// Duration in milliseconds
    pub duration: f64,
    /// Delay before starting in milliseconds
    pub delay: f64,
    /// Easing function
    pub ease: EaseFn,
    /// Optional name for transition management
    pub name: Option<String>,
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            duration: 250.0,
            delay: 0.0,
            ease: ease_linear,
            name: None,
        }
    }
}

/// A transition for animating a single value
pub struct Transition {
    config: TransitionConfig,
    start_value: f64,
    end_value: f64,
    start_time: Option<f64>,
    elapsed: f64,
    state: TransitionState,
    on_start: Option<OnStartFn>,
    on_end: Option<OnEndFn>,
    on_interrupt: Option<OnInterruptFn>,
}

impl Transition {
    /// Create a new transition with default configuration
    pub fn new() -> Self {
        Self {
            config: TransitionConfig::default(),
            start_value: 0.0,
            end_value: 1.0,
            start_time: None,
            elapsed: 0.0,
            state: TransitionState::Pending,
            on_start: None,
            on_end: None,
            on_interrupt: None,
        }
    }

    /// Set the duration in milliseconds
    pub fn duration(mut self, ms: f64) -> Self {
        self.config.duration = ms;
        self
    }

    /// Set the delay in milliseconds
    pub fn delay(mut self, ms: f64) -> Self {
        self.config.delay = ms;
        self
    }

    /// Set the easing function
    pub fn ease(mut self, ease: EaseFn) -> Self {
        self.config.ease = ease;
        self
    }

    /// Set the transition name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = Some(name.into());
        self
    }

    /// Set start and end values
    pub fn from_to(mut self, start: f64, end: f64) -> Self {
        self.start_value = start;
        self.end_value = end;
        self
    }

    /// Set end value (keeps current value as start)
    pub fn to(mut self, end: f64) -> Self {
        self.end_value = end;
        self
    }

    /// Set callback for when transition starts
    pub fn on_start<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_start = Some(Arc::new(callback));
        self
    }

    /// Set callback for when transition ends
    pub fn on_end<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_end = Some(Arc::new(callback));
        self
    }

    /// Set callback for when transition is interrupted
    pub fn on_interrupt<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_interrupt = Some(Arc::new(callback));
        self
    }

    /// Get current transition state
    pub fn state(&self) -> TransitionState {
        self.state
    }

    /// Check if transition is complete
    pub fn is_complete(&self) -> bool {
        matches!(
            self.state,
            TransitionState::Ended | TransitionState::Interrupted
        )
    }

    /// Interrupt the transition
    pub fn interrupt(&mut self) {
        if !self.is_complete() && self.state != TransitionState::Pending {
            self.state = TransitionState::Interrupted;
            if let Some(ref callback) = self.on_interrupt {
                callback();
            }
        }
    }

    /// Update transition with delta time in milliseconds
    /// Returns current interpolated value
    pub fn tick(&mut self, dt: f64) -> f64 {
        // Initialize start time if needed
        if self.start_time.is_none() {
            self.start_time = Some(now());
        }

        self.elapsed += dt;

        // Handle delay
        if self.elapsed < self.config.delay {
            return self.start_value;
        }

        // Transition from Pending to Active
        if self.state == TransitionState::Pending {
            self.state = TransitionState::Active;
            if let Some(ref callback) = self.on_start {
                callback();
            }
        }

        // Calculate progress
        let progress_time = self.elapsed - self.config.delay;
        if progress_time >= self.config.duration {
            // Transition complete
            if self.state == TransitionState::Active {
                self.state = TransitionState::Ended;
                if let Some(ref callback) = self.on_end {
                    callback();
                }
            }
            return self.end_value;
        }

        // Interpolate value
        let t = (progress_time / self.config.duration).clamp(0.0, 1.0);
        let eased_t = (self.config.ease)(t);
        self.start_value + (self.end_value - self.start_value) * eased_t
    }

    /// Get current value without updating time
    pub fn value(&self) -> f64 {
        if self.elapsed < self.config.delay {
            return self.start_value;
        }

        let progress_time = self.elapsed - self.config.delay;
        if progress_time >= self.config.duration {
            return self.end_value;
        }

        let t = (progress_time / self.config.duration).clamp(0.0, 1.0);
        let eased_t = (self.config.ease)(t);
        self.start_value + (self.end_value - self.start_value) * eased_t
    }

    /// Reset transition to initial state
    pub fn reset(&mut self) {
        self.start_time = None;
        self.elapsed = 0.0;
        self.state = TransitionState::Pending;
    }
}

impl Default for Transition {
    fn default() -> Self {
        Self::new()
    }
}

/// Manage multiple named transitions
pub struct TransitionManager {
    transitions: Vec<(String, Transition)>,
}

impl TransitionManager {
    /// Create a new transition manager
    pub fn new() -> Self {
        Self {
            transitions: Vec::new(),
        }
    }

    /// Add or replace a transition by name
    pub fn add(&mut self, name: impl Into<String>, transition: Transition) {
        let name = name.into();

        // Interrupt existing transition with same name
        if let Some(pos) = self.transitions.iter().position(|(n, _)| n == &name) {
            self.transitions[pos].1.interrupt();
            self.transitions.remove(pos);
        }

        self.transitions.push((name, transition));
    }

    /// Update all transitions
    /// Returns HashMap of name -> current value
    pub fn tick(&mut self, dt: f64) -> Vec<(String, f64)> {
        let mut results = Vec::new();

        // Update all transitions and collect values
        for (name, transition) in &mut self.transitions {
            let value = transition.tick(dt);
            results.push((name.clone(), value));
        }

        // Remove completed transitions
        self.transitions.retain(|(_, t)| {
            !matches!(
                t.state(),
                TransitionState::Ended | TransitionState::Interrupted
            )
        });

        results
    }

    /// Get value of a named transition
    pub fn get(&self, name: &str) -> Option<f64> {
        self.transitions
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| t.value())
    }

    /// Interrupt a named transition
    pub fn interrupt(&mut self, name: &str) {
        if let Some((_, transition)) = self.transitions.iter_mut().find(|(n, _)| n == name) {
            transition.interrupt();
        }
    }

    /// Interrupt all transitions
    pub fn interrupt_all(&mut self) {
        for (_, transition) in &mut self.transitions {
            transition.interrupt();
        }
    }

    /// Check if any transitions are active
    pub fn is_animating(&self) -> bool {
        self.transitions
            .iter()
            .any(|(_, t)| t.state() == TransitionState::Active)
    }
}

impl Default for TransitionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ease::ease_cubic_in_out;

    #[test]
    fn test_transition_linear() {
        let mut t = Transition::new().duration(100.0).from_to(0.0, 100.0);

        assert_eq!(t.state(), TransitionState::Pending);

        // Start
        let v = t.tick(0.0);
        assert_eq!(v, 0.0);
        assert_eq!(t.state(), TransitionState::Active);

        // Midpoint
        let v = t.tick(50.0);
        assert!((v - 50.0).abs() < 0.1);

        // End
        let v = t.tick(50.0);
        assert!((v - 100.0).abs() < 0.1);
        assert_eq!(t.state(), TransitionState::Ended);
    }

    #[test]
    fn test_transition_with_delay() {
        let mut t = Transition::new()
            .duration(100.0)
            .delay(50.0)
            .from_to(0.0, 100.0);

        // During delay
        let v = t.tick(25.0);
        assert_eq!(v, 0.0);
        assert_eq!(t.state(), TransitionState::Pending);

        // After delay
        let v = t.tick(25.0);
        assert_eq!(v, 0.0);
        assert_eq!(t.state(), TransitionState::Active);

        // Mid-transition
        let v = t.tick(50.0);
        assert!((v - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_transition_easing() {
        let mut t = Transition::new()
            .duration(100.0)
            .ease(ease_cubic_in_out)
            .from_to(0.0, 100.0);

        t.tick(0.0); // Start
        let v = t.tick(50.0); // Midpoint

        // cubic_in_out at t=0.5 should be 0.5
        assert!((v - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_transition_interrupt() {
        let mut t = Transition::new().duration(100.0).from_to(0.0, 100.0);

        t.tick(0.0);
        t.tick(25.0);
        assert_eq!(t.state(), TransitionState::Active);

        t.interrupt();
        assert_eq!(t.state(), TransitionState::Interrupted);
    }

    #[test]
    fn test_transition_manager() {
        let mut manager = TransitionManager::new();

        manager.add(
            "opacity",
            Transition::new().duration(100.0).from_to(0.0, 1.0),
        );
        manager.add("x", Transition::new().duration(100.0).from_to(0.0, 100.0));

        let results = manager.tick(50.0);
        assert_eq!(results.len(), 2);

        let opacity = manager.get("opacity").unwrap();
        assert!((opacity - 0.5).abs() < 0.1);

        let x = manager.get("x").unwrap();
        assert!((x - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_transition_manager_interrupt() {
        let mut manager = TransitionManager::new();

        manager.add("x", Transition::new().duration(100.0).from_to(0.0, 100.0));
        manager.tick(25.0);

        manager.interrupt("x");
        assert!(!manager.is_animating());
    }

    #[test]
    fn test_transition_replace() {
        let mut manager = TransitionManager::new();

        manager.add("x", Transition::new().duration(100.0).from_to(0.0, 100.0));
        manager.tick(50.0);

        // Replace with new transition
        manager.add("x", Transition::new().duration(100.0).from_to(100.0, 200.0));

        let x = manager.get("x").unwrap();
        assert!((x - 100.0).abs() < 0.1); // Should be at start of new transition
    }
}
