use super::misc::clamp_log_domain;
use super::types::InteractionMode;
use super::wheel_config::WheelConfig;
use d3rs::brush::{BrushConfig, BrushSelection, BrushState, DomainSelection};
use d3rs::scale::{LinearScale, LogScale, Scale};
use d3rs::zoom::{ZoomConfig, ZoomState};

/// Keyboard action that can be applied to chart interaction state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartKeyboardAction {
    /// Zoom in around the plot center.
    ZoomIn,
    /// Zoom out around the plot center.
    ZoomOut,
    /// Pan the visible domain left.
    PanLeft,
    /// Pan the visible domain right.
    PanRight,
    /// Pan the visible domain up.
    PanUp,
    /// Pan the visible domain down.
    PanDown,
    /// Reset zoom to the original domain.
    ResetZoom,
}

/// Map GPUI/platform key names to chart navigation actions.
pub fn keyboard_action_for_key(key: &str) -> Option<ChartKeyboardAction> {
    match key.to_ascii_lowercase().as_str() {
        "+" | "=" | "add" => Some(ChartKeyboardAction::ZoomIn),
        "-" | "_" | "subtract" => Some(ChartKeyboardAction::ZoomOut),
        "left" | "arrowleft" => Some(ChartKeyboardAction::PanLeft),
        "right" | "arrowright" => Some(ChartKeyboardAction::PanRight),
        "up" | "arrowup" => Some(ChartKeyboardAction::PanUp),
        "down" | "arrowdown" => Some(ChartKeyboardAction::PanDown),
        "0" | "r" | "home" => Some(ChartKeyboardAction::ResetZoom),
        _ => None,
    }
}

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
    /// Last hovered domain coordinate, if the pointer is over the plot.
    pub hover_domain: Option<(f64, f64)>,
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
            hover_domain: None,
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
            hover_domain: None,
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

    /// Update the visible domain without recording a zoom-history entry.
    ///
    /// Retained plots use this for pointer-driven navigation so camera frames
    /// remain allocation-free after warm-up.
    pub fn set_viewport_without_history(&mut self, x_min: f64, x_max: f64, y_min: f64, y_max: f64) {
        self.zoom.set_viewport(x_min, x_max, y_min, y_max);
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

    /// Update the retained hover coordinate from plot-relative pixel coordinates.
    pub fn update_hover_pixel(&mut self, x: f32, y: f32) -> Option<(f64, f64)> {
        let (width, height) = self.plot_size;
        if !x.is_finite()
            || !y.is_finite()
            || width <= 0.0
            || height <= 0.0
            || x < 0.0
            || y < 0.0
            || x > width
            || y > height
        {
            self.hover_domain = None;
            return None;
        }

        let domain = self.point_to_domain(x, y);
        self.hover_domain = Some(domain);
        self.hover_domain
    }

    /// Clear retained hover state.
    pub fn clear_hover(&mut self) {
        self.hover_domain = None;
    }

    /// Return the retained hover domain coordinate.
    pub fn hover_domain(&self) -> Option<(f64, f64)> {
        self.hover_domain
    }

    /// Apply a pan delta in plot-relative pixels.
    pub fn pan_by_pixels(&mut self, dx: f32, dy: f32) {
        let (plot_width, plot_height) = self.plot_size;
        if plot_width <= 0.0 || plot_height <= 0.0 || !dx.is_finite() || !dy.is_finite() {
            return;
        }

        let (x_min, x_max) = self.x_domain();
        let (y_min, y_max) = self.y_domain();
        let x_range = x_max - x_min;
        let y_range = y_max - y_min;

        let (mut new_x_min, mut new_x_max) = if self.x_is_log {
            let log_min = x_min.max(1e-10).log10();
            let log_max = x_max.max(1e-10).log10();
            let log_range = log_max - log_min;
            let log_delta = -(dx as f64 / plot_width as f64) * log_range;
            (
                10_f64.powf(log_min + log_delta),
                10_f64.powf(log_max + log_delta),
            )
        } else {
            let delta = -(dx as f64 / plot_width as f64) * x_range;
            (x_min + delta, x_max + delta)
        };

        let (mut new_y_min, mut new_y_max) = if self.y_is_log {
            let log_min = y_min.max(1e-10).log10();
            let log_max = y_max.max(1e-10).log10();
            let log_range = log_max - log_min;
            let log_delta = (dy as f64 / plot_height as f64) * log_range;
            (
                10_f64.powf(log_min + log_delta),
                10_f64.powf(log_max + log_delta),
            )
        } else {
            let delta = (dy as f64 / plot_height as f64) * y_range;
            (y_min + delta, y_max + delta)
        };

        if self.x_is_log {
            (new_x_min, new_x_max) = clamp_log_domain(new_x_min, new_x_max);
        }
        if self.y_is_log {
            (new_y_min, new_y_max) = clamp_log_domain(new_y_min, new_y_max);
        }

        self.zoom_to(new_x_min, new_x_max, new_y_min, new_y_max);
    }

    /// Apply a keyboard interaction using renderer-free state transitions.
    pub fn apply_keyboard_action(
        &mut self,
        action: ChartKeyboardAction,
        pan_step_px: f32,
        zoom_factor: f64,
    ) {
        let pan_step = if pan_step_px.is_finite() && pan_step_px > 0.0 {
            pan_step_px
        } else {
            40.0
        };
        let zoom_factor = if zoom_factor.is_finite() && zoom_factor > 1.0 {
            zoom_factor
        } else {
            1.1
        };

        match action {
            ChartKeyboardAction::ZoomIn => {
                let (width, height) = self.plot_size;
                self.zoom_around_pixel(width * 0.5, height * 0.5, 1.0 / zoom_factor);
            }
            ChartKeyboardAction::ZoomOut => {
                let (width, height) = self.plot_size;
                self.zoom_around_pixel(width * 0.5, height * 0.5, zoom_factor);
            }
            ChartKeyboardAction::PanLeft => self.pan_by_pixels(pan_step, 0.0),
            ChartKeyboardAction::PanRight => self.pan_by_pixels(-pan_step, 0.0),
            ChartKeyboardAction::PanUp => self.pan_by_pixels(0.0, pan_step),
            ChartKeyboardAction::PanDown => self.pan_by_pixels(0.0, -pan_step),
            ChartKeyboardAction::ResetZoom => self.reset_zoom(),
        }
    }

    /// Zoom around a plot-relative pixel coordinate by a scale factor.
    pub fn zoom_around_pixel(&mut self, mouse_x: f32, mouse_y: f32, factor: f64) {
        let (focus_x, focus_y) = self.point_to_domain(mouse_x, mouse_y);
        self.zoom_around_domain(focus_x, focus_y, factor);
    }

    /// Zoom around an explicit data-space focal point.
    ///
    /// Callers with a letterboxed or otherwise transformed viewport can use
    /// this instead of converting their coordinates through the full chart
    /// rectangle.
    pub fn zoom_around_domain(&mut self, focus_x: f64, focus_y: f64, factor: f64) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }

        let (x_min, x_max) = self.x_domain();
        let (y_min, y_max) = self.y_domain();

        let mut new_x_min = focus_x - (focus_x - x_min) * factor;
        let mut new_x_max = focus_x + (x_max - focus_x) * factor;
        let mut new_y_min = focus_y - (focus_y - y_min) * factor;
        let mut new_y_max = focus_y + (y_max - focus_y) * factor;

        if self.x_is_log {
            (new_x_min, new_x_max) = clamp_log_domain(new_x_min, new_x_max);
        }
        if self.y_is_log {
            (new_y_min, new_y_max) = clamp_log_domain(new_y_min, new_y_max);
        }

        self.zoom_to(new_x_min, new_x_max, new_y_min, new_y_max);
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
    // Calculate zoom factor
    let delta = if config.invert { -delta_y } else { delta_y };
    let factor = if delta > 0.0 {
        config.zoom_factor
    } else {
        1.0 / config.zoom_factor
    };

    interaction.zoom_around_pixel(mouse_x, mouse_y, factor);
}

#[cfg(feature = "gpui")]
pub(super) mod interactive_chart {
    use super::super::*;
    use super::*;
    use gpui::prelude::*;
    use gpui::{
        AnyElement, ClickEvent, ElementId, IntoElement, KeyDownEvent, MouseButton, Pixels, Point,
        ScrollDelta, ScrollWheelEvent, div, hsla, px,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Callback type for when zoom state changes
    pub type OnZoomChange = Rc<dyn Fn((f64, f64), (f64, f64))>;
    /// Callback used by host views to request a rebuild after local interaction state changes.
    pub type OnInteractionChange = Rc<dyn Fn(&mut gpui::App)>;

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
        /// Callback when hover, brush, pan, zoom, or reset changes retained state.
        pub on_interaction_change: Option<OnInteractionChange>,
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
                on_interaction_change: None,
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

        /// Request a host-view rebuild whenever retained interaction state changes.
        pub fn on_interaction_change<F>(mut self, callback: F) -> Self
        where
            F: Fn(&mut gpui::App) + 'static,
        {
            self.on_interaction_change = Some(Rc::new(callback));
            self
        }

        fn notify_interaction_change(&self, cx: &mut gpui::App) {
            if let Some(callback) = &self.on_interaction_change {
                callback(cx);
            }
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
        pub(crate) fn to_chart_coords(
            &self,
            pos: Point<Pixels>,
            bounds: Option<gpui::Bounds<Pixels>>,
        ) -> (f32, f32) {
            let config = &self.config;
            let interaction = self.interaction.borrow();
            let (plot_width, plot_height) = interaction.plot_size;
            let origin = bounds.map(|bounds| bounds.origin).unwrap_or_default();

            // Subtract margins to get chart-relative coordinates
            let chart_x = (f32::from(pos.x) - f32::from(origin.x) - config.left_margin)
                .max(0.0)
                .min(plot_width);
            let chart_y = (f32::from(pos.y) - f32::from(origin.y) - config.top_margin)
                .max(0.0)
                .min(plot_height);
            (chart_x, chart_y)
        }

        pub(crate) fn is_over_plot(
            &self,
            pos: Point<Pixels>,
            bounds: Option<gpui::Bounds<Pixels>>,
        ) -> bool {
            let origin = bounds.map(|bounds| bounds.origin).unwrap_or_default();
            let local_x = f32::from(pos.x) - f32::from(origin.x);
            let local_y = f32::from(pos.y) - f32::from(origin.y);
            let (plot_width, plot_height) = self.interaction.borrow().plot_size;
            local_x >= self.config.left_margin
                && local_x <= self.config.left_margin + plot_width
                && local_y >= self.config.top_margin
                && local_y <= self.config.top_margin + plot_height
        }

        /// Apply pan delta to the zoom state
        pub fn apply_pan(&self, dx: f32, dy: f32) {
            let mut interaction = self.interaction.borrow_mut();
            interaction.pan_by_pixels(dx, dy);
        }

        fn apply_keyboard_action(&self, action: ChartKeyboardAction) {
            self.interaction
                .borrow_mut()
                .apply_keyboard_action(action, 40.0, 1.2);
            if let Some(ref callback) = self.on_zoom_change {
                let interaction = self.interaction.borrow();
                callback(interaction.x_domain(), interaction.y_domain());
            }
        }

        fn update_hover(&self, position: Point<Pixels>, bounds: Option<gpui::Bounds<Pixels>>) {
            if !self.is_over_plot(position, bounds) {
                self.interaction.borrow_mut().clear_hover();
                return;
            }
            let (x, y) = self.to_chart_coords(position, bounds);
            self.interaction.borrow_mut().update_hover_pixel(x, y);
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
            // Share one `Rc<InteractiveChartState>` across all event handlers so the
            // inner config/callbacks are not cloned for every handler.
            let state = Rc::new(self.state);
            let state_for_down = state.clone();
            let state_for_move = state.clone();
            let state_for_click = state.clone();
            let state_for_wheel = state.clone();
            let state_for_key = state.clone();
            let state_for_hover = state.clone();
            let state_for_hover_change = state.clone();
            let chart_bounds: Rc<RefCell<Option<gpui::Bounds<Pixels>>>> =
                Rc::new(RefCell::new(None));
            let bounds_for_prepaint = chart_bounds.clone();
            let bounds_for_down = chart_bounds.clone();
            let bounds_for_move = chart_bounds.clone();
            let bounds_for_wheel = chart_bounds.clone();

            let is_zoomed = state.is_zoomed();
            let config = state.config.clone();

            // Track drag state using RefCell for interior mutability
            let drag_start: Rc<RefCell<Option<(f32, f32)>>> = Rc::new(RefCell::new(None));
            let drag_start_down = drag_start.clone();
            let drag_start_move = drag_start.clone();
            let drag_start_up = drag_start.clone();

            div()
                .on_children_prepainted(move |children_bounds, _window, _cx| {
                    if let Some(bounds) = children_bounds.first() {
                        *bounds_for_prepaint.borrow_mut() = Some(*bounds);
                    }
                })
                .id(self.id)
                .relative()
                .focusable()
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
                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                    let (x, y) =
                        state_for_down.to_chart_coords(event.position, *bounds_for_down.borrow());
                    let mode = state_for_down.interaction.borrow().mode;
                    if event.modifiers.shift
                        || (mode == InteractionMode::Brush && !state_for_down.config.enable_pan)
                    {
                        state_for_down.interaction.borrow_mut().start_brush(x, y);
                    } else if state_for_down.config.enable_pan {
                        *drag_start_down.borrow_mut() = Some((x, y));
                    }
                    state_for_down.notify_interaction_change(cx);
                })
                // Mouse move - pan if dragging
                .on_mouse_move(move |event, window, cx| {
                    state_for_hover.update_hover(event.position, *bounds_for_move.borrow());
                    state_for_hover.notify_interaction_change(cx);
                    if state_for_move.interaction.borrow().is_brushing() {
                        let (x, y) = state_for_move
                            .to_chart_coords(event.position, *bounds_for_move.borrow());
                        state_for_move.interaction.borrow_mut().update_brush(x, y);
                        window.refresh();
                    } else if state_for_move.config.enable_pan
                        && let Some((start_x, start_y)) = *drag_start_move.borrow()
                    {
                        let (x, y) = state_for_move
                            .to_chart_coords(event.position, *bounds_for_move.borrow());
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
                })
                // Mouse up - end pan
                .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                    if state.interaction.borrow().is_brushing() {
                        state.interaction.borrow_mut().end_brush(false);
                    }
                    *drag_start_up.borrow_mut() = None;
                    state.notify_interaction_change(cx);
                })
                .on_hover(move |hovered, _window, cx| {
                    if !hovered {
                        state_for_hover_change
                            .interaction
                            .borrow_mut()
                            .clear_hover();
                        state_for_hover_change.notify_interaction_change(cx);
                    }
                })
                .on_key_down(move |event: &KeyDownEvent, window, cx| {
                    if let Some(action) = keyboard_action_for_key(&event.keystroke.key) {
                        state_for_key.apply_keyboard_action(action);
                        state_for_key.notify_interaction_change(cx);
                        cx.stop_propagation();
                        window.refresh();
                    }
                })
                // Click - handle double-click reset
                .on_click(move |event: &ClickEvent, window, cx| {
                    if state_for_click.config.enable_double_click_reset && event.click_count() >= 2
                    {
                        state_for_click.reset_zoom();
                        state_for_click.notify_interaction_change(cx);
                        window.refresh();
                    }
                })
                // Scroll wheel - zoom (only when cursor is over the plot area)
                .on_scroll_wheel(move |event: &ScrollWheelEvent, window, cx| {
                    if state_for_wheel.config.enable_wheel_zoom {
                        let bounds = *bounds_for_wheel.borrow();
                        if !state_for_wheel.is_over_plot(event.position, bounds) {
                            return; // Outside plot area — let the page scroll
                        }

                        let (x, y) = state_for_wheel.to_chart_coords(event.position, bounds);
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
                        state_for_wheel.notify_interaction_change(cx);

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
