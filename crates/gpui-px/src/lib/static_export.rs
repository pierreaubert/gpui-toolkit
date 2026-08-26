//! Static SVG export helpers for renderer-backed chart builders.

use crate::{
    ChartError, ColorScale, DEFAULT_PADDING_FRACTION, ScaleType, extent_log_padded_iter,
    extent_padded, extent_padded_iter, validate_data_array, validate_data_length,
    validate_dimensions, validate_grid_dimensions, validate_monotonic, validate_positive,
    validate_range, validate_range_log,
};
use std::fmt::Write;

/// Options used when exporting a chart to deterministic SVG.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticSvgOptions {
    /// SVG viewport width in CSS pixels.
    pub width: f32,
    /// SVG viewport height in CSS pixels.
    pub height: f32,
    /// Left plot margin.
    pub margin_left: f32,
    /// Right plot margin.
    pub margin_right: f32,
    /// Top plot margin.
    pub margin_top: f32,
    /// Bottom plot margin.
    pub margin_bottom: f32,
    /// Optional SVG background color.
    pub background: Option<u32>,
    /// Whether to draw lightweight axes and grid lines.
    pub show_axes: bool,
}

impl StaticSvgOptions {
    /// Create export options using chart dimensions and default margins.
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            ..Self::default()
        }
    }
}

impl Default for StaticSvgOptions {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            margin_left: 56.0,
            margin_right: 24.0,
            margin_top: 40.0,
            margin_bottom: 48.0,
            background: Some(0xffffff),
            show_axes: true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StaticXySeries<'a> {
    pub(crate) x: &'a [f64],
    pub(crate) y: &'a [f64],
    pub(crate) label: Option<&'a str>,
    pub(crate) color: u32,
    pub(crate) opacity: f32,
    pub(crate) stroke_width: f32,
    pub(crate) point_radius: f32,
    pub(crate) use_secondary_y: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StaticBarSeries<'a> {
    pub(crate) values: &'a [f64],
    pub(crate) label: Option<&'a str>,
    pub(crate) color: u32,
    pub(crate) opacity: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StaticAreaSeries<'a> {
    pub(crate) x: &'a [f64],
    pub(crate) y: &'a [f64],
    pub(crate) y0: Option<&'a [f64]>,
    pub(crate) color: u32,
    pub(crate) opacity: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StaticPieSeries<'a> {
    pub(crate) values: &'a [f64],
    pub(crate) labels: Option<&'a [String]>,
    pub(crate) colors: Option<&'a [u32]>,
    pub(crate) inner_radius_fraction: f64,
    pub(crate) pad_angle: f64,
    pub(crate) corner_radius: f64,
    pub(crate) sort: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StaticHeatmapSeries<'a> {
    pub(crate) z: &'a [f64],
    pub(crate) grid_width: usize,
    pub(crate) grid_height: usize,
    pub(crate) x_values: Option<&'a [f64]>,
    pub(crate) y_values: Option<&'a [f64]>,
    pub(crate) x_scale_type: ScaleType,
    pub(crate) y_scale_type: ScaleType,
    pub(crate) x_range: Option<[f64; 2]>,
    pub(crate) y_range: Option<[f64; 2]>,
    pub(crate) color_scale: &'a ColorScale,
    pub(crate) opacity: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StaticBoxPlotSeries<'a> {
    pub(crate) x: &'a [f64],
    pub(crate) y: &'a [f64],
    pub(crate) x_scale_type: ScaleType,
    pub(crate) y_scale_type: ScaleType,
    pub(crate) num_bins: Option<usize>,
    pub(crate) box_color: u32,
    pub(crate) median_color: u32,
    pub(crate) whisker_color: u32,
    pub(crate) outlier_color: u32,
    pub(crate) box_opacity: f32,
    pub(crate) box_width: f32,
    pub(crate) stroke_width: f32,
    pub(crate) outlier_radius: f32,
}

pub(crate) fn render_line_svg(
    title: Option<&str>,
    x_scale_type: ScaleType,
    y_scale_type: ScaleType,
    x_range: Option<[f64; 2]>,
    y_range: Option<[f64; 2]>,
    y2_range: Option<[f64; 2]>,
    series: &[StaticXySeries<'_>],
    options: StaticSvgOptions,
) -> Result<String, ChartError> {
    validate_xy_series(series, x_scale_type, y_scale_type)?;
    render_xy_svg(
        "line",
        title,
        x_scale_type,
        y_scale_type,
        x_range,
        y_range,
        y2_range,
        series,
        options,
    )
}

pub(crate) fn render_scatter_svg(
    title: Option<&str>,
    x_scale_type: ScaleType,
    y_scale_type: ScaleType,
    x_range: Option<[f64; 2]>,
    y_range: Option<[f64; 2]>,
    series: &[StaticXySeries<'_>],
    options: StaticSvgOptions,
) -> Result<String, ChartError> {
    validate_xy_series(series, x_scale_type, y_scale_type)?;
    render_xy_svg(
        "scatter",
        title,
        x_scale_type,
        y_scale_type,
        x_range,
        y_range,
        None,
        series,
        options,
    )
}

pub(crate) fn render_bar_svg(
    title: Option<&str>,
    categories: &[String],
    y_scale_type: ScaleType,
    y_range: Option<[f64; 2]>,
    series: &[StaticBarSeries<'_>],
    options: StaticSvgOptions,
) -> Result<String, ChartError> {
    validate_dimensions(options.width, options.height)?;
    validate_plot_area(options)?;
    if categories.is_empty() {
        return Err(ChartError::EmptyData {
            field: "categories",
        });
    }
    if series.is_empty() {
        return Err(ChartError::EmptyData { field: "values" });
    }

    for (index, bar_series) in series.iter().enumerate() {
        validate_data_array(
            bar_series.values,
            if index == 0 {
                "values"
            } else {
                "series.values"
            },
        )?;
        validate_data_length(
            categories.len(),
            bar_series.values.len(),
            "categories",
            if index == 0 {
                "values"
            } else {
                "series.values"
            },
        )?;
        if y_scale_type == ScaleType::Log {
            validate_positive(
                bar_series.values,
                if index == 0 {
                    "values"
                } else {
                    "series.values"
                },
            )?;
        }
    }

    let domain = match y_range {
        Some([min, max]) => {
            if y_scale_type == ScaleType::Log {
                validate_range_log(min, max, "y_range")?;
            } else {
                validate_range(min, max, "y_range")?;
            }
            (min, max)
        }
        None => extent_padded_iter(
            series
                .iter()
                .flat_map(|bar_series| bar_series.values.iter().copied())
                .chain((y_scale_type == ScaleType::Linear).then_some(0.0)),
            DEFAULT_PADDING_FRACTION,
        ),
    };

    let layout = StaticLayout::new(options)?;
    let mut svg = svg_header(options);
    draw_title(&mut svg, title, options.width);
    draw_axes(
        &mut svg,
        options,
        &layout,
        Some(categories),
        domain,
        y_scale_type,
    );

    let group_count = categories.len() as f32;
    let series_count = series.len() as f32;
    let group_gap = 8.0_f32.min(layout.plot_width / group_count.max(1.0) * 0.25);
    let group_width = (layout.plot_width - group_gap * (group_count - 1.0).max(0.0)) / group_count;
    let bar_gap = 3.0_f32.min(group_width * 0.15);
    let bar_width =
        ((group_width - bar_gap * (series_count - 1.0).max(0.0)) / series_count).max(1.0);
    let baseline = if y_scale_type == ScaleType::Linear && domain.0 < 0.0 && domain.1 > 0.0 {
        map_linear(0.0, domain.0, domain.1, layout.plot_bottom, layout.plot_top)
    } else {
        map_scaled(
            domain.0,
            domain,
            y_scale_type,
            layout.plot_bottom,
            layout.plot_top,
        )
    };

    svg.push_str("<g class=\"gpui-px-bars\">\n");
    for category_index in 0..categories.len() {
        let group_x = layout.plot_left
            + category_index as f32 * (group_width + group_gap)
            + ((group_width
                - (bar_width * series_count + bar_gap * (series_count - 1.0).max(0.0)))
                / 2.0);

        for (series_index, bar_series) in series.iter().enumerate() {
            let value = bar_series.values[category_index];
            let x = group_x + series_index as f32 * (bar_width + bar_gap);
            let y = map_scaled(
                value,
                domain,
                y_scale_type,
                layout.plot_bottom,
                layout.plot_top,
            );
            let top = y.min(baseline);
            let height = (baseline - y).abs().max(1.0);
            let _ = writeln!(
                svg,
                "<rect x=\"{x:.2}\" y=\"{top:.2}\" width=\"{bar_width:.2}\" height=\"{height:.2}\" fill=\"{}\" opacity=\"{:.3}\"/>",
                hex_color(bar_series.color),
                clamp_opacity(bar_series.opacity),
            );
        }
    }
    svg.push_str("</g>\n");

    draw_legend(
        &mut svg,
        series
            .iter()
            .filter_map(|series| series.label.map(|label| (label, series.color, "square"))),
        &layout,
    );
    svg.push_str("</svg>\n");
    Ok(svg)
}

pub(crate) fn render_area_svg(
    title: Option<&str>,
    x_scale_type: ScaleType,
    y_scale_type: ScaleType,
    series: StaticAreaSeries<'_>,
    options: StaticSvgOptions,
) -> Result<String, ChartError> {
    validate_dimensions(options.width, options.height)?;
    validate_plot_area(options)?;
    validate_data_array(series.x, "x")?;
    validate_data_array(series.y, "y")?;
    validate_data_length(series.x.len(), series.y.len(), "x", "y")?;

    if let Some(y0) = series.y0 {
        validate_data_array(y0, "y0")?;
        validate_data_length(series.x.len(), y0.len(), "x", "y0")?;
    }

    if x_scale_type == ScaleType::Log {
        validate_positive(series.x, "x")?;
    }
    if y_scale_type == ScaleType::Log {
        validate_positive(series.y, "y")?;
        if let Some(y0) = series.y0 {
            validate_positive(y0, "y0")?;
        }
    }

    let x_domain = resolve_xy_domain(None, x_scale_type, "x_range", series.x.iter().copied())?;
    let baseline = if let Some(y0) = series.y0 {
        EitherBaseline::Explicit(y0)
    } else if y_scale_type == ScaleType::Log {
        EitherBaseline::Constant(series.y.iter().copied().fold(f64::INFINITY, f64::min))
    } else {
        EitherBaseline::Constant(0.0)
    };
    let y_domain = match baseline {
        EitherBaseline::Explicit(y0) => auto_xy_domain(
            y_scale_type,
            series.y.iter().copied().chain(y0.iter().copied()),
        ),
        EitherBaseline::Constant(y0) => auto_xy_domain(
            y_scale_type,
            series.y.iter().copied().chain(std::iter::once(y0)),
        ),
    };

    let layout = StaticLayout::new(options)?;
    let mut svg = svg_header(options);
    draw_title(&mut svg, title, options.width);
    draw_axes(&mut svg, options, &layout, None, y_domain, y_scale_type);

    let mut path = String::new();
    for (index, (&x, &y)) in series.x.iter().zip(series.y.iter()).enumerate() {
        let sx = map_scaled(
            x,
            x_domain,
            x_scale_type,
            layout.plot_left,
            layout.plot_right,
        );
        let sy = map_scaled(
            y,
            y_domain,
            y_scale_type,
            layout.plot_bottom,
            layout.plot_top,
        );
        let command = if index == 0 { 'M' } else { 'L' };
        let _ = write!(path, "{command}{sx:.2},{sy:.2}");
    }

    for index in (0..series.x.len()).rev() {
        let x = series.x[index];
        let y0 = match baseline {
            EitherBaseline::Explicit(y0) => y0[index],
            EitherBaseline::Constant(y0) => y0,
        };
        let sx = map_scaled(
            x,
            x_domain,
            x_scale_type,
            layout.plot_left,
            layout.plot_right,
        );
        let sy = map_scaled(
            y0,
            y_domain,
            y_scale_type,
            layout.plot_bottom,
            layout.plot_top,
        );
        let _ = write!(path, "L{sx:.2},{sy:.2}");
    }

    let _ = writeln!(
        svg,
        "<path class=\"gpui-px-area\" d=\"{path}Z\" fill=\"{}\" opacity=\"{:.3}\" stroke=\"{}\" stroke-width=\"1\"/>",
        hex_color(series.color),
        clamp_opacity(series.opacity),
        hex_color(series.color),
    );
    svg.push_str("</svg>\n");
    Ok(svg)
}

pub(crate) fn render_pie_svg(
    title: Option<&str>,
    series: StaticPieSeries<'_>,
    options: StaticSvgOptions,
) -> Result<String, ChartError> {
    use d3rs::shape::{Arc as D3Arc, Pie};

    validate_dimensions(options.width, options.height)?;
    validate_plot_area(options)?;
    validate_data_array(series.values, "values")?;

    if let Some(labels) = series.labels {
        validate_data_length(labels.len(), series.values.len(), "labels", "values")?;
    }

    if series.values.iter().any(|&value| value < 0.0) {
        return Err(ChartError::InvalidData {
            field: "values",
            reason: "pie chart values must be non-negative",
        });
    }

    let total: f64 = series.values.iter().sum();
    if total <= 0.0 {
        return Err(ChartError::InvalidData {
            field: "values",
            reason: "pie chart values must sum to a positive number",
        });
    }

    let layout = StaticLayout::new(options)?;
    let radius = (layout.plot_width.min(layout.plot_bottom - layout.plot_top) as f64 / 2.0) * 0.9;
    let inner_radius = radius * series.inner_radius_fraction.clamp(0.0, 0.99);
    let center_x = (layout.plot_left + layout.plot_right) as f64 / 2.0;
    let center_y = (layout.plot_top + layout.plot_bottom) as f64 / 2.0;
    let pie = Pie::new()
        .pad_angle(series.pad_angle)
        .corner_radius(series.corner_radius)
        .inner_radius(inner_radius)
        .outer_radius(radius)
        .sort(series.sort);
    let arc = D3Arc::new().center(center_x, center_y);
    let palette = series.colors.filter(|colors| !colors.is_empty());

    let mut svg = svg_header(options);
    draw_title(&mut svg, title, options.width);
    svg.push_str("<g class=\"gpui-px-pie\">\n");

    let mut legend_items = Vec::new();
    for slice in pie.generate(series.values, |value| *value) {
        if slice.value <= 0.0 {
            continue;
        }

        let color = color_at(slice.index, palette);
        let label = series
            .labels
            .and_then(|labels| labels.get(slice.index))
            .map(String::as_str);
        let fallback_label;
        let label = match label {
            Some(label) => label,
            None => {
                fallback_label = format!("Slice {}", slice.index + 1);
                &fallback_label
            }
        };
        let percent = (slice.value / total) * 100.0;
        let path = arc
            .generate(&slice.arc)
            .flatten(0.5)
            .into_iter()
            .enumerate()
            .fold(String::new(), |mut path, (index, point)| {
                let command = if index == 0 { 'M' } else { 'L' };
                let _ = write!(path, "{command}{:.2},{:.2}", point.x, point.y);
                path
            });

        if path.is_empty() {
            continue;
        }

        let _ = writeln!(
            svg,
            "<path d=\"{path}Z\" fill=\"{}\"><title>{}: {:.3} ({percent:.2}%)</title></path>",
            hex_color(color),
            escape_xml(label),
            slice.value,
        );
        legend_items.push((label.to_string(), color, "square"));
    }
    svg.push_str("</g>\n");

    draw_legend(
        &mut svg,
        legend_items
            .iter()
            .map(|(label, color, marker)| (label.as_str(), *color, *marker)),
        &layout,
    );
    svg.push_str("</svg>\n");
    Ok(svg)
}

pub(crate) fn render_heatmap_svg(
    title: Option<&str>,
    series: StaticHeatmapSeries<'_>,
    options: StaticSvgOptions,
) -> Result<String, ChartError> {
    validate_dimensions(options.width, options.height)?;
    validate_plot_area(options)?;
    validate_data_array(series.z, "z")?;
    validate_grid_dimensions(series.z, series.grid_width, series.grid_height)?;

    let x_values = resolve_heatmap_axis(
        series.x_values,
        series.grid_width,
        series.x_scale_type,
        "x",
        "grid_width",
        "log scale requires explicit positive x values",
    )?;
    let y_values = resolve_heatmap_axis(
        series.y_values,
        series.grid_height,
        series.y_scale_type,
        "y",
        "grid_height",
        "log scale requires explicit positive y values",
    )?;

    if let Some([min, max]) = series.x_range {
        if series.x_scale_type == ScaleType::Log {
            validate_range_log(min, max, "x_range")?;
        } else {
            validate_range(min, max, "x_range")?;
        }
    }
    if let Some([min, max]) = series.y_range {
        if series.y_scale_type == ScaleType::Log {
            validate_range_log(min, max, "y_range")?;
        } else {
            validate_range(min, max, "y_range")?;
        }
    }

    let x_domain = series
        .x_range
        .map(|[min, max]| (min, max))
        .unwrap_or_else(|| extent_padded(&x_values, 0.0));
    let y_domain = series
        .y_range
        .map(|[min, max]| (min, max))
        .unwrap_or_else(|| extent_padded(&y_values, 0.0));
    let z_domain = extent_padded(series.z, 0.0);

    let layout = StaticLayout::new(options)?;
    let mut svg = svg_header(options);
    draw_title(&mut svg, title, options.width);
    draw_heatmap_axes(&mut svg, options, &layout, x_domain, y_domain);

    let cell_width = layout.plot_width / series.grid_width as f32;
    let plot_height = layout.plot_bottom - layout.plot_top;
    let cell_height = plot_height / series.grid_height as f32;
    let opacity = clamp_opacity(series.opacity);
    svg.push_str("<g class=\"gpui-px-heatmap\">\n");
    for row in 0..series.grid_height {
        for col in 0..series.grid_width {
            let value = series.z[row * series.grid_width + col];
            let normalized = normalize_value(value, z_domain);
            let color = series.color_scale.map(normalized).to_hex();
            let x = layout.plot_left + col as f32 * cell_width;
            let y = layout.plot_top + (series.grid_height - row - 1) as f32 * cell_height;
            let _ = writeln!(
                svg,
                "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{cell_width:.2}\" height=\"{cell_height:.2}\" fill=\"{color}\" opacity=\"{opacity:.3}\"><title>row {row}, column {col}: {value:.3}</title></rect>",
            );
        }
    }
    svg.push_str("</g>\n</svg>\n");
    Ok(svg)
}

pub(crate) fn render_boxplot_svg(
    title: Option<&str>,
    series: StaticBoxPlotSeries<'_>,
    options: StaticSvgOptions,
) -> Result<String, ChartError> {
    validate_dimensions(options.width, options.height)?;
    validate_plot_area(options)?;
    validate_data_array(series.x, "x")?;
    validate_data_array(series.y, "y")?;
    validate_data_length(series.x.len(), series.y.len(), "x", "y")?;
    if series.x_scale_type == ScaleType::Log {
        validate_positive(series.x, "x")?;
    }
    if series.y_scale_type == ScaleType::Log {
        validate_positive(series.y, "y")?;
    }

    let x_domain = extent_padded(series.x, DEFAULT_PADDING_FRACTION);
    let y_domain = extent_padded(series.y, DEFAULT_PADDING_FRACTION);
    let plot_width = (options.width - options.margin_left - options.margin_right).max(0.0);
    let num_bins = series
        .num_bins
        .unwrap_or_else(|| ((plot_width as f64 / 40.0).max(3.0)) as usize);
    if num_bins == 0 {
        return Err(ChartError::InvalidData {
            field: "bins",
            reason: "boxplot bin count must be at least 1",
        });
    }

    let boxes = calculate_boxplot_bins(series.x, series.y, x_domain, num_bins);
    let x_domain = if series.x_scale_type == ScaleType::Log {
        (x_domain.0.max(1e-10), x_domain.1)
    } else {
        x_domain
    };
    let y_domain = if series.y_scale_type == ScaleType::Log {
        (y_domain.0.max(1e-10), y_domain.1)
    } else {
        y_domain
    };

    let layout = StaticLayout::new(options)?;
    let mut svg = svg_header(options);
    draw_title(&mut svg, title, options.width);
    draw_axes(
        &mut svg,
        options,
        &layout,
        None,
        y_domain,
        series.y_scale_type,
    );

    svg.push_str("<g class=\"gpui-px-boxplot\">\n");
    for stats in boxes {
        let x = map_scaled(
            stats.x,
            x_domain,
            series.x_scale_type,
            layout.plot_left,
            layout.plot_right,
        );
        let half_width = series.box_width.max(1.0) / 2.0;
        let q1 = map_scaled(
            stats.q1,
            y_domain,
            series.y_scale_type,
            layout.plot_bottom,
            layout.plot_top,
        );
        let q2 = map_scaled(
            stats.q2,
            y_domain,
            series.y_scale_type,
            layout.plot_bottom,
            layout.plot_top,
        );
        let q3 = map_scaled(
            stats.q3,
            y_domain,
            series.y_scale_type,
            layout.plot_bottom,
            layout.plot_top,
        );
        let whisker_low = map_scaled(
            stats.whisker_low,
            y_domain,
            series.y_scale_type,
            layout.plot_bottom,
            layout.plot_top,
        );
        let whisker_high = map_scaled(
            stats.whisker_high,
            y_domain,
            series.y_scale_type,
            layout.plot_bottom,
            layout.plot_top,
        );
        let box_top = q3.min(q1);
        let box_height = (q3.max(q1) - box_top).max(1.0);
        let box_x = x - half_width;
        let box_width = half_width * 2.0;
        let cap_left = x - half_width * 0.5;
        let cap_right = x + half_width * 0.5;
        let stroke_width = series.stroke_width.max(0.0);
        let _ = writeln!(
            svg,
            "<line x1=\"{x:.2}\" y1=\"{whisker_low:.2}\" x2=\"{x:.2}\" y2=\"{whisker_high:.2}\" stroke=\"{}\" stroke-width=\"{stroke_width:.2}\"/>",
            hex_color(series.whisker_color)
        );
        let _ = writeln!(
            svg,
            "<line x1=\"{cap_left:.2}\" y1=\"{whisker_low:.2}\" x2=\"{cap_right:.2}\" y2=\"{whisker_low:.2}\" stroke=\"{}\" stroke-width=\"{stroke_width:.2}\"/>",
            hex_color(series.whisker_color)
        );
        let _ = writeln!(
            svg,
            "<line x1=\"{cap_left:.2}\" y1=\"{whisker_high:.2}\" x2=\"{cap_right:.2}\" y2=\"{whisker_high:.2}\" stroke=\"{}\" stroke-width=\"{stroke_width:.2}\"/>",
            hex_color(series.whisker_color)
        );
        let _ = writeln!(
            svg,
            "<rect x=\"{box_x:.2}\" y=\"{box_top:.2}\" width=\"{box_width:.2}\" height=\"{box_height:.2}\" fill=\"{}\" opacity=\"{:.3}\" stroke=\"{}\" stroke-width=\"{stroke_width:.2}\"><title>x {:.3}: q1 {:.3}, median {:.3}, q3 {:.3}</title></rect>",
            hex_color(series.box_color),
            clamp_opacity(series.box_opacity),
            hex_color(series.whisker_color),
            stats.x,
            stats.q1,
            stats.q2,
            stats.q3,
        );
        let _ = writeln!(
            svg,
            "<line x1=\"{box_x:.2}\" y1=\"{q2:.2}\" x2=\"{:.2}\" y2=\"{q2:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\"/>",
            box_x + box_width,
            hex_color(series.median_color),
            (stroke_width * 2.0).max(0.0)
        );

        for outlier in stats
            .outliers_low
            .iter()
            .chain(stats.outliers_high.iter())
            .copied()
        {
            let y = map_scaled(
                outlier,
                y_domain,
                series.y_scale_type,
                layout.plot_bottom,
                layout.plot_top,
            );
            let radius = series.outlier_radius.max(1.0);
            let _ = writeln!(
                svg,
                "<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"{radius:.2}\" fill=\"{}\" opacity=\"0.700\"><title>outlier {outlier:.3}</title></circle>",
                hex_color(series.outlier_color)
            );
        }
    }
    svg.push_str("</g>\n</svg>\n");
    Ok(svg)
}

#[derive(Debug, Clone)]
struct StaticBoxStats {
    x: f64,
    q1: f64,
    q2: f64,
    q3: f64,
    whisker_low: f64,
    whisker_high: f64,
    outliers_low: Vec<f64>,
    outliers_high: Vec<f64>,
}

fn calculate_boxplot_bins(
    x_values: &[f64],
    y_values: &[f64],
    x_domain: (f64, f64),
    num_bins: usize,
) -> Vec<StaticBoxStats> {
    let bin_width = (x_domain.1 - x_domain.0) / num_bins as f64;
    let mut bins = vec![Vec::new(); num_bins];
    for (&x, &y) in x_values.iter().zip(y_values.iter()) {
        let bin_index = if bin_width.abs() < f64::EPSILON {
            0
        } else {
            ((x - x_domain.0) / bin_width).floor() as usize
        };
        bins[bin_index.min(num_bins - 1)].push(y);
    }

    bins.into_iter()
        .enumerate()
        .filter_map(|(index, mut bin)| {
            if bin.is_empty() {
                return None;
            }
            bin.sort_by(|a, b| a.total_cmp(b));
            let x = x_domain.0 + (index as f64 + 0.5) * bin_width;
            static_box_stats_from_sorted(x, &bin)
        })
        .collect()
}

fn static_box_stats_from_sorted(x: f64, values: &[f64]) -> Option<StaticBoxStats> {
    if values.is_empty() {
        return None;
    }

    let q1 = percentile_sorted(values, 0.25);
    let q2 = percentile_sorted(values, 0.5);
    let q3 = percentile_sorted(values, 0.75);
    let iqr = q3 - q1;
    let low_limit = q1 - 1.5 * iqr;
    let high_limit = q3 + 1.5 * iqr;
    let mut whisker_low = f64::INFINITY;
    let mut whisker_high = f64::NEG_INFINITY;
    for &value in values {
        if value >= low_limit && value < whisker_low {
            whisker_low = value;
        }
        if value <= high_limit && value > whisker_high {
            whisker_high = value;
        }
    }
    if whisker_low.is_infinite() {
        whisker_low = values.iter().copied().fold(f64::INFINITY, f64::min);
    }
    if whisker_high.is_infinite() {
        whisker_high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    }

    Some(StaticBoxStats {
        x,
        q1,
        q2,
        q3,
        whisker_low,
        whisker_high,
        outliers_low: values
            .iter()
            .copied()
            .filter(|&value| value < whisker_low)
            .collect(),
        outliers_high: values
            .iter()
            .copied()
            .filter(|&value| value > whisker_high)
            .collect(),
    })
}

fn percentile_sorted(values: &[f64], p: f64) -> f64 {
    if values.len() == 1 {
        return values[0];
    }

    let position = p.clamp(0.0, 1.0) * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        values[lower]
    } else {
        let weight = position - lower as f64;
        values[lower] * (1.0 - weight) + values[upper] * weight
    }
}

fn resolve_heatmap_axis(
    values: Option<&[f64]>,
    expected_len: usize,
    scale_type: ScaleType,
    field: &'static str,
    expected_field: &'static str,
    log_auto_axis_reason: &'static str,
) -> Result<Vec<f64>, ChartError> {
    match values {
        Some(values) => {
            if values.len() != expected_len {
                return Err(ChartError::DataLengthMismatch {
                    x_field: field,
                    y_field: expected_field,
                    x_len: values.len(),
                    y_len: expected_len,
                });
            }
            validate_data_array(values, field)?;
            validate_monotonic(values, field)?;
            if scale_type == ScaleType::Log {
                validate_positive(values, field)?;
            }
            Ok(values.to_vec())
        }
        None => {
            if scale_type == ScaleType::Log {
                return Err(ChartError::InvalidData {
                    field,
                    reason: log_auto_axis_reason,
                });
            }
            Ok((0..expected_len).map(|index| index as f64).collect())
        }
    }
}

fn normalize_value(value: f64, domain: (f64, f64)) -> f64 {
    if domain.0 == domain.1 {
        return 0.5;
    }
    ((value - domain.0) / (domain.1 - domain.0)).clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy)]
enum EitherBaseline<'a> {
    Explicit(&'a [f64]),
    Constant(f64),
}

fn render_xy_svg(
    class_name: &str,
    title: Option<&str>,
    x_scale_type: ScaleType,
    y_scale_type: ScaleType,
    x_range: Option<[f64; 2]>,
    y_range: Option<[f64; 2]>,
    y2_range: Option<[f64; 2]>,
    series: &[StaticXySeries<'_>],
    options: StaticSvgOptions,
) -> Result<String, ChartError> {
    validate_dimensions(options.width, options.height)?;
    validate_plot_area(options)?;

    let x_domain = resolve_xy_domain(
        x_range,
        x_scale_type,
        "x_range",
        series.iter().flat_map(|series| series.x.iter().copied()),
    )?;
    let has_primary_y_series = series.iter().any(|series| !series.use_secondary_y);
    let y_domain = if has_primary_y_series {
        resolve_xy_domain(
            y_range,
            y_scale_type,
            "y_range",
            series
                .iter()
                .filter(|series| !series.use_secondary_y)
                .flat_map(|series| series.y.iter().copied()),
        )?
    } else {
        resolve_xy_domain(
            y2_range.or(y_range),
            y_scale_type,
            "y2_range",
            series.iter().flat_map(|series| series.y.iter().copied()),
        )?
    };
    let y2_domain = if series.iter().any(|series| series.use_secondary_y) {
        Some(resolve_xy_domain(
            y2_range,
            y_scale_type,
            "y2_range",
            series
                .iter()
                .filter(|series| series.use_secondary_y)
                .flat_map(|series| series.y.iter().copied()),
        )?)
    } else {
        None
    };

    let layout = StaticLayout::new(options)?;
    let mut svg = svg_header(options);
    draw_title(&mut svg, title, options.width);
    draw_axes(&mut svg, options, &layout, None, y_domain, y_scale_type);

    let _ = writeln!(svg, "<g class=\"gpui-px-{class_name}\">");
    for chart_series in series {
        let y_domain_for_series = if chart_series.use_secondary_y {
            y2_domain.unwrap_or(y_domain)
        } else {
            y_domain
        };

        if class_name == "line" {
            let points = chart_series
                .x
                .iter()
                .zip(chart_series.y.iter())
                .map(|(&x, &y)| {
                    let sx = map_scaled(
                        x,
                        x_domain,
                        x_scale_type,
                        layout.plot_left,
                        layout.plot_right,
                    );
                    let sy = map_scaled(
                        y,
                        y_domain_for_series,
                        y_scale_type,
                        layout.plot_bottom,
                        layout.plot_top,
                    );
                    format!("{sx:.2},{sy:.2}")
                })
                .collect::<Vec<_>>()
                .join(" ");
            let _ = writeln!(
                svg,
                "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" opacity=\"{:.3}\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>",
                points,
                hex_color(chart_series.color),
                chart_series.stroke_width.max(0.0),
                clamp_opacity(chart_series.opacity),
            );
        }

        if class_name == "scatter" || chart_series.point_radius > 0.0 {
            for (&x, &y) in chart_series.x.iter().zip(chart_series.y.iter()) {
                let sx = map_scaled(
                    x,
                    x_domain,
                    x_scale_type,
                    layout.plot_left,
                    layout.plot_right,
                );
                let sy = map_scaled(
                    y,
                    y_domain_for_series,
                    y_scale_type,
                    layout.plot_bottom,
                    layout.plot_top,
                );
                let radius = chart_series.point_radius.max(1.0);
                let _ = writeln!(
                    svg,
                    "<circle cx=\"{sx:.2}\" cy=\"{sy:.2}\" r=\"{radius:.2}\" fill=\"{}\" opacity=\"{:.3}\"/>",
                    hex_color(chart_series.color),
                    clamp_opacity(chart_series.opacity),
                );
            }
        }
    }
    svg.push_str("</g>\n");

    draw_legend(
        &mut svg,
        series
            .iter()
            .filter_map(|series| series.label.map(|label| (label, series.color, class_name))),
        &layout,
    );
    svg.push_str("</svg>\n");
    Ok(svg)
}

fn validate_xy_series(
    series: &[StaticXySeries<'_>],
    x_scale_type: ScaleType,
    y_scale_type: ScaleType,
) -> Result<(), ChartError> {
    if series.is_empty() {
        return Err(ChartError::EmptyData { field: "x" });
    }

    for (index, chart_series) in series.iter().enumerate() {
        let x_field = if index == 0 { "x" } else { "series.x" };
        let y_field = if index == 0 { "y" } else { "series.y" };
        validate_data_array(chart_series.x, x_field)?;
        validate_data_array(chart_series.y, y_field)?;
        validate_data_length(chart_series.x.len(), chart_series.y.len(), x_field, y_field)?;
        if x_scale_type == ScaleType::Log {
            validate_positive(chart_series.x, x_field)?;
        }
        if y_scale_type == ScaleType::Log {
            validate_positive(chart_series.y, y_field)?;
        }
    }
    Ok(())
}

fn resolve_xy_domain(
    range: Option<[f64; 2]>,
    scale_type: ScaleType,
    field: &'static str,
    values: impl Iterator<Item = f64>,
) -> Result<(f64, f64), ChartError> {
    if let Some([min, max]) = range {
        if scale_type == ScaleType::Log {
            validate_range_log(min, max, field)?;
        } else {
            validate_range(min, max, field)?;
        }
        Ok((min, max))
    } else {
        Ok(auto_xy_domain(scale_type, values))
    }
}

fn auto_xy_domain(scale_type: ScaleType, values: impl Iterator<Item = f64>) -> (f64, f64) {
    if scale_type == ScaleType::Log {
        extent_log_padded_iter(values, DEFAULT_PADDING_FRACTION)
    } else {
        extent_padded_iter(values, DEFAULT_PADDING_FRACTION)
    }
}

#[derive(Debug, Clone, Copy)]
struct StaticLayout {
    plot_left: f32,
    plot_top: f32,
    plot_right: f32,
    plot_bottom: f32,
    plot_width: f32,
}

impl StaticLayout {
    fn new(options: StaticSvgOptions) -> Result<Self, ChartError> {
        let plot_left = options.margin_left;
        let plot_top = options.margin_top;
        let plot_right = options.width - options.margin_right;
        let plot_bottom = options.height - options.margin_bottom;
        if plot_right <= plot_left || plot_bottom <= plot_top {
            return Err(ChartError::InvalidDimension {
                field: "width",
                value: options.width,
            });
        }

        Ok(Self {
            plot_left,
            plot_top,
            plot_right,
            plot_bottom,
            plot_width: plot_right - plot_left,
        })
    }
}

pub(crate) fn validate_plot_area(options: StaticSvgOptions) -> Result<(), ChartError> {
    for (field, value) in [
        ("margin_left", options.margin_left),
        ("margin_right", options.margin_right),
        ("margin_top", options.margin_top),
        ("margin_bottom", options.margin_bottom),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(ChartError::InvalidDimension { field, value });
        }
    }
    Ok(())
}

pub(crate) fn svg_header(options: StaticSvgOptions) -> String {
    let mut svg = String::new();
    let _ = writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.0}\" height=\"{:.0}\" viewBox=\"0 0 {:.0} {:.0}\" role=\"img\">",
        options.width, options.height, options.width, options.height
    );
    if let Some(background) = options.background {
        let _ = writeln!(
            svg,
            "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
            hex_color(background)
        );
    }
    svg
}

pub(crate) fn draw_title(svg: &mut String, title: Option<&str>, width: f32) {
    if let Some(title) = title {
        let _ = writeln!(
            svg,
            "<title>{}</title><text x=\"{:.2}\" y=\"24\" text-anchor=\"middle\" font-family=\"system-ui, sans-serif\" font-size=\"16\" fill=\"#222\">{}</text>",
            escape_xml(title),
            width / 2.0,
            escape_xml(title)
        );
    }
}

fn draw_axes(
    svg: &mut String,
    options: StaticSvgOptions,
    layout: &StaticLayout,
    categories: Option<&[String]>,
    y_domain: (f64, f64),
    y_scale_type: ScaleType,
) {
    if !options.show_axes {
        return;
    }

    let axis_color = "#666";
    let grid_color = "#e6e6e6";
    for step in 0..=4 {
        let t = step as f32 / 4.0;
        let y = layout.plot_bottom + (layout.plot_top - layout.plot_bottom) * t;
        let value = if y_scale_type == ScaleType::Log {
            let min = y_domain.0.log10();
            let max = y_domain.1.log10();
            10_f64.powf(min + (max - min) * t as f64)
        } else {
            y_domain.0 + (y_domain.1 - y_domain.0) * t as f64
        };
        let _ = writeln!(
            svg,
            "<line x1=\"{:.2}\" y1=\"{y:.2}\" x2=\"{:.2}\" y2=\"{y:.2}\" stroke=\"{grid_color}\" stroke-width=\"1\"/>",
            layout.plot_left, layout.plot_right
        );
        let _ = writeln!(
            svg,
            "<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"end\" font-family=\"system-ui, sans-serif\" font-size=\"10\" fill=\"{axis_color}\">{:.3}</text>",
            layout.plot_left - 6.0,
            y + 3.0,
            value
        );
    }

    let _ = writeln!(
        svg,
        "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{axis_color}\" stroke-width=\"1\"/>",
        layout.plot_left, layout.plot_bottom, layout.plot_right, layout.plot_bottom
    );
    let _ = writeln!(
        svg,
        "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{axis_color}\" stroke-width=\"1\"/>",
        layout.plot_left, layout.plot_top, layout.plot_left, layout.plot_bottom
    );

    if let Some(categories) = categories {
        let slot = layout.plot_width / categories.len().max(1) as f32;
        for (index, category) in categories.iter().enumerate() {
            let x = layout.plot_left + slot * (index as f32 + 0.5);
            let _ = writeln!(
                svg,
                "<text x=\"{x:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"system-ui, sans-serif\" font-size=\"10\" fill=\"{axis_color}\">{}</text>",
                layout.plot_bottom + 18.0,
                escape_xml(category)
            );
        }
    }
}

fn draw_heatmap_axes(
    svg: &mut String,
    options: StaticSvgOptions,
    layout: &StaticLayout,
    x_domain: (f64, f64),
    y_domain: (f64, f64),
) {
    if !options.show_axes {
        return;
    }

    let axis_color = "#666";
    let grid_color = "#e6e6e6";
    for step in 0..=4 {
        let t = step as f32 / 4.0;
        let x = layout.plot_left + (layout.plot_right - layout.plot_left) * t;
        let y = layout.plot_bottom + (layout.plot_top - layout.plot_bottom) * t;
        let x_value = x_domain.0 + (x_domain.1 - x_domain.0) * t as f64;
        let y_value = y_domain.0 + (y_domain.1 - y_domain.0) * t as f64;
        let _ = writeln!(
            svg,
            "<line x1=\"{x:.2}\" y1=\"{:.2}\" x2=\"{x:.2}\" y2=\"{:.2}\" stroke=\"{grid_color}\" stroke-width=\"1\"/>",
            layout.plot_top, layout.plot_bottom
        );
        let _ = writeln!(
            svg,
            "<line x1=\"{:.2}\" y1=\"{y:.2}\" x2=\"{:.2}\" y2=\"{y:.2}\" stroke=\"{grid_color}\" stroke-width=\"1\"/>",
            layout.plot_left, layout.plot_right
        );
        let _ = writeln!(
            svg,
            "<text x=\"{x:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-family=\"system-ui, sans-serif\" font-size=\"10\" fill=\"{axis_color}\">{x_value:.3}</text>",
            layout.plot_bottom + 18.0
        );
        let _ = writeln!(
            svg,
            "<text x=\"{:.2}\" y=\"{y:.2}\" text-anchor=\"end\" font-family=\"system-ui, sans-serif\" font-size=\"10\" fill=\"{axis_color}\">{y_value:.3}</text>",
            layout.plot_left - 6.0
        );
    }

    let _ = writeln!(
        svg,
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"{axis_color}\" stroke-width=\"1\"/>",
        layout.plot_left,
        layout.plot_top,
        layout.plot_right - layout.plot_left,
        layout.plot_bottom - layout.plot_top
    );
}

fn draw_legend<'a>(
    svg: &mut String,
    items: impl Iterator<Item = (&'a str, u32, &'a str)>,
    layout: &StaticLayout,
) {
    let mut y = layout.plot_top + 12.0;
    let x = layout.plot_right + 8.0;
    let mut wrote_group = false;
    for (label, color, marker) in items {
        if !wrote_group {
            svg.push_str("<g class=\"gpui-px-legend\">\n");
            wrote_group = true;
        }
        match marker {
            "square" => {
                let _ = writeln!(
                    svg,
                    "<rect x=\"{x:.2}\" y=\"{:.2}\" width=\"10\" height=\"10\" fill=\"{}\"/>",
                    y - 8.0,
                    hex_color(color)
                );
            }
            "line" => {
                let _ = writeln!(
                    svg,
                    "<line x1=\"{x:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"2\"/>",
                    y - 4.0,
                    x + 12.0,
                    y - 4.0,
                    hex_color(color)
                );
            }
            _ => {
                let _ = writeln!(
                    svg,
                    "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"5\" fill=\"{}\"/>",
                    x + 5.0,
                    y - 4.0,
                    hex_color(color)
                );
            }
        }
        let _ = writeln!(
            svg,
            "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"system-ui, sans-serif\" font-size=\"11\" fill=\"#333\">{}</text>",
            x + 16.0,
            y,
            escape_xml(label)
        );
        y += 16.0;
    }
    if wrote_group {
        svg.push_str("</g>\n");
    }
}

fn map_scaled(
    value: f64,
    domain: (f64, f64),
    scale_type: ScaleType,
    range_start: f32,
    range_end: f32,
) -> f32 {
    match scale_type {
        ScaleType::Linear => map_linear(value, domain.0, domain.1, range_start, range_end),
        ScaleType::Log => map_linear(
            value.log10(),
            domain.0.log10(),
            domain.1.log10(),
            range_start,
            range_end,
        ),
    }
}

fn map_linear(
    value: f64,
    domain_min: f64,
    domain_max: f64,
    range_start: f32,
    range_end: f32,
) -> f32 {
    if domain_max == domain_min {
        return (range_start + range_end) / 2.0;
    }
    let t = ((value - domain_min) / (domain_max - domain_min)).clamp(0.0, 1.0) as f32;
    range_start + (range_end - range_start) * t
}

fn clamp_opacity(opacity: f32) -> f32 {
    opacity.clamp(0.0, 1.0)
}

fn hex_color(color: u32) -> String {
    format!("#{:06x}", color & 0x00ff_ffff)
}

fn color_at(index: usize, palette: Option<&[u32]>) -> u32 {
    let palette = palette.unwrap_or(crate::pie::DEFAULT_PALETTE.as_slice());
    palette[index % palette.len()]
}

pub(crate) fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChartError, ColorScale, ScaleType, area, bar, boxplot, donut, heatmap, line, pie, scatter,
    };

    #[test]
    fn static_svg_options_default_has_positive_plot_area() {
        let options = StaticSvgOptions::default();
        assert!(StaticLayout::new(options).is_ok());
    }

    #[test]
    fn line_static_export_writes_svg_polyline_and_legend() {
        let x = [1.0, 2.0, 3.0];
        let y = [2.0, 4.0, 3.0];
        let y2 = [1.0, 3.0, 2.0];

        let svg = line(&x, &y)
            .title("A <line> & chart")
            .label("primary")
            .show_points(true)
            .add_series(&y2, Some("secondary"), 0xff7f0e, 2.0, 0.75)
            .to_svg()
            .expect("line svg export should succeed");

        assert!(svg.starts_with("<svg "));
        assert!(svg.contains("A &lt;line&gt; &amp; chart"));
        assert!(svg.contains("<polyline"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("secondary"));
    }

    #[test]
    fn line_static_export_supports_secondary_y_when_primary_is_hidden() {
        let x = [1.0, 2.0, 3.0];
        let y = [2.0, 4.0, 3.0];
        let y2 = [10.0, 30.0, 20.0];

        let svg = line(&x, &y)
            .add_series_y2(&y2, Some("right"), 0xff7f0e, 2.0, 1.0)
            .hidden_series(&[0])
            .to_svg()
            .expect("secondary-only visible line svg export should succeed");

        assert!(svg.contains("<polyline"));
        assert!(svg.contains("right"));
        assert!(!svg.contains("inf"));
    }

    #[test]
    fn scatter_static_export_writes_svg_circles() {
        let x = [1.0, 2.0, 3.0];
        let y = [2.0, 4.0, 3.0];

        let svg = scatter(&x, &y)
            .title("points")
            .label("samples")
            .to_svg_with_options(StaticSvgOptions::new(360.0, 240.0))
            .expect("scatter svg export should succeed");

        assert!(svg.contains("width=\"360\""));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("samples"));
    }

    #[test]
    fn bar_static_export_writes_grouped_rects_and_categories() {
        let categories = ["Q1", "Q2"];
        let current = [10.0, 20.0];
        let previous = [8.0, 16.0];

        let svg = bar(&categories, &current)
            .label("current")
            .add_series(&previous, Some("previous"), 0xff7f0e, 0.8)
            .to_svg()
            .expect("bar svg export should succeed");

        assert!(svg.contains("<rect"));
        assert!(svg.contains("Q1"));
        assert!(svg.contains("previous"));
    }

    #[test]
    fn area_static_export_writes_closed_path() {
        let x = [1.0, 2.0, 3.0];
        let y = [2.0, 4.0, 3.0];

        let svg = area(&x, &y)
            .title("area export")
            .color(0x2ca02c)
            .opacity(0.5)
            .to_svg_with_options(StaticSvgOptions::new(420.0, 240.0))
            .expect("area svg export should succeed");

        assert!(svg.contains("width=\"420\""));
        assert!(svg.contains("area export"));
        assert!(svg.contains("class=\"gpui-px-area\""));
        assert!(svg.contains("<path"));
        assert!(svg.contains("Z\""));
        assert!(svg.contains("fill=\"#2ca02c\""));
        assert!(svg.contains("opacity=\"0.500\""));
    }

    #[test]
    fn area_static_export_supports_explicit_baseline() {
        let x = [1.0, 2.0, 3.0];
        let y = [2.0, 4.0, 3.0];
        let y0 = [1.0, 2.0, 1.5];

        let svg = area(&x, &y)
            .y0(&y0)
            .to_svg()
            .expect("area svg export with baseline should succeed");

        assert!(svg.contains("class=\"gpui-px-area\""));
        assert!(!svg.contains("inf"));
        assert!(!svg.contains("NaN"));
    }

    #[test]
    fn static_export_auto_log_domains_stay_finite() {
        let x = [1.0, 2.0];
        let y = [0.001, 1.0];

        for svg in [
            line(&x, &y)
                .y_scale(ScaleType::Log)
                .to_svg()
                .expect("line SVG with an automatic log domain should export"),
            scatter(&x, &y)
                .y_scale(ScaleType::Log)
                .to_svg()
                .expect("scatter SVG with an automatic log domain should export"),
            area(&x, &y)
                .y_scale(ScaleType::Log)
                .to_svg()
                .expect("area SVG with an automatic log domain should export"),
        ] {
            assert!(!svg.contains("NaN"));
            assert!(!svg.contains("inf"));
        }
    }

    #[test]
    fn pie_static_export_writes_slice_paths_labels_and_legend() {
        let values = [10.0, 20.0, 30.0];

        let svg = pie(&values)
            .title("pie <export>")
            .labels(&["Alpha", "Beta", "Gamma"])
            .sort(false)
            .to_svg_with_options(StaticSvgOptions::new(360.0, 320.0))
            .expect("pie svg export should succeed");

        assert!(svg.contains("width=\"360\""));
        assert!(svg.contains("pie &lt;export&gt;"));
        assert!(svg.contains("class=\"gpui-px-pie\""));
        assert_eq!(svg.matches("<path").count(), 3);
        assert!(svg.contains("Alpha: 10.000"));
        assert!(svg.contains("Beta"));
        assert!(svg.contains("Gamma"));
        assert!(svg.contains("gpui-px-legend"));
    }

    #[test]
    fn donut_static_export_supports_custom_colors_and_holes() {
        let values = [10.0, 20.0];

        let svg = donut(&values)
            .colors(&[0xff0000, 0x00ff00])
            .pad_angle(0.02)
            .corner_radius(2.0)
            .to_svg()
            .expect("donut svg export should succeed");

        assert!(svg.contains("class=\"gpui-px-pie\""));
        assert!(svg.contains("fill=\"#ff0000\""));
        assert!(svg.contains("fill=\"#00ff00\""));
        assert!(!svg.contains("NaN"));
        assert!(!svg.contains("inf"));
    }

    #[test]
    fn heatmap_static_export_writes_grid_rects_and_title() {
        let z = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

        let svg = heatmap(&z, 3, 2)
            .title("heat <map>")
            .color_scale(ColorScale::Inferno)
            .opacity(0.75)
            .to_svg_with_options(StaticSvgOptions::new(420.0, 260.0))
            .expect("heatmap svg export should succeed");

        assert!(svg.contains("width=\"420\""));
        assert!(svg.contains("heat &lt;map&gt;"));
        assert!(svg.contains("class=\"gpui-px-heatmap\""));
        assert_eq!(svg.matches("<rect").count(), 8);
        assert!(svg.contains("row 0, column 0: 1.000"));
        assert!(svg.contains("opacity=\"0.750\""));
    }

    #[test]
    fn heatmap_static_export_supports_custom_log_axes_and_ranges() {
        let z = [1.0, 2.0, 3.0, 4.0];
        let x = [10.0, 100.0];
        let y = [1.0, 10.0];

        let svg = heatmap(&z, 2, 2)
            .x(&x)
            .y(&y)
            .x_scale(ScaleType::Log)
            .y_scale(ScaleType::Log)
            .x_range(10.0, 100.0)
            .y_range(1.0, 10.0)
            .to_svg()
            .expect("heatmap svg export with log axes should succeed");

        assert!(svg.contains("class=\"gpui-px-heatmap\""));
        assert!(!svg.contains("NaN"));
        assert!(!svg.contains("inf"));
    }

    #[test]
    fn boxplot_static_export_writes_boxes_medians_and_outliers() {
        let x = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let y = [1.0, 2.0, 3.0, 4.0, 5.0, 100.0];

        let svg = boxplot(&x, &y)
            .title("box <plot>")
            .bins(1)
            .box_color(0xabcdef)
            .median_color(0x111111)
            .outlier_color(0xff0000)
            .to_svg_with_options(StaticSvgOptions::new(420.0, 260.0))
            .expect("boxplot svg export should succeed");

        assert!(svg.contains("width=\"420\""));
        assert!(svg.contains("box &lt;plot&gt;"));
        assert!(svg.contains("class=\"gpui-px-boxplot\""));
        assert!(svg.contains("fill=\"#abcdef\""));
        assert!(svg.contains("stroke=\"#111111\""));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("outlier 100.000"));
    }

    #[test]
    fn boxplot_static_export_supports_log_scales() {
        let x = [1.0, 10.0, 100.0, 1000.0];
        let y = [1.0, 2.0, 4.0, 8.0];

        let svg = boxplot(&x, &y)
            .x_scale(ScaleType::Log)
            .y_scale(ScaleType::Log)
            .bins(2)
            .to_svg()
            .expect("boxplot svg export with log scales should succeed");

        assert!(svg.contains("class=\"gpui-px-boxplot\""));
        assert!(!svg.contains("NaN"));
        assert!(!svg.contains("inf"));
    }

    #[test]
    fn static_export_preserves_chart_validation_errors() {
        let result = scatter(&[1.0, 2.0], &[1.0, f64::NAN]).to_svg();

        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "y",
                reason: "contains NaN or Infinity"
            })
        ));
    }

    #[test]
    fn heatmap_static_export_preserves_axis_validation_errors() {
        let z = [1.0; 4];
        let result = heatmap(&z, 2, 2).x(&[2.0, 1.0]).to_svg();

        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "must be strictly monotonically increasing"
            })
        ));
    }

    #[test]
    fn heatmap_static_export_requires_explicit_log_axes() {
        let z = [1.0; 4];
        let result = heatmap(&z, 2, 2).x_scale(ScaleType::Log).to_svg();

        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "log scale requires explicit positive x values"
            })
        ));
    }

    #[test]
    fn boxplot_static_export_preserves_zero_bin_validation_errors() {
        let result = boxplot(&[1.0, 2.0], &[3.0, 4.0]).bins(0).to_svg();

        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "bins",
                reason: "boxplot bin count must be at least 1"
            })
        ));
    }

    #[test]
    fn area_static_export_preserves_baseline_validation_errors() {
        let result = area(&[1.0, 2.0], &[2.0, 3.0]).y0(&[1.0]).to_svg();

        assert!(matches!(
            result,
            Err(ChartError::DataLengthMismatch {
                x_field: "x",
                y_field: "y0",
                x_len: 2,
                y_len: 1
            })
        ));
    }

    #[test]
    fn pie_static_export_preserves_label_validation_errors() {
        let result = pie(&[10.0, 20.0, 30.0]).labels(&["A", "B"]).to_svg();

        assert!(matches!(
            result,
            Err(ChartError::DataLengthMismatch {
                x_field: "labels",
                y_field: "values",
                x_len: 2,
                y_len: 3
            })
        ));
    }

    #[test]
    fn pie_static_export_rejects_non_positive_totals() {
        let result = pie(&[0.0, 0.0]).to_svg();

        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "values",
                reason: "pie chart values must sum to a positive number"
            })
        ));
    }

    #[test]
    fn static_export_validates_log_scale_ranges() {
        let result = bar(&["A", "B"], &[0.0, 2.0])
            .y_scale(ScaleType::Log)
            .to_svg();

        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "values",
                reason: "contains non-positive values for log scale"
            })
        ));
    }
}
