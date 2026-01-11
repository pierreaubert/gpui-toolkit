//! gpui-px Showcase - Demonstrates all chart types with the Plotly Express-style API.
//!
//! This showcase demonstrates the high-level gpui-px charting API built on top of d3rs.
//! Navigate through sections using the sidebar to see examples of each chart type.

use gpui::*;
use gpui_px::*;
use gpui_px::interaction::{InteractiveChart, InteractiveChartConfig, InteractiveChartState};
use gpui_ui_kit::{MiniApp, MiniAppConfig};

fn main() {
    MiniApp::run(
        MiniAppConfig::new("gpui-px Showcase").size(1200.0, 800.0),
        |cx| cx.new(ShowcaseApp::new),
    );
}

// ============================================================================
// Demo Sections
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ChartSection {
    #[default]
    Overview,
    Scatter,
    Line,
    Bar,
    BoxPlot,
    LogScales,
    Heatmap,
    Contour,
    Isoline,
    Treemap,
    Gallery,
}

impl ChartSection {
    fn all() -> &'static [ChartSection] {
        &[
            ChartSection::Overview,
            ChartSection::Scatter,
            ChartSection::Line,
            ChartSection::Bar,
            ChartSection::BoxPlot,
            ChartSection::LogScales,
            ChartSection::Heatmap,
            ChartSection::Contour,
            ChartSection::Isoline,
            ChartSection::Treemap,
            ChartSection::Gallery,
        ]
    }

    fn label(&self) -> &'static str {
        match self {
            ChartSection::Overview => "Overview",
            ChartSection::Scatter => "Scatter",
            ChartSection::Line => "Line",
            ChartSection::Bar => "Bar",
            ChartSection::BoxPlot => "Box Plot",
            ChartSection::LogScales => "Log Scales",
            ChartSection::Heatmap => "Heatmap",
            ChartSection::Contour => "Contour",
            ChartSection::Isoline => "Isoline",
            ChartSection::Treemap => "Treemap",
            ChartSection::Gallery => "Gallery",
        }
    }
}

// ============================================================================
// Showcase Application
// ============================================================================

struct ShowcaseApp {
    current_section: ChartSection,
    // Demo data
    scatter_x: Vec<f64>,
    scatter_y: Vec<f64>,
    line_x: Vec<f64>,
    line_y: Vec<f64>,
    bar_categories: Vec<String>,
    bar_values: Vec<f64>,
    boxplot_x: Vec<f64>,
    boxplot_y: Vec<f64>,
    heatmap_z: Vec<f64>,
    heatmap_size: usize,
    contour_z: Vec<f64>,
    contour_size: usize,
    // Interactive state
    heatmap_color_scale: ColorScale,
    contour_color_scale: ColorScale,
    // Interactive chart states for pan/zoom
    scatter_chart_state: InteractiveChartState,
    line_chart_state: InteractiveChartState,
}

impl ShowcaseApp {
    fn new(_cx: &mut Context<Self>) -> Self {
        let (scatter_x, scatter_y) = generate_scatter_data();
        let (line_x, line_y) = generate_line_data();
        let (bar_categories, bar_values) = generate_bar_data();
        let (boxplot_x, boxplot_y) = generate_boxplot_data();
        let heatmap_size = 30;
        let heatmap_z = generate_grid_data(heatmap_size);
        let contour_size = 50;
        let contour_z = generate_grid_data(contour_size);

        // Create interactive chart states with appropriate domains
        let config = InteractiveChartConfig::new()
            .with_left_margin(50.0)
            .with_top_margin(30.0);

        // Calculate actual data ranges for scatter chart (with padding)
        let scatter_x_min = scatter_x.iter().cloned().fold(f64::INFINITY, f64::min);
        let scatter_x_max = scatter_x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let scatter_y_min = scatter_y.iter().cloned().fold(f64::INFINITY, f64::min);
        let scatter_y_max = scatter_y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let scatter_x_pad = (scatter_x_max - scatter_x_min) * 0.05;
        let scatter_y_pad = (scatter_y_max - scatter_y_min) * 0.05;

        // Calculate actual data ranges for line chart (with padding)
        let line_x_min = line_x.iter().cloned().fold(f64::INFINITY, f64::min);
        let line_x_max = line_x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let line_y_min = line_y.iter().cloned().fold(f64::INFINITY, f64::min);
        let line_y_max = line_y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let line_x_pad = (line_x_max - line_x_min) * 0.05;
        let line_y_pad = (line_y_max - line_y_min) * 0.05;

        Self {
            current_section: ChartSection::default(),
            scatter_x: scatter_x.clone(),
            scatter_y: scatter_y.clone(),
            line_x: line_x.clone(),
            line_y: line_y.clone(),
            bar_categories,
            bar_values,
            boxplot_x,
            boxplot_y,
            heatmap_z,
            heatmap_size,
            contour_z,
            contour_size,
            heatmap_color_scale: ColorScale::Viridis,
            contour_color_scale: ColorScale::Viridis,
            // Interactive chart states - domains based on actual data ranges
            scatter_chart_state: InteractiveChartState::new(
                scatter_x_min - scatter_x_pad,
                scatter_x_max + scatter_x_pad,
                scatter_y_min - scatter_y_pad,
                scatter_y_max + scatter_y_pad,
            )
            .with_size(600.0, 400.0)
            .with_config(config.clone()),
            line_chart_state: InteractiveChartState::new(
                line_x_min - line_x_pad,
                line_x_max + line_x_pad,
                line_y_min - line_y_pad,
                line_y_max + line_y_pad,
            )
            .with_size(600.0, 400.0)
            .with_config(config),
        }
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.current_section;

        div()
            .w(px(200.0))
            .h_full()
            .bg(rgb(0x1e1e1e))
            .border_r_1()
            .border_color(rgb(0x3c3c3c))
            .flex()
            .flex_col()
            .p_4()
            .gap_2()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xffffff))
                    .mb_4()
                    .child("gpui-px"),
            )
            .children(ChartSection::all().iter().map(|&section| {
                let is_selected = section == current;
                let bg = if is_selected {
                    rgb(0x007acc)
                } else {
                    rgb(0x1e1e1e)
                };
                let hover_bg = if is_selected {
                    rgb(0x007acc)
                } else {
                    rgb(0x2d2d2d)
                };

                div()
                    .id(ElementId::Name(section.label().into()))
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(bg)
                    .hover(|s| s.bg(hover_bg))
                    .text_color(rgb(0xffffff))
                    .child(section.label())
                    .on_click(cx.listener(move |this, _, _window, _cx| {
                        this.current_section = section;
                    }))
            }))
    }

    fn render_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let content: Div = match self.current_section {
            ChartSection::Overview => self.render_overview(),
            ChartSection::Scatter => self.render_scatter_demo(),
            ChartSection::Line => self.render_line_demo(),
            ChartSection::Bar => self.render_bar_demo(),
            ChartSection::BoxPlot => self.render_boxplot_demo(),
            ChartSection::LogScales => self.render_logscales_demo(),
            ChartSection::Heatmap => self.render_heatmap_demo(cx),
            ChartSection::Contour => self.render_contour_demo(cx),
            ChartSection::Isoline => self.render_isoline_demo(),
            ChartSection::Treemap => self.render_treemap_demo(),
            ChartSection::Gallery => self.render_gallery(),
        };

        div()
            .id("content-scroll")
            .flex_1()
            .h_full()
            .overflow_y_scroll()
            .bg(rgb(0xffffff))
            .p_8()
            .child(content)
    }

    // ========================================================================
    // Overview Section
    // ========================================================================

    fn render_overview(&self) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Welcome to gpui-px"),
            )
            .child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .max_w(px(600.0))
                    .child("gpui-px is a high-level Plotly Express-style charting API built on top of d3rs. Create beautiful charts in just a few lines of code."),
            )
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Chart Types"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .ml_4()
                    .child(self.feature_item("Scatter", "Individual data points with x,y coordinates"))
                    .child(self.feature_item("Line", "Time series and trends with connected points"))
                    .child(self.feature_item("Bar", "Categorical data comparisons"))
                    .child(self.feature_item("Heatmap", "2D scalar fields with color scales"))
                    .child(self.feature_item("Contour", "Filled bands between thresholds"))
                    .child(self.feature_item("Isoline", "Unfilled contour lines at specific levels"))
                    .child(self.feature_item("Treemap", "Hierarchical data as nested rectangles")),
            )
            .child(
                div()
                    .mt_6()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Quick Example"),
            )
            .child(self.code_block(
                "// Create a scatter plot in 3 lines\nlet chart = scatter(&x, &y)\n    .title(\"My Chart\")\n    .build()?;",
            ))
            .child(
                div()
                    .mt_6()
                    .p_4()
                    .bg(rgb(0xf5f5f5))
                    .rounded_md()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child("Use the sidebar to explore each chart type"),
                    ),
            )
    }

    fn feature_item(&self, title: &str, desc: &str) -> Div {
        div()
            .flex()
            .gap_2()
            .child(
                div()
                    .text_color(rgb(0x007acc))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!("{title}:")),
            )
            .child(div().text_color(rgb(0x666666)).child(desc.to_string()))
    }

    fn code_block(&self, code: &str) -> Div {
        div().p_4().bg(rgb(0x2d2d2d)).rounded_md().child(
            div()
                .text_sm()
                .font_family("Monaco")
                .text_color(rgb(0x9cdcfe))
                .whitespace_nowrap()
                .child(code.to_string()),
        )
    }

    // ========================================================================
    // Scatter Section
    // ========================================================================

    fn render_scatter_demo(&self) -> Div {
        // Get zoom state
        let is_zoomed = self.scatter_chart_state.is_zoomed();

        let mut chart_builder = scatter(&self.scatter_x, &self.scatter_y)
            .title(format!(
                "Spiral Pattern{}",
                if is_zoomed { " (zoomed)" } else { "" }
            ))
            .color(0x1f77b4)
            .point_radius(5.0)
            .size(600.0, 400.0);

        // Only set explicit ranges when zoomed
        if is_zoomed {
            let (x_min, x_max) = self.scatter_chart_state.x_domain();
            let (y_min, y_max) = self.scatter_chart_state.y_domain();
            chart_builder = chart_builder.x_range(x_min, x_max).y_range(y_min, y_max);
        }

        let chart = chart_builder.build().unwrap();

        let interactive_chart =
            InteractiveChart::new("scatter-chart", chart, self.scatter_chart_state.clone()).build();

        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Scatter Plot"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .max_w(px(600.0))
                    .child("Displays individual data points with x,y coordinates. Ideal for exploring correlations, identifying clusters, and spotting outliers."),
            )
            .child(interactive_chart)
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x888888))
                    .child("Drag to pan • Scroll to zoom • Double-click to reset"),
            )
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Code"),
            )
            .child(self.code_block(
                "scatter(&x, &y)\n    .title(\"My Scatter\")\n    .color(0x1f77b4)\n    .point_radius(5.0)\n    .size(600.0, 400.0)\n    .build()?",
            ))
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Features"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .ml_4()
                    .child(div().text_sm().child("• Automatic axis scaling with padding"))
                    .child(div().text_sm().child("• Custom colors via 0xRRGGBB hex values"))
                    .child(div().text_sm().child("• Adjustable point radius and opacity"))
                    .child(div().text_sm().child("• Title rendering at top of chart")),
            )
    }

    // ========================================================================
    // Line Section
    // ========================================================================

    fn render_line_demo(&self) -> Div {
        // Get zoom state
        let is_zoomed = self.line_chart_state.is_zoomed();

        let mut chart_builder = line(&self.line_x, &self.line_y)
            .title(format!(
                "Sine Wave{}",
                if is_zoomed { " (zoomed)" } else { "" }
            ))
            .color(0xff7f0e)
            .stroke_width(2.0)
            .size(600.0, 400.0);

        // Only set explicit ranges when zoomed
        if is_zoomed {
            let (x_min, x_max) = self.line_chart_state.x_domain();
            let (y_min, y_max) = self.line_chart_state.y_domain();
            chart_builder = chart_builder.x_range(x_min, x_max).y_range(y_min, y_max);
        }

        let chart = chart_builder.build().unwrap();

        let interactive_chart =
            InteractiveChart::new("line-chart", chart, self.line_chart_state.clone()).build();

        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Line Chart"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .max_w(px(600.0))
                    .child("Connects data points with lines to show trends over continuous domains. Perfect for time series, measurements, and sequential data."),
            )
            .child(interactive_chart)
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x888888))
                    .child("Drag to pan • Scroll to zoom • Double-click to reset"),
            )
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Code"),
            )
            .child(self.code_block(
                "line(&x, &y)\n    .title(\"Sine Wave\")\n    .color(0xff7f0e)\n    .stroke_width(2.0)\n    .size(600.0, 400.0)\n    .build()?",
            ))
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Features"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .ml_4()
                    .child(div().text_sm().child("• Smooth curve interpolation options"))
                    .child(div().text_sm().child("• Configurable stroke width"))
                    .child(div().text_sm().child("• Optional data point markers"))
                    .child(div().text_sm().child("• Automatic domain calculation")),
            )
    }

    // ========================================================================
    // Bar Section
    // ========================================================================

    fn render_bar_demo(&self) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Bar Chart"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .max_w(px(600.0))
                    .child("Compares values across discrete categories. Great for showing rankings, counts, and distributions."),
            )
            .child({
                bar(&self.bar_categories, &self.bar_values)
                    .title("Weekly Activity")
                    .color(0x2ca02c)
                    .size(600.0, 400.0)
                    .build()
                    .unwrap()
            })
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Code"),
            )
            .child(self.code_block(
                "bar(&categories, &values)\n    .title(\"Weekly Activity\")\n    .color(0x2ca02c)\n    .size(600.0, 400.0)\n    .build()?",
            ))
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Features"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .ml_4()
                    .child(div().text_sm().child("• Category labels on x-axis"))
                    .child(div().text_sm().child("• Automatic bar width calculation"))
                    .child(div().text_sm().child("• Custom colors and opacity"))
                    .child(div().text_sm().child("• Support for negative values")),
            )
    }

    // ========================================================================
    // Box Plot Section
    // ========================================================================

    fn render_boxplot_demo(&self) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Box Plot"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .max_w(px(600.0))
                    .child("Displays distribution of data based on quartiles. Shows median, interquartile range (IQR), whiskers extending to 1.5×IQR, and outliers as individual points."),
            )
            .child({
                boxplot(&self.boxplot_x, &self.boxplot_y)
                    .title("Morley Speed of Light Experiment")
                    .box_color(0xdddddd)
                    .median_color(0x000000)
                    .whisker_color(0x333333)
                    .outlier_color(0xd62728)
                    .box_width(30.0)
                    .size(600.0, 400.0)
                    .build()
                    .unwrap()
            })
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Code"),
            )
            .child(self.code_block(
                "boxplot(&x, &y)\n    .title(\"Distribution\")\n    .box_color(0xdddddd)\n    .median_color(0x000000)\n    .size(600.0, 400.0)\n    .build()?",
            ))
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Features"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .ml_4()
                    .child(div().text_sm().child("• Automatic quartile calculation (Q1, Q2, Q3)"))
                    .child(div().text_sm().child("• Whiskers extend to 1.5×IQR from box"))
                    .child(div().text_sm().child("• Outliers displayed as individual points"))
                    .child(div().text_sm().child("• Customizable colors for box, median, whiskers, outliers"))
                    .child(div().text_sm().child("• Support for log scale axes")),
            )
    }

    // ========================================================================
    // Log Scales Section
    // ========================================================================

    fn render_logscales_demo(&self) -> Div {
        // Generate logarithmic data - power law relationship y = x^0.8
        // More data points spanning 4 decades (10 to 100,000)
        let log_x: Vec<f64> = (0..50)
            .map(|i| 10.0 * 10_f64.powf(i as f64 / 12.5)) // 10 to ~100,000
            .collect();
        let log_y: Vec<f64> = log_x.iter().map(|&x| x.powf(0.8)).collect();

        let freq_x: Vec<f64> = (0..50)
            .map(|i| 20.0 * 10_f64.powf(i as f64 / 15.0))
            .collect();
        let freq_y: Vec<f64> = freq_x
            .iter()
            .map(|&f| {
                // Simulated frequency response
                if f < 100.0 {
                    -12.0 * (100.0 - f) / 80.0
                } else if f > 5000.0 {
                    -6.0 * (f - 5000.0) / 15000.0
                } else {
                    0.0
                }
            })
            .collect();

        let bar_cats: Vec<&str> = vec!["10", "100", "1K", "10K", "100K"];
        let bar_vals: Vec<f64> = vec![10.0, 100.0, 1000.0, 10000.0, 100000.0];

        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Logarithmic Scales"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .max_w(px(600.0))
                    .child("Logarithmic scales are essential for visualizing data spanning multiple orders of magnitude, such as frequency responses (20 Hz to 20 kHz), power measurements, and exponential growth."),
            )
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Scatter Plot - Log-Log Scale"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child("Power-law relationship y = x^0.8 - appears as a straight line on log-log axes"),
            )
            .child({
                scatter(&log_x, &log_y)
                    .title("Power Law: y = x^0.8")
                    .color(0xe377c2)
                    .x_scale(ScaleType::Log)
                    .y_scale(ScaleType::Log)
                    .point_radius(4.0)
                    .size(600.0, 350.0)
                    .build()
                    .unwrap()
            })
            .child(self.code_block(
                "scatter(&x, &y)\n    .x_scale(ScaleType::Log)\n    .y_scale(ScaleType::Log)\n    .build()?",
            ))
            .child(
                div()
                    .mt_6()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Line Chart - Logarithmic X-Axis"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child("Frequency response plot with logarithmic frequency axis - standard for audio engineering"),
            )
            .child({
                line(&freq_x, &freq_y)
                    .title("Frequency Response (20 Hz - 20 kHz)")
                    .color(0x1f77b4)
                    .x_scale(ScaleType::Log)
                    .stroke_width(2.5)
                    .size(600.0, 350.0)
                    .build()
                    .unwrap()
            })
            .child(self.code_block(
                "line(&freq, &db)\n    .x_scale(ScaleType::Log)\n    .build()?",
            ))
            .child(
                div()
                    .mt_6()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Bar Chart - Logarithmic Y-Axis"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child("Bar chart with log scale for values spanning multiple magnitudes"),
            )
            .child({
                bar(&bar_cats, &bar_vals)
                    .title("Logarithmic Values")
                    .color(0x2ca02c)
                    .y_scale(ScaleType::Log)
                    .size(600.0, 350.0)
                    .build()
                    .unwrap()
            })
            .child(self.code_block(
                "bar(&categories, &values)\n    .y_scale(ScaleType::Log)\n    .build()?",
            ))
            .child(
                div()
                    .mt_6()
                    .p_4()
                    .bg(rgb(0xfef3c7))
                    .border_1()
                    .border_color(rgb(0xfbbf24))
                    .rounded_md()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0x92400e))
                                    .child("Important Notes:"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x78350f))
                                    .child("• Logarithmic scales require all values to be positive"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x78350f))
                                    .child("• Zero and negative values will cause validation errors"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x78350f))
                                    .child("• Each decade (10x) gets equal spacing on the axis"),
                            ),
                    ),
            )
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Supported Chart Types"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .ml_4()
                    .child(div().text_sm().child("• Scatter: Both X and Y axes can be logarithmic"))
                    .child(div().text_sm().child("• Line: Both X and Y axes can be logarithmic"))
                    .child(div().text_sm().child("• Bar: Only Y-axis (values) can be logarithmic"))
                    .child(div().text_sm().child("• Heatmap: Both axes support logarithmic scaling"))
                    .child(div().text_sm().child("• Contour/Isoline: Both axes support logarithmic scaling")),
            )
    }

    // ========================================================================
    // Heatmap Section
    // ========================================================================

    fn render_heatmap_demo(&mut self, cx: &mut Context<Self>) -> Div {
        let entity = cx.entity().clone();

        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Heatmap"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .max_w(px(600.0))
                    .child("Visualizes 2D scalar fields using color. Perfect for spectrograms, correlation matrices, and geographic data."),
            )
            .child({
                heatmap(&self.heatmap_z, self.heatmap_size, self.heatmap_size)
                    .title("Gaussian Peaks")
                    .color_scale(self.heatmap_color_scale.clone())
                    .size(500.0, 500.0)
                    .build()
                    .unwrap()
            })
            // Color scale selector
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(rgb(0xf8f8f8))
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(0xe0e0e0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Color Scale"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .children(
                                [
                                    ("Viridis", ColorScaleType::Viridis),
                                    ("Plasma", ColorScaleType::Plasma),
                                    ("Inferno", ColorScaleType::Inferno),
                                    ("Magma", ColorScaleType::Magma),
                                    ("Heat", ColorScaleType::Heat),
                                    ("Greys", ColorScaleType::Greys),
                                ]
                                .into_iter()
                                .map(|(label, scale_type)| {
                                    let entity = entity.clone();
                                    let is_selected = is_color_scale_type(&self.heatmap_color_scale, scale_type);

                                    div()
                                        .id(ElementId::Name(format!("hm-{}", label).into()))
                                        .px_3()
                                        .py_1()
                                        .rounded_md()
                                        .cursor_pointer()
                                        .bg(if is_selected {
                                            rgb(0x007acc)
                                        } else {
                                            rgb(0xe0e0e0)
                                        })
                                        .text_color(if is_selected {
                                            rgb(0xffffff)
                                        } else {
                                            rgb(0x333333)
                                        })
                                        .text_xs()
                                        .child(label)
                                        .on_click(move |_, _window, cx| {
                                            entity.update(cx, |this, _| {
                                                this.heatmap_color_scale = scale_type.to_color_scale();
                                            });
                                        })
                                }),
                            ),
                    ),
            )
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Code"),
            )
            .child(self.code_block(
                "heatmap(&z, width, height)\n    .title(\"My Heatmap\")\n    .color_scale(ColorScale::Viridis)\n    .size(500.0, 500.0)\n    .build()?",
            ))
    }

    // ========================================================================
    // Contour Section
    // ========================================================================

    fn render_contour_demo(&mut self, cx: &mut Context<Self>) -> Div {
        let entity = cx.entity().clone();

        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Contour Chart"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .max_w(px(600.0))
                    .child("Shows filled bands between threshold values. Great for topographic visualizations and density estimation."),
            )
            .child({
                contour(&self.contour_z, self.contour_size, self.contour_size)
                    .title("Filled Contours")
                    .color_scale(self.contour_color_scale.clone())
                    .size(500.0, 500.0)
                    .build()
                    .unwrap()
            })
            // Color scale selector
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .bg(rgb(0xf8f8f8))
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(0xe0e0e0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Color Scale"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .children(
                                [
                                    ("Viridis", ColorScaleType::Viridis),
                                    ("Plasma", ColorScaleType::Plasma),
                                    ("Inferno", ColorScaleType::Inferno),
                                    ("Coolwarm", ColorScaleType::Coolwarm),
                                ]
                                .into_iter()
                                .map(|(label, scale_type)| {
                                    let entity = entity.clone();
                                    let is_selected = is_color_scale_type(&self.contour_color_scale, scale_type);

                                    div()
                                        .id(ElementId::Name(format!("ct-{}", label).into()))
                                        .px_3()
                                        .py_1()
                                        .rounded_md()
                                        .cursor_pointer()
                                        .bg(if is_selected {
                                            rgb(0x007acc)
                                        } else {
                                            rgb(0xe0e0e0)
                                        })
                                        .text_color(if is_selected {
                                            rgb(0xffffff)
                                        } else {
                                            rgb(0x333333)
                                        })
                                        .text_xs()
                                        .child(label)
                                        .on_click(move |_, _window, cx| {
                                            entity.update(cx, |this, _| {
                                                this.contour_color_scale = scale_type.to_color_scale();
                                            });
                                        })
                                }),
                            ),
                    ),
            )
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Code"),
            )
            .child(self.code_block(
                "contour(&z, width, height)\n    .title(\"My Contours\")\n    .thresholds(vec![0.0, 0.25, 0.5, 0.75, 1.0])\n    .color_scale(ColorScale::Viridis)\n    .build()?",
            ))
    }

    // ========================================================================
    // Isoline Section
    // ========================================================================

    fn render_isoline_demo(&self) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Isoline Chart"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .max_w(px(600.0))
                    .child("Draws unfilled contour lines at specific levels. Useful for elevation maps, pressure fields, and level curves."),
            )
            .child({
                isoline(&self.contour_z, self.contour_size, self.contour_size)
                    .title("Contour Lines")
                    .color(0x333333)
                    .stroke_width(1.5)
                    .size(500.0, 500.0)
                    .build()
                    .unwrap()
            })
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Code"),
            )
            .child(self.code_block(
                "isoline(&z, width, height)\n    .title(\"My Isolines\")\n    .levels(vec![0.2, 0.4, 0.6, 0.8])\n    .color(0x333333)\n    .stroke_width(1.5)\n    .build()?",
            ))
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Features"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .ml_4()
                    .child(div().text_sm().child("• Unfilled contour lines (vs filled Contour)"))
                    .child(div().text_sm().child("• Custom stroke color and width"))
                    .child(div().text_sm().child("• Auto-generated or custom levels"))
                    .child(div().text_sm().child("• Line opacity control")),
            )
    }

    // ========================================================================
    // Treemap Section
    // ========================================================================

    fn render_treemap_demo(&self) -> Div {
        use d3rs::color::ColorScheme;
        use gpui_px::{TilingMethod, TreemapNode, treemap};

        // Create sample hierarchical data representing a file system
        let file_system = TreemapNode::with_children(
            "root",
            vec![
                TreemapNode::with_children(
                    "src",
                    vec![
                        TreemapNode::new("main.rs", 45.0),
                        TreemapNode::new("lib.rs", 32.0),
                        TreemapNode::new("utils.rs", 18.0),
                        TreemapNode::with_children(
                            "components",
                            vec![
                                TreemapNode::new("button.rs", 12.0),
                                TreemapNode::new("input.rs", 15.0),
                                TreemapNode::new("layout.rs", 20.0),
                            ],
                        ),
                    ],
                ),
                TreemapNode::with_children(
                    "tests",
                    vec![
                        TreemapNode::new("integration.rs", 28.0),
                        TreemapNode::new("unit.rs", 22.0),
                    ],
                ),
                TreemapNode::with_children(
                    "docs",
                    vec![
                        TreemapNode::new("README.md", 8.0),
                        TreemapNode::new("CONTRIBUTING.md", 5.0),
                    ],
                ),
                TreemapNode::new("Cargo.toml", 6.0),
            ],
        );

        // Sales data example
        let sales_data = TreemapNode::with_children(
            "Global Sales",
            vec![
                TreemapNode::with_children(
                    "North America",
                    vec![
                        TreemapNode::new("USA", 450.0),
                        TreemapNode::new("Canada", 85.0),
                        TreemapNode::new("Mexico", 65.0),
                    ],
                ),
                TreemapNode::with_children(
                    "Europe",
                    vec![
                        TreemapNode::new("Germany", 180.0),
                        TreemapNode::new("France", 145.0),
                        TreemapNode::new("UK", 160.0),
                        TreemapNode::new("Spain", 95.0),
                    ],
                ),
                TreemapNode::with_children(
                    "Asia",
                    vec![
                        TreemapNode::new("China", 320.0),
                        TreemapNode::new("Japan", 220.0),
                        TreemapNode::new("India", 175.0),
                    ],
                ),
            ],
        );

        // Portfolio allocation
        let portfolio = TreemapNode::with_children(
            "Portfolio",
            vec![
                TreemapNode::with_children(
                    "Stocks",
                    vec![
                        TreemapNode::new("Tech", 35.0),
                        TreemapNode::new("Healthcare", 20.0),
                        TreemapNode::new("Finance", 15.0),
                    ],
                ),
                TreemapNode::with_children(
                    "Bonds",
                    vec![
                        TreemapNode::new("Government", 18.0),
                        TreemapNode::new("Corporate", 12.0),
                    ],
                ),
            ],
        );

        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Treemap Chart"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .max_w(px(600.0))
                    .child("Displays hierarchical data as nested rectangles. The size of each rectangle represents a quantitative value. Great for visualizing file systems, organizational structures, and part-to-whole relationships."),
            )
            .child(
                div()
                    .mt_2()
                    .text_base()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Interactive: Hover over rectangles to highlight, click to log to console"),
            )
            .child({
                treemap(&file_system)
                    .title("File System Visualization (KB)")
                    .tiling_method(TilingMethod::Squarify)
                    .padding(2.0)
                    .size(600.0, 400.0)
                    .on_click(|name, value| {
                        eprintln!("Clicked: {} (value: {})", name, value);
                    })
                    .build()
                    .unwrap()
            })
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Code"),
            )
            .child(self.code_block(
                "let root = TreemapNode::with_children(\"root\", vec![\n    TreemapNode::new(\"file1.rs\", 45.0),\n    TreemapNode::with_children(\"dir\", vec![\n        TreemapNode::new(\"file2.rs\", 32.0),\n    ]),\n]);\n\ntreemap(&root)\n    .title(\"File System\")\n    .tiling_method(TilingMethod::Squarify)\n    .padding(2.0)\n    .build()?",
            ))
            .child(
                div()
                    .mt_6()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("More Examples"),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child({
                        treemap(&sales_data)
                            .title("Global Sales (Millions)")
                            .tiling_method(TilingMethod::Binary)
                            .color_scheme(ColorScheme::category10())
                            .size(350.0, 300.0)
                            .build()
                            .unwrap()
                    })
                    .child({
                        treemap(&portfolio)
                            .title("Portfolio Allocation (%)")
                            .tiling_method(TilingMethod::SliceDice)
                            .color_scheme(ColorScheme::pastel())
                            .padding(3.0)
                            .size(350.0, 300.0)
                            .build()
                            .unwrap()
                    }),
            )
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Features"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .ml_4()
                    .child(div().text_sm().child("• Multiple tiling algorithms (Squarify, Binary, Slice, Dice, SliceDice)"))
                    .child(div().text_sm().child("• Hierarchical data with unlimited nesting"))
                    .child(div().text_sm().child("• Custom color schemes (Tableau10, Category10, Pastel1, etc.)"))
                    .child(div().text_sm().child("• Interactive hover highlighting and click handlers"))
                    .child(div().text_sm().child("• Configurable padding between rectangles"))
                    .child(div().text_sm().child("• Automatic labels for larger rectangles")),
            )
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Tiling Methods"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .ml_4()
                    .child(div().text_sm().child("• Squarify: Creates more square-like rectangles (best aspect ratios)"))
                    .child(div().text_sm().child("• Binary: Splits alternating horizontal/vertical"))
                    .child(div().text_sm().child("• Slice: Horizontal strips only"))
                    .child(div().text_sm().child("• Dice: Vertical strips only"))
                    .child(div().text_sm().child("• SliceDice: Alternates between Slice and Dice per level")),
            )
            .child(
                div()
                    .mt_4()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Customization"),
            )
            .child(self.code_block(
                "treemap(&data)\n    .title(\"My Treemap\")\n    .tiling_method(TilingMethod::Binary)\n    .color_scheme(ColorScheme::category10())\n    .padding(3.0)\n    .on_click(|name, value| {\n        println!(\"Clicked: {} ({})\", name, value);\n    })\n    .hover(true)  // Enable hover highlighting\n    .build()?",
            ))
    }

    // ========================================================================
    // Gallery Section
    // ========================================================================

    fn render_gallery(&self) -> Div {
        let small_w = 350.0;
        let small_h = 250.0;

        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Chart Gallery"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child("All gpui-px chart types at a glance"),
            )
            // Row 1: Scatter, Line, Bar
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child({
                        scatter(&self.scatter_x, &self.scatter_y)
                            .title("Scatter")
                            .color(0x1f77b4)
                            .point_radius(4.0)
                            .size(small_w, small_h)
                            .build()
                            .unwrap()
                    })
                    .child({
                        line(&self.line_x, &self.line_y)
                            .title("Line")
                            .color(0xff7f0e)
                            .size(small_w, small_h)
                            .build()
                            .unwrap()
                    })
                    .child({
                        bar(&self.bar_categories, &self.bar_values)
                            .title("Bar")
                            .color(0x2ca02c)
                            .size(small_w, small_h)
                            .build()
                            .unwrap()
                    }),
            )
            // Row 2: Box Plot, Heatmap, Contour
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child({
                        boxplot(&self.boxplot_x, &self.boxplot_y)
                            .title("Box Plot")
                            .box_color(0xdddddd)
                            .median_color(0x000000)
                            .size(small_w, small_h)
                            .build()
                            .unwrap()
                    })
                    .child({
                        heatmap(&self.heatmap_z, self.heatmap_size, self.heatmap_size)
                            .title("Heatmap")
                            .color_scale(ColorScale::Viridis)
                            .size(small_w, small_h)
                            .build()
                            .unwrap()
                    })
                    .child({
                        contour(&self.contour_z, self.contour_size, self.contour_size)
                            .title("Contour")
                            .color_scale(ColorScale::Plasma)
                            .size(small_w, small_h)
                            .build()
                            .unwrap()
                    }),
            )
            // Row 3: Isoline
            .child(div().flex().gap_4().child({
                isoline(&self.contour_z, self.contour_size, self.contour_size)
                    .title("Isoline")
                    .color(0x333333)
                    .size(small_w, small_h)
                    .build()
                    .unwrap()
            }))
    }
}

impl Render for ShowcaseApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .child(self.render_sidebar(cx))
            .child(self.render_content(cx))
    }
}

// ============================================================================
// Helper Types for Color Scale Selection
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum ColorScaleType {
    Viridis,
    Plasma,
    Inferno,
    Magma,
    Heat,
    Coolwarm,
    Greys,
}

impl ColorScaleType {
    fn to_color_scale(self) -> ColorScale {
        match self {
            ColorScaleType::Viridis => ColorScale::Viridis,
            ColorScaleType::Plasma => ColorScale::Plasma,
            ColorScaleType::Inferno => ColorScale::Inferno,
            ColorScaleType::Magma => ColorScale::Magma,
            ColorScaleType::Heat => ColorScale::Heat,
            ColorScaleType::Coolwarm => ColorScale::Coolwarm,
            ColorScaleType::Greys => ColorScale::Greys,
        }
    }
}

fn is_color_scale_type(scale: &ColorScale, scale_type: ColorScaleType) -> bool {
    matches!(
        (scale, scale_type),
        (ColorScale::Viridis, ColorScaleType::Viridis)
            | (ColorScale::Plasma, ColorScaleType::Plasma)
            | (ColorScale::Inferno, ColorScaleType::Inferno)
            | (ColorScale::Magma, ColorScaleType::Magma)
            | (ColorScale::Heat, ColorScaleType::Heat)
            | (ColorScale::Coolwarm, ColorScaleType::Coolwarm)
            | (ColorScale::Greys, ColorScaleType::Greys)
    )
}

// ============================================================================
// Data Generators
// ============================================================================

/// Generate spiral scatter data
fn generate_scatter_data() -> (Vec<f64>, Vec<f64>) {
    (0..100)
        .map(|i| {
            let t = i as f64 * 0.15;
            let r = 10.0 + t * 3.0;
            (50.0 + r * t.cos(), 50.0 + r * t.sin())
        })
        .unzip()
}

/// Generate sine wave data
fn generate_line_data() -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&xi| (xi * 2.0).sin() * 40.0 + 50.0).collect();
    (x, y)
}

/// Generate weekly bar data
fn generate_bar_data() -> (Vec<String>, Vec<f64>) {
    let categories = vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        .into_iter()
        .map(String::from)
        .collect();
    let values = vec![45.0, 62.0, 55.0, 78.0, 68.0, 35.0, 28.0];
    (categories, values)
}

/// Generate box plot data (simulated Morley speed of light experiment)
///
/// This generates data similar to the classic Morley experiment dataset,
/// with 5 experiments each containing 20 measurements.
fn generate_boxplot_data() -> (Vec<f64>, Vec<f64>) {
    // Morley experiment: 5 runs, each with 20 measurements
    // Values are speed of light measurements (deviations from 299,000 km/s)
    // Based on the original D3 example data patterns
    let experiment_data: Vec<Vec<f64>> = vec![
        vec![
            850., 740., 900., 1070., 930., 850., 950., 980., 980., 880., 1000., 980., 930., 650.,
            760., 810., 1000., 1000., 960., 960.,
        ],
        vec![
            960., 940., 960., 940., 880., 800., 850., 880., 900., 840., 830., 790., 810., 880.,
            880., 830., 800., 790., 760., 800.,
        ],
        vec![
            880., 880., 880., 860., 720., 720., 620., 860., 970., 950., 880., 910., 850., 870.,
            840., 840., 850., 840., 840., 840.,
        ],
        vec![
            890., 810., 810., 820., 800., 770., 760., 740., 750., 760., 910., 920., 890., 860.,
            880., 720., 840., 850., 850., 780.,
        ],
        vec![
            890., 840., 780., 810., 760., 810., 790., 810., 820., 850., 870., 870., 810., 740.,
            810., 940., 950., 800., 810., 870.,
        ],
    ];

    let mut x = Vec::new();
    let mut y = Vec::new();

    for (exp_idx, measurements) in experiment_data.iter().enumerate() {
        let exp_num = (exp_idx + 1) as f64;
        for &value in measurements {
            x.push(exp_num);
            y.push(value);
        }
    }

    (x, y)
}

/// Generate 2D grid data with Gaussian peaks
fn generate_grid_data(size: usize) -> Vec<f64> {
    let mut z = vec![0.0; size * size];
    for j in 0..size {
        for i in 0..size {
            let x = (i as f64 / size as f64) * 4.0 - 2.0;
            let y = (j as f64 / size as f64) * 4.0 - 2.0;
            // Two Gaussian peaks
            let peak1 = (-((x - 0.5).powi(2) + (y - 0.5).powi(2)) / 0.5).exp();
            let peak2 = 0.7 * (-((x + 0.8).powi(2) + (y + 0.8).powi(2)) / 0.3).exp();
            z[j * size + i] = peak1 + peak2;
        }
    }
    z
}
