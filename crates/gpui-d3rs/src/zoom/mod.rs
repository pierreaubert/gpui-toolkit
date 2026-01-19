//! Zoom module for chart zoom state management (d3-zoom inspired)
//!
//! This module provides zoom state management for interactive charts,
//! tracking domain bounds for X and Y axes with support for zoom history.
//!
//! # Features
//!
//! - **Domain Zoom**: Zoom to specific domain ranges (e.g., from brush selection)
//! - **Zoom History**: Track zoom stack for reset functionality
//! - **Log Scale Support**: Properly clamps domains for log scales
//! - **Double-click Reset**: Restore original view
//!
//! # Example
//!
//! ```rust,no_run
//! use d3rs::zoom::ZoomState;
//!
//! let mut zoom = ZoomState::new(20.0, 20000.0, -40.0, 10.0);
//!
//! // Zoom to a specific region
//! zoom.zoom_to(100.0, 5000.0, -20.0, 5.0);
//!
//! // Check if zoomed
//! assert!(zoom.is_zoomed());
//!
//! // Reset to original view
//! zoom.reset();
//! assert!(!zoom.is_zoomed());
//! ```

/// Zoom state for a 2D chart
#[derive(Debug, Clone, PartialEq)]
pub struct ZoomState {
    /// Original X domain (min, max)
    original_x: (f64, f64),
    /// Original Y domain (min, max)
    original_y: (f64, f64),
    /// Current X domain (min, max)
    current_x: (f64, f64),
    /// Current Y domain (min, max)
    current_y: (f64, f64),
    /// Zoom history stack (for nested zooming)
    history: Vec<((f64, f64), (f64, f64))>,
    /// Whether X-axis is logarithmic
    x_is_log: bool,
    /// Whether Y-axis is logarithmic
    y_is_log: bool,
}

impl ZoomState {
    /// Create a new zoom state with the given original domain bounds
    pub fn new(x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> Self {
        Self {
            original_x: (x_min, x_max),
            original_y: (y_min, y_max),
            current_x: (x_min, x_max),
            current_y: (y_min, y_max),
            history: Vec::new(),
            x_is_log: false,
            y_is_log: false,
        }
    }

    /// Set whether X-axis is logarithmic (for proper clamping)
    pub fn with_log_x(mut self, is_log: bool) -> Self {
        self.x_is_log = is_log;
        self
    }

    /// Set whether Y-axis is logarithmic (for proper clamping)
    pub fn with_log_y(mut self, is_log: bool) -> Self {
        self.y_is_log = is_log;
        self
    }

    /// Zoom to a specific domain region
    ///
    /// The domain values are clamped to the original bounds and
    /// validated for log scales (must be positive).
    pub fn zoom_to(&mut self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) {
        // Save current state to history
        self.history.push((self.current_x, self.current_y));

        // Clamp to original bounds
        let new_x_min = x_min.max(self.original_x.0);
        let new_x_max = x_max.min(self.original_x.1);
        let new_y_min = y_min.max(self.original_y.0);
        let new_y_max = y_max.min(self.original_y.1);

        // For log scales, ensure positive values
        let (new_x_min, new_x_max) = if self.x_is_log {
            (new_x_min.max(1e-10), new_x_max.max(1e-10))
        } else {
            (new_x_min, new_x_max)
        };

        let (new_y_min, new_y_max) = if self.y_is_log {
            (new_y_min.max(1e-10), new_y_max.max(1e-10))
        } else {
            (new_y_min, new_y_max)
        };

        // Ensure min < max
        if new_x_min < new_x_max && new_y_min < new_y_max {
            self.current_x = (new_x_min, new_x_max);
            self.current_y = (new_y_min, new_y_max);
        }
    }

    /// Reset to original view (clears all zoom history)
    pub fn reset(&mut self) {
        self.current_x = self.original_x;
        self.current_y = self.original_y;
        self.history.clear();
    }

    /// Zoom back one level in history
    pub fn zoom_back(&mut self) -> bool {
        if let Some((x, y)) = self.history.pop() {
            self.current_x = x;
            self.current_y = y;
            true
        } else {
            false
        }
    }

    /// Check if currently zoomed (different from original view)
    pub fn is_zoomed(&self) -> bool {
        self.current_x != self.original_x || self.current_y != self.original_y
    }

    /// Get current X domain
    pub fn x_domain(&self) -> (f64, f64) {
        self.current_x
    }

    /// Get current Y domain
    pub fn y_domain(&self) -> (f64, f64) {
        self.current_y
    }

    /// Get original X domain
    pub fn original_x_domain(&self) -> (f64, f64) {
        self.original_x
    }

    /// Get original Y domain
    pub fn original_y_domain(&self) -> (f64, f64) {
        self.original_y
    }

    /// Get zoom level (depth of zoom history)
    /// Returns the number of times zoom_to() has been called
    pub fn zoom_level(&self) -> usize {
        self.history.len()
    }

    /// Update original bounds (e.g., when data changes)
    pub fn set_original(&mut self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) {
        self.original_x = (x_min, x_max);
        self.original_y = (y_min, y_max);
        // Reset to new bounds
        self.reset();
    }
}

impl Default for ZoomState {
    fn default() -> Self {
        Self::new(0.0, 1.0, 0.0, 1.0)
    }
}

/// Zoom configuration
#[derive(Debug, Clone)]
pub struct ZoomConfig {
    /// Enable X-axis zooming
    pub zoom_x: bool,
    /// Enable Y-axis zooming
    pub zoom_y: bool,
    /// Minimum zoom extent as fraction of original (e.g., 0.01 = 1%)
    pub min_extent: f64,
    /// Maximum zoom extent as fraction of original (e.g., 10.0 = 1000%)
    pub max_extent: f64,
}

impl Default for ZoomConfig {
    fn default() -> Self {
        Self {
            zoom_x: true,
            zoom_y: true,
            min_extent: 0.001,
            max_extent: 100.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zoom_state_creation() {
        let zoom = ZoomState::new(0.0, 100.0, -10.0, 10.0);
        assert_eq!(zoom.x_domain(), (0.0, 100.0));
        assert_eq!(zoom.y_domain(), (-10.0, 10.0));
        assert!(!zoom.is_zoomed());
    }

    #[test]
    fn test_zoom_to() {
        let mut zoom = ZoomState::new(0.0, 100.0, -10.0, 10.0);
        zoom.zoom_to(25.0, 75.0, -5.0, 5.0);

        assert!(zoom.is_zoomed());
        assert_eq!(zoom.x_domain(), (25.0, 75.0));
        assert_eq!(zoom.y_domain(), (-5.0, 5.0));
    }

    #[test]
    fn test_zoom_clamping() {
        let mut zoom = ZoomState::new(0.0, 100.0, -10.0, 10.0);
        // Try to zoom outside original bounds
        zoom.zoom_to(-50.0, 150.0, -20.0, 20.0);

        // Should be clamped to original bounds
        assert_eq!(zoom.x_domain(), (0.0, 100.0));
        assert_eq!(zoom.y_domain(), (-10.0, 10.0));
    }

    #[test]
    fn test_zoom_reset() {
        let mut zoom = ZoomState::new(0.0, 100.0, -10.0, 10.0);
        zoom.zoom_to(25.0, 75.0, -5.0, 5.0);
        assert!(zoom.is_zoomed());

        zoom.reset();
        assert!(!zoom.is_zoomed());
        assert_eq!(zoom.x_domain(), (0.0, 100.0));
    }

    #[test]
    fn test_zoom_history() {
        let mut zoom = ZoomState::new(0.0, 100.0, -10.0, 10.0);

        // First zoom
        zoom.zoom_to(25.0, 75.0, -5.0, 5.0);
        assert_eq!(zoom.zoom_level(), 1);

        // Second zoom (nested)
        zoom.zoom_to(40.0, 60.0, -2.0, 2.0);
        assert_eq!(zoom.zoom_level(), 2);

        // Go back one level
        assert!(zoom.zoom_back());
        assert_eq!(zoom.x_domain(), (25.0, 75.0));
        assert_eq!(zoom.zoom_level(), 1);

        // Go back to original
        assert!(zoom.zoom_back());
        assert_eq!(zoom.x_domain(), (0.0, 100.0));
        assert!(!zoom.is_zoomed());
    }

    #[test]
    fn test_log_scale_clamping() {
        let mut zoom = ZoomState::new(20.0, 20000.0, 0.0, 100.0).with_log_x(true);

        // Try to zoom to negative X values (invalid for log scale)
        zoom.zoom_to(-10.0, 1000.0, 0.0, 50.0);

        // X min should be clamped to positive value (original min)
        assert!(zoom.x_domain().0 >= 20.0);
        assert_eq!(zoom.x_domain().1, 1000.0);
    }
}
