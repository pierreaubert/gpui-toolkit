//! Brush module for rectangle selection (d3-brush inspired)
//!
//! This module provides brush selection functionality similar to d3-brush,
//! allowing users to select rectangular regions on charts for zooming.
//!
//! # Features
//!
//! - **Rectangle Selection**: Click and drag to select a region
//! - **Visual Feedback**: Selection overlay with configurable styling
//! - **Zoom Integration**: Converts pixel selections to domain coordinates
//! - **Log Scale Support**: Properly handles logarithmic scale inversion
//!
//! # Example
//!
//! ```rust,no_run
//! use d3rs::brush::{BrushState, BrushSelection};
//!
//! let mut state = BrushState::new();
//! state.start(100.0, 50.0); // Start drag at (100, 50)
//! state.update(200.0, 150.0); // Update while dragging
//!
//! if let Some(selection) = state.end() {
//!     println!("Selected: {:?}", selection);
//! }
//! ```

use crate::scale::Scale;

/// A rectangular selection in pixel coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushSelection {
    /// Left edge in pixels
    pub x0: f64,
    /// Top edge in pixels
    pub y0: f64,
    /// Right edge in pixels
    pub x1: f64,
    /// Bottom edge in pixels
    pub y1: f64,
}

impl BrushSelection {
    /// Create a new selection from two corner points
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        // Normalize so x0 <= x1 and y0 <= y1
        Self {
            x0: x0.min(x1),
            y0: y0.min(y1),
            x1: x0.max(x1),
            y1: y0.max(y1),
        }
    }

    /// Get the width of the selection
    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }

    /// Get the height of the selection
    pub fn height(&self) -> f64 {
        self.y1 - self.y0
    }

    /// Check if the selection is too small to be meaningful
    pub fn is_trivial(&self, min_size: f64) -> bool {
        self.width() < min_size || self.height() < min_size
    }

    /// Convert pixel selection to domain coordinates using provided scales
    ///
    /// The scale's range determines the Y-axis direction:
    /// - For inverted Y-axis (common in charts): use `.range(height, 0.0)`
    /// - For non-inverted Y-axis: use `.range(0.0, height)`
    /// The scale's invert() method handles the conversion correctly in both cases.
    pub fn to_domain<X: Scale<f64, f64>, Y: Scale<f64, f64>>(
        &self,
        x_scale: &X,
        y_scale: &Y,
    ) -> DomainSelection {
        // Direct scale inversion - the scale's range direction determines
        // the mapping from pixel coordinates to domain coordinates
        let x0_raw = x_scale.invert(self.x0).unwrap_or(x_scale.domain().0);
        let x1_raw = x_scale.invert(self.x1).unwrap_or(x_scale.domain().1);
        let y0_raw = y_scale.invert(self.y0).unwrap_or(y_scale.domain().0);
        let y1_raw = y_scale.invert(self.y1).unwrap_or(y_scale.domain().1);

        // Normalize to ensure x0 <= x1 and y0 <= y1
        let (x0, x1) = if x0_raw <= x1_raw {
            (x0_raw, x1_raw)
        } else {
            (x1_raw, x0_raw)
        };
        let (y0, y1) = if y0_raw <= y1_raw {
            (y0_raw, y1_raw)
        } else {
            (y1_raw, y0_raw)
        };

        DomainSelection { x0, y0, x1, y1 }
    }
}

/// A rectangular selection in domain (data) coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomainSelection {
    /// Minimum X value in domain units
    pub x0: f64,
    /// Minimum Y value in domain units
    pub y0: f64,
    /// Maximum X value in domain units
    pub x1: f64,
    /// Maximum Y value in domain units
    pub y1: f64,
}

impl DomainSelection {
    /// Create a new domain selection
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Self {
            x0: x0.min(x1),
            y0: y0.min(y1),
            x1: x0.max(x1),
            y1: y0.max(y1),
        }
    }
}

/// State machine for brush selection
#[derive(Debug, Clone, Default)]
pub struct BrushState {
    /// Starting point of the drag (if active)
    start: Option<(f64, f64)>,
    /// Current point of the drag (if active)
    current: Option<(f64, f64)>,
    /// Whether a drag is currently active
    dragging: bool,
}

impl BrushState {
    /// Create a new brush state
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new brush selection at the given pixel coordinates
    pub fn start(&mut self, x: f64, y: f64) {
        self.start = Some((x, y));
        self.current = Some((x, y));
        self.dragging = true;
    }

    /// Update the brush selection while dragging
    pub fn update(&mut self, x: f64, y: f64) {
        if self.dragging {
            self.current = Some((x, y));
        }
    }

    /// End the brush selection and return the result if valid
    pub fn end(&mut self) -> Option<BrushSelection> {
        if !self.dragging {
            return None;
        }

        let result = match (self.start, self.current) {
            (Some((x0, y0)), Some((x1, y1))) => Some(BrushSelection::new(x0, y0, x1, y1)),
            _ => None,
        };

        self.reset();
        result
    }

    /// Cancel the current brush selection
    pub fn reset(&mut self) {
        self.start = None;
        self.current = None;
        self.dragging = false;
    }

    /// Check if a brush selection is currently active
    pub fn is_active(&self) -> bool {
        self.dragging
    }

    /// Get the current selection rectangle (if dragging)
    pub fn current_selection(&self) -> Option<BrushSelection> {
        if !self.dragging {
            return None;
        }

        match (self.start, self.current) {
            (Some((x0, y0)), Some((x1, y1))) => Some(BrushSelection::new(x0, y0, x1, y1)),
            _ => None,
        }
    }
}

/// Configuration for brush overlay rendering
#[derive(Debug, Clone)]
pub struct BrushConfig {
    /// Fill color for selection overlay (RGBA)
    pub fill_color: (u8, u8, u8, u8),
    /// Stroke color for selection border
    pub stroke_color: (u8, u8, u8),
    /// Stroke width for selection border
    pub stroke_width: f32,
    /// Minimum selection size in pixels (smaller selections are ignored)
    pub min_size: f64,
}

impl Default for BrushConfig {
    fn default() -> Self {
        Self {
            fill_color: (100, 150, 200, 80), // Semi-transparent blue
            stroke_color: (70, 130, 180),    // Steel blue
            stroke_width: 1.0,
            min_size: 5.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::{LinearScale, LogScale};

    #[test]
    fn test_brush_selection_normalization() {
        // Selection created with corners in different orders should normalize
        let sel1 = BrushSelection::new(0.0, 0.0, 100.0, 100.0);
        let sel2 = BrushSelection::new(100.0, 100.0, 0.0, 0.0);
        let sel3 = BrushSelection::new(0.0, 100.0, 100.0, 0.0);

        assert_eq!(sel1.x0, sel2.x0);
        assert_eq!(sel1.y0, sel2.y0);
        assert_eq!(sel1.x1, sel2.x1);
        assert_eq!(sel1.y1, sel2.y1);
        assert_eq!(sel1.x0, sel3.x0);
    }

    #[test]
    fn test_brush_selection_dimensions() {
        let sel = BrushSelection::new(10.0, 20.0, 110.0, 80.0);
        assert_eq!(sel.width(), 100.0);
        assert_eq!(sel.height(), 60.0);
    }

    #[test]
    fn test_brush_state_lifecycle() {
        let mut state = BrushState::new();
        assert!(!state.is_active());

        state.start(0.0, 0.0);
        assert!(state.is_active());

        state.update(100.0, 50.0);
        let current = state.current_selection().unwrap();
        assert_eq!(current.width(), 100.0);
        assert_eq!(current.height(), 50.0);

        let final_sel = state.end().unwrap();
        assert_eq!(final_sel.width(), 100.0);
        assert!(!state.is_active());
    }

    #[test]
    fn test_linear_scale_inversion() {
        let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 500.0);
        let y_scale = LinearScale::new().domain(-10.0, 10.0).range(0.0, 200.0);

        // Select middle 50% of the chart
        let sel = BrushSelection::new(125.0, 50.0, 375.0, 150.0);
        let domain = sel.to_domain(&x_scale, &y_scale);

        // X should be 25% to 75% of domain = 25 to 75
        assert!((domain.x0 - 25.0).abs() < 0.01);
        assert!((domain.x1 - 75.0).abs() < 0.01);

        // Y is inverted: pixel 50-150 corresponds to domain -5 to 5
        // pixel 50 = top = high domain value, pixel 150 = bottom = low domain value
        assert!((domain.y0 - (-5.0)).abs() < 0.01);
        assert!((domain.y1 - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_log_scale_inversion() {
        let x_scale = LogScale::new().domain(20.0, 20000.0).range(0.0, 500.0);
        let y_scale = LinearScale::new().domain(-40.0, 10.0).range(0.0, 200.0);

        // Select a region
        let sel = BrushSelection::new(0.0, 0.0, 250.0, 100.0);
        let domain = sel.to_domain(&x_scale, &y_scale);

        // X: 0 -> 20Hz, 250 (half range) -> geometric mean of 20 and 20000
        assert!((domain.x0 - 20.0).abs() < 0.1);
        // sqrt(20 * 20000) = sqrt(400000) ≈ 632
        assert!((domain.x1 - 632.0).abs() < 5.0);

        // Y: non-inverted linear scale
        // pixel 0 -> domain -40, pixel 100 -> domain -15 (midpoint)
        // DomainSelection normalizes so y0 < y1
        assert!((domain.y0 - (-40.0)).abs() < 0.1);
        assert!((domain.y1 - (-15.0)).abs() < 0.1);
    }
}
