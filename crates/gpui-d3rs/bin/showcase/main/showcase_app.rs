use super::contour_render_mode::ContourRenderMode;
use super::demo_section::DemoSection;
use super::geo_projection_type::GeoProjectionType;
use gpui::prelude::*;
use gpui::*;
use gpui_builder::{
    Axis, ContainerNode, LayoutNode, Sizing, SlotNode, solve, types::LayoutPreferences,
};
use gpui_design::DesignExt;
use gpui_ui_kit::theme::ThemeExt;
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(feature = "profiler")]
use gpui_profiler::{AllocProbe, AllocSnapshot};

/// No-op snapshot/probe used when the `profiler` feature is disabled.
#[cfg(not(feature = "profiler"))]
#[derive(Debug, Clone, Copy, Default)]
struct AllocSnapshot {
    pub bytes: usize,
    pub count: usize,
}

#[cfg(not(feature = "profiler"))]
struct AllocProbe;

#[cfg(not(feature = "profiler"))]
impl AllocProbe {
    fn new() -> Self {
        Self
    }
    fn sample(&mut self, _label: &str) -> AllocSnapshot {
        AllocSnapshot::default()
    }
}

pub struct ShowcaseApp {
    pub current_section: DemoSection,
    // Available content dimensions (updated each render from window bounds)
    pub content_width: f32,
    pub content_height: f32,
    // Geo demo parameters
    pub geo_projection_type: GeoProjectionType,
    pub geo_rotation_lon: f64,
    pub geo_rotation_lat: f64,
    pub geo_zoom: f64,
    pub stippling_iterations: usize, // current displayed iteration
    pub stippling_target: usize,     // target iteration (user selected)
    pub stippling_state: Option<d3rs::examples::voronoi_stippling::StipplingState>,
    pub stippling_density: Option<Vec<f64>>,
    pub stippling_img_size: (usize, usize),
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
    pub volcano_color_scale:
        super::showcase_modules::d3_examples::volcano_contours::VolcanoColorScale,
    pub volcano_show_stroke: bool,
    // D3 KDE example parameters
    pub kde_bandwidth: f64,
    pub kde_kernel_type: super::showcase_modules::d3_examples::KernelType,
    pub kde_show_histogram: bool,
    pub kde_bin_count: usize,
    // D3 Treemap example parameters
    pub treemap_tiling: super::showcase_modules::d3_examples::TilingMethod,
    pub treemap_padding: f32,
    // D3 Stacked/Grouped Bars example parameters
    pub stacked_bars_layout: super::showcase_modules::d3_examples::BarLayout,
    pub stacked_bars_n_series: usize,
    pub stacked_bars_m_samples: usize,
    pub stacked_bars_animation_progress: f64,
    pub stacked_bars_animating: bool,
    // Force Simulation
    pub force_simulation: d3rs::force::Simulation,
    pub force_running: bool,
    pub force_node_positions: Rc<RefCell<Vec<(f32, f32)>>>,
    // Cached expensive D3 example data so it is not recomputed every render.
    pub hexbin_cache: Option<Rc<super::showcase_modules::d3_examples::hexbin::HexbinCache>>,
    pub force_directed_cache:
        Option<Rc<super::showcase_modules::d3_examples::force_directed::ForceDirectedCache>>,
    /// Retained large scatter cache; both canonical normalized points and its
    /// density pyramid survive showcase re-renders.
    pub lod_scatter: d3rs::gpu2d::LodScatter,
    // Horizon Chart
    pub horizon_data: Vec<f64>,
    pub horizon_offset: f64,
    pub horizon_animating: bool,
    // Data toggle
    pub use_large_data: bool,
    // Dragging state
    pub is_dragging: bool,
    pub last_mouse_pos: Option<Point<Pixels>>,
    // Cached expensive showcase data so it is not regenerated every render.
    pub surface_plot_cache: Option<super::showcase_modules::surface_plots::SurfacePlotCache>,
    pub surface_plot_camera_freq_response: d3rs::surface::SurfaceCamera,
    pub surface_plot_camera_freq_2d: d3rs::surface::SurfaceCamera,
    pub surface_plot_camera_spectral: d3rs::surface::SurfaceCamera,
    pub surface_plot_drag: Option<(usize, Point<Pixels>)>,
    // Allocation probe for tracking heap allocations during interactive events.
    alloc_probe: AllocProbe,
    last_render_alloc: AllocSnapshot,
    last_mouse_move_alloc: AllocSnapshot,
    last_sample: Option<(String, AllocSnapshot)>,
    last_window_size: Option<Size<Pixels>>,
    // Snapshot state
    pub snapshot_mode: bool,
    pub snapshot_list: Vec<DemoSection>,
    pub snapshot_index: usize,
    pub snapshot_wait_frames: usize,
}

impl ShowcaseApp {
    pub(super) fn new(_cx: &mut Context<Self>) -> Self {
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
            content_width: 700.0,
            content_height: 600.0,
            // Geo demo defaults
            geo_projection_type: GeoProjectionType::default(),
            geo_rotation_lon: 0.0,
            geo_rotation_lat: 0.0,
            geo_zoom: 1.0,
            stippling_iterations: 0,
            stippling_target: 40,
            stippling_state: None,
            stippling_density: None,
            stippling_img_size: (0, 0),
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
                super::showcase_modules::d3_examples::volcano_contours::VolcanoColorScale::default(),
            volcano_show_stroke: false,
            // D3 KDE defaults
            kde_bandwidth: 7.0,
            kde_kernel_type: super::showcase_modules::d3_examples::KernelType::default(),
            kde_show_histogram: true,
            kde_bin_count: 20,
            // D3 Treemap defaults
            treemap_tiling: super::showcase_modules::d3_examples::TilingMethod::default(),
            treemap_padding: 1.0,
            // D3 Stacked/Grouped Bars defaults
            stacked_bars_layout: super::showcase_modules::d3_examples::BarLayout::default(),
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
            force_node_positions: Rc::new(RefCell::new(Vec::new())),
            hexbin_cache: None,
            force_directed_cache: None,
            lod_scatter: {
                let mut state = 0x9E37_79B9_u32;
                let points = (0..80_000)
                    .map(|index| {
                        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                        let x_jitter = (state as f64 / u32::MAX as f64 - 0.5) * 22.0;
                        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                        let y_jitter = (state as f64 / u32::MAX as f64 - 0.5) * 18.0;
                        let (center_x, center_y) = match index % 3 {
                            0 => (30.0, 36.0),
                            1 => (58.0, 64.0),
                            _ => (76.0, 28.0),
                        };
                        ((center_x + x_jitter) / 100.0, (center_y + y_jitter) / 100.0)
                    })
                    .collect();
                d3rs::gpu2d::LodScatter::from_normalized(points, 512)
            },
            // Horizon Chart defaults
            horizon_data: (0..200).map(|i| (i as f64 * 0.1).sin() * 20.0).collect(),
            horizon_offset: 0.0,
            horizon_animating: false,
            use_large_data: false,
            is_dragging: false,
            last_mouse_pos: None,
            surface_plot_cache: None,
            surface_plot_camera_freq_response: d3rs::surface::SurfaceCamera::new()
                .with_rotation(30.0, 45.0)
                .with_zoom(1.0),
            surface_plot_camera_freq_2d: d3rs::surface::SurfaceCamera::new()
                .with_rotation(35.0, 50.0)
                .with_zoom(1.0),
            surface_plot_camera_spectral: d3rs::surface::SurfaceCamera::new()
                .with_rotation(25.0, 40.0)
                .with_zoom(1.0),
            surface_plot_drag: None,
            alloc_probe: AllocProbe::new(),
            last_render_alloc: AllocSnapshot::default(),
            last_mouse_move_alloc: AllocSnapshot::default(),
            last_sample: None,
            last_window_size: None,
            snapshot_mode,
            snapshot_list: DemoSection::all(),
            snapshot_index: 0,
            snapshot_wait_frames: 3, // Wait 60 frames initially
        }
    }

    /// Sample allocations and remember the result as the most recent sample.
    fn record_sample(&mut self, label: &str) -> AllocSnapshot {
        let delta = self.alloc_probe.sample(label);
        self.last_sample = Some((label.to_string(), delta));
        delta
    }

    /// Start a background timer that animates the horizon chart data.
    pub(super) fn ensure_horizon_animation(&mut self, cx: &mut Context<Self>) {
        if self.horizon_animating {
            return;
        }
        self.horizon_animating = true;

        cx.spawn(async move |this: WeakEntity<ShowcaseApp>, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(16))
                    .await;

                let still_active = this
                    .update(cx, |app, cx| {
                        let active = app.current_section == DemoSection::D3Horizon
                            || app.current_section == DemoSection::D3RealtimeHorizon;
                        if !active {
                            app.horizon_animating = false;
                            return false;
                        }

                        app.horizon_offset += 0.1;
                        let len = app.horizon_data.len();
                        for i in 0..len {
                            app.horizon_data[i] = ((i as f64 * 0.1) + app.horizon_offset).sin()
                                * 20.0
                                + ((i as f64 * 0.03) - app.horizon_offset * 0.5).cos() * 10.0;
                        }
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);

                if !still_active {
                    break;
                }
            }
        })
        .detach();
    }

    /// Ensure the expensive surface-plot data is cached and matches the current
    /// generation parameters. Rebuilds only when the cache key changes.
    pub(super) fn ensure_surface_plot_cache(&mut self) {
        use super::showcase_modules::surface_plots::{
            SURFACE_PLOT_CACHE_KEY, build_surface_plot_cache,
        };

        if self
            .surface_plot_cache
            .as_ref()
            .is_some_and(|cache| cache.key == SURFACE_PLOT_CACHE_KEY)
        {
            return;
        }
        self.surface_plot_cache = Some(build_surface_plot_cache(SURFACE_PLOT_CACHE_KEY));
    }

    /// Advance the force simulation by five ticks and copy the new node
    /// positions into the render cache.
    pub(super) fn tick_force_simulation(&mut self) {
        for _ in 0..5 {
            self.force_simulation.tick();
        }
        let mut positions = self.force_node_positions.borrow_mut();
        positions.clear();
        positions.extend(self.force_simulation.nodes.iter().map(|n| {
            let n = n.borrow();
            (n.x as f32, n.y as f32)
        }));
    }

    pub(super) fn solve_layout(&self, w: f32, h: f32) -> f32 {
        let content_children: &[LayoutNode<'_>] = &[
            LayoutNode::Slot(SlotNode {
                id: "sidebar",
                sizing: Sizing::fractional(0.18, 120.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
            LayoutNode::Slot(SlotNode {
                id: "content",
                sizing: Sizing::flex(200.0),
                priority: 1.0,
                collapsible: false,
                display_tiers: &[],
                collapse_label: None,
            }),
        ];

        let root = LayoutNode::Container(ContainerNode {
            id: "root",
            axis: Axis::Horizontal,
            auto_axis: Some(1.0),
            sizing: Sizing::flex(0.0),
            children: content_children,
            divider_size: 0.0,
        });

        let prefs = LayoutPreferences::new(&[], &[]);
        let solved = solve(&root, w, h, &prefs);
        solved.find("sidebar").map(|n| n.width).unwrap_or(120.0)
    }

    pub(super) fn render_sidebar(
        &mut self,
        sidebar_width: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let current = self.current_section;
        let theme = cx.theme();
        let ds = cx.design();

        div()
            .w(px(sidebar_width))
            .id("sidebar-scroll")
            .h_full()
            .bg(theme.surface)
            .border_r_1()
            .border_color(theme.border)
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .p(px(ds.spacing.card_padding))
            .gap(px(ds.spacing.control_gap))
            .child(
                div()
                    .text_size(px(ds.typography.large_size))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .mb(px(ds.spacing.control_gap))
                    .child("d3rs Showcase"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(ds.spacing.control_gap))
                    .mb(px(ds.spacing.section_gap))
                    .child(
                        div()
                            .text_size(px(ds.typography.small_size))
                            .text_color(theme.text_muted)
                            .child("World Data:"),
                    )
                    .child(
                        div()
                            .id("data-toggle")
                            .px(px(ds.spacing.control_padding_x * 0.7))
                            .py(px(ds.spacing.control_padding_y * 0.5))
                            .rounded(px(ds.corners.sm))
                            .cursor_pointer()
                            .bg(if self.use_large_data {
                                theme.accent
                            } else {
                                theme.surface_hover
                            })
                            .text_color(if self.use_large_data {
                                theme.text_on_accent
                            } else {
                                theme.text_primary
                            })
                            .text_size(px(ds.typography.small_size * 0.85))
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
                    theme.accent
                } else {
                    theme.surface
                };
                let hover_bg = if is_selected {
                    theme.accent
                } else {
                    theme.surface_hover
                };
                let text_color = if is_selected {
                    theme.text_on_accent
                } else {
                    theme.text_primary
                };

                div()
                    .id(ElementId::Name(section.label().into()))
                    .px(px(ds.spacing.control_padding_x))
                    .py(px(ds.spacing.control_padding_y))
                    .rounded(px(ds.corners.md))
                    .cursor_pointer()
                    .bg(bg)
                    .hover(move |s| s.bg(hover_bg))
                    .text_color(text_color)
                    .child(section.label())
                    .on_click(cx.listener(move |this, _, _window, _cx| {
                        this.current_section = section;
                    }))
            }))
    }

    pub(super) fn render_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let content: Div = match self.current_section {
            DemoSection::Overview => super::showcase_modules::overview::render(self, cx),
            DemoSection::Scales => super::showcase_modules::scales::render(self, cx),
            DemoSection::Axes => super::showcase_modules::axes::render(self, cx),
            DemoSection::BarCharts => super::showcase_modules::bar_charts::render(self, cx),
            DemoSection::LineCharts => super::showcase_modules::line_charts::render(self, cx),
            DemoSection::ScatterPlots => super::showcase_modules::scatter_plots::render(self, cx),
            DemoSection::LodLargeData => {
                super::showcase_modules::scatter_plots::render_lod(self, cx)
            }
            DemoSection::SurfacePlots => {
                self.ensure_surface_plot_cache();
                super::showcase_modules::surface_plots::render(self, cx)
            }
            DemoSection::QuadTree => super::showcase_modules::quadtree::render(self, cx),
            DemoSection::Contours => super::showcase_modules::contours::render(self, cx),
            DemoSection::Transitions => super::showcase_modules::transitions::render(self, cx),
            DemoSection::Geo => super::showcase_modules::geo::render(self, cx),
            DemoSection::Colors => super::showcase_modules::colors::render(self),
            // D3 Observable Examples
            DemoSection::D3VolcanoContours => {
                super::showcase_modules::d3_examples::render(self, cx)
            }
            DemoSection::D3KDE => {
                super::showcase_modules::d3_examples::kernel_density_estimation::render(self, cx)
            }
            DemoSection::D3Treemap => {
                super::showcase_modules::d3_examples::treemap::render(self, cx)
            }
            DemoSection::D3StackedBars => {
                super::showcase_modules::d3_examples::stacked_grouped_bars::render(self, cx)
            }
            DemoSection::D3Versor => super::showcase_modules::d3_examples::versor::render(self, cx),
            DemoSection::D3Histogram => {
                super::showcase_modules::d3_examples::histogram::render(self, cx)
            }
            DemoSection::D3Revenue => {
                super::showcase_modules::d3_examples::revenue::render(self, cx)
            }
            DemoSection::D3Horizon => {
                super::showcase_modules::d3_examples::horizon::render(self, cx)
            }
            DemoSection::D3Choropleth => {
                super::showcase_modules::d3_examples::choropleth::render(self, cx)
            }
            // New D3 Examples
            DemoSection::D3Sankey => super::showcase_modules::d3_examples::sankey::render(self, cx),
            DemoSection::D3Calendar => {
                super::showcase_modules::d3_examples::calendar::render(self, cx)
            }
            DemoSection::D3RadialLine => {
                super::showcase_modules::d3_examples::radial_line::render(self, cx)
            }
            DemoSection::D3ParallelCoordinates => {
                super::showcase_modules::d3_examples::parallel_coordinates::render(self, cx)
            }
            DemoSection::Hierarchy => super::showcase_modules::hierarchy::render(self, cx),
            DemoSection::Force => super::showcase_modules::force::render(self, cx),
            DemoSection::Chord => super::showcase_modules::chord::render(self, cx),
            // Observable Examples (golden-tested, using src/examples/ compute)
            DemoSection::D3Hexbin => super::showcase_modules::d3_examples::hexbin::render(self, cx),
            DemoSection::D3PieChart => {
                super::showcase_modules::d3_examples::pie_chart::render(self, cx)
            }
            DemoSection::D3DonutChart => {
                super::showcase_modules::d3_examples::donut_chart::render(self, cx)
            }
            DemoSection::D3LineChart => {
                super::showcase_modules::d3_examples::line_chart::render(self, cx)
            }
            DemoSection::D3Streamgraph => {
                super::showcase_modules::d3_examples::streamgraph::render(self, cx)
            }
            DemoSection::D3StackedBar => {
                super::showcase_modules::d3_examples::stacked_bar::render(self, cx)
            }
            DemoSection::D3StackedArea => {
                super::showcase_modules::d3_examples::stacked_area::render(self, cx)
            }
            DemoSection::D3BoxPlot => {
                super::showcase_modules::d3_examples::box_plot::render(self, cx)
            }
            DemoSection::D3ChordDiagram => {
                super::showcase_modules::d3_examples::chord::render(self, cx)
            }
            DemoSection::D3ForceDirected => {
                super::showcase_modules::d3_examples::force_directed::render(self, cx)
            }
            DemoSection::D3ParallelSets => {
                super::showcase_modules::d3_examples::parallel_sets::render(self, cx)
            }
            DemoSection::D3DifferenceChart => {
                super::showcase_modules::d3_examples::difference_chart::render(self, cx)
            }
            DemoSection::D3Ridgeline => {
                super::showcase_modules::d3_examples::ridgeline::render(self, cx)
            }
            DemoSection::D3RealtimeHorizon => {
                super::showcase_modules::d3_examples::realtime_horizon::render(self, cx)
            }
            DemoSection::D3RadialTree => {
                super::showcase_modules::d3_examples::radial_tree::render(self, cx)
            }
            DemoSection::D3RadialCluster => {
                super::showcase_modules::d3_examples::radial_tree::render_cluster(self, cx)
            }
            DemoSection::D3CirclePacking => {
                super::showcase_modules::d3_examples::circle_packing::render(self, cx)
            }
            DemoSection::D3Sunburst => {
                super::showcase_modules::d3_examples::sunburst::render(self, cx)
            }
            DemoSection::D3VoronoiAirports => {
                super::showcase_modules::d3_examples::voronoi_airports::render(self, cx)
            }
            DemoSection::D3TemperatureTrends => {
                super::showcase_modules::d3_examples::temperature_trends::render(self, cx)
            }
            DemoSection::D3HertzsprungRussell => {
                super::showcase_modules::d3_examples::hertzsprung_russell::render(self, cx)
            }
            DemoSection::D3VoronoiLabels => {
                super::showcase_modules::d3_examples::voronoi_labels::render(self, cx)
            }
            DemoSection::D3ElectricUsage => {
                super::showcase_modules::d3_examples::electric_usage::render(self, cx)
            }
            DemoSection::D3StarMap => {
                super::showcase_modules::d3_examples::star_map::render(self, cx)
            }
            DemoSection::D3VoronoiStippling => {
                super::showcase_modules::d3_examples::voronoi_stippling::render(self, cx)
            }
        };

        let theme = cx.theme();
        let ds = cx.design();

        div()
            .id("content-scroll")
            .flex_1()
            .h_full()
            .overflow_y_scroll()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p(px(ds.spacing.section_gap * 2.0))
            .child(content)
    }

    /// Small in-UI overlay showing the last measured allocation deltas.
    pub(super) fn render_alloc_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let ds = cx.design();
        let render_ok = self.last_render_alloc.count == 0;
        let mouse_ok = self.last_mouse_move_alloc.count == 0;
        let last_ok = self.last_sample.as_ref().is_none_or(|(_, s)| s.count == 0);

        div()
            .id("alloc-overlay")
            .absolute()
            .top(px(ds.spacing.section_gap))
            .right(px(ds.spacing.section_gap))
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(if render_ok && mouse_ok && last_ok {
                theme.surface
            } else {
                theme.error
            })
            .text_color(if render_ok && mouse_ok && last_ok {
                theme.text_primary
            } else {
                theme.text_on_accent
            })
            .shadow_lg()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_size(px(ds.typography.small_size * 0.85))
                    .child(format!(
                        "render: {} bytes / {} allocs",
                        self.last_render_alloc.bytes, self.last_render_alloc.count
                    )),
            )
            .child(
                div()
                    .text_size(px(ds.typography.small_size * 0.85))
                    .child(format!(
                        "mouse: {} bytes / {} allocs",
                        self.last_mouse_move_alloc.bytes, self.last_mouse_move_alloc.count
                    )),
            )
            .children(self.last_sample.as_ref().map(|(label, snapshot)| {
                div()
                    .text_size(px(ds.typography.small_size * 0.85))
                    .child(format!(
                        "last ({label}): {} bytes / {} allocs",
                        snapshot.bytes, snapshot.count
                    ))
            }))
    }
}

impl Render for ShowcaseApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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

        let bounds = window.bounds();
        let w: f32 = bounds.size.width.into();
        let h: f32 = bounds.size.height.into();

        // Detect window resize and sample allocations triggered by it.
        let resized = self
            .last_window_size
            .is_some_and(|last| last != bounds.size);
        self.last_window_size = Some(bounds.size);
        if resized {
            self.record_sample("resize");
        }

        let sidebar_width = self.solve_layout(w, h);
        let ds = cx.design();
        self.content_width = (w - sidebar_width - ds.spacing.section_gap * 4.0).max(400.0);
        self.content_height = (h - ds.spacing.section_gap * 4.0).max(300.0);

        let result = div()
            .id("d3rs-showcase-root")
            .relative()
            .size_full()
            .flex()
            .flex_row()
            .on_mouse_move(cx.listener(|this, _event, _window, _cx| {
                this.last_mouse_move_alloc = this.record_sample("mouse-move");
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, _window, _cx| {
                    this.record_sample("mouse-down");
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _event, _window, _cx| {
                    this.record_sample("mouse-up");
                }),
            )
            .on_scroll_wheel(cx.listener(|this, _event, _window, _cx| {
                this.record_sample("scroll");
            }))
            .child(self.render_sidebar(sidebar_width, cx))
            .child(self.render_content(cx))
            .child(self.render_alloc_overlay(cx));

        self.last_render_alloc = self.record_sample("render");
        result
    }
}
