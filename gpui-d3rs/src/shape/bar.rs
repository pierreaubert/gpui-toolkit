//! Bar chart rendering

use crate::color::{ColorScheme, D3Color};
use crate::scale::Scale;
use gpui::prelude::*;
use gpui::*;
use std::collections::BTreeMap;

/// Configuration for bar chart rendering
#[derive(Clone)]
pub struct BarConfig {
    /// Fill color for bars
    pub fill_color: D3Color,
    /// Opacity of bars (0.0 - 1.0)
    pub opacity: f32,
    /// Gap between bars in pixels
    pub bar_gap: f32,
    /// Corner radius for bars
    pub border_radius: f32,
    /// Optional stroke color
    pub stroke_color: Option<D3Color>,
    /// Stroke width in pixels
    pub stroke_width: f32,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            fill_color: D3Color::from_hex(0x4682b4), // Steel blue
            opacity: 0.8,
            bar_gap: 2.0,
            border_radius: 2.0,
            stroke_color: None,
            stroke_width: 1.0,
        }
    }
}

impl BarConfig {
    /// Create a new bar configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the fill color
    pub fn fill_color(mut self, color: D3Color) -> Self {
        self.fill_color = color;
        self
    }

    /// Set the opacity
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the gap between bars
    pub fn bar_gap(mut self, gap: f32) -> Self {
        self.bar_gap = gap;
        self
    }

    /// Set the border radius
    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    /// Set the stroke color
    pub fn stroke_color(mut self, color: D3Color) -> Self {
        self.stroke_color = Some(color);
        self
    }

    /// Set the stroke width
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }
}

/// Data point for a bar chart
#[derive(Debug, Clone)]
pub struct BarDatum {
    /// Category or x-axis value
    pub category: String,
    /// Value (height) of the bar
    pub value: f64,
}

impl BarDatum {
    /// Create a new bar datum
    pub fn new(category: impl Into<String>, value: f64) -> Self {
        Self {
            category: category.into(),
            value,
        }
    }
}

/// Render a bar chart
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::prelude::*;
/// use d3rs::shape::{render_bars, BarConfig, BarDatum};
///
/// let x_scale = LinearScale::new().domain(0.0, 5.0).range(0.0, 400.0);
/// let y_scale = LinearScale::new().domain(0.0, 100.0).range(300.0, 0.0);
///
/// let data = vec![
///     BarDatum::new("A", 50.0),
///     BarDatum::new("B", 80.0),
///     BarDatum::new("C", 30.0),
/// ];
///
/// let config = BarConfig::new().fill_color(D3Color::from_hex(0x4682b4));
/// // render_bars(&x_scale, &y_scale, &data, 400.0, 300.0, &config)
/// ```
pub fn render_bars<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[BarDatum],
    width: f32,
    height: f32,
    config: &BarConfig,
) -> impl IntoElement
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    let (x_min, x_max) = x_scale.range();
    let (y_min, y_max) = y_scale.range();
    let x_range_span = x_max - x_min;
    let y_range_span = y_max - y_min;

    // Calculate bar width based on number of bars
    let bar_count = data.len() as f32;
    let available_width = width - (config.bar_gap * (bar_count - 1.0));
    let bar_width = if bar_count > 0.0 {
        available_width / bar_count
    } else {
        0.0
    };

    // Get baseline (zero point in y scale)
    let (y_domain_min, y_domain_max) = y_scale.domain();
    let baseline = if y_domain_min <= 0.0 && y_domain_max >= 0.0 {
        y_scale.scale(0.0)
    } else {
        y_scale.scale(y_domain_min)
    };
    let baseline_pos = 1.0 - ((baseline - y_min) / y_range_span) as f32;

    div()
        .absolute()
        .inset_0()
        .children(data.iter().enumerate().map(|(i, datum)| {
            let x_value = i as f64 + 0.5; // Center bars at integer positions
            let x_range = x_scale.scale(x_value);
            let x_pos = ((x_range - x_min) / x_range_span) as f32;

            let y_range = y_scale.scale(datum.value);
            // Invert Y for screen coordinates (bottom-to-top becomes top-to-bottom)
            let y_pos = 1.0 - ((y_range - y_min) / y_range_span) as f32;

            // Calculate bar height (from baseline to value) - convert from relative to pixels
            let bar_height_rel = (baseline_pos - y_pos).abs();
            let bar_height_px = bar_height_rel * height;
            let bar_top = if datum.value >= 0.0 {
                y_pos
            } else {
                baseline_pos
            };
            let bar_top_px = bar_top * height;

            let fill = config.fill_color.to_rgba();

            let mut bar = div()
                .absolute()
                .left(relative(x_pos))
                .top(px(bar_top_px))
                .w(px(bar_width))
                .h(px(bar_height_px))
                .ml(px(-bar_width / 2.0)) // Center the bar
                .bg(fill)
                .opacity(config.opacity);

            if config.border_radius > 0.0 {
                bar = bar.rounded(px(config.border_radius));
            }

            if let Some(stroke) = &config.stroke_color {
                bar = bar
                    .border_color(stroke.to_rgba())
                    .border(px(config.stroke_width));
            }

            bar
        }))
}

// =============================================================================
// Grouped Bar Charts
// =============================================================================

/// Data point for a grouped bar chart
#[derive(Debug, Clone)]
pub struct GroupedBarDatum {
    /// Category (x-axis group label, e.g., "Q1", "Q2")
    pub category: String,
    /// Series name (e.g., "Product A", "Product B")
    pub series: String,
    /// Value (height) of the bar
    pub value: f64,
}

impl GroupedBarDatum {
    /// Create a new grouped bar datum
    pub fn new(category: impl Into<String>, series: impl Into<String>, value: f64) -> Self {
        Self {
            category: category.into(),
            series: series.into(),
            value,
        }
    }
}

/// Configuration for grouped bar chart rendering
#[derive(Clone)]
pub struct GroupedBarConfig {
    /// Color scheme for series (cycles through colors)
    pub color_scheme: ColorScheme,
    /// Optional explicit colors per series (overrides color_scheme)
    pub series_colors: Option<Vec<D3Color>>,
    /// Opacity of bars (0.0 - 1.0)
    pub opacity: f32,
    /// Gap between groups in pixels
    pub group_gap: f32,
    /// Gap between bars within a group in pixels
    pub bar_gap: f32,
    /// Corner radius for bars
    pub border_radius: f32,
    /// Optional stroke color
    pub stroke_color: Option<D3Color>,
    /// Stroke width in pixels
    pub stroke_width: f32,
}

impl Default for GroupedBarConfig {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::category10(),
            series_colors: None,
            opacity: 0.8,
            group_gap: 8.0,
            bar_gap: 2.0,
            border_radius: 2.0,
            stroke_color: None,
            stroke_width: 1.0,
        }
    }
}

impl GroupedBarConfig {
    /// Create a new grouped bar configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the color scheme for series
    pub fn color_scheme(mut self, scheme: ColorScheme) -> Self {
        self.color_scheme = scheme;
        self
    }

    /// Set explicit colors for each series
    pub fn series_colors(mut self, colors: Vec<D3Color>) -> Self {
        self.series_colors = Some(colors);
        self
    }

    /// Set the opacity
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the gap between groups
    pub fn group_gap(mut self, gap: f32) -> Self {
        self.group_gap = gap;
        self
    }

    /// Set the gap between bars within a group
    pub fn bar_gap(mut self, gap: f32) -> Self {
        self.bar_gap = gap;
        self
    }

    /// Set the border radius
    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    /// Set the stroke color
    pub fn stroke_color(mut self, color: D3Color) -> Self {
        self.stroke_color = Some(color);
        self
    }

    /// Set the stroke width
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Get color for a series by index
    fn get_series_color(&self, index: usize) -> D3Color {
        if let Some(ref colors) = self.series_colors
            && index < colors.len()
        {
            return colors[index];
        }
        self.color_scheme.color(index)
    }
}

/// Metadata about grouped bar data
#[derive(Debug, Clone)]
pub struct GroupedBarMeta {
    /// Ordered list of category names
    pub categories: Vec<String>,
    /// Ordered list of series names
    pub series: Vec<String>,
    /// Minimum value across all data
    pub min_value: f64,
    /// Maximum value across all data
    pub max_value: f64,
}

/// Analyze grouped bar data to extract metadata
pub fn analyze_grouped_data(data: &[GroupedBarDatum]) -> GroupedBarMeta {
    // Use BTreeMap to maintain consistent ordering
    let mut categories_set: BTreeMap<String, usize> = BTreeMap::new();
    let mut series_set: BTreeMap<String, usize> = BTreeMap::new();
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;

    for datum in data {
        let cat_len = categories_set.len();
        categories_set
            .entry(datum.category.clone())
            .or_insert(cat_len);

        let ser_len = series_set.len();
        series_set.entry(datum.series.clone()).or_insert(ser_len);

        min_value = min_value.min(datum.value);
        max_value = max_value.max(datum.value);
    }

    // Sort by insertion order
    let mut categories: Vec<_> = categories_set.into_iter().collect();
    categories.sort_by_key(|(_, idx)| *idx);
    let categories: Vec<String> = categories.into_iter().map(|(name, _)| name).collect();

    let mut series: Vec<_> = series_set.into_iter().collect();
    series.sort_by_key(|(_, idx)| *idx);
    let series: Vec<String> = series.into_iter().map(|(name, _)| name).collect();

    GroupedBarMeta {
        categories,
        series,
        min_value,
        max_value,
    }
}

/// Render a grouped bar chart
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::prelude::*;
/// use d3rs::shape::{render_grouped_bars, GroupedBarConfig, GroupedBarDatum, analyze_grouped_data};
///
/// let data = vec![
///     GroupedBarDatum::new("Q1", "Product A", 50.0),
///     GroupedBarDatum::new("Q1", "Product B", 80.0),
///     GroupedBarDatum::new("Q2", "Product A", 70.0),
///     GroupedBarDatum::new("Q2", "Product B", 60.0),
/// ];
///
/// let meta = analyze_grouped_data(&data);
/// let y_scale = LinearScale::new().domain(0.0, meta.max_value).range(300.0, 0.0);
///
/// let config = GroupedBarConfig::new();
/// // render_grouped_bars(&y_scale, &data, &meta, 400.0, 300.0, &config)
/// ```
pub fn render_grouped_bars<YS>(
    y_scale: &YS,
    data: &[GroupedBarDatum],
    meta: &GroupedBarMeta,
    width: f32,
    height: f32,
    config: &GroupedBarConfig,
) -> impl IntoElement
where
    YS: Scale<f64, f64>,
{
    let num_categories = meta.categories.len() as f32;
    let num_series = meta.series.len() as f32;

    if num_categories == 0.0 || num_series == 0.0 {
        return div().absolute().inset_0();
    }

    // Calculate group and bar widths
    let total_group_gaps = config.group_gap * (num_categories - 1.0).max(0.0);
    let available_width = width - total_group_gaps;
    let group_width = available_width / num_categories;

    let total_bar_gaps = config.bar_gap * (num_series - 1.0).max(0.0);
    let available_bar_width = group_width - total_bar_gaps;
    let bar_width = available_bar_width / num_series;

    // Build category and series index maps
    let category_index: BTreeMap<&str, usize> = meta
        .categories
        .iter()
        .enumerate()
        .map(|(i, c)| (c.as_str(), i))
        .collect();

    let series_index: BTreeMap<&str, usize> = meta
        .series
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();

    // Get baseline (zero point in y scale)
    let (y_min, y_max) = y_scale.range();
    let y_range_span = y_max - y_min;
    let (y_domain_min, y_domain_max) = y_scale.domain();
    let baseline = if y_domain_min <= 0.0 && y_domain_max >= 0.0 {
        y_scale.scale(0.0)
    } else {
        y_scale.scale(y_domain_min)
    };
    let baseline_pos = 1.0 - ((baseline - y_min) / y_range_span) as f32;

    div()
        .absolute()
        .inset_0()
        .children(data.iter().filter_map(|datum| {
            let cat_idx = *category_index.get(datum.category.as_str())?;
            let ser_idx = *series_index.get(datum.series.as_str())?;

            // Calculate x position for this bar
            // Group start position
            let group_start = cat_idx as f32 * (group_width + config.group_gap);
            // Bar position within group
            let bar_offset = ser_idx as f32 * (bar_width + config.bar_gap);
            let x_pos = group_start + bar_offset;

            // Calculate y position
            let y_range = y_scale.scale(datum.value);
            let y_pos = 1.0 - ((y_range - y_min) / y_range_span) as f32;

            // Calculate bar height (from baseline to value)
            let bar_height_rel = (baseline_pos - y_pos).abs();
            let bar_height_px = bar_height_rel * height;
            let bar_top = if datum.value >= 0.0 {
                y_pos
            } else {
                baseline_pos
            };
            let bar_top_px = bar_top * height;

            let fill = config.get_series_color(ser_idx).to_rgba();

            let mut bar = div()
                .absolute()
                .left(px(x_pos))
                .top(px(bar_top_px))
                .w(px(bar_width))
                .h(px(bar_height_px))
                .bg(fill)
                .opacity(config.opacity);

            if config.border_radius > 0.0 {
                bar = bar.rounded(px(config.border_radius));
            }

            if let Some(stroke) = &config.stroke_color {
                bar = bar
                    .border_color(stroke.to_rgba())
                    .border(px(config.stroke_width));
            }

            Some(bar)
        }))
}
