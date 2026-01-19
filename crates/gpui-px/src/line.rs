//! Line chart - Plotly Express style API.

use crate::error::ChartError;
use crate::{
    DEFAULT_COLOR, DEFAULT_HEIGHT, DEFAULT_PADDING_FRACTION, DEFAULT_TITLE_FONT_SIZE,
    DEFAULT_WIDTH, ScaleType, TITLE_AREA_HEIGHT, extent_padded, validate_data_array,
    validate_data_length, validate_dimensions, validate_positive,
};
use d3rs::axis::{AxisConfig, AxisTheme, render_axis};
use d3rs::color::D3Color;
use d3rs::grid::{GridConfig, render_grid};
use d3rs::scale::{LinearScale, LogScale};
use d3rs::shape::{CurveType, LineConfig, LinePoint, render_line};
use d3rs::text::{VectorFontConfig, render_vector_text};
use gpui::prelude::*;
use gpui::{AnyElement, App, ElementId, IntoElement, Rgba, Window, div, px, rgb};
use std::collections::HashSet;
use std::rc::Rc;

/// Position of the legend relative to the chart
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum LegendPosition {
    /// Legend on the right side of the chart (default)
    #[default]
    Right,
    /// Legend on the left side of the chart
    Left,
    /// Legend above the chart
    Top,
    /// Legend below the chart
    Bottom,
    /// Legend is hidden
    Hidden,
}

/// Theme for chart styling
#[derive(Debug, Clone)]
pub struct ChartTheme {
    /// Background color for plot area
    pub plot_background: Rgba,
    /// Grid line color
    pub grid_color: Rgba,
    /// Axis line color
    pub axis_line_color: Rgba,
    /// Axis label color
    pub axis_label_color: Rgba,
    /// Title text color
    pub title_color: Rgba,
    /// Legend text color
    pub legend_text_color: Rgba,
}

impl Default for ChartTheme {
    fn default() -> Self {
        Self {
            plot_background: rgb(0xf8f8f8),
            grid_color: rgba(0x000000, 0.1),
            axis_line_color: rgba(0x000000, 0.2),
            axis_label_color: rgba(0x000000, 0.6),
            title_color: rgba(0x000000, 0.8),
            legend_text_color: rgba(0x000000, 0.6),
        }
    }
}

/// Helper to create Rgba with alpha
fn rgba(hex: u32, alpha: f32) -> Rgba {
    Rgba {
        r: ((hex >> 16) & 0xFF) as f32 / 255.0,
        g: ((hex >> 8) & 0xFF) as f32 / 255.0,
        b: (hex & 0xFF) as f32 / 255.0,
        a: alpha,
    }
}

/// Axis theme adapter for d3rs
struct ChartAxisTheme {
    axis_line_color: Rgba,
    axis_label_color: Rgba,
}

impl AxisTheme for ChartAxisTheme {
    fn axis_line_color(&self) -> Rgba {
        self.axis_line_color
    }

    fn axis_label_color(&self) -> Rgba {
        self.axis_label_color
    }
}

/// Format tick labels for log scales with k/M suffixes
fn format_log_tick(value: f64) -> String {
    let abs_value = value.abs();

    // Handle zero
    if abs_value < 1e-10 {
        return "0".to_string();
    }

    // Format based on magnitude
    if abs_value >= 1_000_000.0 {
        // Millions: 1M, 2M, etc.
        let millions = value / 1_000_000.0;
        if millions.fract().abs() < 1e-10 {
            format!("{:.0}M", millions)
        } else {
            format!("{:.1}M", millions)
        }
    } else if abs_value >= 1_000.0 {
        // Thousands: 1k, 10k, 100k, etc.
        let thousands = value / 1_000.0;
        if thousands.fract().abs() < 1e-10 {
            format!("{:.0}k", thousands)
        } else {
            format!("{:.1}k", thousands)
        }
    } else if abs_value >= 1.0 {
        // Regular values >= 1
        if value.fract().abs() < 1e-10 {
            format!("{:.0}", value)
        } else {
            format!("{:.1}", value)
        }
    } else {
        // Small values < 1
        format!("{:.2}", value)
    }
}

/// Generate smart tick values for log scales to prevent label collision
/// Shows 1,2,3,4,5,10,20,30,40,50,100,... pattern
fn generate_log_ticks(min: f64, max: f64) -> Vec<f64> {
    let mut ticks = Vec::new();

    // Find the starting decade (power of 10)
    let start_exp = min.log10().floor() as i32;
    let end_exp = max.log10().ceil() as i32;

    for exp in start_exp..=end_exp {
        let base = 10_f64.powi(exp);

        // For each decade, show: 1, 2, 3, 4, 5, 10 (which becomes 1 of next decade)
        // This gives us: 1k, 2k, 3k, 4k, 5k, 10k, 20k, 30k, 40k, 50k, 100k, etc.
        for multiplier in [1.0, 2.0, 3.0, 4.0, 5.0] {
            let tick = base * multiplier;
            if tick >= min && tick <= max {
                ticks.push(tick);
            }
        }
    }

    // Add the final decade marker if we don't already have it
    let final_decade = 10_f64.powi(end_exp);
    if final_decade <= max && !ticks.contains(&final_decade) {
        ticks.push(final_decade);
    }

    ticks.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ticks.dedup();
    ticks
}

/// A single series in a line chart
#[derive(Debug, Clone)]
struct LineSeries {
    /// Optional custom X values (if None, uses the primary X values)
    x: Option<Vec<f64>>,
    y: Vec<f64>,
    label: Option<String>,
    color: u32,
    stroke_width: f32,
    opacity: f32,
    /// Whether this series uses the secondary (right) Y-axis
    use_secondary_axis: bool,
}

/// Callback type for legend click events
pub type LegendClickCallback = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// Line chart builder.
#[derive(Clone)]
pub struct LineChart {
    x: Vec<f64>,
    // Primary series (backwards compatible)
    y: Vec<f64>,
    label: Option<String>,
    color: u32,
    stroke_width: f32,
    opacity: f32,
    // Additional series
    series: Vec<LineSeries>,
    // Common settings
    title: Option<String>,
    x_label: Option<String>,
    y_label: Option<String>,
    curve: CurveType,
    show_points: bool,
    width: f32,
    height: f32,
    x_scale_type: ScaleType,
    y_scale_type: ScaleType,
    x_range: Option<[f64; 2]>,
    y_range: Option<[f64; 2]>,
    show_legend: bool,
    legend_position: LegendPosition,
    /// Whether legend_position was explicitly set by user
    legend_position_explicit: bool,
    /// Target aspect ratio for the graph (height = width * ratio)
    graph_ratio: f32,
    theme: ChartTheme,
    // Secondary Y-axis settings
    y2_label: Option<String>,
    y2_range: Option<[f64; 2]>,
    /// Set of hidden series indices (0 = primary series, 1+ = additional series)
    hidden_series: HashSet<usize>,
    /// Callback when a legend item is clicked (receives series index)
    on_legend_click: Option<LegendClickCallback>,
}

impl std::fmt::Debug for LineChart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LineChart")
            .field("x_len", &self.x.len())
            .field("y_len", &self.y.len())
            .field("series_count", &self.series.len())
            .field("title", &self.title)
            .field("hidden_series", &self.hidden_series)
            .finish()
    }
}

impl LineChart {
    /// Set chart title (rendered at top of chart).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set X-axis label.
    pub fn x_label(mut self, label: impl Into<String>) -> Self {
        self.x_label = Some(label.into());
        self
    }

    /// Set Y-axis label.
    pub fn y_label(mut self, label: impl Into<String>) -> Self {
        self.y_label = Some(label.into());
        self
    }

    /// Set label for legend entry.
    ///
    /// When a label is set, the legend will automatically be shown.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::line;
    /// let chart = line(&[1.0, 2.0], &[1.0, 2.0])
    ///     .label("Series A")
    ///     .build();
    /// ```
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.show_legend = true;
        self
    }

    /// Set line color as 24-bit RGB hex value (format: 0xRRGGBB).
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::line;
    /// let chart = line(&[1.0], &[1.0])
    ///     .color(0xff7f0e)  // Plotly orange
    ///     .build();
    /// ```
    pub fn color(mut self, hex: u32) -> Self {
        self.color = hex;
        self
    }

    /// Set line stroke width in pixels.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Set line opacity (0.0 - 1.0).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set curve interpolation type.
    pub fn curve(mut self, curve: CurveType) -> Self {
        self.curve = curve;
        self
    }

    /// Show data points on the line.
    pub fn show_points(mut self, show: bool) -> Self {
        self.show_points = show;
        self
    }

    /// Set chart dimensions.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set X-axis scale type (linear or log).
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::{line, ScaleType};
    /// let chart = line(&[10.0, 100.0, 1000.0], &[1.0, 2.0, 3.0])
    ///     .x_scale(ScaleType::Log)
    ///     .build();
    /// ```
    pub fn x_scale(mut self, scale: ScaleType) -> Self {
        self.x_scale_type = scale;
        self
    }

    /// Set Y-axis scale type (linear or log).
    pub fn y_scale(mut self, scale: ScaleType) -> Self {
        self.y_scale_type = scale;
        self
    }

    /// Set the X-axis display range.
    ///
    /// When set, only data points within this range are displayed, and the
    /// axis is scaled to show exactly this range. Points outside the range
    /// are clipped.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::line;
    /// // Show only the range from 100 Hz to 10000 Hz
    /// let chart = line(&[20.0, 100.0, 1000.0, 10000.0, 20000.0], &[1.0, 2.0, 3.0, 4.0, 5.0])
    ///     .x_range(100.0, 10000.0)
    ///     .build();
    /// ```
    pub fn x_range(mut self, min: f64, max: f64) -> Self {
        self.x_range = Some([min, max]);
        self
    }

    /// Set the Y-axis display range.
    ///
    /// When set, only data points within this range are displayed, and the
    /// axis is scaled to show exactly this range. Points outside the range
    /// are clipped.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::line;
    /// // Show only Y values from -10 dB to +10 dB
    /// let chart = line(&[1.0, 2.0, 3.0, 4.0], &[-20.0, 5.0, -5.0, 15.0])
    ///     .y_range(-10.0, 10.0)
    ///     .build();
    /// ```
    pub fn y_range(mut self, min: f64, max: f64) -> Self {
        self.y_range = Some([min, max]);
        self
    }

    /// Add an additional data series to the chart.
    ///
    /// All series share the same X-axis data. This allows overlaying multiple
    /// lines on a single chart.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::line;
    /// let x = vec![1.0, 2.0, 3.0];
    /// let y1 = vec![1.0, 2.0, 3.0];
    /// let y2 = vec![3.0, 2.0, 1.0];
    /// let chart = line(&x, &y1)
    ///     .label("Series 1")
    ///     .color(0x3b82f6)
    ///     .add_series(&y2, Some("Series 2"), 0xff7f0e, 2.0, 1.0)
    ///     .build();
    /// ```
    pub fn add_series(
        mut self,
        y: &[f64],
        label: Option<impl Into<String>>,
        color: u32,
        stroke_width: f32,
        opacity: f32,
    ) -> Self {
        self.series.push(LineSeries {
            x: None,
            y: y.to_vec(),
            label: label.map(|l| l.into()),
            color,
            stroke_width,
            opacity,
            use_secondary_axis: false,
        });
        // Auto-enable legend if any series has a label
        if self.series.iter().any(|s| s.label.is_some()) {
            self.show_legend = true;
        }
        self
    }

    /// Add an additional series with custom X values.
    ///
    /// Use this when the series has different X coordinates than the primary series.
    /// Useful for reference lines or overlaying data with different sampling.
    pub fn add_series_with_x(
        mut self,
        x: &[f64],
        y: &[f64],
        label: Option<impl Into<String>>,
        color: u32,
        stroke_width: f32,
        opacity: f32,
    ) -> Self {
        self.series.push(LineSeries {
            x: Some(x.to_vec()),
            y: y.to_vec(),
            label: label.map(|l| l.into()),
            color,
            stroke_width,
            opacity,
            use_secondary_axis: false,
        });
        // Auto-enable legend if any series has a label
        if self.series.iter().any(|s| s.label.is_some()) {
            self.show_legend = true;
        }
        self
    }

    /// Set label for secondary Y-axis (right side).
    ///
    /// When a secondary axis label is set, series added with `add_series_y2`
    /// will be plotted against the right Y-axis.
    pub fn y2_label(mut self, label: impl Into<String>) -> Self {
        self.y2_label = Some(label.into());
        self
    }

    /// Set the secondary Y-axis display range.
    ///
    /// This sets the range for series added with `add_series_y2`.
    pub fn y2_range(mut self, min: f64, max: f64) -> Self {
        self.y2_range = Some([min, max]);
        self
    }

    /// Add a series that uses the secondary (right) Y-axis.
    ///
    /// Series added with this method will be plotted against a separate
    /// Y-axis on the right side of the chart.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::line;
    /// let x = vec![1.0, 2.0, 3.0];
    /// let spl = vec![80.0, 85.0, 82.0];  // SPL in dB
    /// let di = vec![5.0, 6.0, 5.5];      // Directivity Index
    /// let chart = line(&x, &spl)
    ///     .label("SPL")
    ///     .y_label("SPL (dB)")
    ///     .y2_label("DI (dB)")
    ///     .add_series_y2(&di, Some("DI"), 0xff7f0e, 2.0, 1.0)
    ///     .build();
    /// ```
    pub fn add_series_y2(
        mut self,
        y: &[f64],
        label: Option<impl Into<String>>,
        color: u32,
        stroke_width: f32,
        opacity: f32,
    ) -> Self {
        self.series.push(LineSeries {
            x: None,
            y: y.to_vec(),
            label: label.map(|l| l.into()),
            color,
            stroke_width,
            opacity,
            use_secondary_axis: true,
        });
        // Auto-enable legend if any series has a label
        if self.series.iter().any(|s| s.label.is_some()) {
            self.show_legend = true;
        }
        self
    }

    /// Add a series with custom X values that uses the secondary (right) Y-axis.
    pub fn add_series_y2_with_x(
        mut self,
        x: &[f64],
        y: &[f64],
        label: Option<impl Into<String>>,
        color: u32,
        stroke_width: f32,
        opacity: f32,
    ) -> Self {
        self.series.push(LineSeries {
            x: Some(x.to_vec()),
            y: y.to_vec(),
            label: label.map(|l| l.into()),
            color,
            stroke_width,
            opacity,
            use_secondary_axis: true,
        });
        // Auto-enable legend if any series has a label
        if self.series.iter().any(|s| s.label.is_some()) {
            self.show_legend = true;
        }
        self
    }

    /// Set the chart theme.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::{line, ChartTheme};
    /// let chart = line(&[1.0, 2.0], &[1.0, 2.0])
    ///     .theme(ChartTheme::default())
    ///     .build();
    /// ```
    pub fn theme(mut self, theme: ChartTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the legend position.
    ///
    /// Controls where the legend is displayed relative to the chart area.
    /// Available positions: `Right` (default), `Left`, `Top`, `Bottom`.
    ///
    /// When not explicitly set, the legend position is automatically chosen
    /// to achieve a graph aspect ratio closest to `graph_ratio`.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::{line, LegendPosition};
    /// let chart = line(&[1.0, 2.0], &[1.0, 2.0])
    ///     .label("Data")
    ///     .legend_position(LegendPosition::Bottom)
    ///     .build();
    /// ```
    pub fn legend_position(mut self, position: LegendPosition) -> Self {
        self.legend_position = position;
        self.legend_position_explicit = true;
        self
    }

    /// Set which series are hidden (not rendered).
    ///
    /// Series are indexed starting from 0 (primary series), then 1, 2, etc. for
    /// additional series added via `add_series()`.
    ///
    /// Hidden series still appear in the legend (grayed out) and can be toggled
    /// back on by clicking if `on_legend_click` is set.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::line;
    ///
    /// let chart = line(&[1.0, 2.0], &[1.0, 2.0])
    ///     .hidden_series(&[1, 2]) // Hide series 1 and 2
    ///     .build();
    /// ```
    pub fn hidden_series(mut self, indices: &[usize]) -> Self {
        self.hidden_series = indices.iter().copied().collect();
        self
    }

    /// Set callback for when a legend item is clicked.
    ///
    /// The callback receives the series index (0 = primary, 1+ = additional series).
    /// Use this to implement toggle visibility by updating `hidden_series` and re-rendering.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::line;
    /// use std::rc::Rc;
    /// use std::cell::RefCell;
    ///
    /// let hidden = Rc::new(RefCell::new(std::collections::HashSet::new()));
    /// let hidden_clone = hidden.clone();
    ///
    /// let chart = line(&[1.0, 2.0], &[1.0, 2.0])
    ///     .on_legend_click(move |index, _window, _cx| {
    ///         let mut set = hidden_clone.borrow_mut();
    ///         if set.contains(&index) {
    ///             set.remove(&index);
    ///         } else {
    ///             set.insert(index);
    ///         }
    ///         // Trigger re-render here
    ///     })
    ///     .build();
    /// ```
    pub fn on_legend_click<F>(mut self, callback: F) -> Self
    where
        F: Fn(usize, &mut Window, &mut App) + 'static,
    {
        self.on_legend_click = Some(Rc::new(callback));
        self
    }

    /// Set the target aspect ratio for the graph area.
    ///
    /// The ratio is defined as `height / width`. Default is `1.414` (≈ √2, similar to A4 paper).
    ///
    /// When a legend is shown and `legend_position` is not explicitly set,
    /// the legend position is automatically chosen to achieve an aspect ratio
    /// closest to this target ratio.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::line;
    /// let chart = line(&[1.0, 2.0], &[1.0, 2.0])
    ///     .label("Data")
    ///     .graph_ratio(1.0)  // Square aspect ratio
    ///     .build();
    /// ```
    pub fn graph_ratio(mut self, ratio: f32) -> Self {
        self.graph_ratio = ratio;
        self
    }

    /// Build and validate the chart, returning renderable element.
    pub fn build(self) -> Result<impl IntoElement, ChartError> {
        // Validate inputs
        validate_data_array(&self.x, "x")?;
        validate_data_array(&self.y, "y")?;
        validate_data_length(self.x.len(), self.y.len(), "x", "y")?;
        validate_dimensions(self.width, self.height)?;

        // Validate all additional series
        for series in &self.series {
            validate_data_array(&series.y, "series.y")?;
            if let Some(ref x) = series.x {
                // Series has custom X values
                validate_data_array(x, "series.x")?;
                validate_data_length(x.len(), series.y.len(), "series.x", "series.y")?;
                if self.x_scale_type == ScaleType::Log {
                    validate_positive(x, "series.x")?;
                }
            } else {
                // Series shares primary X values
                validate_data_length(self.x.len(), series.y.len(), "x", "series.y")?;
            }
            if self.y_scale_type == ScaleType::Log {
                validate_positive(&series.y, "series.y")?;
            }
        }

        // Validate positive values for log scales
        if self.x_scale_type == ScaleType::Log {
            validate_positive(&self.x, "x")?;
        }
        if self.y_scale_type == ScaleType::Log {
            validate_positive(&self.y, "y")?;
        }

        // Check if we have secondary axis series
        let has_secondary_axis = self.series.iter().any(|s| s.use_secondary_axis);

        // Define margins - increase right margin if secondary axis is needed
        let margin_left = 50.0;
        let margin_bottom = 30.0;
        let margin_top = 10.0;
        let margin_right = if has_secondary_axis { 60.0 } else { 20.0 };

        // Calculate plot area (reserve space for title if present)
        let title_height = if self.title.is_some() {
            TITLE_AREA_HEIGHT
        } else {
            0.0
        };

        // Calculate legend dimensions based on position
        // Formula: color_indicator_width + gap + estimated_text_width + padding
        // Color indicator: 16px, gap: 8px (gap_2), padding: 8px (p_2 on both sides)
        let legend_gap = 20.0; // Gap between chart and legend

        // Count legend items and calculate max label length
        let mut legend_item_count = 0;
        let mut max_label_len = 0;

        if self.show_legend {
            if let Some(ref label) = self.label {
                legend_item_count += 1;
                max_label_len = max_label_len.max(label.len());
            }

            for series in &self.series {
                if let Some(ref label) = series.label {
                    legend_item_count += 1;
                    max_label_len = max_label_len.max(label.len());
                }
            }
        }

        let has_legend_items = legend_item_count > 0;

        // Calculate base legend dimensions for each orientation
        // Estimate ~7 pixels per character for text_xs font
        let estimated_text_width = (max_label_len as f32) * 7.0;
        let single_item_width = 16.0 + 8.0 + estimated_text_width + 16.0; // color + gap + text + padding
        let single_item_height = 24.0; // Approximate height for a legend item with padding

        // Vertical legend dimensions (for Left/Right)
        let vertical_legend_width = single_item_width;
        let vertical_legend_height = (legend_item_count as f32) * single_item_height + 16.0;

        // Horizontal legend dimensions (for Top/Bottom)
        let horizontal_legend_width = (legend_item_count as f32) * (single_item_width + 16.0);
        let horizontal_legend_height = single_item_height + 8.0;

        // Base available dimensions (without legend)
        let base_available_width = self.width as f64 - margin_left - margin_right;
        let base_available_height =
            self.height as f64 - title_height as f64 - margin_top - margin_bottom;

        // Determine legend position (auto-select if not explicit)
        let legend_position = if has_legend_items && !self.legend_position_explicit {
            // Calculate plot dimensions and aspect ratios for each position
            let target_ratio = self.graph_ratio as f64;

            // Helper to calculate how close a ratio is to target
            let ratio_distance = |plot_w: f64, plot_h: f64| -> f64 {
                if plot_w <= 0.0 || plot_h <= 0.0 {
                    return f64::MAX;
                }
                let ratio = plot_h / plot_w;
                (ratio - target_ratio).abs()
            };

            // Left/Right: subtract legend width from available width
            let lr_plot_width = base_available_width - (vertical_legend_width + legend_gap) as f64;
            let lr_plot_height = base_available_height;
            let lr_distance = ratio_distance(lr_plot_width, lr_plot_height);

            // Top/Bottom: subtract legend height from available height
            let tb_plot_width = base_available_width;
            let tb_plot_height =
                base_available_height - (horizontal_legend_height + legend_gap) as f64;
            let tb_distance = ratio_distance(tb_plot_width, tb_plot_height);

            // Choose the orientation that gives ratio closest to target
            if lr_distance <= tb_distance {
                // Vertical legend is better - choose Right as default
                LegendPosition::Right
            } else {
                // Horizontal legend is better - choose Bottom as default
                LegendPosition::Bottom
            }
        } else {
            self.legend_position
        };

        // Calculate final legend dimensions based on chosen position
        let (legend_width, legend_height) = if has_legend_items {
            match legend_position {
                LegendPosition::Left | LegendPosition::Right => {
                    (vertical_legend_width, vertical_legend_height)
                }
                LegendPosition::Top | LegendPosition::Bottom => {
                    (horizontal_legend_width, horizontal_legend_height)
                }
                LegendPosition::Hidden => (0.0, 0.0),
            }
        } else {
            (0.0, 0.0)
        };

        // Calculate plot dimensions, accounting for legend position
        let width_for_legend = match legend_position {
            LegendPosition::Left | LegendPosition::Right if has_legend_items => {
                legend_width + legend_gap
            }
            _ => 0.0,
        };
        let height_for_legend = match legend_position {
            LegendPosition::Top | LegendPosition::Bottom if has_legend_items => {
                legend_height + legend_gap
            }
            _ => 0.0,
        };

        let plot_width =
            (self.width as f64 - margin_left - margin_right - width_for_legend as f64).max(0.0);
        let plot_height = (self.height as f64
            - title_height as f64
            - margin_top
            - margin_bottom
            - height_for_legend as f64)
            .max(0.0);

        // Calculate domains with padding - include all series in Y-axis range
        // Use user-provided ranges if set, otherwise auto-calculate from data
        let (x_min, x_max) = if let Some([min, max]) = self.x_range {
            // User-specified range - use exactly as provided (no padding)
            (min, max)
        } else if self.x_scale_type == ScaleType::Log {
            // For log scale, use multiplicative padding to avoid going negative
            let min = self.x.iter().copied().fold(f64::INFINITY, f64::min);
            let max = self.x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let padding_factor = 1.0 + DEFAULT_PADDING_FRACTION;
            (min / padding_factor, max * padding_factor)
        } else {
            extent_padded(&self.x, DEFAULT_PADDING_FRACTION)
        };

        // Collect Y values from primary series and non-secondary additional series
        let mut primary_y_values: Vec<f64> = self.y.clone();
        for series in &self.series {
            if !series.use_secondary_axis {
                primary_y_values.extend_from_slice(&series.y);
            }
        }
        let (y_min, y_max) = if let Some([min, max]) = self.y_range {
            // User-specified range - use exactly as provided (no padding)
            (min, max)
        } else if self.y_scale_type == ScaleType::Log {
            // For log scale, use multiplicative padding
            let min = primary_y_values
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);
            let max = primary_y_values
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            let padding_factor = 1.0 + DEFAULT_PADDING_FRACTION;
            (min / padding_factor, max * padding_factor)
        } else {
            extent_padded(&primary_y_values, DEFAULT_PADDING_FRACTION)
        };

        // Calculate secondary Y axis domain if needed
        let (y2_min, y2_max) = if has_secondary_axis {
            let mut secondary_y_values: Vec<f64> = Vec::new();
            for series in &self.series {
                if series.use_secondary_axis {
                    secondary_y_values.extend_from_slice(&series.y);
                }
            }
            if let Some([min, max]) = self.y2_range {
                (min, max)
            } else if secondary_y_values.is_empty() {
                (0.0, 1.0) // Default fallback
            } else {
                extent_padded(&secondary_y_values, DEFAULT_PADDING_FRACTION)
            }
        } else {
            (0.0, 1.0) // Placeholder, won't be used
        };

        // Create data points for primary series
        // Check if primary series is hidden
        let primary_hidden = self.hidden_series.contains(&0);

        let primary_data: Vec<LinePoint> = self
            .x
            .iter()
            .zip(self.y.iter())
            .map(|(&x, &y)| LinePoint::new(x, y))
            .collect();

        // Create configs for primary series
        let primary_config = LineConfig::new()
            .stroke_color(D3Color::from_hex(self.color))
            .stroke_width(self.stroke_width)
            .opacity(self.opacity)
            .curve(self.curve)
            .show_points(self.show_points);

        // Prepare additional series data and configs, separating primary and secondary axis series
        // Skip hidden series
        let mut series_data_configs: Vec<(Vec<LinePoint>, LineConfig)> = Vec::new();
        let mut secondary_series_data_configs: Vec<(Vec<LinePoint>, LineConfig)> = Vec::new();
        for (i, series) in self.series.iter().enumerate() {
            // Series index is i+1 (primary is 0)
            if self.hidden_series.contains(&(i + 1)) {
                continue; // Skip hidden series
            }

            // Use custom X values if provided, otherwise use primary X values
            let x_values = series.x.as_ref().unwrap_or(&self.x);
            let series_points: Vec<LinePoint> = x_values
                .iter()
                .zip(series.y.iter())
                .map(|(&x, &y)| LinePoint::new(x, y))
                .collect();

            let series_config = LineConfig::new()
                .stroke_color(D3Color::from_hex(series.color))
                .stroke_width(series.stroke_width)
                .opacity(series.opacity)
                .curve(self.curve)
                .show_points(self.show_points);

            if series.use_secondary_axis {
                secondary_series_data_configs.push((series_points, series_config));
            } else {
                series_data_configs.push((series_points, series_config));
            }
        }

        let axis_theme = ChartAxisTheme {
            axis_line_color: self.theme.axis_line_color,
            axis_label_color: self.theme.axis_label_color,
        };

        let grid_config = GridConfig::with_lines()
            .with_line_width(0.5)
            .with_line_opacity(0.3);

        // Build the element based on scale types
        let chart_content: AnyElement = match (self.x_scale_type, self.y_scale_type) {
            (ScaleType::Linear, ScaleType::Linear) => {
                let x_scale = LinearScale::new()
                    .domain(x_min, x_max)
                    .range(0.0, plot_width);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height, 0.0);

                // Create secondary Y scale if needed
                let y2_scale = LinearScale::new()
                    .domain(y2_min, y2_max)
                    .range(plot_height, 0.0);

                // Build plot area with grid and all lines
                let mut plot_area = div()
                    .w(px(plot_width as f32))
                    .h(px(plot_height as f32))
                    .relative()
                    .overflow_hidden()
                    .bg(self.theme.plot_background)
                    .child(render_grid(
                        &x_scale,
                        &y_scale,
                        &grid_config,
                        plot_width as f32,
                        plot_height as f32,
                        &axis_theme,
                    ));

                // Render all additional series first (so primary is on top)
                for (series_data, series_config) in &series_data_configs {
                    plot_area = plot_area.child(render_line(
                        &x_scale,
                        &y_scale,
                        series_data,
                        series_config,
                    ));
                }

                // Render primary series on top (if not hidden)
                if !primary_hidden {
                    plot_area = plot_area.child(render_line(
                        &x_scale,
                        &y_scale,
                        &primary_data,
                        &primary_config,
                    ));
                }

                // Render secondary axis series using secondary Y scale
                for (series_data, series_config) in &secondary_series_data_configs {
                    plot_area = plot_area.child(render_line(
                        &x_scale,
                        &y2_scale,
                        series_data,
                        series_config,
                    ));
                }

                // Create axis configs with labels
                let mut y_axis_config = AxisConfig::left().with_label_font_size(8.0);
                if let Some(ref label) = self.y_label {
                    y_axis_config = y_axis_config.with_title(label.clone());
                }

                let mut x_axis_config = AxisConfig::bottom()
                    .with_ticks(20)
                    .with_label_font_size(8.0);
                if let Some(ref label) = self.x_label {
                    x_axis_config = x_axis_config.with_title(label.clone());
                }

                // Build chart with optional secondary Y axis
                if has_secondary_axis {
                    let mut y2_axis_config = AxisConfig::right().with_label_font_size(8.0);
                    if let Some(ref label) = self.y2_label {
                        y2_axis_config = y2_axis_config.with_title(label.clone());
                    }

                    div()
                        .flex()
                        .child(render_axis(
                            &y_scale,
                            &y_axis_config,
                            plot_height as f32,
                            &axis_theme,
                        ))
                        .child(div().flex().flex_col().child(plot_area).child(render_axis(
                            &x_scale,
                            &x_axis_config,
                            plot_width as f32,
                            &axis_theme,
                        )))
                        .child(render_axis(
                            &y2_scale,
                            &y2_axis_config,
                            plot_height as f32,
                            &axis_theme,
                        ))
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .child(render_axis(
                            &y_scale,
                            &y_axis_config,
                            plot_height as f32,
                            &axis_theme,
                        ))
                        .child(div().flex().flex_col().child(plot_area).child(render_axis(
                            &x_scale,
                            &x_axis_config,
                            plot_width as f32,
                            &axis_theme,
                        )))
                        .into_any_element()
                }
            }
            (ScaleType::Log, ScaleType::Linear) => {
                let x_scale = LogScale::new().domain(x_min, x_max).range(0.0, plot_width);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height, 0.0);

                // Create secondary Y scale if needed
                let y2_scale = LinearScale::new()
                    .domain(y2_min, y2_max)
                    .range(plot_height, 0.0);

                // Build plot area with grid and all lines
                let mut plot_area = div()
                    .w(px(plot_width as f32))
                    .h(px(plot_height as f32))
                    .relative()
                    .overflow_hidden()
                    .bg(self.theme.plot_background)
                    .child(render_grid(
                        &x_scale,
                        &y_scale,
                        &grid_config,
                        plot_width as f32,
                        plot_height as f32,
                        &axis_theme,
                    ));

                // Render all primary axis series first
                for (series_data, series_config) in &series_data_configs {
                    plot_area = plot_area.child(render_line(
                        &x_scale,
                        &y_scale,
                        series_data,
                        series_config,
                    ));
                }

                // Render primary series on top (if not hidden)
                if !primary_hidden {
                    plot_area = plot_area.child(render_line(
                        &x_scale,
                        &y_scale,
                        &primary_data,
                        &primary_config,
                    ));
                }

                // Render secondary axis series using secondary Y scale
                for (series_data, series_config) in &secondary_series_data_configs {
                    plot_area = plot_area.child(render_line(
                        &x_scale,
                        &y2_scale,
                        series_data,
                        series_config,
                    ));
                }

                // Create axis configs with labels and angled X labels for log scale
                let mut y_axis_config = AxisConfig::left().with_label_font_size(8.0);
                if let Some(ref label) = self.y_label {
                    y_axis_config = y_axis_config.with_title(label.clone());
                }

                // Generate smart tick values for log X axis to prevent collision
                let x_ticks = generate_log_ticks(x_min, x_max);
                let mut x_axis_config = AxisConfig::bottom()
                    .with_tick_values(x_ticks)
                    .with_label_angle(-45.0)
                    .with_label_font_size(8.0)
                    .with_formatter(format_log_tick); // Use k/M formatting for log scale
                if let Some(ref label) = self.x_label {
                    x_axis_config = x_axis_config.with_title(label.clone());
                }

                // Build chart with optional secondary Y axis
                if has_secondary_axis {
                    let mut y2_axis_config = AxisConfig::right().with_label_font_size(8.0);
                    if let Some(ref label) = self.y2_label {
                        y2_axis_config = y2_axis_config.with_title(label.clone());
                    }

                    div()
                        .flex()
                        .child(render_axis(
                            &y_scale,
                            &y_axis_config,
                            plot_height as f32,
                            &axis_theme,
                        ))
                        .child(div().flex().flex_col().child(plot_area).child(render_axis(
                            &x_scale,
                            &x_axis_config,
                            plot_width as f32,
                            &axis_theme,
                        )))
                        .child(render_axis(
                            &y2_scale,
                            &y2_axis_config,
                            plot_height as f32,
                            &axis_theme,
                        ))
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .child(render_axis(
                            &y_scale,
                            &y_axis_config,
                            plot_height as f32,
                            &axis_theme,
                        ))
                        .child(div().flex().flex_col().child(plot_area).child(render_axis(
                            &x_scale,
                            &x_axis_config,
                            plot_width as f32,
                            &axis_theme,
                        )))
                        .into_any_element()
                }
            }
            (ScaleType::Linear, ScaleType::Log) => {
                let x_scale = LinearScale::new()
                    .domain(x_min, x_max)
                    .range(0.0, plot_width);
                let y_scale = LogScale::new().domain(y_min, y_max).range(plot_height, 0.0);

                // Create secondary Y scale if needed
                let y2_scale = LinearScale::new()
                    .domain(y2_min, y2_max)
                    .range(plot_height, 0.0);

                // Build plot area with grid and all lines
                let mut plot_area = div()
                    .w(px(plot_width as f32))
                    .h(px(plot_height as f32))
                    .relative()
                    .overflow_hidden()
                    .bg(self.theme.plot_background)
                    .child(render_grid(
                        &x_scale,
                        &y_scale,
                        &grid_config,
                        plot_width as f32,
                        plot_height as f32,
                        &axis_theme,
                    ));

                // Render all additional series first
                for (series_data, series_config) in &series_data_configs {
                    plot_area = plot_area.child(render_line(
                        &x_scale,
                        &y_scale,
                        series_data,
                        series_config,
                    ));
                }

                // Render primary series on top (if not hidden)
                if !primary_hidden {
                    plot_area = plot_area.child(render_line(
                        &x_scale,
                        &y_scale,
                        &primary_data,
                        &primary_config,
                    ));
                }

                // Render secondary axis series using secondary Y scale
                for (series_data, series_config) in &secondary_series_data_configs {
                    plot_area = plot_area.child(render_line(
                        &x_scale,
                        &y2_scale,
                        series_data,
                        series_config,
                    ));
                }

                // Create axis configs with labels
                // Generate smart tick values for log Y axis to prevent collision
                let y_ticks = generate_log_ticks(y_min, y_max);
                let mut y_axis_config = AxisConfig::left()
                    .with_tick_values(y_ticks)
                    .with_label_font_size(8.0)
                    .with_formatter(format_log_tick); // Use k/M formatting for log scale
                if let Some(ref label) = self.y_label {
                    y_axis_config = y_axis_config.with_title(label.clone());
                }

                let mut x_axis_config = AxisConfig::bottom()
                    .with_ticks(20)
                    .with_label_font_size(8.0);
                if let Some(ref label) = self.x_label {
                    x_axis_config = x_axis_config.with_title(label.clone());
                }

                // Build chart with optional secondary Y axis
                if has_secondary_axis {
                    let mut y2_axis_config = AxisConfig::right().with_label_font_size(8.0);
                    if let Some(ref label) = self.y2_label {
                        y2_axis_config = y2_axis_config.with_title(label.clone());
                    }

                    div()
                        .flex()
                        .child(render_axis(
                            &y_scale,
                            &y_axis_config,
                            plot_height as f32,
                            &axis_theme,
                        ))
                        .child(div().flex().flex_col().child(plot_area).child(render_axis(
                            &x_scale,
                            &x_axis_config,
                            plot_width as f32,
                            &axis_theme,
                        )))
                        .child(render_axis(
                            &y2_scale,
                            &y2_axis_config,
                            plot_height as f32,
                            &axis_theme,
                        ))
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .child(render_axis(
                            &y_scale,
                            &y_axis_config,
                            plot_height as f32,
                            &axis_theme,
                        ))
                        .child(div().flex().flex_col().child(plot_area).child(render_axis(
                            &x_scale,
                            &x_axis_config,
                            plot_width as f32,
                            &axis_theme,
                        )))
                        .into_any_element()
                }
            }
            (ScaleType::Log, ScaleType::Log) => {
                let x_scale = LogScale::new().domain(x_min, x_max).range(0.0, plot_width);
                let y_scale = LogScale::new().domain(y_min, y_max).range(plot_height, 0.0);

                // Create secondary Y scale if needed
                let y2_scale = LinearScale::new()
                    .domain(y2_min, y2_max)
                    .range(plot_height, 0.0);

                // Build plot area with grid and all lines
                let mut plot_area = div()
                    .w(px(plot_width as f32))
                    .h(px(plot_height as f32))
                    .relative()
                    .overflow_hidden()
                    .bg(self.theme.plot_background)
                    .child(render_grid(
                        &x_scale,
                        &y_scale,
                        &grid_config,
                        plot_width as f32,
                        plot_height as f32,
                        &axis_theme,
                    ));

                // Render all additional series first
                for (series_data, series_config) in &series_data_configs {
                    plot_area = plot_area.child(render_line(
                        &x_scale,
                        &y_scale,
                        series_data,
                        series_config,
                    ));
                }

                // Render primary series on top (if not hidden)
                if !primary_hidden {
                    plot_area = plot_area.child(render_line(
                        &x_scale,
                        &y_scale,
                        &primary_data,
                        &primary_config,
                    ));
                }

                // Render secondary axis series using secondary Y scale
                for (series_data, series_config) in &secondary_series_data_configs {
                    plot_area = plot_area.child(render_line(
                        &x_scale,
                        &y2_scale,
                        series_data,
                        series_config,
                    ));
                }

                // Create axis configs with labels and angled X labels for log scale
                // Generate smart tick values for both log axes to prevent collision
                let y_ticks = generate_log_ticks(y_min, y_max);
                let mut y_axis_config = AxisConfig::left()
                    .with_tick_values(y_ticks)
                    .with_label_font_size(8.0)
                    .with_formatter(format_log_tick); // Use k/M formatting for log scale
                if let Some(ref label) = self.y_label {
                    y_axis_config = y_axis_config.with_title(label.clone());
                }

                let x_ticks = generate_log_ticks(x_min, x_max);
                let mut x_axis_config = AxisConfig::bottom()
                    .with_tick_values(x_ticks)
                    .with_label_angle(-45.0)
                    .with_label_font_size(8.0)
                    .with_formatter(format_log_tick); // Use k/M formatting for log scale
                if let Some(ref label) = self.x_label {
                    x_axis_config = x_axis_config.with_title(label.clone());
                }

                // Build chart with optional secondary Y axis
                if has_secondary_axis {
                    let mut y2_axis_config = AxisConfig::right().with_label_font_size(8.0);
                    if let Some(ref label) = self.y2_label {
                        y2_axis_config = y2_axis_config.with_title(label.clone());
                    }

                    div()
                        .flex()
                        .child(render_axis(
                            &y_scale,
                            &y_axis_config,
                            plot_height as f32,
                            &axis_theme,
                        ))
                        .child(div().flex().flex_col().child(plot_area).child(render_axis(
                            &x_scale,
                            &x_axis_config,
                            plot_width as f32,
                            &axis_theme,
                        )))
                        .child(render_axis(
                            &y2_scale,
                            &y2_axis_config,
                            plot_height as f32,
                            &axis_theme,
                        ))
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .child(render_axis(
                            &y_scale,
                            &y_axis_config,
                            plot_height as f32,
                            &axis_theme,
                        ))
                        .child(div().flex().flex_col().child(plot_area).child(render_axis(
                            &x_scale,
                            &x_axis_config,
                            plot_width as f32,
                            &axis_theme,
                        )))
                        .into_any_element()
                }
            }
        };

        // Collect legend items if enabled
        // Collect legend items: (series_index, color, label)
        let mut legend_items: Vec<(usize, u32, String)> = Vec::new();
        if has_legend_items {
            // Add primary series to legend if it has a label (index 0)
            if let Some(label) = &self.label {
                legend_items.push((0, self.color, label.clone()));
            }

            // Add all additional series to legend (index 1, 2, ...)
            for (i, series) in self.series.iter().enumerate() {
                if let Some(label) = &series.label {
                    legend_items.push((i + 1, series.color, label.clone()));
                }
            }
        }

        // Build container with optional title
        let mut container = div()
            .w(px(self.width))
            .h(px(self.height))
            .relative()
            .flex()
            .flex_col();

        // Add title if present
        if let Some(title) = &self.title {
            let font_config = VectorFontConfig::horizontal(
                DEFAULT_TITLE_FONT_SIZE,
                self.theme.title_color.into(),
            );
            container = container.child(
                div()
                    .w_full()
                    .h(px(title_height))
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(render_vector_text(title, &font_config)),
            );
        }

        // Add chart content and legend based on position
        if !legend_items.is_empty() {
            // Build interactive legend element
            let hidden_series = self.hidden_series.clone();
            let on_click = self.on_legend_click.clone();
            let legend_text_color = self.theme.legend_text_color;

            let build_legend_item = move |series_idx: usize, color: u32, label: String| {
                let is_hidden = hidden_series.contains(&series_idx);
                let callback = on_click.clone();

                // Base item div with ID for click handling
                let mut item = div()
                    .id(ElementId::NamedInteger(
                        "legend-item".into(),
                        series_idx as u64,
                    ))
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded_sm()
                    .px_1()
                    .cursor_pointer();

                // Add hover effect
                item = item.hover(|s| s.bg(gpui::rgba(0x00000010)));

                // Color swatch - grayed out if hidden
                let swatch_color = if is_hidden {
                    gpui::rgba(0xccccccff)
                } else {
                    rgb(color)
                };
                item = item.child(div().w(px(16.0)).h(px(3.0)).bg(swatch_color));

                // Label - with strikethrough and faded if hidden
                let label_color = if is_hidden {
                    gpui::rgba(0x00000040)
                } else {
                    legend_text_color
                };
                let label_div = div().text_xs().text_color(label_color).child(label);
                item = item.child(label_div);

                // Add click handler if callback provided
                if let Some(cb) = callback {
                    item = item.on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                        cb(series_idx, window, cx);
                    });
                }

                item
            };

            match legend_position {
                LegendPosition::Right => {
                    // Vertical legend on the right (current default behavior)
                    let mut legend_column = div().flex().flex_col().gap_2().p_2();
                    for (idx, color, label) in legend_items {
                        legend_column =
                            legend_column.child(build_legend_item(idx, color, label.clone()));
                    }

                    container = container.child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(legend_gap))
                            .child(chart_content)
                            .child(div().w(px(legend_width)).child(legend_column)),
                    );
                }
                LegendPosition::Left => {
                    // Vertical legend on the left
                    let mut legend_column = div().flex().flex_col().gap_2().p_2();
                    for (idx, color, label) in legend_items {
                        legend_column =
                            legend_column.child(build_legend_item(idx, color, label.clone()));
                    }

                    container = container.child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(legend_gap))
                            .child(div().w(px(legend_width)).child(legend_column))
                            .child(chart_content),
                    );
                }
                LegendPosition::Top => {
                    // Horizontal legend above the chart
                    let mut legend_row = div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap_4()
                        .p_2()
                        .justify_center();
                    for (idx, color, label) in legend_items {
                        legend_row = legend_row.child(build_legend_item(idx, color, label.clone()));
                    }

                    container = container.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(legend_gap))
                            .child(div().h(px(legend_height)).child(legend_row))
                            .child(chart_content),
                    );
                }
                LegendPosition::Bottom => {
                    // Horizontal legend below the chart
                    let mut legend_row = div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap_4()
                        .p_2()
                        .justify_center();
                    for (idx, color, label) in legend_items {
                        legend_row = legend_row.child(build_legend_item(idx, color, label.clone()));
                    }

                    container = container.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(legend_gap))
                            .child(chart_content)
                            .child(div().h(px(legend_height)).child(legend_row)),
                    );
                }
                LegendPosition::Hidden => {
                    // No legend, just add chart content
                    container = container.child(div().relative().child(chart_content));
                }
            }
        } else {
            // No legend, just add chart content
            container = container.child(div().relative().child(chart_content));
        }

        Ok(container)
    }
}

/// Create a line chart from x and y data.
///
/// # Example
///
/// ```rust,no_run
/// use gpui_px::{line, CurveType};
///
/// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let y = vec![2.0, 4.0, 3.0, 5.0, 4.5];
///
/// let chart = line(&x, &y)
///     .title("My Line Chart")
///     .color(0xff7f0e)
///     .curve(CurveType::Linear)
///     .show_points(true)
///     .build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
pub fn line(x: &[f64], y: &[f64]) -> LineChart {
    LineChart {
        x: x.to_vec(),
        y: y.to_vec(),
        title: None,
        x_label: None,
        y_label: None,
        label: None,
        color: DEFAULT_COLOR,
        stroke_width: 2.0,
        opacity: 1.0,
        series: Vec::new(),
        curve: CurveType::Linear,
        show_points: false,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        x_scale_type: ScaleType::Linear,
        y_scale_type: ScaleType::Linear,
        x_range: None,
        y_range: None,
        show_legend: false,
        legend_position: LegendPosition::default(),
        legend_position_explicit: false,
        graph_ratio: 1.414, // √2 ≈ A4 paper aspect ratio
        theme: ChartTheme::default(),
        y2_label: None,
        y2_range: None,
        hidden_series: HashSet::new(),
        on_legend_click: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_empty_x_data() {
        let result = line(&[], &[1.0, 2.0, 3.0]).build();
        assert!(matches!(result, Err(ChartError::EmptyData { field: "x" })));
    }

    #[test]
    fn test_line_empty_y_data() {
        let result = line(&[1.0, 2.0, 3.0], &[]).build();
        assert!(matches!(result, Err(ChartError::EmptyData { field: "y" })));
    }

    #[test]
    fn test_line_data_length_mismatch() {
        let result = line(&[1.0, 2.0, 3.0, 4.0], &[1.0, 2.0]).build();
        assert!(matches!(
            result,
            Err(ChartError::DataLengthMismatch {
                x_field: "x",
                y_field: "y",
                x_len: 4,
                y_len: 2,
            })
        ));
    }

    #[test]
    fn test_line_infinity_in_x() {
        let result = line(&[1.0, 2.0, f64::NEG_INFINITY], &[1.0, 2.0, 3.0]).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "contains NaN or Infinity"
            })
        ));
    }

    #[test]
    fn test_line_nan_in_y() {
        let result = line(&[1.0, 2.0, 3.0], &[1.0, f64::NAN, 3.0]).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "y",
                reason: "contains NaN or Infinity"
            })
        ));
    }

    #[test]
    fn test_line_successful_build() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 3.0, 5.0, 4.5];
        let result = line(&x, &y).title("Test Line").color(0xff7f0e).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_builder_chain() {
        let result = line(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0])
            .title("My Line")
            .color(0x00ff00)
            .stroke_width(3.0)
            .opacity(0.8)
            .curve(CurveType::Linear)
            .show_points(true)
            .size(800.0, 600.0)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_log_x_scale() {
        let x = vec![10.0, 100.0, 1000.0, 10000.0];
        let y = vec![1.0, 2.0, 3.0, 4.0];
        let result = line(&x, &y).x_scale(ScaleType::Log).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_log_y_scale() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![10.0, 100.0, 1000.0, 10000.0];
        let result = line(&x, &y).y_scale(ScaleType::Log).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_log_xy_scale() {
        let x = vec![10.0, 100.0, 1000.0];
        let y = vec![20.0, 200.0, 2000.0];
        let result = line(&x, &y)
            .x_scale(ScaleType::Log)
            .y_scale(ScaleType::Log)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_log_x_negative_values() {
        let x = vec![-10.0, -5.0, 5.0, 10.0];
        let y = vec![1.0, 2.0, 3.0, 4.0];
        let result = line(&x, &y).x_scale(ScaleType::Log).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "contains non-positive values for log scale"
            })
        ));
    }

    #[test]
    fn test_line_log_y_zero_value() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![0.0, 1.0, 2.0, 3.0];
        let result = line(&x, &y).y_scale(ScaleType::Log).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "y",
                reason: "contains non-positive values for log scale"
            })
        ));
    }

    #[test]
    fn test_line_log_scale_with_curve() {
        let x = vec![10.0, 100.0, 1000.0];
        let y = vec![1.0, 2.0, 3.0];
        let result = line(&x, &y)
            .title("Log Scale Line")
            .x_scale(ScaleType::Log)
            .curve(CurveType::Linear)
            .show_points(true)
            .build();
        assert!(result.is_ok());
    }

    // ============================================================================
    // Range Clipping Tests
    // ============================================================================

    #[test]
    fn test_line_x_range() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = line(&x, &y).x_range(2.0, 4.0).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_y_range() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = line(&x, &y).y_range(15.0, 45.0).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_both_ranges() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = line(&x, &y).x_range(1.5, 4.5).y_range(15.0, 45.0).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_x_range_with_log_scale() {
        let x = vec![10.0, 100.0, 1000.0, 10000.0];
        let y = vec![1.0, 2.0, 3.0, 4.0];
        let result = line(&x, &y)
            .x_scale(ScaleType::Log)
            .x_range(50.0, 5000.0)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_y_range_with_log_scale() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![10.0, 100.0, 1000.0, 10000.0];
        let result = line(&x, &y)
            .y_scale(ScaleType::Log)
            .y_range(50.0, 5000.0)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_range_with_title_and_labels() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = line(&x, &y)
            .title("Range Test")
            .x_label("X Axis")
            .y_label("Y Axis")
            .x_range(1.0, 5.0)
            .y_range(10.0, 50.0)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_range_outside_data() {
        // Range extends beyond data - should still work (shows empty space)
        let x = vec![2.0, 3.0, 4.0];
        let y = vec![20.0, 30.0, 40.0];
        let result = line(&x, &y).x_range(0.0, 10.0).y_range(0.0, 100.0).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_range_narrower_than_data() {
        // Range is narrower than data - clips some data points visually
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = line(&x, &y).x_range(3.0, 7.0).y_range(3.0, 7.0).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_range_with_multiple_series() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y1 = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let y2 = vec![5.0, 15.0, 25.0, 35.0, 45.0];
        let result = line(&x, &y1)
            .label("Series 1")
            .add_series(&y2, Some("Series 2"), 0xff7f0e, 2.0, 1.0)
            .x_range(1.5, 4.5)
            .y_range(10.0, 45.0)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_range_negative_values() {
        let x = vec![-5.0, -2.0, 0.0, 2.0, 5.0];
        let y = vec![-10.0, -5.0, 0.0, 5.0, 10.0];
        let result = line(&x, &y).x_range(-3.0, 3.0).y_range(-8.0, 8.0).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_line_range_frequency_response_use_case() {
        // Typical audio frequency response display: 20 Hz to 20 kHz on log scale
        let x: Vec<f64> = (1..=100)
            .map(|i| 20.0 * (1000.0_f64).powf(i as f64 / 100.0))
            .collect();
        let y: Vec<f64> = x.iter().map(|_| 0.0).collect(); // flat response
        let result = line(&x, &y)
            .x_scale(ScaleType::Log)
            .x_range(20.0, 20000.0)
            .y_range(-20.0, 20.0)
            .title("Frequency Response")
            .x_label("Frequency (Hz)")
            .y_label("dB")
            .build();
        assert!(result.is_ok());
    }
}
