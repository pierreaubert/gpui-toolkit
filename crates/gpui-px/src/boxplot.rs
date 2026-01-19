//! Box plot - Plotly Express style API.
//!
//! Box plots display the distribution of data based on quartiles, showing:
//! - The median (Q2) as a line
//! - The interquartile range (IQR) as a box from Q1 to Q3
//! - Whiskers extending to 1.5×IQR or data min/max
//! - Outliers as individual points

use crate::error::ChartError;
use crate::{
    DEFAULT_COLOR, DEFAULT_HEIGHT, DEFAULT_PADDING_FRACTION, DEFAULT_TITLE_FONT_SIZE,
    DEFAULT_WIDTH, ScaleType, TITLE_AREA_HEIGHT, extent_padded, validate_data_array,
    validate_data_length, validate_dimensions, validate_positive,
};
use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::color::D3Color;
use d3rs::grid::{GridConfig, render_grid};
use d3rs::scale::{LinearScale, LogScale, Scale};
use d3rs::text::{VectorFontConfig, render_vector_text};
use gpui::prelude::*;
use gpui::{AnyElement, IntoElement, div, hsla, px, rgb};

/// Statistics for a single box in a box plot
#[derive(Debug, Clone)]
pub struct BoxStats {
    /// X position (bin center)
    pub x: f64,
    /// First quartile (25th percentile)
    pub q1: f64,
    /// Median (50th percentile)
    pub q2: f64,
    /// Third quartile (75th percentile)
    pub q3: f64,
    /// Lower whisker extent
    pub whisker_low: f64,
    /// Upper whisker extent
    pub whisker_high: f64,
    /// Outliers below the lower whisker
    pub outliers_low: Vec<f64>,
    /// Outliers above the upper whisker
    pub outliers_high: Vec<f64>,
}

impl BoxStats {
    /// Calculate box statistics from a sorted slice of values
    fn from_sorted(x: f64, sorted_values: &[f64]) -> Option<Self> {
        if sorted_values.is_empty() {
            return None;
        }

        let n = sorted_values.len();

        // Calculate quartiles using linear interpolation
        let q1 = percentile(sorted_values, 0.25);
        let q2 = percentile(sorted_values, 0.50);
        let q3 = percentile(sorted_values, 0.75);

        let iqr = q3 - q1;
        let whisker_low_limit = q1 - 1.5 * iqr;
        let whisker_high_limit = q3 + 1.5 * iqr;

        // Find actual whisker positions (closest data points within limits)
        let whisker_low = sorted_values
            .iter()
            .copied()
            .find(|&v| v >= whisker_low_limit)
            .unwrap_or(sorted_values[0]);

        let whisker_high = sorted_values
            .iter()
            .copied()
            .rev()
            .find(|&v| v <= whisker_high_limit)
            .unwrap_or(sorted_values[n - 1]);

        // Collect outliers
        let outliers_low: Vec<f64> = sorted_values
            .iter()
            .copied()
            .filter(|&v| v < whisker_low)
            .collect();

        let outliers_high: Vec<f64> = sorted_values
            .iter()
            .copied()
            .filter(|&v| v > whisker_high)
            .collect();

        Some(BoxStats {
            x,
            q1,
            q2,
            q3,
            whisker_low,
            whisker_high,
            outliers_low,
            outliers_high,
        })
    }
}

/// Calculate percentile using linear interpolation
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }

    let n = sorted.len();
    let index = p * (n - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    let frac = index - lower as f64;

    if lower == upper || upper >= n {
        sorted[lower.min(n - 1)]
    } else {
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    }
}

/// Box plot builder.
#[derive(Debug, Clone)]
pub struct BoxPlotChart {
    x: Vec<f64>,
    y: Vec<f64>,
    title: Option<String>,
    box_color: u32,
    median_color: u32,
    whisker_color: u32,
    outlier_color: u32,
    box_opacity: f32,
    box_width: f32,
    stroke_width: f32,
    outlier_radius: f32,
    num_bins: Option<usize>,
    width: f32,
    height: f32,
    x_scale_type: ScaleType,
    y_scale_type: ScaleType,
}

impl BoxPlotChart {
    /// Set chart title (rendered at top of chart).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set box fill color as 24-bit RGB hex value (format: 0xRRGGBB).
    pub fn box_color(mut self, hex: u32) -> Self {
        self.box_color = hex;
        self
    }

    /// Set median line color.
    pub fn median_color(mut self, hex: u32) -> Self {
        self.median_color = hex;
        self
    }

    /// Set whisker line color.
    pub fn whisker_color(mut self, hex: u32) -> Self {
        self.whisker_color = hex;
        self
    }

    /// Set outlier point color.
    pub fn outlier_color(mut self, hex: u32) -> Self {
        self.outlier_color = hex;
        self
    }

    /// Set box opacity (0.0 - 1.0).
    pub fn box_opacity(mut self, opacity: f32) -> Self {
        self.box_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set box width in pixels.
    pub fn box_width(mut self, width: f32) -> Self {
        self.box_width = width;
        self
    }

    /// Set stroke width for median and whisker lines.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Set outlier point radius.
    pub fn outlier_radius(mut self, radius: f32) -> Self {
        self.outlier_radius = radius;
        self
    }

    /// Set the number of bins for grouping data.
    /// If not set, automatically calculated based on chart width.
    pub fn bins(mut self, n: usize) -> Self {
        self.num_bins = Some(n);
        self
    }

    /// Set chart dimensions.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set X-axis scale type (linear or log).
    pub fn x_scale(mut self, scale: ScaleType) -> Self {
        self.x_scale_type = scale;
        self
    }

    /// Set Y-axis scale type (linear or log).
    pub fn y_scale(mut self, scale: ScaleType) -> Self {
        self.y_scale_type = scale;
        self
    }

    /// Build and validate the chart, returning renderable element.
    pub fn build(self) -> Result<impl IntoElement, ChartError> {
        // Validate inputs
        validate_data_array(&self.x, "x")?;
        validate_data_array(&self.y, "y")?;
        validate_data_length(self.x.len(), self.y.len(), "x", "y")?;
        validate_dimensions(self.width, self.height)?;

        // Validate positive values for log scale
        if self.x_scale_type == ScaleType::Log {
            validate_positive(&self.x, "x")?;
        }
        if self.y_scale_type == ScaleType::Log {
            validate_positive(&self.y, "y")?;
        }

        // Define margins
        let margin_left = 60.0;
        let margin_bottom = 30.0;
        let margin_top = 10.0;
        let margin_right = 20.0;

        // Calculate plot area
        let title_height = if self.title.is_some() {
            TITLE_AREA_HEIGHT
        } else {
            0.0
        };

        let plot_width = (self.width as f64 - margin_left - margin_right).max(0.0);
        let plot_height =
            (self.height as f64 - title_height as f64 - margin_top - margin_bottom).max(0.0);

        // Calculate domains
        let (x_min, x_max) = extent_padded(&self.x, DEFAULT_PADDING_FRACTION);
        let (y_min, y_max) = extent_padded(&self.y, DEFAULT_PADDING_FRACTION);

        // Calculate number of bins
        let num_bins = self
            .num_bins
            .unwrap_or_else(|| (plot_width / 40.0).max(3.0) as usize);

        // Bin the data
        let boxes = self.calculate_boxes(x_min, x_max, num_bins);

        // Build based on scale types
        let chart_content =
            self.render_chart(&boxes, x_min, x_max, y_min, y_max, plot_width, plot_height);

        // Build container with optional title
        let mut container = div()
            .w(px(self.width))
            .h(px(self.height))
            .relative()
            .flex()
            .flex_col();

        // Add title if present
        if let Some(title) = &self.title {
            let font_config =
                VectorFontConfig::horizontal(DEFAULT_TITLE_FONT_SIZE, hsla(0.0, 0.0, 0.2, 1.0));
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

        // Add chart content
        container = container.child(div().relative().child(chart_content));

        Ok(container)
    }

    /// Calculate box statistics for each bin
    fn calculate_boxes(&self, x_min: f64, x_max: f64, num_bins: usize) -> Vec<BoxStats> {
        let bin_width = (x_max - x_min) / num_bins as f64;

        // Group data points by bin
        let mut bins: Vec<Vec<f64>> = vec![Vec::new(); num_bins];

        for (&x, &y) in self.x.iter().zip(self.y.iter()) {
            let bin_idx = ((x - x_min) / bin_width).floor() as usize;
            let bin_idx = bin_idx.min(num_bins - 1);
            bins[bin_idx].push(y);
        }

        // Calculate statistics for each non-empty bin
        bins.iter_mut()
            .enumerate()
            .filter_map(|(i, bin)| {
                if bin.is_empty() {
                    return None;
                }
                bin.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let x_center = x_min + (i as f64 + 0.5) * bin_width;
                BoxStats::from_sorted(x_center, bin)
            })
            .collect()
    }

    /// Render the chart content
    fn render_chart(
        &self,
        boxes: &[BoxStats],
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        plot_width: f64,
        plot_height: f64,
    ) -> AnyElement {
        let theme = DefaultAxisTheme;

        match (self.x_scale_type, self.y_scale_type) {
            (ScaleType::Linear, ScaleType::Linear) => {
                let x_scale = LinearScale::new()
                    .domain(x_min, x_max)
                    .range(0.0, plot_width);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height, 0.0);

                self.render_with_scales(&x_scale, &y_scale, boxes, plot_width, plot_height, &theme)
            }
            (ScaleType::Log, ScaleType::Linear) => {
                let x_scale = LogScale::new()
                    .domain(x_min.max(1e-10), x_max)
                    .range(0.0, plot_width);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height, 0.0);

                self.render_with_scales(&x_scale, &y_scale, boxes, plot_width, plot_height, &theme)
            }
            (ScaleType::Linear, ScaleType::Log) => {
                let x_scale = LinearScale::new()
                    .domain(x_min, x_max)
                    .range(0.0, plot_width);
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(plot_height, 0.0);

                self.render_with_scales(&x_scale, &y_scale, boxes, plot_width, plot_height, &theme)
            }
            (ScaleType::Log, ScaleType::Log) => {
                let x_scale = LogScale::new()
                    .domain(x_min.max(1e-10), x_max)
                    .range(0.0, plot_width);
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(plot_height, 0.0);

                self.render_with_scales(&x_scale, &y_scale, boxes, plot_width, plot_height, &theme)
            }
        }
    }

    /// Render with specific scale types
    fn render_with_scales<XS, YS>(
        &self,
        x_scale: &XS,
        y_scale: &YS,
        boxes: &[BoxStats],
        plot_width: f64,
        plot_height: f64,
        theme: &DefaultAxisTheme,
    ) -> AnyElement
    where
        XS: Scale<f64, f64>,
        YS: Scale<f64, f64>,
    {
        let box_color = D3Color::from_hex(self.box_color).to_rgba();
        let median_color = D3Color::from_hex(self.median_color).to_rgba();
        let whisker_color = D3Color::from_hex(self.whisker_color).to_rgba();
        let outlier_color = D3Color::from_hex(self.outlier_color).to_rgba();

        // Render all boxes
        let box_elements: Vec<AnyElement> = boxes
            .iter()
            .flat_map(|stats| {
                let x_px = x_scale.scale(stats.x) as f32;
                let half_width = self.box_width / 2.0;

                let q1_px = y_scale.scale(stats.q1) as f32;
                let q2_px = y_scale.scale(stats.q2) as f32;
                let q3_px = y_scale.scale(stats.q3) as f32;
                let whisker_low_px = y_scale.scale(stats.whisker_low) as f32;
                let whisker_high_px = y_scale.scale(stats.whisker_high) as f32;

                let box_top = q3_px.min(q1_px);
                let box_bottom = q3_px.max(q1_px);
                let box_height = (box_bottom - box_top).max(1.0);

                let mut elements: Vec<AnyElement> = Vec::new();

                // Whisker line (vertical line from low to high)
                elements.push(
                    div()
                        .absolute()
                        .left(px(x_px - 0.5))
                        .top(px(whisker_high_px.min(whisker_low_px)))
                        .w(px(self.stroke_width))
                        .h(px((whisker_low_px - whisker_high_px).abs().max(1.0)))
                        .bg(whisker_color)
                        .into_any_element(),
                );

                // Lower whisker cap (horizontal line)
                elements.push(
                    div()
                        .absolute()
                        .left(px(x_px - half_width * 0.5))
                        .top(px(whisker_low_px - self.stroke_width / 2.0))
                        .w(px(half_width))
                        .h(px(self.stroke_width))
                        .bg(whisker_color)
                        .into_any_element(),
                );

                // Upper whisker cap (horizontal line)
                elements.push(
                    div()
                        .absolute()
                        .left(px(x_px - half_width * 0.5))
                        .top(px(whisker_high_px - self.stroke_width / 2.0))
                        .w(px(half_width))
                        .h(px(self.stroke_width))
                        .bg(whisker_color)
                        .into_any_element(),
                );

                // Box (IQR)
                elements.push(
                    div()
                        .absolute()
                        .left(px(x_px - half_width))
                        .top(px(box_top))
                        .w(px(self.box_width))
                        .h(px(box_height))
                        .bg(box_color)
                        .opacity(self.box_opacity)
                        .border_1()
                        .border_color(whisker_color)
                        .into_any_element(),
                );

                // Median line
                elements.push(
                    div()
                        .absolute()
                        .left(px(x_px - half_width))
                        .top(px(q2_px - self.stroke_width))
                        .w(px(self.box_width))
                        .h(px(self.stroke_width * 2.0))
                        .bg(median_color)
                        .into_any_element(),
                );

                // Outliers
                for &outlier in &stats.outliers_low {
                    let y_px = y_scale.scale(outlier) as f32;
                    elements.push(
                        div()
                            .absolute()
                            .left(px(x_px - self.outlier_radius))
                            .top(px(y_px - self.outlier_radius))
                            .w(px(self.outlier_radius * 2.0))
                            .h(px(self.outlier_radius * 2.0))
                            .rounded_full()
                            .bg(outlier_color)
                            .opacity(0.7)
                            .into_any_element(),
                    );
                }

                for &outlier in &stats.outliers_high {
                    let y_px = y_scale.scale(outlier) as f32;
                    elements.push(
                        div()
                            .absolute()
                            .left(px(x_px - self.outlier_radius))
                            .top(px(y_px - self.outlier_radius))
                            .w(px(self.outlier_radius * 2.0))
                            .h(px(self.outlier_radius * 2.0))
                            .rounded_full()
                            .bg(outlier_color)
                            .opacity(0.7)
                            .into_any_element(),
                    );
                }

                elements
            })
            .collect();

        div()
            .flex()
            .child(render_axis(
                y_scale,
                &AxisConfig::left(),
                plot_height as f32,
                theme,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w(px(plot_width as f32))
                            .h(px(plot_height as f32))
                            .relative()
                            .bg(rgb(0xf8f8f8))
                            .child(render_grid(
                                x_scale,
                                y_scale,
                                &GridConfig::default(),
                                plot_width as f32,
                                plot_height as f32,
                                theme,
                            ))
                            .children(box_elements),
                    )
                    .child(render_axis(
                        x_scale,
                        &AxisConfig::bottom(),
                        plot_width as f32,
                        theme,
                    )),
            )
            .into_any_element()
    }
}

/// Create a box plot from x and y data.
///
/// The data is binned by x values, and for each bin, box-and-whisker statistics
/// are calculated from the y values.
///
/// # Example
///
/// ```rust,ignore
/// use gpui_px::boxplot;
///
/// // Generate some sample data
/// let x: Vec<f64> = (0..100).map(|i| (i / 10) as f64).collect();
/// let y: Vec<f64> = x.iter().map(|&xi| xi * 2.0 + rand::random::<f64>() * 10.0).collect();
///
/// let chart = boxplot(&x, &y)
///     .title("Distribution by Group")
///     .box_color(0xdddddd)
///     .median_color(0x000000)
///     .build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
pub fn boxplot(x: &[f64], y: &[f64]) -> BoxPlotChart {
    BoxPlotChart {
        x: x.to_vec(),
        y: y.to_vec(),
        title: None,
        box_color: 0xdddddd,
        median_color: 0x000000,
        whisker_color: 0x333333,
        outlier_color: DEFAULT_COLOR,
        box_opacity: 1.0,
        box_width: 20.0,
        stroke_width: 2.0,
        outlier_radius: 3.0,
        num_bins: None,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        x_scale_type: ScaleType::Linear,
        y_scale_type: ScaleType::Linear,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_basic() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile(&values, 0.0) - 1.0).abs() < 1e-10);
        assert!((percentile(&values, 0.5) - 3.0).abs() < 1e-10);
        assert!((percentile(&values, 1.0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_percentile_quartiles() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0];
        let q1 = percentile(&values, 0.25);
        let q2 = percentile(&values, 0.50);
        let q3 = percentile(&values, 0.75);
        assert!((q2 - 6.0).abs() < 1e-10); // Median
        assert!(q1 < q2);
        assert!(q2 < q3);
    }

    #[test]
    fn test_box_stats_from_sorted() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let stats = BoxStats::from_sorted(5.0, &values).unwrap();

        assert!((stats.x - 5.0).abs() < 1e-10);
        assert!(stats.q1 < stats.q2);
        assert!(stats.q2 < stats.q3);
        assert!(stats.whisker_low <= stats.q1);
        assert!(stats.whisker_high >= stats.q3);
    }

    #[test]
    fn test_box_stats_with_outliers() {
        // Create data with outliers
        let values = vec![1.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 100.0];
        let stats = BoxStats::from_sorted(0.0, &values).unwrap();

        // 1.0 and 100.0 should be outliers
        assert!(!stats.outliers_low.is_empty() || !stats.outliers_high.is_empty());
    }

    #[test]
    fn test_boxplot_empty_data() {
        let result = boxplot(&[], &[]).build();
        assert!(matches!(result, Err(ChartError::EmptyData { .. })));
    }

    #[test]
    fn test_boxplot_mismatched_lengths() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0];
        let result = boxplot(&x, &y).build();
        assert!(matches!(result, Err(ChartError::DataLengthMismatch { .. })));
    }

    #[test]
    fn test_boxplot_successful_build() {
        let x: Vec<f64> = (0..100).map(|i| (i / 10) as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| xi * 2.0).collect();

        let result = boxplot(&x, &y)
            .title("Test Box Plot")
            .box_color(0xcccccc)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_boxplot_builder_chain() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 20.0, 15.0, 25.0, 30.0];

        let result = boxplot(&x, &y)
            .title("My Box Plot")
            .box_color(0xdddddd)
            .median_color(0xff0000)
            .whisker_color(0x333333)
            .outlier_color(0x0000ff)
            .box_opacity(0.8)
            .box_width(25.0)
            .stroke_width(3.0)
            .outlier_radius(4.0)
            .bins(5)
            .size(800.0, 600.0)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_boxplot_log_scale_positive_values() {
        let x = vec![10.0, 100.0, 1000.0, 10000.0];
        let y = vec![1.0, 10.0, 100.0, 1000.0];

        let result = boxplot(&x, &y)
            .x_scale(ScaleType::Log)
            .y_scale(ScaleType::Log)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_boxplot_log_scale_negative_values() {
        let x = vec![-1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0, 3.0];

        let result = boxplot(&x, &y).x_scale(ScaleType::Log).build();
        assert!(matches!(result, Err(ChartError::InvalidData { .. })));
    }
}
