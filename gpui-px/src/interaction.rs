//! Interactive chart support with brush, zoom, and mouse event handling.
//!
//! This module provides interactive capabilities for gpui-px charts,
//! integrating d3rs brush and zoom functionality with GPUI mouse events.
//!
//! # Features
//!
//! - **Brush Selection**: Click and drag to select rectangular regions
//! - **Zoom to Selection**: Zoom into brushed regions
//! - **Zoom History**: Navigate back through zoom levels
//! - **Double-click Reset**: Reset to original view
//! - **Hover Events**: Track mouse position for tooltips
//!
//! # Example
//!
//! ```rust,no_run
//! use gpui_px::interaction::{ChartInteraction, InteractionMode};
//!
//! // Create interaction state for a chart
//! let mut interaction = ChartInteraction::new(20.0, 20000.0, -40.0, 10.0)
//!     .with_log_x(true)
//!     .with_size(500.0, 300.0)
//!     .with_mode(InteractionMode::Brush);
//!
//! // Handle mouse events
//! interaction.start_brush(100.0, 50.0);
//! interaction.update_brush(400.0, 200.0);
//!
//! // End brush and apply zoom
//! if let Some(selection) = interaction.end_brush(true) {
//!     println!("Zoomed to: {:?}", selection);
//! }
//!
//! // Double-click to reset
//! interaction.reset_zoom();
//! ```

use d3rs::brush::{BrushConfig, BrushSelection, BrushState, DomainSelection};
use d3rs::scale::{LinearScale, LogScale, Scale};
use d3rs::zoom::{ZoomConfig, ZoomState};
use std::sync::Arc;

// Re-export d3rs types for convenience
pub use d3rs::brush::{
    BrushConfig as BrushConfigD3, BrushSelection as BrushSelectionD3,
    DomainSelection as DomainSelectionD3,
};
pub use d3rs::interpolate::zoom::{ZoomParams, ZoomView, interpolate_zoom, zoom_duration};
pub use d3rs::zoom::{ZoomConfig as ZoomConfigD3, ZoomState as ZoomStateD3};

/// Chart interaction mode
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum InteractionMode {
    /// No interaction enabled
    #[default]
    None,
    /// Brush selection mode (click and drag to select)
    Brush,
    /// Pan mode (click and drag to pan)
    Pan,
    /// Zoom mode (scroll wheel to zoom)
    Zoom,
}

/// Callback type for brush end events
pub type BrushEndCallback = Arc<dyn Fn(DomainSelection) + Send + Sync>;

/// Callback type for zoom change events
pub type ZoomChangeCallback = Arc<dyn Fn(&ZoomState) + Send + Sync>;

/// Callback type for hover events (position in domain coordinates)
pub type HoverCallback = Arc<dyn Fn(Option<(f64, f64)>) + Send + Sync>;

/// Callback type for click events (position in domain coordinates)
pub type ClickCallback = Arc<dyn Fn(f64, f64) + Send + Sync>;

/// Chart interaction state that can be shared between components.
///
/// This struct maintains the state of brush selection and zoom levels,
/// allowing multiple components to react to chart interactions.
#[derive(Clone)]
pub struct ChartInteraction {
    /// Current brush state
    pub brush: BrushState,
    /// Current zoom state
    pub zoom: ZoomState,
    /// Brush configuration
    pub brush_config: BrushConfig,
    /// Zoom configuration
    pub zoom_config: ZoomConfig,
    /// Current interaction mode
    pub mode: InteractionMode,
    /// Whether X-axis uses log scale
    pub x_is_log: bool,
    /// Whether Y-axis uses log scale
    pub y_is_log: bool,
    /// Plot dimensions (width, height)
    pub plot_size: (f32, f32),
}

impl Default for ChartInteraction {
    fn default() -> Self {
        Self {
            brush: BrushState::new(),
            zoom: ZoomState::default(),
            brush_config: BrushConfig::default(),
            zoom_config: ZoomConfig::default(),
            mode: InteractionMode::None,
            x_is_log: false,
            y_is_log: false,
            plot_size: (600.0, 400.0),
        }
    }
}

impl ChartInteraction {
    /// Create a new chart interaction state with specified domain bounds.
    pub fn new(x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> Self {
        Self {
            brush: BrushState::new(),
            zoom: ZoomState::new(x_min, x_max, y_min, y_max),
            brush_config: BrushConfig::default(),
            zoom_config: ZoomConfig::default(),
            mode: InteractionMode::Brush,
            x_is_log: false,
            y_is_log: false,
            plot_size: (600.0, 400.0),
        }
    }

    /// Set X-axis to logarithmic scale.
    pub fn with_log_x(mut self, is_log: bool) -> Self {
        self.x_is_log = is_log;
        self.zoom = self.zoom.with_log_x(is_log);
        self
    }

    /// Set Y-axis to logarithmic scale.
    pub fn with_log_y(mut self, is_log: bool) -> Self {
        self.y_is_log = is_log;
        self.zoom = self.zoom.with_log_y(is_log);
        self
    }

    /// Set the plot dimensions.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.plot_size = (width, height);
        self
    }

    /// Set the interaction mode.
    pub fn with_mode(mut self, mode: InteractionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set brush configuration.
    pub fn with_brush_config(mut self, config: BrushConfig) -> Self {
        self.brush_config = config;
        self
    }

    /// Set zoom configuration.
    pub fn with_zoom_config(mut self, config: ZoomConfig) -> Self {
        self.zoom_config = config;
        self
    }

    /// Start a brush selection at the given pixel coordinates.
    pub fn start_brush(&mut self, x: f32, y: f32) {
        self.brush.start(x as f64, y as f64);
    }

    /// Update the brush selection while dragging.
    pub fn update_brush(&mut self, x: f32, y: f32) {
        self.brush.update(x as f64, y as f64);
    }

    /// End the brush selection and optionally apply zoom.
    ///
    /// Returns the domain selection if the brush was valid.
    pub fn end_brush(&mut self, apply_zoom: bool) -> Option<DomainSelection> {
        let pixel_selection = self.brush.end()?;

        // Check if selection is too small
        if pixel_selection.is_trivial(self.brush_config.min_size) {
            return None;
        }

        // Convert to domain coordinates
        let domain = self.pixel_to_domain(&pixel_selection);

        // Apply zoom if requested
        if apply_zoom {
            self.zoom
                .zoom_to(domain.x0, domain.x1, domain.y0, domain.y1);
        }

        Some(domain)
    }

    /// Cancel the current brush selection.
    pub fn cancel_brush(&mut self) {
        self.brush.reset();
    }

    /// Get the current brush selection rectangle (if active).
    pub fn current_brush_selection(&self) -> Option<BrushSelection> {
        self.brush.current_selection()
    }

    /// Check if a brush selection is currently active.
    pub fn is_brushing(&self) -> bool {
        self.brush.is_active()
    }

    /// Zoom to a specific domain region.
    pub fn zoom_to(&mut self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) {
        self.zoom.zoom_to(x_min, x_max, y_min, y_max);
    }

    /// Reset zoom to original view.
    pub fn reset_zoom(&mut self) {
        self.zoom.reset();
    }

    /// Go back one zoom level.
    pub fn zoom_back(&mut self) -> bool {
        self.zoom.zoom_back()
    }

    /// Check if currently zoomed.
    pub fn is_zoomed(&self) -> bool {
        self.zoom.is_zoomed()
    }

    /// Get current X domain.
    pub fn x_domain(&self) -> (f64, f64) {
        self.zoom.x_domain()
    }

    /// Get current Y domain.
    pub fn y_domain(&self) -> (f64, f64) {
        self.zoom.y_domain()
    }

    /// Get the current zoom level (number of zoom operations).
    pub fn zoom_level(&self) -> usize {
        self.zoom.zoom_level()
    }

    /// Convert pixel coordinates to domain coordinates.
    pub fn pixel_to_domain(&self, selection: &BrushSelection) -> DomainSelection {
        let (width, height) = self.plot_size;
        let (x_min, x_max) = self.zoom.x_domain();
        let (y_min, y_max) = self.zoom.y_domain();

        if self.x_is_log {
            let x_scale = LogScale::new()
                .domain(x_min.max(1e-10), x_max)
                .range(0.0, width as f64);
            if self.y_is_log {
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(height as f64, 0.0);
                selection.to_domain(&x_scale, &y_scale)
            } else {
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(height as f64, 0.0);
                selection.to_domain(&x_scale, &y_scale)
            }
        } else {
            let x_scale = LinearScale::new()
                .domain(x_min, x_max)
                .range(0.0, width as f64);
            if self.y_is_log {
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(height as f64, 0.0);
                selection.to_domain(&x_scale, &y_scale)
            } else {
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(height as f64, 0.0);
                selection.to_domain(&x_scale, &y_scale)
            }
        }
    }

    /// Convert a single pixel point to domain coordinates.
    pub fn point_to_domain(&self, x: f32, y: f32) -> (f64, f64) {
        let (width, height) = self.plot_size;
        let (x_min, x_max) = self.zoom.x_domain();
        let (y_min, y_max) = self.zoom.y_domain();

        let domain_x = if self.x_is_log {
            let x_scale = LogScale::new()
                .domain(x_min.max(1e-10), x_max)
                .range(0.0, width as f64);
            x_scale.invert(x as f64).unwrap_or(x_min)
        } else {
            let x_scale = LinearScale::new()
                .domain(x_min, x_max)
                .range(0.0, width as f64);
            x_scale.invert(x as f64).unwrap_or(x_min)
        };

        let domain_y = if self.y_is_log {
            let y_scale = LogScale::new()
                .domain(y_min.max(1e-10), y_max)
                .range(height as f64, 0.0);
            y_scale.invert(y as f64).unwrap_or(y_min)
        } else {
            let y_scale = LinearScale::new()
                .domain(y_min, y_max)
                .range(height as f64, 0.0);
            y_scale.invert(y as f64).unwrap_or(y_min)
        };

        (domain_x, domain_y)
    }
}

/// Mouse event state for tracking interactions.
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseState {
    /// Current mouse position in pixels (relative to plot area)
    pub position: Option<(f32, f32)>,
    /// Whether the left mouse button is pressed
    pub left_down: bool,
    /// Whether the right mouse button is pressed
    pub right_down: bool,
    /// Last click timestamp for double-click detection
    pub last_click_time: Option<std::time::Instant>,
    /// Last click position for double-click detection
    pub last_click_pos: Option<(f32, f32)>,
}

impl MouseState {
    /// Check if this is a double-click event.
    ///
    /// Returns true if the click occurred within 300ms and 5 pixels of the last click.
    pub fn is_double_click(&self, x: f32, y: f32) -> bool {
        if let (Some(last_time), Some((last_x, last_y))) =
            (self.last_click_time, self.last_click_pos)
        {
            let elapsed = last_time.elapsed();
            let distance = ((x - last_x).powi(2) + (y - last_y).powi(2)).sqrt();
            elapsed.as_millis() < 300 && distance < 5.0
        } else {
            false
        }
    }

    /// Record a click for double-click detection.
    pub fn record_click(&mut self, x: f32, y: f32) {
        self.last_click_time = Some(std::time::Instant::now());
        self.last_click_pos = Some((x, y));
    }
}

/// Configuration for chart mouse wheel behavior.
#[derive(Debug, Clone, Copy)]
pub struct WheelConfig {
    /// Zoom factor per scroll step (default: 1.1)
    pub zoom_factor: f64,
    /// Enable horizontal scroll for X-axis panning
    pub horizontal_pan: bool,
    /// Invert scroll direction
    pub invert: bool,
}

impl Default for WheelConfig {
    fn default() -> Self {
        Self {
            zoom_factor: 1.1,
            horizontal_pan: true,
            invert: false,
        }
    }
}

/// Apply mouse wheel zoom to chart interaction state.
///
/// # Arguments
/// * `interaction` - The chart interaction state to modify
/// * `delta_y` - Vertical scroll delta (positive = zoom out, negative = zoom in)
/// * `mouse_x` - Mouse X position in pixels (for zoom center)
/// * `mouse_y` - Mouse Y position in pixels (for zoom center)
/// * `config` - Wheel configuration
pub fn apply_wheel_zoom(
    interaction: &mut ChartInteraction,
    delta_y: f32,
    mouse_x: f32,
    mouse_y: f32,
    config: &WheelConfig,
) {
    let (x_min, x_max) = interaction.x_domain();
    let (y_min, y_max) = interaction.y_domain();

    // Get mouse position in domain coordinates
    let (focus_x, focus_y) = interaction.point_to_domain(mouse_x, mouse_y);

    // Calculate zoom factor
    let delta = if config.invert { -delta_y } else { delta_y };
    let factor = if delta > 0.0 {
        config.zoom_factor
    } else {
        1.0 / config.zoom_factor
    };

    // Apply zoom centered on mouse position
    let new_x_min = focus_x - (focus_x - x_min) * factor;
    let new_x_max = focus_x + (x_max - focus_x) * factor;
    let new_y_min = focus_y - (focus_y - y_min) * factor;
    let new_y_max = focus_y + (y_max - focus_y) * factor;

    interaction.zoom_to(new_x_min, new_x_max, new_y_min, new_y_max);
}

// ============================================================================
// GPUI-specific rendering functions (only available with gpui feature)
// ============================================================================

#[cfg(feature = "gpui")]
mod gpui_render {
    use super::*;
    use d3rs::zoom::ZoomState;
    use gpui::prelude::*;
    use gpui::{IntoElement, div, hsla, px};

    /// Render a brush selection overlay.
    ///
    /// This renders a semi-transparent rectangle showing the current brush selection.
    pub fn render_brush_overlay(
        selection: &BrushSelection,
        config: &BrushConfig,
    ) -> impl IntoElement {
        let x = selection.x0 as f32;
        let y = selection.y0 as f32;
        let width = selection.width() as f32;
        let height = selection.height() as f32;

        let (_r, _g, _b, a) = config.fill_color;

        div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(width))
            .h(px(height))
            .bg(hsla(210.0 / 360.0, 0.5, 0.6, a as f32 / 255.0))
            .border_1()
            .border_color(hsla(210.0 / 360.0, 0.5, 0.4, 1.0))
    }

    /// Render a zoom indicator showing the current zoom level.
    pub fn render_zoom_indicator(zoom: &ZoomState, x: f32, y: f32) -> impl IntoElement {
        let level = zoom.zoom_level();
        if level == 0 {
            return div().into_any_element();
        }

        let text = format!("Zoom: {}x", level);

        div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .px_2()
            .py_1()
            .bg(hsla(0.0, 0.0, 0.2, 0.8))
            .rounded_md()
            .child(text)
            .text_color(hsla(0.0, 0.0, 1.0, 1.0))
            .text_xs()
            .into_any_element()
    }

    /// Render a reset button for zoom.
    pub fn render_reset_button<F>(x: f32, y: f32, on_click: F) -> impl IntoElement
    where
        F: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    {
        div()
            .id("reset-button")
            .absolute()
            .left(px(x))
            .top(px(y))
            .px_2()
            .py_1()
            .bg(hsla(0.0, 0.0, 0.3, 0.9))
            .rounded_md()
            .child("Reset")
            .text_color(hsla(0.0, 0.0, 1.0, 1.0))
            .text_xs()
            .cursor_pointer()
            .hover(|s| s.bg(hsla(0.0, 0.0, 0.4, 0.9)))
            .on_click(on_click)
    }

    /// Render crosshairs at the mouse position.
    pub fn render_crosshairs(x: f32, y: f32, width: f32, height: f32) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .size_full()
            .child(
                // Vertical line
                div()
                    .absolute()
                    .left(px(x))
                    .top_0()
                    .w_px()
                    .h(px(height))
                    .bg(hsla(0.0, 0.0, 0.5, 0.5)),
            )
            .child(
                // Horizontal line
                div()
                    .absolute()
                    .left_0()
                    .top(px(y))
                    .w(px(width))
                    .h_px()
                    .bg(hsla(0.0, 0.0, 0.5, 0.5)),
            )
    }
}

#[cfg(feature = "gpui")]
pub use gpui_render::{
    render_brush_overlay, render_crosshairs, render_reset_button, render_zoom_indicator,
};

// ============================================================================
// InteractiveChart Component
// ============================================================================

#[cfg(feature = "gpui")]
mod interactive_chart {
    use super::*;
    use gpui::prelude::*;
    use gpui::{
        AnyElement, ClickEvent, ElementId, IntoElement, MouseButton, Pixels, Point, ScrollDelta,
        ScrollWheelEvent, div, hsla, px,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Callback type for when zoom state changes
    pub type OnZoomChange = Rc<dyn Fn((f64, f64), (f64, f64))>;

    /// Configuration for interactive chart behavior
    #[derive(Clone)]
    pub struct InteractiveChartConfig {
        /// Enable pan/drag with left mouse button
        pub enable_pan: bool,
        /// Enable scroll wheel zoom
        pub enable_wheel_zoom: bool,
        /// Enable double-click to reset zoom
        pub enable_double_click_reset: bool,
        /// Show zoom indicator when zoomed
        pub show_zoom_indicator: bool,
        /// Wheel zoom configuration
        pub wheel_config: WheelConfig,
        /// Left margin (for axis labels) - mouse coordinates are adjusted by this
        pub left_margin: f32,
        /// Top margin (for title) - mouse coordinates are adjusted by this
        pub top_margin: f32,
    }

    impl Default for InteractiveChartConfig {
        fn default() -> Self {
            Self {
                enable_pan: true,
                enable_wheel_zoom: true,
                enable_double_click_reset: true,
                show_zoom_indicator: true,
                wheel_config: WheelConfig::default(),
                left_margin: 50.0,
                top_margin: 30.0,
            }
        }
    }

    impl InteractiveChartConfig {
        /// Create a new config with all interactions enabled
        pub fn new() -> Self {
            Self::default()
        }

        /// Set left margin for axis labels
        pub fn with_left_margin(mut self, margin: f32) -> Self {
            self.left_margin = margin;
            self
        }

        /// Set top margin for title
        pub fn with_top_margin(mut self, margin: f32) -> Self {
            self.top_margin = margin;
            self
        }

        /// Enable or disable pan/drag
        pub fn with_pan(mut self, enable: bool) -> Self {
            self.enable_pan = enable;
            self
        }

        /// Enable or disable wheel zoom
        pub fn with_wheel_zoom(mut self, enable: bool) -> Self {
            self.enable_wheel_zoom = enable;
            self
        }

        /// Enable or disable double-click reset
        pub fn with_double_click_reset(mut self, enable: bool) -> Self {
            self.enable_double_click_reset = enable;
            self
        }
    }

    /// Shared state for interactive chart that can be passed to chart builders
    #[derive(Clone)]
    pub struct InteractiveChartState {
        /// The chart interaction state (zoom, brush)
        pub interaction: Rc<RefCell<ChartInteraction>>,
        /// Configuration
        pub config: InteractiveChartConfig,
        /// Callback when zoom changes
        pub on_zoom_change: Option<OnZoomChange>,
    }

    impl InteractiveChartState {
        /// Create a new interactive chart state with specified domain bounds
        pub fn new(x_min: f64, x_max: f64, y_min: f64, y_max: f64) -> Self {
            Self {
                interaction: Rc::new(RefCell::new(ChartInteraction::new(
                    x_min, x_max, y_min, y_max,
                ))),
                config: InteractiveChartConfig::default(),
                on_zoom_change: None,
            }
        }

        /// Set X-axis to logarithmic scale
        pub fn with_log_x(self, is_log: bool) -> Self {
            self.interaction.borrow_mut().x_is_log = is_log;
            {
                let mut interaction = self.interaction.borrow_mut();
                interaction.zoom = interaction.zoom.clone().with_log_x(is_log);
            }
            self
        }

        /// Set Y-axis to logarithmic scale
        pub fn with_log_y(self, is_log: bool) -> Self {
            self.interaction.borrow_mut().y_is_log = is_log;
            {
                let mut interaction = self.interaction.borrow_mut();
                interaction.zoom = interaction.zoom.clone().with_log_y(is_log);
            }
            self
        }

        /// Set the plot dimensions
        pub fn with_size(self, width: f32, height: f32) -> Self {
            self.interaction.borrow_mut().plot_size = (width, height);
            self
        }

        /// Set the configuration
        pub fn with_config(mut self, config: InteractiveChartConfig) -> Self {
            self.config = config;
            self
        }

        /// Set callback for zoom changes
        pub fn on_zoom_change<F>(mut self, callback: F) -> Self
        where
            F: Fn((f64, f64), (f64, f64)) + 'static,
        {
            self.on_zoom_change = Some(Rc::new(callback));
            self
        }

        /// Get current X domain (for use in chart builders)
        pub fn x_domain(&self) -> (f64, f64) {
            self.interaction.borrow().x_domain()
        }

        /// Get current Y domain (for use in chart builders)
        pub fn y_domain(&self) -> (f64, f64) {
            self.interaction.borrow().y_domain()
        }

        /// Check if currently zoomed
        pub fn is_zoomed(&self) -> bool {
            self.interaction.borrow().is_zoomed()
        }

        /// Get current brush selection
        pub fn current_brush_selection(&self) -> Option<BrushSelection> {
            self.interaction.borrow().current_brush_selection()
        }

        /// Reset zoom to original view
        pub fn reset_zoom(&self) {
            self.interaction.borrow_mut().reset_zoom();
            if let Some(ref callback) = self.on_zoom_change {
                let interaction = self.interaction.borrow();
                callback(interaction.x_domain(), interaction.y_domain());
            }
        }

        /// Convert pixel coordinates to chart-relative coordinates
        /// Uses the configured margins to offset from the element position
        fn to_chart_coords(&self, pos: Point<Pixels>) -> (f32, f32) {
            let config = &self.config;
            let interaction = self.interaction.borrow();
            let (plot_width, plot_height) = interaction.plot_size;

            // Subtract margins to get chart-relative coordinates
            let chart_x = (f32::from(pos.x) - config.left_margin)
                .max(0.0)
                .min(plot_width);
            let chart_y = (f32::from(pos.y) - config.top_margin)
                .max(0.0)
                .min(plot_height);
            (chart_x, chart_y)
        }

        /// Apply pan delta to the zoom state
        pub fn apply_pan(&self, dx: f32, dy: f32) {
            let mut interaction = self.interaction.borrow_mut();
            let (plot_width, plot_height) = interaction.plot_size;
            let (x_min, x_max) = interaction.x_domain();
            let (y_min, y_max) = interaction.y_domain();

            // Convert pixel delta to domain delta
            let x_range = x_max - x_min;
            let y_range = y_max - y_min;

            // For log scale, we need to handle panning differently
            let (new_x_min, new_x_max) = if interaction.x_is_log {
                // For log scale, pan in log space
                let log_min = x_min.log10();
                let log_max = x_max.log10();
                let log_range = log_max - log_min;
                let log_delta = -(dx as f64 / plot_width as f64) * log_range;
                (10_f64.powf(log_min + log_delta), 10_f64.powf(log_max + log_delta))
            } else {
                let delta = -(dx as f64 / plot_width as f64) * x_range;
                (x_min + delta, x_max + delta)
            };

            let (new_y_min, new_y_max) = if interaction.y_is_log {
                let log_min = y_min.log10();
                let log_max = y_max.log10();
                let log_range = log_max - log_min;
                let log_delta = (dy as f64 / plot_height as f64) * log_range;
                (10_f64.powf(log_min + log_delta), 10_f64.powf(log_max + log_delta))
            } else {
                // Y is inverted (screen coords vs domain coords)
                let delta = (dy as f64 / plot_height as f64) * y_range;
                (y_min + delta, y_max + delta)
            };

            interaction.zoom_to(new_x_min, new_x_max, new_y_min, new_y_max);
        }
    }

    /// Builder for creating an interactive chart wrapper
    pub struct InteractiveChart {
        /// The chart element to wrap
        child: AnyElement,
        /// Shared state
        state: InteractiveChartState,
        /// Element ID for the wrapper
        id: ElementId,
    }

    impl InteractiveChart {
        /// Create a new interactive chart wrapper
        pub fn new(
            id: impl Into<ElementId>,
            child: impl IntoElement,
            state: InteractiveChartState,
        ) -> Self {
            Self {
                child: child.into_any_element(),
                state,
                id: id.into(),
            }
        }

        /// Build the interactive chart element
        pub fn build(self) -> impl IntoElement {
            let state = self.state.clone();
            let state_for_down = self.state.clone();
            let state_for_move = self.state.clone();
            let state_for_click = self.state.clone();
            let state_for_wheel = self.state.clone();

            let is_zoomed = state.is_zoomed();
            let config = state.config.clone();

            // Track drag state using RefCell for interior mutability
            let drag_start: Rc<RefCell<Option<(f32, f32)>>> = Rc::new(RefCell::new(None));
            let drag_start_down = drag_start.clone();
            let drag_start_move = drag_start.clone();
            let drag_start_up = drag_start.clone();

            div()
                .id(self.id)
                .relative()
                .cursor_grab()
                .child(self.child)
                // Zoom indicator
                .when(is_zoomed && config.show_zoom_indicator, |el| {
                    el.child(
                        div()
                            .absolute()
                            .right(px(10.0))
                            .top(px(10.0))
                            .px_2()
                            .py_1()
                            .bg(hsla(0.0, 0.0, 0.2, 0.7))
                            .rounded_md()
                            .text_xs()
                            .text_color(hsla(0.0, 0.0, 1.0, 0.9))
                            .child("Zoomed (double-click to reset)"),
                    )
                })
                // Mouse down - start pan
                .on_mouse_down(MouseButton::Left, move |event, _window, _cx| {
                    if state_for_down.config.enable_pan {
                        let (x, y) = state_for_down.to_chart_coords(event.position);
                        *drag_start_down.borrow_mut() = Some((x, y));
                    }
                })
                // Mouse move - pan if dragging
                .on_mouse_move(move |event, window, _cx| {
                    if state_for_move.config.enable_pan {
                        if let Some((start_x, start_y)) = *drag_start_move.borrow() {
                            let (x, y) = state_for_move.to_chart_coords(event.position);
                            let dx = x - start_x;
                            let dy = y - start_y;
                            if dx.abs() > 1.0 || dy.abs() > 1.0 {
                                state_for_move.apply_pan(dx, dy);
                                // Update drag start to current position for continuous panning
                                *drag_start_move.borrow_mut() = Some((x, y));
                                // Trigger re-render
                                window.refresh();
                            }
                        }
                    }
                })
                // Mouse up - end pan
                .on_mouse_up(MouseButton::Left, move |_event, _window, _cx| {
                    *drag_start_up.borrow_mut() = None;
                })
                // Click - handle double-click reset
                .on_click(move |event: &ClickEvent, window, _cx| {
                    if state_for_click.config.enable_double_click_reset && event.click_count() >= 2
                    {
                        state_for_click.reset_zoom();
                        window.refresh();
                    }
                })
                // Scroll wheel - zoom
                .on_scroll_wheel(move |event: &ScrollWheelEvent, window, _cx| {
                    if state_for_wheel.config.enable_wheel_zoom {
                        let (x, y) = state_for_wheel.to_chart_coords(event.position);
                        let delta_y = match event.delta {
                            ScrollDelta::Lines(lines) => lines.y,
                            ScrollDelta::Pixels(pixels) => f32::from(pixels.y) * 0.01,
                        };

                        apply_wheel_zoom(
                            &mut state_for_wheel.interaction.borrow_mut(),
                            delta_y,
                            x,
                            y,
                            &state_for_wheel.config.wheel_config,
                        );

                        // Notify zoom change
                        if let Some(ref callback) = state_for_wheel.on_zoom_change {
                            let interaction = state_for_wheel.interaction.borrow();
                            callback(interaction.x_domain(), interaction.y_domain());
                        }

                        // Trigger re-render
                        window.refresh();
                    }
                })
        }
    }

    /// Helper function to wrap a chart element with interactive behavior
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use gpui_px::{line, ScaleType};
    /// use gpui_px::interaction::{InteractiveChartState, interactive};
    ///
    /// // Create shared state
    /// let state = InteractiveChartState::new(20.0, 20000.0, -40.0, 10.0)
    ///     .with_log_x(true)
    ///     .with_size(800.0, 400.0);
    ///
    /// // Build chart with zoom-adjusted ranges
    /// let chart = line(&freq, &spl)
    ///     .x_scale(ScaleType::Log)
    ///     .x_range(state.x_domain().0, state.x_domain().1)
    ///     .y_range(state.y_domain().0, state.y_domain().1)
    ///     .build()?;
    ///
    /// // Wrap with interactive behavior
    /// let interactive_chart = interactive("my-chart", chart, state.clone())
    ///     .build(cx, app);
    /// ```
    pub fn interactive(
        id: impl Into<ElementId>,
        child: impl IntoElement,
        state: InteractiveChartState,
    ) -> InteractiveChart {
        InteractiveChart::new(id, child, state)
    }
}

#[cfg(feature = "gpui")]
pub use interactive_chart::{
    InteractiveChart, InteractiveChartConfig, InteractiveChartState, OnZoomChange, interactive,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_interaction_creation() {
        let interaction = ChartInteraction::new(0.0, 100.0, -10.0, 10.0);
        assert_eq!(interaction.x_domain(), (0.0, 100.0));
        assert_eq!(interaction.y_domain(), (-10.0, 10.0));
        assert!(!interaction.is_zoomed());
    }

    #[test]
    fn test_brush_lifecycle() {
        let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0).with_size(500.0, 500.0);

        assert!(!interaction.is_brushing());

        interaction.start_brush(100.0, 100.0);
        assert!(interaction.is_brushing());

        interaction.update_brush(300.0, 300.0);
        let selection = interaction.current_brush_selection().unwrap();
        assert_eq!(selection.width(), 200.0);
        assert_eq!(selection.height(), 200.0);

        let domain = interaction.end_brush(false).unwrap();
        assert!(!interaction.is_brushing());
        // Domain values depend on scale conversion
        assert!(domain.x1 > domain.x0);
        assert!(domain.y1 > domain.y0);
    }

    #[test]
    fn test_brush_with_zoom() {
        let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0).with_size(500.0, 500.0);

        // Brush the center 50% of the chart
        interaction.start_brush(125.0, 125.0);
        interaction.update_brush(375.0, 375.0);
        interaction.end_brush(true);

        // Should now be zoomed
        assert!(interaction.is_zoomed());
        let (x_min, x_max) = interaction.x_domain();
        assert!(x_min > 0.0 && x_max < 100.0);
    }

    #[test]
    fn test_zoom_reset() {
        let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0);

        interaction.zoom_to(25.0, 75.0, 25.0, 75.0);
        assert!(interaction.is_zoomed());

        interaction.reset_zoom();
        assert!(!interaction.is_zoomed());
        assert_eq!(interaction.x_domain(), (0.0, 100.0));
    }

    #[test]
    fn test_zoom_back() {
        let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0);

        interaction.zoom_to(25.0, 75.0, 25.0, 75.0);
        interaction.zoom_to(35.0, 65.0, 35.0, 65.0);
        assert_eq!(interaction.zoom_level(), 2);

        interaction.zoom_back();
        assert_eq!(interaction.zoom_level(), 1);
        assert_eq!(interaction.x_domain(), (25.0, 75.0));

        interaction.zoom_back();
        assert!(!interaction.is_zoomed());
    }

    #[test]
    fn test_log_scale_interaction() {
        let interaction = ChartInteraction::new(20.0, 20000.0, -40.0, 10.0)
            .with_log_x(true)
            .with_size(500.0, 200.0);

        // Get domain point at center of chart
        let (x, _y) = interaction.point_to_domain(250.0, 100.0);

        // For log scale, center should be geometric mean: sqrt(20 * 20000) ≈ 632
        assert!((x - 632.0).abs() < 50.0);
    }

    #[test]
    fn test_double_click_detection() {
        let mut state = MouseState::default();

        // First click - not a double click
        assert!(!state.is_double_click(100.0, 100.0));
        state.record_click(100.0, 100.0);

        // Immediate second click - should be double click
        assert!(state.is_double_click(101.0, 101.0));

        // Far away click - not a double click
        assert!(!state.is_double_click(200.0, 200.0));
    }

    #[test]
    fn test_wheel_zoom() {
        let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0).with_size(500.0, 500.0);
        let config = WheelConfig::default();

        let original_x = interaction.x_domain();

        // Zoom in (negative delta)
        apply_wheel_zoom(&mut interaction, -1.0, 250.0, 250.0, &config);

        // Should be zoomed in (smaller domain range)
        let new_x = interaction.x_domain();
        assert!(new_x.1 - new_x.0 < original_x.1 - original_x.0);
    }

    #[test]
    fn test_interaction_mode() {
        let interaction = ChartInteraction::default();
        assert_eq!(interaction.mode, InteractionMode::None);

        let interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0);
        assert_eq!(interaction.mode, InteractionMode::Brush);

        let interaction = interaction.with_mode(InteractionMode::Zoom);
        assert_eq!(interaction.mode, InteractionMode::Zoom);
    }

    #[test]
    fn test_brush_config() {
        let config = BrushConfig {
            fill_color: (255, 0, 0, 128),
            stroke_color: (255, 0, 0),
            stroke_width: 2.0,
            min_size: 10.0,
        };

        let interaction = ChartInteraction::default().with_brush_config(config.clone());
        assert_eq!(interaction.brush_config.min_size, 10.0);
    }

    #[test]
    fn test_trivial_brush_rejected() {
        let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0).with_size(500.0, 500.0);

        // Very small brush (less than min_size of 5.0)
        interaction.start_brush(100.0, 100.0);
        interaction.update_brush(102.0, 102.0);
        let result = interaction.end_brush(false);

        // Should return None because selection is too small
        assert!(result.is_none());
    }

    #[test]
    fn test_cancel_brush() {
        let mut interaction = ChartInteraction::new(0.0, 100.0, 0.0, 100.0);

        interaction.start_brush(100.0, 100.0);
        assert!(interaction.is_brushing());

        interaction.cancel_brush();
        assert!(!interaction.is_brushing());
        assert!(interaction.current_brush_selection().is_none());
    }

    #[cfg(feature = "gpui")]
    mod interactive_chart_state_tests {
        use super::super::interactive_chart::*;

        #[test]
        fn test_interactive_chart_state_creation() {
            let state = InteractiveChartState::new(20.0, 20000.0, -40.0, 10.0);
            assert_eq!(state.x_domain(), (20.0, 20000.0));
            assert_eq!(state.y_domain(), (-40.0, 10.0));
            assert!(!state.is_zoomed());
        }

        #[test]
        fn test_interactive_chart_state_with_log_x() {
            let state = InteractiveChartState::new(20.0, 20000.0, -40.0, 10.0).with_log_x(true);
            assert!(state.interaction.borrow().x_is_log);
        }

        #[test]
        fn test_interactive_chart_state_with_size() {
            let state =
                InteractiveChartState::new(20.0, 20000.0, -40.0, 10.0).with_size(800.0, 400.0);
            assert_eq!(state.interaction.borrow().plot_size, (800.0, 400.0));
        }

        #[test]
        fn test_interactive_chart_state_reset_zoom() {
            let state = InteractiveChartState::new(0.0, 100.0, 0.0, 100.0);

            // Zoom in
            state.interaction.borrow_mut().zoom_to(25.0, 75.0, 25.0, 75.0);
            assert!(state.is_zoomed());

            // Reset
            state.reset_zoom();
            assert!(!state.is_zoomed());
            assert_eq!(state.x_domain(), (0.0, 100.0));
        }

        #[test]
        fn test_interactive_chart_config() {
            let config = InteractiveChartConfig::new()
                .with_left_margin(60.0)
                .with_top_margin(40.0)
                .with_pan(true)
                .with_wheel_zoom(true)
                .with_double_click_reset(true);

            assert_eq!(config.left_margin, 60.0);
            assert_eq!(config.top_margin, 40.0);
            assert!(config.enable_pan);
            assert!(config.enable_wheel_zoom);
            assert!(config.enable_double_click_reset);
        }

        #[test]
        fn test_interactive_chart_state_with_config() {
            let config = InteractiveChartConfig::new()
                .with_left_margin(80.0)
                .with_pan(false);

            let state =
                InteractiveChartState::new(0.0, 100.0, 0.0, 100.0).with_config(config.clone());

            assert_eq!(state.config.left_margin, 80.0);
            assert!(!state.config.enable_pan);
        }
    }
}
