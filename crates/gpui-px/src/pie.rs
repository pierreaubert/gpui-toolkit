//! Pie chart - Plotly Express style API.

use crate::error::ChartError;
use crate::{
    DEFAULT_HEIGHT, DEFAULT_TITLE_FONT_SIZE, DEFAULT_WIDTH, TITLE_AREA_HEIGHT, validate_data_array,
    validate_data_length, validate_dimensions,
};
use d3rs::color::D3Color;
use d3rs::shape::{Arc, Pie};
use d3rs::text::{VectorFontConfig, render_vector_text};
use gpui::prelude::*;
use gpui::{IntoElement, PathBuilder, canvas, div, hsla, point, px};

/// Default color palette (Plotly)
const DEFAULT_PALETTE: [u32; 10] = [
    0x1f77b4, 0xff7f0e, 0x2ca02c, 0xd62728, 0x9467bd, 0x8c564b, 0xe377c2, 0x7f7f7f, 0xbcbd22,
    0x17becf,
];

/// Pie chart builder.
#[derive(Clone)]
pub struct PieChart {
    labels: Option<Vec<String>>,
    values: Vec<f64>,
    title: Option<String>,
    inner_radius_fraction: f64, // 0.0 to 1.0 of outer radius
    pad_angle: f64,
    corner_radius: f64,
    colors: Option<Vec<u32>>,
    width: f32,
    height: f32,
    sort: bool,
}

impl PieChart {
    /// Set chart title (rendered at top of chart).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set custom colors for slices.
    pub fn colors(mut self, colors: &[u32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }

    /// Set hole size fraction (0.0 to 1.0).
    /// 0.0 = full pie, 0.5 = donut with hole half the radius.
    pub fn hole(mut self, fraction: f64) -> Self {
        self.inner_radius_fraction = fraction.clamp(0.0, 0.99);
        self
    }

    /// Set padding angle between slices (in radians).
    pub fn pad_angle(mut self, angle: f64) -> Self {
        self.pad_angle = angle;
        self
    }

    /// Set corner radius for slices.
    pub fn corner_radius(mut self, radius: f64) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Sort slices by value (descending). Default is true.
    pub fn sort(mut self, sort: bool) -> Self {
        self.sort = sort;
        self
    }

    /// Set chart dimensions.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Build and validate the chart, returning renderable element.
    pub fn build(self) -> Result<impl IntoElement, ChartError> {
        // Validate inputs
        validate_data_array(&self.values, "values")?;
        validate_dimensions(self.width, self.height)?;

        if let Some(ref labels) = self.labels {
            validate_data_length(labels.len(), self.values.len(), "labels", "values")?;
        }

        // Calculate plot area
        let title_height = if self.title.is_some() {
            TITLE_AREA_HEIGHT
        } else {
            0.0
        };
        let plot_height = self.height - title_height;
        let plot_width = self.width;

        // Calculate radius
        let radius = (plot_width.min(plot_height) / 2.0) as f64 * 0.9; // 90% fit
        let inner_radius = radius * self.inner_radius_fraction;

        // Prepare pie generator
        let pie = Pie::new()
            .pad_angle(self.pad_angle)
            .corner_radius(self.corner_radius)
            .inner_radius(inner_radius)
            .outer_radius(radius)
            .sort(self.sort);

        // Generate slices
        let slices = pie.generate(&self.values, |v| *v);

        // Determine colors
        let colors: Vec<u32> = match self.colors {
            Some(c) => c.iter().cycle().take(slices.len()).copied().collect(),
            None => DEFAULT_PALETTE
                .iter()
                .cycle()
                .take(slices.len())
                .copied()
                .collect(),
        };

        // Create arc generator
        let arc_gen = Arc::new();

        // Render function
        let render_element = canvas(
            move |bounds, _, _| (slices, colors, arc_gen, bounds, plot_width, plot_height),
            move |_, (slices, colors, arc_gen, bounds, plot_width, plot_height), window, _| {
                let origin_x: f32 = bounds.origin.x.into();
                let origin_y: f32 = bounds.origin.y.into();
                let center_x = origin_x + plot_width / 2.0;
                let center_y = origin_y + plot_height / 2.0;

                let arc_gen = arc_gen.center(center_x as f64, center_y as f64);

                for (i, slice) in slices.iter().enumerate() {
                    let color = D3Color::from_hex(colors[i % colors.len()]);
                    let fill_color = color.to_rgba();

                    let path = arc_gen.generate(&slice.arc);
                    let points = path.flatten(0.5);

                    if points.is_empty() {
                        continue;
                    }

                    let mut builder = PathBuilder::fill();

                    builder.move_to(point(px(points[0].x as f32), px(points[0].y as f32)));
                    for p in points.iter().skip(1) {
                        builder.line_to(point(px(p.x as f32), px(p.y as f32)));
                    }

                    builder.close();

                    if let Ok(gpui_path) = builder.build() {
                        window.paint_path(gpui_path, fill_color);
                    }
                }
            },
        );

        // Build container
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

        // Add plot area
        container = container.child(
            div()
                .w(px(self.width))
                .h(px(plot_height))
                .relative()
                .child(render_element),
        );

        Ok(container)
    }
}

/// Create a pie chart from values.
///
/// # Example
///
/// ```rust,no_run
/// use gpui_px::pie;
///
/// let values = vec![10.0, 20.0, 30.0, 40.0];
/// let labels = vec!["A", "B", "C", "D"];
///
/// let chart = pie(&values)
///     .labels(&labels)
///     .title("My Pie Chart")
///     .build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
pub fn pie(values: &[f64]) -> PieChart {
    PieChart {
        labels: None,
        values: values.to_vec(),
        title: None,
        inner_radius_fraction: 0.0,
        pad_angle: 0.0,
        corner_radius: 0.0,
        colors: None,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        sort: true,
    }
}

impl PieChart {
    /// Set labels for slices (used for tooltips/legend - currently unused).
    pub fn labels(mut self, labels: &[impl ToString]) -> Self {
        self.labels = Some(labels.iter().map(|l| l.to_string()).collect());
        self
    }
}

/// Create a donut chart from values (shorthand for pie with hole).
///
/// # Example
///
/// ```rust,no_run
/// use gpui_px::donut;
///
/// let values = vec![10.0, 20.0, 30.0];
/// let chart = donut(&values).title("My Donut").build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
pub fn donut(values: &[f64]) -> PieChart {
    pie(values).hole(0.5)
}
