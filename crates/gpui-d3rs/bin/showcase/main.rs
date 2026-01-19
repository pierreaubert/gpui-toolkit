//! d3rs Showcase - Unified demo application
//!
//! Demonstrates all d3rs functionality in a single application with tabbed navigation.

use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::{MiniApp, MiniAppConfig};

mod showcase_modules;

// Demo sections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DemoSection {
    #[default]
    Overview,
    Scales,
    Axes,
    BarCharts,
    LineCharts,
    ScatterPlots,
    SurfacePlots,
    QuadTree,
    Contours,
    Transitions,
    Geo,
    Colors,
    Hierarchy,
    Force,
    Chord,
    // D3 Observable Examples
    D3VolcanoContours,
    D3KDE,
    D3Treemap,
    D3StackedBars,
    D3Versor,
    D3Histogram,
    D3Revenue,
    D3Horizon,
    D3Choropleth,
}

impl DemoSection {
    fn all() -> Vec<Self> {
        vec![
            Self::Overview,
            Self::Scales,
            Self::Axes,
            Self::BarCharts,
            Self::LineCharts,
            Self::ScatterPlots,
            Self::SurfacePlots,
            Self::QuadTree,
            Self::Contours,
            Self::Transitions,
            Self::Geo,
            Self::Colors,
            Self::Hierarchy,
            Self::Force,
            Self::Chord,
            // D3 Observable Examples
            Self::D3VolcanoContours,
            Self::D3KDE,
            Self::D3Treemap,
            Self::D3StackedBars,
            Self::D3Versor,
            Self::D3Histogram,
            Self::D3Revenue,
            Self::D3Horizon,
            Self::D3Choropleth,
        ]
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Scales => "Scales",
            Self::Axes => "Axes",
            Self::BarCharts => "Bar Charts",
            Self::LineCharts => "Line Charts",
            Self::ScatterPlots => "Scatter Plots",
            Self::SurfacePlots => "Surface Plots",
            Self::QuadTree => "QuadTree",
            Self::Contours => "Contours",
            Self::Transitions => "Transitions",
            Self::Geo => "Geo",
            Self::Colors => "Colors",
            Self::Hierarchy => "Hierarchy",
            Self::Force => "Force Graph",
            Self::Chord => "Chord Diagram",
            // D3 Observable Examples
            Self::D3VolcanoContours => "D3: Volcano",
            Self::D3KDE => "D3: KDE",
            Self::D3Treemap => "D3: Treemap",
            Self::D3StackedBars => "D3: Stacked Bars",
            Self::D3Versor => "D3: Versor Dragging",
            Self::D3Histogram => "D3: Histogram",
            Self::D3Revenue => "D3: Revenue Stream",
            Self::D3Horizon => "D3: Horizon Chart",
            Self::D3Choropleth => "D3: Choropleth",
        }
    }
}

/// Contour rendering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContourRenderMode {
    #[default]
    Isoline,
    Surface,
    Heatmap,
}

impl ContourRenderMode {
    fn label(&self) -> &'static str {
        match self {
            Self::Isoline => "Isoline",
            Self::Surface => "Surface",
            Self::Heatmap => "Heatmap",
        }
    }

    fn next(&self) -> Self {
        match self {
            Self::Isoline => Self::Surface,
            Self::Surface => Self::Heatmap,
            Self::Heatmap => Self::Isoline,
        }
    }
}

/// Geographic projection type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GeoProjectionType {
    #[default]
    Mercator,
    Equirectangular,
    Orthographic,
    Stereographic,
    ConicEqualArea,
}

impl GeoProjectionType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Mercator => "Mercator",
            Self::Equirectangular => "Equirectangular",
            Self::Orthographic => "Orthographic",
            Self::Stereographic => "Stereographic",
            Self::ConicEqualArea => "Conic Equal-Area",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Mercator,
            Self::Equirectangular,
            Self::Orthographic,
            Self::Stereographic,
            Self::ConicEqualArea,
        ]
    }
}

pub struct ShowcaseApp {
    pub current_section: DemoSection,
    // Geo demo parameters
    pub geo_projection_type: GeoProjectionType,
    pub geo_rotation_lon: f64,
    pub geo_rotation_lat: f64,
    // Contour demo parameters
    pub contour_grid_size: usize,
    pub contour_num_levels: usize,
    pub contour_peak1_x: f32,
    pub contour_peak1_y: f32,
    pub contour_peak2_x: f32,
    pub contour_peak2_y: f32,
    pub density_bandwidth: f32,
    pub density_num_points: usize,
    pub contour_render_mode: ContourRenderMode,
    // QuadTree demo parameters
    pub quadtree_query_x: f32,
    pub quadtree_query_y: f32,
    pub quadtree_search_radius: f32,
    // D3 Volcano Contours example parameters
    pub volcano_num_thresholds: usize,
    pub volcano_color_scale: showcase_modules::d3_examples::volcano_contours::VolcanoColorScale,
    pub volcano_show_stroke: bool,
    // D3 KDE example parameters
    pub kde_bandwidth: f64,
    pub kde_kernel_type: showcase_modules::d3_examples::KernelType,
    pub kde_show_histogram: bool,
    pub kde_bin_count: usize,
    // D3 Treemap example parameters
    pub treemap_tiling: showcase_modules::d3_examples::TilingMethod,
    pub treemap_padding: f32,
    // D3 Stacked/Grouped Bars example parameters
    pub stacked_bars_layout: showcase_modules::d3_examples::BarLayout,
    pub stacked_bars_n_series: usize,
    pub stacked_bars_m_samples: usize,
    pub stacked_bars_animation_progress: f64,
    pub stacked_bars_animating: bool,
    // Force Simulation
    pub force_simulation: d3rs::force::Simulation,
    pub force_running: bool,
    // Horizon Chart
    pub horizon_data: Vec<f64>,
    pub horizon_offset: f64,
    // Data toggle
    pub use_large_data: bool,
    // Dragging state
    pub is_dragging: bool,
    pub last_mouse_pos: Option<Point<Pixels>>,
    // Snapshot state
    pub snapshot_mode: bool,
    pub snapshot_list: Vec<DemoSection>,
    pub snapshot_index: usize,
    pub snapshot_wait_frames: usize,
}

impl ShowcaseApp {
    fn new(_cx: &mut Context<Self>) -> Self {
        let args: Vec<String> = std::env::args().collect();
        let snapshot_mode = args.iter().any(|arg| arg == "--snapshot");

        // Create output directory if needed
        if snapshot_mode {
            let output_dir = std::path::Path::new("docs/images");
            if !output_dir.exists() {
                std::fs::create_dir_all(output_dir).ok();
            }
        }

        Self {
            current_section: DemoSection::default(),
            // Geo demo defaults
            geo_projection_type: GeoProjectionType::default(),
            geo_rotation_lon: 0.0,
            geo_rotation_lat: 0.0,
            contour_grid_size: 50,
            contour_num_levels: 5,
            contour_peak1_x: 0.3,
            contour_peak1_y: 0.3,
            contour_peak2_x: -0.4,
            contour_peak2_y: -0.2,
            density_bandwidth: 0.08,
            density_num_points: 100,
            contour_render_mode: ContourRenderMode::default(),
            quadtree_query_x: 50.0,
            quadtree_query_y: 50.0,
            quadtree_search_radius: 15.0,
            // D3 Volcano Contours defaults
            volcano_num_thresholds: 20,
            volcano_color_scale:
                showcase_modules::d3_examples::volcano_contours::VolcanoColorScale::default(),
            volcano_show_stroke: false,
            // D3 KDE defaults
            kde_bandwidth: 7.0,
            kde_kernel_type: showcase_modules::d3_examples::KernelType::default(),
            kde_show_histogram: true,
            kde_bin_count: 20,
            // D3 Treemap defaults
            treemap_tiling: showcase_modules::d3_examples::TilingMethod::default(),
            treemap_padding: 1.0,
            // D3 Stacked/Grouped Bars defaults
            stacked_bars_layout: showcase_modules::d3_examples::BarLayout::default(),
            stacked_bars_n_series: 5,
            stacked_bars_m_samples: 40,
            stacked_bars_animation_progress: 0.0,
            stacked_bars_animating: false,
            // Force Simulation
            force_simulation: {
                // Initialize simulation
                use d3rs::force::{ForceCenter, ForceManyBody, Simulation, SimulationNode};
                let width = 800.0;
                let height = 600.0;
                let mut nodes = Vec::new();
                for i in 0..50 {
                    let x = width / 2.0 + (i as f64 * 13.0 % 100.0 - 50.0);
                    let y = height / 2.0 + (i as f64 * 17.0 % 100.0 - 50.0);
                    nodes.push(SimulationNode::new(i, x, y));
                }
                Simulation::new(nodes)
                    .force(Box::new(ForceManyBody::new()))
                    .force(Box::new(ForceCenter::new(width / 2.0, height / 2.0)))
            },
            force_running: false,
            // Horizon Chart defaults
            horizon_data: (0..200).map(|i| (i as f64 * 0.1).sin() * 20.0).collect(),
            horizon_offset: 0.0,
            use_large_data: false,
            is_dragging: false,
            last_mouse_pos: None,
            snapshot_mode,
            snapshot_list: DemoSection::all(),
            snapshot_index: 0,
            snapshot_wait_frames: 3, // Wait 60 frames initially
        }
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.current_section;

        div()
            .w(px(200.0))
            .id("sidebar-scroll")
            .h_full()
            .bg(rgb(0x1e1e1e))
            .border_r_1()
            .border_color(rgb(0x3c3c3c))
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .p_4()
            .gap_2()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xffffff))
                    .mb_2()
                    .child("d3rs Showcase"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .mb_4()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xaaaaaa))
                            .child("World Data:"),
                    )
                    .child(
                        div()
                            .id("data-toggle")
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .cursor_pointer()
                            .bg(if self.use_large_data {
                                rgb(0x448844)
                            } else {
                                rgb(0x444444)
                            })
                            .text_color(rgb(0xffffff))
                            .text_xs()
                            .child(if self.use_large_data {
                                "Large (50m)"
                            } else {
                                "Small (Simp)"
                            })
                            .on_click(cx.listener(|this, _, _, _| {
                                this.use_large_data = !this.use_large_data;
                            })),
                    ),
            )
            .children(DemoSection::all().into_iter().map(|section| {
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
            DemoSection::Overview => showcase_modules::overview::render(self),
            DemoSection::Scales => showcase_modules::scales::render(self),
            DemoSection::Axes => showcase_modules::axes::render(self),
            DemoSection::BarCharts => showcase_modules::bar_charts::render(self),
            DemoSection::LineCharts => showcase_modules::line_charts::render(self),
            DemoSection::ScatterPlots => showcase_modules::scatter_plots::render(self),
            DemoSection::SurfacePlots => showcase_modules::surface_plots::render(self, cx),
            DemoSection::QuadTree => showcase_modules::quadtree::render(self, cx),
            DemoSection::Contours => showcase_modules::contours::render(self, cx),
            DemoSection::Transitions => showcase_modules::transitions::render(self),
            DemoSection::Geo => showcase_modules::geo::render(self, cx),
            DemoSection::Colors => showcase_modules::colors::render(self),
            // D3 Observable Examples
            DemoSection::D3VolcanoContours => showcase_modules::d3_examples::render(self, cx),
            DemoSection::D3KDE => {
                showcase_modules::d3_examples::kernel_density_estimation::render(self, cx)
            }
            DemoSection::D3Treemap => showcase_modules::d3_examples::treemap::render(self, cx),
            DemoSection::D3StackedBars => {
                showcase_modules::d3_examples::stacked_grouped_bars::render(self, cx)
            }
            DemoSection::D3Versor => showcase_modules::d3_examples::versor::render(self, cx),
            DemoSection::D3Histogram => showcase_modules::d3_examples::histogram::render(self, cx),
            DemoSection::D3Revenue => showcase_modules::d3_examples::revenue::render(self, cx),
            DemoSection::D3Horizon => showcase_modules::d3_examples::horizon::render(self, cx),
            DemoSection::D3Choropleth => {
                showcase_modules::d3_examples::choropleth::render(self, cx)
            }
            DemoSection::Hierarchy => showcase_modules::hierarchy::render(self, cx),
            DemoSection::Force => showcase_modules::force::render(self, cx),
            DemoSection::Chord => showcase_modules::chord::render(self, cx),
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
}

impl Render for ShowcaseApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Snapshot automation logic
        if self.snapshot_mode {
            if self.snapshot_index == 0 {
                println!("Starting snapshot automation...");
            }
            cx.notify(); // Request next frame

            if self.snapshot_index < self.snapshot_list.len() {
                // Determine output path
                let section = self.snapshot_list[self.snapshot_index];
                let index = self.snapshot_index;
                let label = section
                    .label()
                    .replace(" ", "_")
                    .replace(":", "")
                    .to_lowercase();

                // Ensure output directory exists (relative to CWD)
                let output_dir = std::path::Path::new("docs/images");
                if !output_dir.exists() {
                    std::fs::create_dir_all(output_dir)
                        .expect("Failed to create docs/images directory");
                }

                let output_path = format!("docs/images/demo_{:02}_{}.png", index, label);
                println!("Capturing: {} -> {}", section.label(), output_path);

                // Try to get window ID via osascript (macOS specific) to capture only the window
                // Process name usually matches binary name "d3rs-showcase"
                let window_id = std::process::Command::new("osascript")
                    .args(["-e", "tell application \"System Events\" to get id of window 1 of (first process whose name contains \"showcase\")"])
                    .output()
                    .ok()
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                    .map(|s| s.trim().to_string());

                let mut cmd = std::process::Command::new("screencapture");
                cmd.arg("-x"); // silent

                if let Some(wid) = window_id {
                    // Capture specific window
                    cmd.arg("-l").arg(wid);
                } else {
                    // Fallback to main monitor
                    cmd.arg("-m");
                }

                let _ = cmd.arg(&output_path).output();

                // Advance to next demo
                self.snapshot_index += 1;
                if self.snapshot_index < self.snapshot_list.len() {
                    self.current_section = self.snapshot_list[self.snapshot_index];
                    cx.notify();
                } else {
                    println!("Snapshot automation complete.");
                    cx.quit();
                }
            } else {
                cx.quit();
            }
        }

        // Realtime animation for Horizon Chart
        if self.current_section == DemoSection::D3Horizon {
            self.horizon_offset += 0.1;
            // Update data: simulate random walk or scrolling sine wave
            let len = self.horizon_data.len();
            for i in 0..len {
                self.horizon_data[i] = ((i as f64 * 0.1) + self.horizon_offset).sin() * 20.0
                    + ((i as f64 * 0.03) - self.horizon_offset * 0.5).cos() * 10.0;
            }
            cx.notify();
        }

        div()
            .flex()
            .size_full()
            .child(self.render_sidebar(cx))
            .child(self.render_content(cx))
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("d3rs Showcase")
            .size(1000.0, 800.0)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(ShowcaseApp::new),
    );
}
